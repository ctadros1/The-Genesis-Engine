//! C13.3's per-world reduction: traditions that outlive individuals, with
//! the genotype-matched control (`lifesim-tradition-v1`).
//!
//! `specifications/era-and-tradition-detection.md` fixes what a tradition
//! claim requires, and this module computes exactly those four things and
//! nothing more:
//!
//! 1. **A behavioral variant**: a cluster over per-organism action mixes
//!    (the seven-class census histogram over one action window,
//!    normalized to milli), formed by the same deterministic
//!    threshold-and-union-find method `lifesim-similarity-v1` uses -
//!    single linkage under an L1 threshold, labels assigned by first
//!    appearance in a stable order. The spec says "event-derived"
//!    histograms; as built the histograms come from the action census
//!    series, which is the same quantity recorded bounded (per-action
//!    events do not exist by design - action is censused, not evented).
//! 2. **Local concentration**: the variant's frequency inside a quadrat
//!    neighbourhood exceeds its global frequency by a stated factor, at
//!    both endpoints.
//! 3. **Persistence beyond individuals**: the two endpoints are separated
//!    by more than three times the run's own median completed lifespan
//!    (from the demography reduction over the same event log), and no
//!    individual is a neighbourhood member at both endpoints.
//! 4. **The genotype-matched control**: the variant's endpoint frequency
//!    must exceed its frequency in a cohort of clustered organisms
//!    *outside* the neighbourhood whose mean pedigree relatedness to the
//!    neighbourhood is within a stated tolerance of the neighbourhood's
//!    own internal mean. Genetic distance is pedigree kinship
//!    ([`crate::fidelity::Pedigree`], exact Q32) for the reason recorded
//!    there: genomes exist only at the horizon, the pedigree covers
//!    everyone ever born. A candidate with an empty cohort is counted as
//!    uncontrolled, never reported as a tradition - the spec's rule that
//!    a reader must be able to judge whether the control had power.
//!
//! Every candidate that fails is counted by the reason it failed
//! (silence is not a result), and nothing here decides a criterion: the
//! 15-of-30 reading belongs to the pre-registered campaign analysis
//! (ADR-0022 A5, ADR-0016).

use crate::arrival::ArrivalError;
use crate::demography::world_demography;
use crate::fidelity::Pedigree;
use sim_core::Event;
use sim_persist::{ActionSampleSet, SpatialSample};
use std::collections::{BTreeMap, BTreeSet};

pub const TRADITION_VERSION: &str = "lifesim-tradition-v1";

/// Analysis parameters, named so a report can echo every one (the spec's
/// report-contents rule).
#[derive(Clone, Copy, Debug)]
pub struct TraditionPlan {
    pub founder_count: u32,
    /// World geometry for the quadrat map, in the samples' frame.
    pub cells_x: u32,
    pub cells_y: u32,
    pub cell_size_fp: i32,
    /// Quadrat side in cells; one quadrat is one neighbourhood.
    pub quadrat_cells: u32,
    /// Variant clustering: mixes within this L1 distance (milli) are the
    /// same variant (single linkage, inclusive at the threshold).
    pub cluster_threshold_milli: i64,
    /// Local frequency must exceed global frequency times this factor in
    /// milli (2000 = twice the global frequency).
    pub concentration_factor_milli: i64,
    /// Minimum neighbourhood size at each endpoint.
    pub min_neighbourhood: usize,
    /// Control-cohort matching tolerance on mean relatedness, Q32.
    pub match_tolerance_q32: u64,
}

/// One (quadrat, variant) tradition finding, with its control beside it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraditionFinding {
    pub quadrat: u32,
    pub variant: u32,
    pub start_tick: u64,
    pub end_tick: u64,
    pub neighbourhood_start: usize,
    pub neighbourhood_end: usize,
    pub freq_start_milli: i64,
    pub freq_end_milli: i64,
    pub global_freq_start_milli: i64,
    pub global_freq_end_milli: i64,
    /// The genotype-matched control statistic, its cohort size, and the
    /// tolerance that built it - the three numbers the spec requires in
    /// every finding.
    pub control_cohort: usize,
    pub control_freq_milli: i64,
    pub match_tolerance_q32: u64,
}

