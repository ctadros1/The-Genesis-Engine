//! C13.8-C13.10's per-world reductions: repeated-association communities,
//! their persistence against a proximity-matched null, and the directed-
//! aggression ledger (`lifesim-communities-v1`).
//!
//! C13.8 asks whether repeated-association networks "show communities that
//! persist beyond the stated window, and are not explained by spatial
//! proximity alone". Every quantity here is computed from campaign
//! artifacts through the same fail-closed identity join every other
//! Phase 13 detector uses, and nothing decides a criterion: the
//! pre-registered addendum reads these numbers (ADR-0016; the plan's own
//! rule that communities are defined offline and never enter the
//! simulation).
//!
//! - **Association**: two organisms are associated in a window (a fixed
//!   count of consecutive spatial samples) when they were within
//!   `association_radius_fp` of each other in at least `edge_min_count`
//!   of the window's samples. The plan says networks "built from the
//!   event log"; positions are not evented by design, so as built the
//!   association instrument is the spatial series - the same bounded
//!   record the exposure and arrival detectors read.
//! - **Communities**: connected components of the kept edges
//!   (deterministic union-find, labels by first appearance in ascending
//!   ID order), of at least `min_community` members.
//! - **Persistence**: a community chain is a sequence of communities in
//!   consecutive windows linked by Jaccard overlap of at least
//!   `persistence_jaccard_milli` (inclusive), matched greedily in
//!   ascending order of each community's smallest member ID with ties
//!   broken the same way; a chain of at least `min_chain_windows`
//!   windows is persistent.
//! - **The proximity-matched null**: within each window, organism
//!   identities are permuted among organisms sharing a home quadrat
//!   (their modal quadrat over that window's samples), by Fisher-Yates
//!   on the kernel's `Analysis` stream keyed on the recorded analysis
//!   seed. The permutation preserves every window's spatial structure
//!   exactly - the same bodies stand in the same places - while
//!   destroying individual identity continuity beyond what home-range
//!   locality provides. Real persistent chains are compared against the
//!   p95 of the shuffled chain counts; a detector that finds
//!   "communities" in the shuffles is finding home ranges, and this null
//!   is what catches it (the era-and-tradition spec's null-control
//!   discipline).
//! - **Aggression (C13.9/C13.10, descriptive)**: `Damage` events land in
//!   the window covering their tick. An attack is *within* when attacker
//!   and target belong to the same community there, *between* when they
//!   belong to different ones, *unaffiliated* otherwise - each counted,
//!   with opportunity-pair denominators (co-membership pairs within,
//!   cross-community pairs between) so the rates are per-million-pairs
//!   and comparable. Local numerical advantage compares each side's
//!   community headcount within the association radius at the window's
//!   nearest sample; coalitions are targets attacked by two or more
//!   distinct attackers in one window. C13.10's expected nulls are
//!   stated in the plan; these are the numbers they are read against.

use crate::arrival::ArrivalError;
use crate::fidelity::spatial_positions_by_id;
use sim_core::{Event, EventKind, RngSystem, named_random};
use sim_persist::SpatialSample;
use std::collections::{BTreeMap, BTreeSet};

pub const COMMUNITIES_VERSION: &str = "lifesim-communities-v1";

/// Analysis parameters, named so a report can echo every one.
#[derive(Clone, Copy, Debug)]
pub struct CommunityPlan {
    pub founder_count: u32,
    /// Association radius in the samples' fixed-point frame.
    pub association_radius_fp: i32,
    /// Consecutive spatial samples per window (a trailing partial window
    /// is dropped, counted in `samples_dropped`).
    pub window_samples: usize,
    /// Minimum co-occurrence count for an association edge.
    pub edge_min_count: u32,
    /// Minimum members for a component to be a community.
    pub min_community: usize,
    /// Jaccard threshold for chain links, milli, inclusive.
    pub persistence_jaccard_milli: i64,
    /// Windows a chain must span to count as persistent.
    pub min_chain_windows: usize,
    /// Proximity-matched permutations for the null.
    pub shuffles: u32,
    pub analysis_seed: u64,
    /// Quadrat geometry for home-quadrat matching in the shuffle.
    pub cells_x: u32,
    pub cells_y: u32,
    pub cell_size_fp: i32,
    pub quadrat_cells: u32,
}