/// One world's tradition summary. Rejection counters are per candidate
/// (a concentrated (quadrat, variant) at some start endpoint).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldTraditions {
    pub median_lifespan_ticks: u64,
    /// The realized persistence requirement: strictly more than three
    /// median lifespans, in ticks.
    pub persistence_gap_ticks: u64,
    pub endpoints_evaluated: u64,
    pub candidates: u64,
    pub rejected_end_concentration: u64,
    pub rejected_turnover: u64,
    pub rejected_no_cohort: u64,
    pub rejected_control: u64,
    /// Findings that passed all four requirements, deduplicated to the
    /// earliest start endpoint per (quadrat, variant).
    pub findings: Vec<TraditionFinding>,
}

/// Deterministic single-linkage clustering under an inclusive L1
/// threshold: the `lifesim-similarity-v1` method over action mixes.
/// Labels are assigned by first appearance in input order and are dense.
fn cluster_mixes(mixes: &[[i64; 7]], threshold_milli: i64) -> Vec<u32> {
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
    let mut parent: Vec<usize> = (0..mixes.len()).collect();
    for a in 0..mixes.len() {
        for b in (a + 1)..mixes.len() {
            let l1: i64 = mixes[a]
                .iter()
                .zip(mixes[b].iter())
                .map(|(x, y)| (x - y).abs())
                .sum();
            if l1 <= threshold_milli {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    // Union toward the smaller index so labels follow
                    // first appearance.
                    let (lo, hi) = (ra.min(rb), ra.max(rb));
                    parent[hi] = lo;
                }
            }
        }
    }
    let mut label_of_root: BTreeMap<usize, u32> = BTreeMap::new();
    let mut labels = Vec::with_capacity(mixes.len());
    for index in 0..mixes.len() {
        let root = find(&mut parent, index);
        let next = label_of_root.len() as u32;
        labels.push(*label_of_root.entry(root).or_insert(next));
    }
    labels
}

/// Per-organism mixes for the action window ending at `sample_index`,
/// milli-normalized; organisms with no actions or no prior row are absent.
fn window_mixes(actions: &[ActionSampleSet], sample_index: usize) -> BTreeMap<u64, [i64; 7]> {
    let mut mixes = BTreeMap::new();
    if sample_index == 0 {
        return mixes;
    }
    let previous: BTreeMap<u64, &[u32; 7]> = actions[sample_index - 1]
        .records
        .iter()
        .map(|record| (record.id, &record.counts))
        .collect();
    for record in &actions[sample_index].records {
        let Some(&was) = previous.get(&record.id) else {
            continue;
        };
        let mut delta = [0_u64; 7];
        let mut total = 0_u64;
        for (slot, (&now, &before)) in delta.iter_mut().zip(record.counts.iter().zip(was.iter())) {
            *slot = u64::from(now.saturating_sub(before));
            total += *slot;
        }
        if total == 0 {
            continue;
        }
        let mut mix = [0_i64; 7];
        for (out, &count) in mix.iter_mut().zip(delta.iter()) {
            *out = (count * 1000 / total) as i64;
        }
        mixes.insert(record.id, mix);
    }
    mixes
}

fn quadrat_of(x_fp: i32, y_fp: i32, plan: &TraditionPlan) -> u32 {
    let cell_x = (x_fp / plan.cell_size_fp)
        .min(plan.cells_x as i32 - 1)
        .max(0) as u32;
    let cell_y = (y_fp / plan.cell_size_fp)
        .min(plan.cells_y as i32 - 1)
        .max(0) as u32;
    let quadrats_x = plan.cells_x.div_ceil(plan.quadrat_cells);
    (cell_y / plan.quadrat_cells) * quadrats_x + cell_x / plan.quadrat_cells
}

/// Frequency of `variant` among `members`, milli. Zero for empty.
fn frequency_milli(members: &[(u64, u32)], variant: u32) -> i64 {
    if members.is_empty() {
        return 0;
    }
    let carriers = members
        .iter()
        .filter(|&&(_, label)| label == variant)
        .count();
    (carriers as i64) * 1000 / members.len() as i64
}

/// Reduce one world's artifacts to its tradition summary.
pub fn world_traditions(
    events: &[Event],
    actions: &[ActionSampleSet],
    spatial: &[SpatialSample],
    plan: &TraditionPlan,
) -> Result<WorldTraditions, ArrivalError> {
    let mut summary = WorldTraditions {
        median_lifespan_ticks: world_demography(events).median_lifespan_ticks,
        ..WorldTraditions::default()
    };
    if summary.median_lifespan_ticks == 0 {
        // No completed lifespan: the persistence requirement is undefined
        // and the world is reported as such, not scanned with a zero gap.
        return Ok(summary);
    }
    summary.persistence_gap_ticks = summary.median_lifespan_ticks * 3 + 1;

    let pedigree = Pedigree::from_events(events);
    let positions = crate::fidelity::spatial_positions_by_id(events, spatial, plan.founder_count)?;
    let position_at = |tick: u64, id: u64| -> Option<(i32, i32)> {
        positions
            .iter()
            .find(|(sample_tick, _)| *sample_tick == tick)
            .and_then(|(_, map)| map.get(&id).copied())
    };

    let mut found: BTreeMap<(u32, u32), TraditionFinding> = BTreeMap::new();

    for start_index in 1..actions.len() {
        let start_tick = actions[start_index].tick;
        let Some(end_index) = (start_index + 1..actions.len())
            .find(|&index| actions[index].tick >= start_tick + summary.persistence_gap_ticks)
        else {
            continue;
        };
        let end_tick = actions[end_index].tick;
        summary.endpoints_evaluated += 1;

        let start_mixes = window_mixes(actions, start_index);
        let end_mixes = window_mixes(actions, end_index);

        // Joint clustering across both endpoints, so "the same variant"
        // is one label space. Order: start rows ascending id, then end
        // rows ascending id.
        let joint: Vec<(u64, bool, [i64; 7])> = start_mixes
            .iter()
            .map(|(&id, &mix)| (id, false, mix))
            .chain(end_mixes.iter().map(|(&id, &mix)| (id, true, mix)))
            .collect();
        let labels = cluster_mixes(
            &joint.iter().map(|&(_, _, mix)| mix).collect::<Vec<_>>(),
            plan.cluster_threshold_milli,
        );

        let mut start_members: BTreeMap<u32, Vec<(u64, u32)>> = BTreeMap::new();
        let mut end_members: BTreeMap<u32, Vec<(u64, u32)>> = BTreeMap::new();
        let mut end_all: Vec<(u64, u32, u32)> = Vec::new();
        let mut start_global: Vec<(u64, u32)> = Vec::new();
        let mut end_global: Vec<(u64, u32)> = Vec::new();
        for (&(id, is_end, _), &label) in joint.iter().zip(labels.iter()) {
            let tick = if is_end { end_tick } else { start_tick };
            let Some((x_fp, y_fp)) = position_at(tick, id) else {
                continue;
            };
            let quadrat = quadrat_of(x_fp, y_fp, plan);
            if is_end {
                end_members.entry(quadrat).or_default().push((id, label));
                end_all.push((id, label, quadrat));
                end_global.push((id, label));
            } else {
                start_members.entry(quadrat).or_default().push((id, label));
                start_global.push((id, label));
            }
        }

        for (&quadrat, members_start) in &start_members {
            if members_start.len() < plan.min_neighbourhood {
                continue;
            }
            let variants: BTreeSet<u32> = members_start.iter().map(|&(_, label)| label).collect();
            for &variant in &variants {
                let freq_start = frequency_milli(members_start, variant);
                let global_start = frequency_milli(&start_global, variant);
                if freq_start * 1000 <= plan.concentration_factor_milli * global_start {
                    continue;
                }
                summary.candidates += 1;

                let Some(members_end) = end_members.get(&quadrat) else {
                    summary.rejected_end_concentration += 1;
                    continue;
                };
                if members_end.len() < plan.min_neighbourhood {
                    summary.rejected_end_concentration += 1;
                    continue;
                }
                let freq_end = frequency_milli(members_end, variant);
                let global_end = frequency_milli(&end_global, variant);
                if freq_end * 1000 <= plan.concentration_factor_milli * global_end {
                    summary.rejected_end_concentration += 1;
                    continue;
                }

                // Turnover: no individual in the neighbourhood at both
                // endpoints.
                let start_ids: BTreeSet<u64> = members_start.iter().map(|&(id, _)| id).collect();
                if members_end.iter().any(|&(id, _)| start_ids.contains(&id)) {
                    summary.rejected_turnover += 1;
                    continue;
                }

                // The genotype-matched control at the end endpoint: every
                // clustered outsider whose mean relatedness to the
                // neighbourhood is within tolerance of the
                // neighbourhood's own internal mean.
                let neighbourhood: Vec<u64> = members_end.iter().map(|&(id, _)| id).collect();
                let internal_mean = mean_internal_relatedness(&pedigree, &neighbourhood);
                let cohort: Vec<(u64, u32)> = end_all
                    .iter()
                    .filter(|&&(_, _, member_quadrat)| member_quadrat != quadrat)
                    .filter(|&&(id, _, _)| {
                        mean_relatedness_to(&pedigree, id, &neighbourhood).abs_diff(internal_mean)
                            <= plan.match_tolerance_q32
                    })
                    .map(|&(id, label, _)| (id, label))
                    .collect();
                if cohort.is_empty() {
                    summary.rejected_no_cohort += 1;
                    continue;
                }
                let control_freq = frequency_milli(&cohort, variant);
                if freq_end <= control_freq {
                    summary.rejected_control += 1;
                    continue;
                }

                found.entry((quadrat, variant)).or_insert(TraditionFinding {
                    quadrat,
                    variant,
                    start_tick,
                    end_tick,
                    neighbourhood_start: members_start.len(),
                    neighbourhood_end: members_end.len(),
                    freq_start_milli: freq_start,
                    freq_end_milli: freq_end,
                    global_freq_start_milli: global_start,
                    global_freq_end_milli: global_end,
                    control_cohort: cohort.len(),
                    control_freq_milli: control_freq,
                    match_tolerance_q32: plan.match_tolerance_q32,
                });
            }
        }
    }

    summary.findings = found.into_values().collect();
    Ok(summary)
}