/// One world's community and aggression summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldCommunities {
    pub windows: usize,
    pub samples_dropped: usize,
    pub communities_total: usize,
    /// Persistent chains in the real data.
    pub persistent_chains: usize,
    /// The p95 of persistent-chain counts over the shuffles, and the max.
    pub null_p95_chains: usize,
    pub null_max_chains: usize,
    pub shuffles: u32,
    // C13.9/C13.10 descriptive ledger.
    pub attacks_total: u64,
    pub attacks_within: u64,
    pub attacks_between: u64,
    pub attacks_unaffiliated: u64,
    pub attacks_outside_windows: u64,
    pub pairs_within: u64,
    pub pairs_between: u64,
    /// Attacks per million opportunity pairs; zero denominators report 0.
    pub within_rate_micro: i64,
    pub between_rate_micro: i64,
    pub attacks_with_local_advantage: u64,
    pub attacks_with_local_disadvantage: u64,
    pub attacks_even: u64,
    /// Targets attacked by two or more distinct attackers in one window.
    pub coalition_targets: u64,
    pub attacked_targets: u64,
}

fn quadrat_of(x_fp: i32, y_fp: i32, plan: &CommunityPlan) -> u32 {
    let cell_x = (x_fp / plan.cell_size_fp)
        .min(plan.cells_x as i32 - 1)
        .max(0) as u32;
    let cell_y = (y_fp / plan.cell_size_fp)
        .min(plan.cells_y as i32 - 1)
        .max(0) as u32;
    let quadrats_x = plan.cells_x.div_ceil(plan.quadrat_cells);
    (cell_y / plan.quadrat_cells) * quadrats_x + cell_x / plan.quadrat_cells
}

fn find(parent: &mut [usize], index: usize) -> usize {
    let mut root = index;
    while parent[root] != root {
        root = parent[root];
    }
    let mut walk = index;
    while parent[walk] != root {
        let next = parent[walk];
        parent[walk] = root;
        walk = next;
    }
    root
}

/// Pairs within `radius` at one sample, by grid buckets (bounded work at
/// campaign densities; the naive quadratic scan is exactly what the
/// kernel's own spatial index exists to avoid). Shared with the fidelity
/// detector, whose exposure enumeration is the same computation.
pub(crate) fn close_pairs(
    positions: &BTreeMap<u64, (i32, i32)>,
    radius_fp: i32,
    bucket_fp: i32,
) -> Vec<(u64, u64)> {
    let mut buckets: BTreeMap<(i32, i32), Vec<u64>> = BTreeMap::new();
    for (&id, &(x, y)) in positions {
        buckets
            .entry((x / bucket_fp, y / bucket_fp))
            .or_default()
            .push(id);
    }
    let reach = (radius_fp / bucket_fp) + 1;
    let radius_squared = i64::from(radius_fp) * i64::from(radius_fp);
    let mut pairs = Vec::new();
    for (&(bx, by), members) in &buckets {
        for &a in members {
            let &(ax, ay) = &positions[&a];
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let Some(neighbours) = buckets.get(&(bx + dx, by + dy)) else {
                        continue;
                    };
                    for &b in neighbours {
                        if b <= a {
                            continue;
                        }
                        let &(bx_fp, by_fp) = &positions[&b];
                        let ddx = i64::from(ax) - i64::from(bx_fp);
                        let ddy = i64::from(ay) - i64::from(by_fp);
                        if ddx * ddx + ddy * ddy <= radius_squared {
                            pairs.push((a, b));
                        }
                    }
                }
            }
        }
    }
    pairs
}

/// One window's communities: member sets, each of at least
/// `min_community`, ordered by smallest member ID.
fn window_communities(
    samples: &[(u64, BTreeMap<u64, (i32, i32)>)],
    plan: &CommunityPlan,
) -> Vec<BTreeSet<u64>> {
    let mut counts: BTreeMap<(u64, u64), u32> = BTreeMap::new();
    for (_, positions) in samples {
        for pair in close_pairs(positions, plan.association_radius_fp, plan.cell_size_fp) {
            *counts.entry(pair).or_default() += 1;
        }
    }
    let ids: Vec<u64> = BTreeSet::from_iter(
        counts
            .iter()
            .filter(|entry| *entry.1 >= plan.edge_min_count)
            .flat_map(|(&(a, b), _)| [a, b]),
    )
    .into_iter()
    .collect();
    let index_of: BTreeMap<u64, usize> = ids
        .iter()
        .enumerate()
        .map(|(index, &id)| (id, index))
        .collect();
    let mut parent: Vec<usize> = (0..ids.len()).collect();
    for (&(a, b), &count) in &counts {
        if count >= plan.edge_min_count {
            let (ra, rb) = (
                find(&mut parent, index_of[&a]),
                find(&mut parent, index_of[&b]),
            );
            if ra != rb {
                let (lo, hi) = (ra.min(rb), ra.max(rb));
                parent[hi] = lo;
            }
        }
    }
    let mut members: BTreeMap<usize, BTreeSet<u64>> = BTreeMap::new();
    for (index, &id) in ids.iter().enumerate() {
        members
            .entry(find(&mut parent, index))
            .or_default()
            .insert(id);
    }
    members
        .into_values()
        .filter(|community| community.len() >= plan.min_community)
        .collect()
}