fn mean_internal_relatedness(pedigree: &Pedigree, members: &[u64]) -> u64 {
    if members.len() < 2 {
        return 0;
    }
    let mut sum = 0_u128;
    let mut pairs = 0_u128;
    for (index, &a) in members.iter().enumerate() {
        for &b in &members[index + 1..] {
            sum += u128::from(pedigree.relatedness_q32(a, b));
            pairs += 1;
        }
    }
    (sum / pairs) as u64
}

fn mean_relatedness_to(pedigree: &Pedigree, id: u64, members: &[u64]) -> u64 {
    if members.is_empty() {
        return 0;
    }
    let sum: u128 = members
        .iter()
        .map(|&member| u128::from(pedigree.relatedness_q32(id, member)))
        .sum();
    (sum / members.len() as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{DeathCause, EventKind};

    fn event(tick: u64, kind: EventKind) -> Event {
        Event { tick, kind }
    }

    fn birth(tick: u64, id: u64) -> Event {
        event(tick, EventKind::Birth { id, parent_id: 1 })
    }

    fn death(tick: u64, id: u64) -> Event {
        event(
            tick,
            EventKind::Death {
                id,
                cause: DeathCause::Starvation,
            },
        )
    }

    fn sample_set(tick: u64, rows: &[(u64, [u32; 7])]) -> ActionSampleSet {
        ActionSampleSet {
            tick,
            records: rows
                .iter()
                .map(|&(id, counts)| sim_persist::ActionRecord {
                    id,
                    age_ticks: tick,
                    counts,
                })
                .collect(),
        }
    }

    fn only(class: usize, count: u32) -> [u32; 7] {
        let mut counts = [0_u32; 7];
        counts[class] = count;
        counts
    }

    /// 8x8 cells of 1000 fp, quadrats of 4 cells: four neighbourhoods.
    fn base_plan() -> TraditionPlan {
        TraditionPlan {
            founder_count: 4,
            cells_x: 8,
            cells_y: 8,
            cell_size_fp: 1_000,
            quadrat_cells: 4,
            cluster_threshold_milli: 100,
            concentration_factor_milli: 1_500,
            min_neighbourhood: 2,
            match_tolerance_q32: u64::MAX,
        }
    }

    const LEFT: (i32, i32) = (500, 500);
    const RIGHT: (i32, i32) = (7_500, 7_500);

    /// The clustering primitive is the spec's method exactly: inclusive
    /// at the threshold, transitive through a chain, labels by first
    /// appearance.
    #[test]
    fn clustering_is_single_linkage_inclusive_at_the_threshold() {
        let base = [1000, 0, 0, 0, 0, 0, 0];
        let at_threshold = [950, 50, 0, 0, 0, 0, 0]; // L1 = 100 from base
        let past_threshold = [949, 51, 0, 0, 0, 0, 0]; // L1 = 102 from base
        let labels = cluster_mixes(&[base, at_threshold, past_threshold], 100);
        // base joins at_threshold at exactly the threshold, and
        // past_threshold joins at_threshold at L1 = 2: single linkage
        // merges all three.
        assert_eq!(labels, vec![0, 0, 0]);
        let labels = cluster_mixes(&[base, past_threshold], 100);
        assert_eq!(labels, vec![0, 1], "102 apart must not merge");
    }

    /// Gate E scripted ground truth: a rest-variant neighbourhood on the
    /// left persists past three median lifespans with complete turnover
    /// (founders 1,2 die; 10,11 born later carry it), while the matched
    /// cohort on the right (infinite tolerance, so both attackers) rests
    /// never. Exactly one finding, every number predicted.
    #[test]
    fn a_scripted_tradition_with_turnover_is_found_with_its_control() {
        let mut events = vec![
            // Two completed lifespans of 100 ticks fix the median.
            birth(0, 20),
            birth(0, 21),
            death(100, 20),
            death(100, 21),
            birth(100, 10),
            birth(100, 11),
            death(600, 1),
            death(600, 2),
        ];
        events.sort_by_key(|entry| entry.tick);
        // Endpoints t = 500 and end = 1000 (gap 500 > 301).
        let actions = vec![
            sample_set(0, &[(1, [0; 7]), (2, [0; 7]), (3, [0; 7]), (4, [0; 7])]),
            sample_set(
                500,
                &[
                    (1, only(0, 10)),
                    (2, only(0, 10)),
                    (3, only(6, 10)),
                    (4, only(6, 10)),
                    (10, [0; 7]),
                    (11, [0; 7]),
                ],
            ),
            sample_set(
                1000,
                &[
                    (3, only(6, 20)),
                    (4, only(6, 20)),
                    (10, only(0, 10)),
                    (11, only(0, 10)),
                ],
            ),
        ];
        let spatial = vec![
            SpatialSample {
                tick: 0,
                // Alive at 0: 1,2,3,4,20,21 - the lifespan pair sits far
                // right and acts never.
                positions: vec![LEFT, LEFT, RIGHT, RIGHT, RIGHT, RIGHT],
            },
            SpatialSample {
                tick: 500,
                // Alive at 500: 1,2,3,4,10,11.
                positions: vec![LEFT, LEFT, RIGHT, RIGHT, LEFT, LEFT],
            },
            SpatialSample {
                tick: 1000,
                // Alive at 1000: 3,4,10,11.
                positions: vec![RIGHT, RIGHT, LEFT, LEFT],
            },
        ];
        let summary =
            world_traditions(&events, &actions, &spatial, &base_plan()).expect("join holds");
        assert_eq!(summary.median_lifespan_ticks, 100);
        assert_eq!(summary.persistence_gap_ticks, 301);
        assert_eq!(summary.findings.len(), 1, "{summary:?}");
        let finding = &summary.findings[0];
        assert_eq!((finding.start_tick, finding.end_tick), (500, 1000));
        assert_eq!(finding.neighbourhood_start, 2);
        assert_eq!(finding.neighbourhood_end, 2);
        assert_eq!(finding.freq_start_milli, 1000);
        assert_eq!(finding.freq_end_milli, 1000);
        assert_eq!(finding.global_freq_start_milli, 500);
        assert_eq!(finding.global_freq_end_milli, 500);
        assert_eq!(finding.control_cohort, 2);
        assert_eq!(finding.control_freq_milli, 0);
    }

    /// The turnover requirement: the same shape, but founder 1 survives
    /// and stays - present in the neighbourhood at both endpoints - and
    /// the candidate is rejected as turnover, never found.
    #[test]
    fn a_surviving_member_rejects_the_candidate_by_turnover() {
        let mut events = vec![
            birth(0, 20),
            birth(0, 21),
            death(100, 20),
            death(100, 21),
            birth(100, 10),
            death(600, 2),
        ];
        events.sort_by_key(|entry| entry.tick);
        let actions = vec![
            sample_set(0, &[(1, [0; 7]), (2, [0; 7]), (3, [0; 7]), (4, [0; 7])]),
            sample_set(
                500,
                &[
                    (1, only(0, 10)),
                    (2, only(0, 10)),
                    (3, only(6, 10)),
                    (4, only(6, 10)),
                    (10, [0; 7]),
                ],
            ),
            sample_set(
                1000,
                &[
                    (1, only(0, 20)),
                    (3, only(6, 20)),
                    (4, only(6, 20)),
                    (10, only(0, 10)),
                ],
            ),
        ];
        let spatial = vec![
            SpatialSample {
                tick: 0,
                positions: vec![LEFT, LEFT, RIGHT, RIGHT, RIGHT, RIGHT],
            },
            // Alive at 500: 1,2,3,4,10.
            SpatialSample {
                tick: 500,
                positions: vec![LEFT, LEFT, RIGHT, RIGHT, LEFT],
            },
            // Alive at 1000: 1,3,4,10.
            SpatialSample {
                tick: 1000,
                positions: vec![LEFT, RIGHT, RIGHT, LEFT],
            },
        ];
        let summary =
            world_traditions(&events, &actions, &spatial, &base_plan()).expect("join holds");
        assert_eq!(summary.findings.len(), 0);
        assert!(summary.rejected_turnover >= 1, "{summary:?}");
    }

    /// The decisive control: kin elsewhere share the variant, so the
    /// matched cohort's frequency equals the neighbourhood's and the
    /// candidate is rejected as inherited, never reported as a tradition.
    /// Structure: parents 1,2 have two sibling pairs - 10,11 rest on the
    /// left until they die, 14,15 (born later) rest on the left after
    /// them (turnover holds), and 12,13 rest on the RIGHT the whole time.
    /// Under a tight kinship tolerance the control cohort for the
    /// left-at-end neighbourhood {14,15} is exactly its siblings {12,13}
    /// (r = 1/2), not the unrelated attackers 3,4 - and the siblings
    /// carry the same variant, which is what "inherited, not
    /// transmitted" looks like.
    #[test]
    fn an_inherited_variant_fails_the_genotype_matched_control() {
        let mut events = vec![
            birth(0, 20),
            birth(0, 21),
            death(100, 20),
            death(100, 21),
            death(150, 1),
            death(150, 2),
            death(1_100, 10),
            death(1_100, 11),
        ];
        for (id, tick) in [
            (10_u64, 100_u64),
            (11, 100),
            (12, 100),
            (13, 100),
            (14, 600),
            (15, 600),
        ] {
            events.push(Event {
                tick,
                kind: EventKind::PairedBirth {
                    id,
                    parent_a: 1,
                    parent_b: 2,
                    genome_hash: 0,
                    invest_a_milli: 0,
                    invest_b_milli: 0,
                    mutated_trait_genes: 0,
                    mutated_neural_genes: 0,
                },
            });
        }
        events.sort_by_key(|entry| entry.tick);
        // Endpoints: start 1000 (window 500->1000), end 1500 (window
        // 1000->1500; gap 500 > 301).
        let actions = vec![
            sample_set(
                500,
                &[
                    (3, [0; 7]),
                    (4, [0; 7]),
                    (10, [0; 7]),
                    (11, [0; 7]),
                    (12, [0; 7]),
                    (13, [0; 7]),
                ],
            ),
            sample_set(
                1_000,
                &[
                    (3, only(6, 10)),
                    (4, only(6, 10)),
                    (10, only(0, 10)),
                    (11, only(0, 10)),
                    (12, only(0, 10)),
                    (13, only(0, 10)),
                    (14, [0; 7]),
                    (15, [0; 7]),
                ],
            ),
            sample_set(
                1_500,
                &[
                    (3, only(6, 20)),
                    (4, only(6, 20)),
                    (12, only(0, 20)),
                    (13, only(0, 20)),
                    (14, only(0, 10)),
                    (15, only(0, 10)),
                ],
            ),
        ];
        let spatial = vec![
            // Alive at 1000: 3,4,10,11,12,13,14,15 in id order.
            SpatialSample {
                tick: 1_000,
                positions: vec![RIGHT, RIGHT, LEFT, LEFT, RIGHT, RIGHT, LEFT, LEFT],
            },
            // Alive at 1500: 3,4,12,13,14,15.
            SpatialSample {
                tick: 1_500,
                positions: vec![RIGHT, RIGHT, RIGHT, RIGHT, LEFT, LEFT],
            },
        ];
        let mut plan = base_plan();
        plan.match_tolerance_q32 = crate::fidelity::KINSHIP_ONE_Q32 / 8;
        let summary = world_traditions(&events, &actions, &spatial, &plan).expect("join holds");
        assert_eq!(summary.findings.len(), 0, "{summary:?}");
        assert!(summary.rejected_control >= 1, "{summary:?}");
    }

    /// Concentration is strict at the factor: a local frequency exactly
    /// at factor x global is NOT concentrated. The left quadrat rests at
    /// 750 milli against a global 500 with factor 1500 - exactly the
    /// boundary - and must not become a candidate; only the right
    /// quadrat's fully-concentrated attack variant does (and then dies
    /// on turnover, which is fine: the count under test is candidates).
    #[test]
    fn concentration_exactly_at_the_factor_boundary_is_rejected() {
        let mut events = vec![birth(0, 20), birth(0, 21), death(100, 20), death(100, 21)];
        events.sort_by_key(|entry| entry.tick);
        let actions = vec![
            sample_set(
                0,
                &[
                    (1, [0; 7]),
                    (2, [0; 7]),
                    (3, [0; 7]),
                    (4, [0; 7]),
                    (5, [0; 7]),
                    (6, [0; 7]),
                ],
            ),
            sample_set(
                500,
                &[
                    (1, only(0, 10)),
                    (2, only(0, 10)),
                    (3, only(0, 10)),
                    (4, only(6, 10)),
                    (5, only(6, 10)),
                    (6, only(6, 10)),
                ],
            ),
            sample_set(
                1000,
                &[
                    (1, only(0, 20)),
                    (2, only(0, 20)),
                    (3, only(0, 20)),
                    (4, only(6, 20)),
                    (5, only(6, 20)),
                    (6, only(6, 20)),
                ],
            ),
        ];
        let spatial = vec![
            SpatialSample {
                tick: 0,
                positions: vec![LEFT, LEFT, LEFT, LEFT, RIGHT, RIGHT, RIGHT, RIGHT],
            },
            SpatialSample {
                tick: 500,
                positions: vec![LEFT, LEFT, LEFT, LEFT, RIGHT, RIGHT],
            },
            SpatialSample {
                tick: 1000,
                positions: vec![LEFT, LEFT, LEFT, LEFT, RIGHT, RIGHT],
            },
        ];
        let mut plan = base_plan();
        plan.founder_count = 6;
        let summary = world_traditions(&events, &actions, &spatial, &plan).expect("join holds");
        // Left rest: 750 * 1000 == 1500 * 500 exactly - not a candidate.
        // Left attack (250 vs 500): not a candidate. Right attack
        // (1000 vs 500): the one candidate, later rejected by turnover.
        assert_eq!(summary.candidates, 1, "{summary:?}");
        assert_eq!(summary.rejected_turnover, 1, "{summary:?}");
        assert_eq!(summary.findings.len(), 0);
    }

    /// No completed lifespan: the persistence requirement is undefined
    /// and the world says so instead of scanning with a zero gap.
    #[test]
    fn a_world_with_no_completed_lifespan_reports_itself_undefined() {
        let summary = world_traditions(&[], &[], &[], &base_plan()).expect("empty is fine");
        assert_eq!(summary.median_lifespan_ticks, 0);
        assert_eq!(summary.persistence_gap_ticks, 0);
        assert_eq!(summary.endpoints_evaluated, 0);
        assert_eq!(summary.findings.len(), 0);
    }
}