fn jaccard_milli(a: &BTreeSet<u64>, b: &BTreeSet<u64>) -> i64 {
    let intersection = a.intersection(b).count() as i64;
    let union = (a.len() + b.len()) as i64 - intersection;
    if union == 0 {
        0
    } else {
        intersection * 1000 / union
    }
}

/// Persistent-chain count over a sequence of windows' community lists.
fn persistent_chains(windows: &[Vec<BTreeSet<u64>>], plan: &CommunityPlan) -> usize {
    let mut chains = 0_usize;
    let mut lengths: Vec<Vec<usize>> = Vec::with_capacity(windows.len());
    for (w, communities) in windows.iter().enumerate() {
        let mut current = vec![1_usize; communities.len()];
        if w > 0 {
            let previous = &windows[w - 1];
            let mut taken = vec![false; communities.len()];
            for (p, community) in previous.iter().enumerate() {
                // Best successor by Jaccard; the first maximum wins, and
                // `communities` is already ordered by smallest member.
                let mut best: Option<(i64, usize)> = None;
                for (c, candidate) in communities.iter().enumerate() {
                    if taken[c] {
                        continue;
                    }
                    let score = jaccard_milli(community, candidate);
                    if score >= plan.persistence_jaccard_milli
                        && best.map_or(true, |(best_score, _)| score > best_score)
                    {
                        best = Some((score, c));
                    }
                }
                if let Some((_, c)) = best {
                    taken[c] = true;
                    current[c] = lengths[w - 1][p] + 1;
                } else if lengths[w - 1][p] >= plan.min_chain_windows {
                    // A chain that was long enough ends here.
                    chains += 1;
                }
            }
        }
        lengths.push(current);
    }
    if let Some(last) = lengths.last() {
        chains += last
            .iter()
            .filter(|&&length| length >= plan.min_chain_windows)
            .count();
    }
    chains
}

/// Reduce one world's artifacts to its community and aggression summary.
pub fn world_communities(
    events: &[Event],
    spatial: &[SpatialSample],
    plan: &CommunityPlan,
) -> Result<WorldCommunities, ArrivalError> {
    let mut summary = WorldCommunities {
        shuffles: plan.shuffles,
        ..WorldCommunities::default()
    };
    let positions = spatial_positions_by_id(events, spatial, plan.founder_count)?;
    let window_count = positions.len() / plan.window_samples;
    summary.samples_dropped = positions.len() - window_count * plan.window_samples;
    summary.windows = window_count;

    let mut windows: Vec<Vec<BTreeSet<u64>>> = Vec::with_capacity(window_count);
    let mut window_ranges: Vec<(u64, u64)> = Vec::with_capacity(window_count);
    let mut home_quadrats: Vec<BTreeMap<u64, u32>> = Vec::with_capacity(window_count);
    for w in 0..window_count {
        let samples = &positions[w * plan.window_samples..(w + 1) * plan.window_samples];
        window_ranges.push((samples[0].0, samples[samples.len() - 1].0));
        let communities = window_communities(samples, plan);
        summary.communities_total += communities.len();
        // Home quadrat: modal over the window's samples, ties to the
        // smaller quadrat index.
        let mut tallies: BTreeMap<u64, BTreeMap<u32, u32>> = BTreeMap::new();
        for (_, sample_positions) in samples {
            for (&id, &(x, y)) in sample_positions {
                *tallies
                    .entry(id)
                    .or_default()
                    .entry(quadrat_of(x, y, plan))
                    .or_default() += 1;
            }
        }
        home_quadrats.push(
            tallies
                .into_iter()
                .map(|(id, counts)| {
                    let (&quadrat, _) = counts
                        .iter()
                        .max_by_key(|entry| (*entry.1, std::cmp::Reverse(*entry.0)))
                        .expect("at least one sample");
                    (id, quadrat)
                })
                .collect(),
        );
        windows.push(communities);
    }

    summary.persistent_chains = persistent_chains(&windows, plan);

    // The proximity-matched null.
    let mut null_counts: Vec<usize> = Vec::with_capacity(plan.shuffles as usize);
    for shuffle in 0..plan.shuffles {
        let mut relabeled: Vec<Vec<BTreeSet<u64>>> = Vec::with_capacity(window_count);
        for (w, communities) in windows.iter().enumerate() {
            let mut groups: BTreeMap<u32, Vec<u64>> = BTreeMap::new();
            for (&id, &quadrat) in &home_quadrats[w] {
                groups.entry(quadrat).or_default().push(id);
            }
            let mut mapping: BTreeMap<u64, u64> = BTreeMap::new();
            for (quadrat, ids) in &groups {
                let mut shuffled = ids.clone();
                let mut draw_index = 0_u32;
                for index in (1..shuffled.len()).rev() {
                    let draw = named_random(
                        plan.analysis_seed,
                        u64::from(shuffle),
                        RngSystem::Analysis,
                        (u64::from(*quadrat) << 32) | w as u64,
                        draw_index,
                    );
                    draw_index += 1;
                    let other = (draw % (index as u64 + 1)) as usize;
                    shuffled.swap(index, other);
                }
                for (&from, &to) in ids.iter().zip(shuffled.iter()) {
                    mapping.insert(from, to);
                }
            }
            relabeled.push(
                communities
                    .iter()
                    .map(|community| {
                        community
                            .iter()
                            .map(|id| *mapping.get(id).unwrap_or(id))
                            .collect()
                    })
                    .collect(),
            );
        }
        null_counts.push(persistent_chains(&relabeled, plan));
    }
    null_counts.sort_unstable();
    summary.null_p95_chains = if null_counts.is_empty() {
        0
    } else {
        null_counts[(null_counts.len() * 95).div_ceil(100).saturating_sub(1)]
    };
    summary.null_max_chains = null_counts.last().copied().unwrap_or(0);

    // The aggression ledger.
    let memberships: Vec<BTreeMap<u64, usize>> = windows
        .iter()
        .map(|communities| {
            communities
                .iter()
                .enumerate()
                .flat_map(|(index, community)| community.iter().map(move |&id| (id, index)))
                .collect()
        })
        .collect();
    for communities in &windows {
        let sizes: Vec<u64> = communities.iter().map(|c| c.len() as u64).collect();
        summary.pairs_within += sizes.iter().map(|&s| s * (s - 1) / 2).sum::<u64>();
        for (i, &a) in sizes.iter().enumerate() {
            for &b in &sizes[i + 1..] {
                summary.pairs_between += a * b;
            }
        }
    }

    let mut attackers_of: BTreeMap<(usize, u64), BTreeSet<u64>> = BTreeMap::new();
    for event in events {
        let EventKind::Damage {
            attacker, target, ..
        } = event.kind
        else {
            continue;
        };
        summary.attacks_total += 1;
        let Some(w) = window_ranges
            .iter()
            .position(|&(first, last)| event.tick >= first && event.tick <= last)
        else {
            summary.attacks_outside_windows += 1;
            continue;
        };
        attackers_of
            .entry((w, target))
            .or_default()
            .insert(attacker);
        let membership = &memberships[w];
        match (membership.get(&attacker), membership.get(&target)) {
            (Some(&a), Some(&b)) if a == b => summary.attacks_within += 1,
            (Some(&a), Some(&b)) => {
                summary.attacks_between += 1;
                let samples = &positions[w * plan.window_samples..(w + 1) * plan.window_samples];
                let (_, nearest) = samples
                    .iter()
                    .min_by_key(|(tick, _)| tick.abs_diff(event.tick))
                    .expect("windows are non-empty");
                let count_side = |centre: u64, community: usize| -> u64 {
                    let Some(&(cx, cy)) = nearest.get(&centre) else {
                        return 0;
                    };
                    let radius = i64::from(plan.association_radius_fp);
                    windows[w][community]
                        .iter()
                        .filter(|&&member| {
                            nearest.get(&member).is_some_and(|&(x, y)| {
                                let dx = i64::from(x) - i64::from(cx);
                                let dy = i64::from(y) - i64::from(cy);
                                dx * dx + dy * dy <= radius * radius
                            })
                        })
                        .count() as u64
                };
                let attacker_side = count_side(attacker, a);
                let target_side = count_side(target, b);
                match attacker_side.cmp(&target_side) {
                    std::cmp::Ordering::Greater => summary.attacks_with_local_advantage += 1,
                    std::cmp::Ordering::Less => summary.attacks_with_local_disadvantage += 1,
                    std::cmp::Ordering::Equal => summary.attacks_even += 1,
                }
            }
            _ => summary.attacks_unaffiliated += 1,
        }
    }
    summary.attacked_targets = attackers_of
        .keys()
        .map(|&(_, target)| target)
        .collect::<BTreeSet<_>>()
        .len() as u64;
    summary.coalition_targets = attackers_of
        .values()
        .filter(|attackers| attackers.len() >= 2)
        .count() as u64;
    summary.within_rate_micro = if summary.pairs_within == 0 {
        0
    } else {
        (summary.attacks_within as i128 * 1_000_000 / summary.pairs_within as i128) as i64
    };
    summary.between_rate_micro = if summary.pairs_between == 0 {
        0
    } else {
        (summary.attacks_between as i128 * 1_000_000 / summary.pairs_between as i128) as i64
    };
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::DeathCause;

    fn base_plan() -> CommunityPlan {
        CommunityPlan {
            founder_count: 8,
            association_radius_fp: 1_000,
            window_samples: 2,
            edge_min_count: 2,
            min_community: 2,
            persistence_jaccard_milli: 300,
            min_chain_windows: 2,
            shuffles: 20,
            analysis_seed: 0x1373_0004,
            cells_x: 8,
            cells_y: 8,
            cell_size_fp: 4_000,
            quadrat_cells: 8,
        }
    }

    fn sample(tick: u64, positions: Vec<(i32, i32)>) -> SpatialSample {
        SpatialSample { tick, positions }
    }

    fn damage(tick: u64, attacker: u64, target: u64) -> Event {
        Event {
            tick,
            kind: EventKind::Damage {
                attacker,
                target,
                raw_milli: 100,
                applied_milli: 100,
                health_milli: 500,
            },
        }
    }

    /// Two tight groups in ONE quadrat (so the shuffle can mix them),
    /// stable across two windows: two real persistent chains, and a null
    /// that breaks them - the permutation swaps members across the two
    /// communities while every body stays where it stood.
    #[test]
    fn persistent_communities_beat_the_proximity_matched_null() {
        let near = [(0, 0), (100, 0), (0, 100), (100, 100)];
        let far = [
            (3_000, 3_000),
            (3_100, 3_000),
            (3_000, 3_100),
            (3_100, 3_100),
        ];
        let layout: Vec<(i32, i32)> = near.iter().chain(far.iter()).copied().collect();
        let spatial: Vec<SpatialSample> = (0..4)
            .map(|index| sample(index * 50, layout.clone()))
            .collect();
        // Threshold 800: only an identity relabeling keeps a chain (a
        // 4-member community needs full overlap to clear it), so the
        // within-quadrat shuffle across the two co-quadrat communities
        // breaks chains almost surely while the real data's Jaccard-1000
        // links are untouched. At a lax threshold a random 4-of-8
        // relabel keeps Jaccard ~333 and the null would legitimately
        // tie - that is the null doing its job, not a failure.
        let mut plan = base_plan();
        plan.persistence_jaccard_milli = 800;
        let summary = world_communities(&[], &spatial, &plan).expect("join holds");
        assert_eq!(summary.windows, 2);
        assert_eq!(summary.communities_total, 4, "{summary:?}");
        assert_eq!(summary.persistent_chains, 2, "{summary:?}");
        assert!(
            summary.null_p95_chains < summary.persistent_chains,
            "{summary:?}"
        );
    }

    /// Communities in different quadrats are immune to the shuffle by
    /// construction (a within-quadrat permutation maps each community to
    /// itself when the community IS its quadrat's population): the null
    /// equals the real count, and the classifier reports exactly that -
    /// spatial locality alone explains such "communities".
    #[test]
    fn a_community_that_is_its_quadrat_is_explained_by_proximity() {
        let mut plan = base_plan();
        plan.quadrat_cells = 1;
        let near = [(0, 0), (100, 0), (0, 100), (100, 100)];
        let far = [
            (20_000, 20_000),
            (20_100, 20_000),
            (20_000, 20_100),
            (20_100, 20_100),
        ];
        let layout: Vec<(i32, i32)> = near.iter().chain(far.iter()).copied().collect();
        let spatial: Vec<SpatialSample> = (0..4)
            .map(|index| sample(index * 50, layout.clone()))
            .collect();
        let summary = world_communities(&[], &spatial, &plan).expect("join holds");
        assert_eq!(summary.persistent_chains, 2);
        assert_eq!(summary.null_p95_chains, 2, "{summary:?}");
    }

    /// The Jaccard link is inclusive at the threshold: a community of 4
    /// replacing one member has Jaccard 3/5 = 600; at threshold 600 the
    /// chain holds, at 601 it breaks.
    #[test]
    fn the_persistence_link_is_inclusive_at_the_jaccard_threshold() {
        let near_a = [(0, 0), (100, 0), (0, 100), (100, 100)];
        let mut layout_one: Vec<(i32, i32)> = near_a.to_vec();
        layout_one.push((30_000, 30_000));
        let mut layout_two: Vec<(i32, i32)> = near_a[..3].to_vec();
        layout_two.push((30_000, 30_000));
        layout_two.push((100, 100));
        let spatial = vec![
            sample(0, layout_one.clone()),
            sample(50, layout_one),
            sample(100, layout_two.clone()),
            sample(150, layout_two),
        ];
        for (threshold, chains) in [(600_i64, 1_usize), (601, 0)] {
            let mut plan = base_plan();
            plan.founder_count = 5;
            plan.persistence_jaccard_milli = threshold;
            plan.shuffles = 0;
            let summary = world_communities(&[], &spatial, &plan).expect("join holds");
            assert_eq!(summary.persistent_chains, chains, "threshold {threshold}");
        }
    }

    /// The aggression ledger on a scripted layout: two communities, one
    /// within-attack, two between-attacks on one target (a coalition),
    /// one unaffiliated attacker, exact opportunity denominators.
    #[test]
    fn the_aggression_ledger_counts_within_between_and_coalitions_exactly() {
        let near = [(0, 0), (100, 0), (0, 100), (100, 100)];
        let far = [
            (3_000, 3_000),
            (3_100, 3_000),
            (3_000, 3_100),
            (3_100, 3_100),
        ];
        let mut layout: Vec<(i32, i32)> = near.iter().chain(far.iter()).copied().collect();
        layout.push((16_000, 16_000)); // organism 9, alone mid-map.
        let spatial: Vec<SpatialSample> = (0..2)
            .map(|index| sample(index * 50, layout.clone()))
            .collect();
        let events = vec![
            damage(10, 1, 2),
            damage(20, 1, 5),
            damage(30, 2, 5),
            damage(40, 9, 1),
        ];
        let mut plan = base_plan();
        plan.founder_count = 9;
        plan.shuffles = 0;
        let summary = world_communities(&events, &spatial, &plan).expect("join holds");
        assert_eq!(summary.windows, 1);
        assert_eq!(summary.attacks_total, 4);
        assert_eq!(summary.attacks_within, 1);
        assert_eq!(summary.attacks_between, 2);
        assert_eq!(summary.attacks_unaffiliated, 1);
        assert_eq!(summary.pairs_within, 12);
        assert_eq!(summary.pairs_between, 16);
        assert_eq!(summary.within_rate_micro, 1_000_000 / 12);
        assert_eq!(summary.between_rate_micro, 125_000);
        assert_eq!(summary.coalition_targets, 1);
        assert_eq!(summary.attacked_targets, 3);
        assert_eq!(summary.attacks_even, 2);
    }

    /// An attack outside every window is counted as such, never guessed
    /// into one; and the join still fails closed on a population
    /// mismatch.
    #[test]
    fn attacks_outside_windows_are_counted_and_the_join_fails_closed() {
        let layout = vec![(0, 0), (100, 0)];
        let spatial = vec![sample(100, layout.clone()), sample(150, layout)];
        let events = vec![damage(500, 1, 2)];
        let mut plan = base_plan();
        plan.founder_count = 2;
        plan.shuffles = 0;
        let summary = world_communities(&events, &spatial, &plan).expect("join holds");
        assert_eq!(summary.attacks_outside_windows, 1);

        let events = vec![Event {
            tick: 50,
            kind: EventKind::Death {
                id: 1,
                cause: DeathCause::Starvation,
            },
        }];
        let torn = vec![sample(100, vec![(0, 0), (100, 0)])];
        assert!(world_communities(&events, &torn, &plan).is_err());
    }
}
