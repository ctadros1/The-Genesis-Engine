//! C13.2's per-world reduction: transmission fidelity with the genetic
//! control (`lifesim-fidelity-v1`).
//!
//! The criterion asks for "the correlation between a demonstrator's action
//! policy and an observer's post-exposure policy, controlled for their
//! genetic similarity". Uncontrolled, that correlation is confounded to
//! the point of meaninglessness: organisms near each other are kin, and
//! kin share policy genetically. The control here is the matched-pair
//! form the tradition specification mandates for the same reason
//! (`specifications/era-and-tradition-detection.md` criterion 4): every
//! exposed pair is compared against a non-exposed pair in the same
//! kinship bin and the same window, so what survives the subtraction is
//! the association with *exposure*, not with relatedness or with the
//! calendar.
//!
//! Three instruments, all exact and all from campaign artifacts:
//!
//! - **Policy** is the seven-class action mix (`ActionClass::ALL`) over
//!   one action-census window: the difference of two cumulative rows,
//!   normalized to milli fractions. Similarity between two policies is
//!   `1000 - L1/2` in milli - 1000 for identical mixes, 0 for disjoint
//!   ones. The census rows carry entity IDs, so no positional join is
//!   needed on this artifact.
//! - **Exposure** is spatial: observer within `exposure_radius_fp` of the
//!   demonstrator at any spatial sample tick inside the demonstrator's
//!   window, boundary inclusive (never one short). Positions come from
//!   the spatial series through the same fail-closed alive-set replay
//!   the arrival detector uses ([`crate::arrival`]).
//! - **Genetic similarity** is pedigree kinship, reconstructed exactly
//!   from the birth events. Genomes exist only in the final snapshot, so
//!   a genotype distance would be measurable only for organisms alive at
//!   the horizon - a survivor-biased sample. The pedigree covers every
//!   organism ever born. Kinship is Wright's coefficient in Q32 fixed
//!   point (halvings are shifts, so the arithmetic is exact until it
//!   truncates to zero below 2^-32, far under every bin boundary);
//!   founders are unrelated by construction and say so. Entity IDs are
//!   allocated at birth in increasing order, so the recursion always
//!   descends through the younger member's parents and is well-founded
//!   without a depth map.
//!
//! What this module deliberately does not do: decide anything. It reports
//! the per-bin table (exposed and control counts and mean similarities)
//! and the weighted fidelity delta, and the campaign's F-curve across the
//! corruption sweep is assembled from these world reductions by the
//! pre-registered analysis (ADR-0022 A5; ADR-0016).

use crate::arrival::ArrivalError;
use sim_core::{Event, EventKind};
use sim_persist::{ActionSampleSet, SpatialSample};
use std::collections::{BTreeMap, BTreeSet};

pub const FIDELITY_VERSION: &str = "lifesim-fidelity-v1";

/// Kinship in Q32: `kinship_q32 / 2^32` is Wright's f. Self-kinship of a
/// non-inbred organism is 1/2 (`1 << 31`); parent-offspring and full
/// siblings are 1/4. The bins below are on **relatedness** r = 2f.
pub const KINSHIP_ONE_Q32: u64 = 1 << 32;

/// Relatedness bin edges in Q32 of r = 2f, half-open `(lo, hi]` except
/// the first bin, which is exactly r = 0. Chosen at the natural pedigree
/// plateaus: unrelated; distant (r <= 1/8); the half-sib band (r <= 1/4);
/// the full-sib/parent band (r <= 1/2); closer than sibs (inbred lines).
pub const RELATEDNESS_BIN_EDGES_Q32: [u64; 4] = [
    0,
    KINSHIP_ONE_Q32 / 8,
    KINSHIP_ONE_Q32 / 4,
    KINSHIP_ONE_Q32 / 2,
];
pub const RELATEDNESS_BIN_COUNT: usize = 5;

/// One kinship bin's ledger.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FidelityBin {
    pub exposed_pairs: u64,
    /// Sum of exposed-pair similarities, milli. Mean = sum / pairs.
    pub exposed_similarity_sum_milli: i64,
    pub control_pairs: u64,
    pub control_similarity_sum_milli: i64,
    /// Exposed pairs for which no same-bin control pair existed in the
    /// window. Counted, never silently dropped: a bin whose every exposed
    /// pair is unmatched has a control with no power, and the reader must
    /// see that (the tradition spec's own rule about control power).
    pub unmatched_exposed: u64,
}

/// One world's fidelity summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldFidelity {
    /// Consecutive action-window pairs analyzed (demonstration window k,
    /// observation window k+1).
    pub windows_analyzed: u64,
    pub bins: [FidelityBin; RELATEDNESS_BIN_COUNT],
    /// Weighted mean over bins of (mean exposed similarity - mean control
    /// similarity), weights = matched exposed pairs; milli. None when no
    /// bin has both an exposed and a control pair - a delta over nothing
    /// refuses to exist rather than reading as zero.
    pub fidelity_delta_milli: Option<i64>,
    pub exposed_pairs_total: u64,
    pub control_pairs_total: u64,
    /// Exposed pairs excluded by the per-window cap (stride-sampled out).
    pub exposed_skipped_by_cap: u64,
    /// Windows whose shared control-scan budget ran out with exposed
    /// pairs still unmatched.
    pub control_budget_exhausted_windows: u64,
}

/// Analysis parameters, named so a report can echo them.
#[derive(Clone, Copy, Debug)]
pub struct FidelityPlan {
    /// Exposure radius in the samples' fixed-point frame (a campaign uses
    /// the kernel's own perception radius).
    pub exposure_radius_fp: i32,
    /// Founders in the world (IDs `1..=n`), for the spatial join.
    pub founder_count: u32,
    /// At most this many exposed pairs are scored per window,
    /// stride-sampled deterministically from the full enumeration; the
    /// rest are counted in `exposed_skipped_by_cap`, never silently
    /// dropped. 0 = unlimited (tests; a campaign world at density needs
    /// the cap or the reduction is quadratic).
    pub exposed_cap_per_window: usize,
    /// At most this many candidate control pairs are examined per window
    /// across ALL exposed pairs (a shared cursor - each control pair is
    /// consumed at most once). Exhaustion is counted in
    /// `control_budget_exhausted_windows`. 0 = unlimited.
    pub control_budget_per_window: usize,
    /// Analyze every Nth window triple (0 or 1 = every one). At campaign
    /// density a per-window reduction over every window is minutes per
    /// world; the estimand is a rate, and a deterministic stride over
    /// windows samples it without bias. Echoed via `windows_analyzed`
    /// against the series length.
    pub window_stride: usize,
}

fn action_mix_milli(delta: &[u32; 7]) -> Option<[i64; 7]> {
    let total: u64 = delta.iter().map(|&count| u64::from(count)).sum();
    if total == 0 {
        return None;
    }
    let mut mix = [0_i64; 7];
    for (slot, &count) in mix.iter_mut().zip(delta.iter()) {
        *slot = (u64::from(count) * 1000 / total) as i64;
    }
    Some(mix)
}

fn similarity_milli(a: &[i64; 7], b: &[i64; 7]) -> i64 {
    let l1: i64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    1000 - l1 / 2
}

fn relatedness_bin(r_q32: u64) -> usize {
    if r_q32 == 0 {
        return 0;
    }
    for (index, &edge) in RELATEDNESS_BIN_EDGES_Q32.iter().enumerate().skip(1) {
        if r_q32 <= edge {
            return index;
        }
    }
    RELATEDNESS_BIN_COUNT - 1
}

/// Exact pedigree kinship in Q32, memoized. Built once per world from the
/// birth events; founders (and any organism with no recorded parents) are
/// unrelated to everyone and non-inbred.
pub struct Pedigree {
    parents: BTreeMap<u64, (u64, u64)>,
    memo: std::cell::RefCell<BTreeMap<(u64, u64), u64>>,
}

impl Pedigree {
    pub fn from_events(events: &[Event]) -> Pedigree {
        let mut parents = BTreeMap::new();
        for event in events {
            if let EventKind::PairedBirth {
                id,
                parent_a,
                parent_b,
                ..
            } = event.kind
            {
                parents.insert(id, (parent_a, parent_b));
            }
        }
        Pedigree {
            parents,
            memo: std::cell::RefCell::new(BTreeMap::new()),
        }
    }

    /// Wright's kinship f(a, b) in Q32.
    pub fn kinship_q32(&self, a: u64, b: u64) -> u64 {
        if a == b {
            return match self.parents.get(&a) {
                // f(a,a) = (1 + f(father, mother)) / 2.
                Some(&(pa, pb)) => (KINSHIP_ONE_Q32 + self.kinship_q32(pa, pb)) >> 1,
                None => KINSHIP_ONE_Q32 >> 1,
            };
        }
        let key = (a.min(b), a.max(b));
        if let Some(&cached) = self.memo.borrow().get(&key) {
            return cached;
        }
        // IDs are allocated at birth in increasing order, so the younger
        // member (larger id) cannot be an ancestor of the older one;
        // recursing through its parents is the standard well-founded
        // descent.
        let (older, younger) = (key.0, key.1);
        let value = match self.parents.get(&younger) {
            Some(&(pa, pb)) => (self.kinship_q32(pa, older) + self.kinship_q32(pb, older)) >> 1,
            None => 0,
        };
        self.memo.borrow_mut().insert(key, value);
        value
    }

    /// Relatedness r = 2f in Q32.
    pub fn relatedness_q32(&self, a: u64, b: u64) -> u64 {
        self.kinship_q32(a, b) << 1
    }
}

/// Reduce one world's artifacts to its fidelity summary.
///
/// For each consecutive triple of action samples (window k demonstrates,
/// window k+1 observes): a pair (d, o), d != o, both carrying mixes on
/// their respective sides, is **exposed** when o was within the radius of
/// d at any spatial sample tick inside window k. Each exposed pair is
/// matched to the first control pair - same kinship bin, not exposed to
/// each other in window k, neither member of the exposed pair - found
/// scanning ascending IDs. Unmatched exposed pairs are counted per bin.
pub fn world_fidelity(
    events: &[Event],
    actions: &[ActionSampleSet],
    spatial: &[SpatialSample],
    plan: &FidelityPlan,
) -> Result<WorldFidelity, ArrivalError> {
    let pedigree = Pedigree::from_events(events);
    let mut summary = WorldFidelity::default();
    let id_positions = spatial_positions_by_id(events, spatial, plan.founder_count)?;

    let stride = plan.window_stride.max(1);
    for windows in actions.windows(3).step_by(stride) {
        let [before, during, after] = windows else {
            continue;
        };
        summary.windows_analyzed += 1;

        let rows_before: BTreeMap<u64, &[u32; 7]> =
            before.records.iter().map(|r| (r.id, &r.counts)).collect();
        let rows_during: BTreeMap<u64, &[u32; 7]> =
            during.records.iter().map(|r| (r.id, &r.counts)).collect();
        let rows_after: BTreeMap<u64, &[u32; 7]> =
            after.records.iter().map(|r| (r.id, &r.counts)).collect();

        // Demonstration mixes over [before, during); observation mixes
        // over [during, after). An organism absent from a bounding sample
        // has no mix on that side and forms no pair there.
        let mut demonstration: BTreeMap<u64, [i64; 7]> = BTreeMap::new();
        let mut observation: BTreeMap<u64, [i64; 7]> = BTreeMap::new();
        for (&id, &during_counts) in &rows_during {
            if let Some(&before_counts) = rows_before.get(&id) {
                let mut delta = [0_u32; 7];
                for (slot, (&now, &was)) in delta
                    .iter_mut()
                    .zip(during_counts.iter().zip(before_counts.iter()))
                {
                    *slot = now.saturating_sub(was);
                }
                if let Some(mix) = action_mix_milli(&delta) {
                    demonstration.insert(id, mix);
                }
            }
            if let Some(&after_counts) = rows_after.get(&id) {
                let mut delta = [0_u32; 7];
                for (slot, (&next, &now)) in delta
                    .iter_mut()
                    .zip(after_counts.iter().zip(during_counts.iter()))
                {
                    *slot = next.saturating_sub(now);
                }
                if let Some(mix) = action_mix_milli(&delta) {
                    observation.insert(id, mix);
                }
            }
        }

        // Nearness for this window, from the bucketed pair scan at the
        // window's OPENING sample (the first spatial sample at or after
        // `before.tick`). One sample, not a union over the window: at
        // campaign density the union is billions of pair inserts per
        // world, and exposure-at-the-opening-sample is the documented
        // instrument - the report's resolution statement covers it the
        // way the arrival detector's covers its 50-tick samples.
        let mut near_set: BTreeSet<(u64, u64)> = BTreeSet::new();
        if let Some((_, positions)) = id_positions
            .iter()
            .find(|(tick, _)| *tick >= before.tick && *tick < during.tick)
        {
            for (a, b) in crate::communities::close_pairs(
                positions,
                plan.exposure_radius_fp,
                plan.exposure_radius_fp.max(1),
            ) {
                near_set.insert((a.min(b), a.max(b)));
            }
        }
        let near = |a: u64, b: u64| -> bool { near_set.contains(&(a.min(b), a.max(b))) };

        let demonstrators: Vec<u64> = demonstration.keys().copied().collect();
        let observers: Vec<u64> = observation.keys().copied().collect();

        // Exposed pairs from the near set (both orientations where the
        // roles have mixes), sorted ascending (demonstrator, observer),
        // then stride-sampled to the cap - a deterministic every-nth
        // selection with the remainder counted. Kinship is evaluated
        // only for the pairs that survive the stride: the pedigree
        // recursion is the expensive step, and running it for every
        // near pair made the reduction unusable at campaign density.
        let mut exposed: Vec<(u64, u64)> = Vec::new();
        for &(a, b) in &near_set {
            for (demonstrator, observer) in [(a, b), (b, a)] {
                if demonstration.contains_key(&demonstrator) && observation.contains_key(&observer)
                {
                    exposed.push((demonstrator, observer));
                }
            }
        }
        exposed.sort_unstable();
        let kept: Vec<(u64, u64)> =
            if plan.exposed_cap_per_window > 0 && exposed.len() > plan.exposed_cap_per_window {
                let stride = exposed.len().div_ceil(plan.exposed_cap_per_window);
                let kept: Vec<_> = exposed.iter().copied().step_by(stride).collect();
                summary.exposed_skipped_by_cap += (exposed.len() - kept.len()) as u64;
                kept
            } else {
                exposed
            };
        let selected: Vec<(u64, u64, usize)> = kept
            .into_iter()
            .map(|(demonstrator, observer)| {
                let bin = relatedness_bin(pedigree.relatedness_q32(demonstrator, observer));
                (demonstrator, observer, bin)
            })
            .collect();

        // The shared control scan: candidate pairs in ascending
        // (demonstrator, observer) order, examined at most once each
        // under one per-window budget. A pair that is globally invalid
        // (a self-pair, or the two were near each other) is consumed on
        // sight; every other examined pair is banked in its OWN
        // kinship bin's queue and consumed only when an exposed pair
        // actually uses it as its control. A pair that merely shares a
        // member with the CURRENT exposed pair stays banked for the next
        // one - consuming it there would starve later pairs, which is
        // exactly what the independent pass's weighting test caught in
        // the first cursor design.
        let mut bin_queues: Vec<std::collections::VecDeque<(u64, u64)>> =
            vec![std::collections::VecDeque::new(); RELATEDNESS_BIN_COUNT];
        let mut scan_demo = 0_usize;
        let mut scan_obs = 0_usize;
        let mut budget = plan.control_budget_per_window;
        let unlimited = plan.control_budget_per_window == 0;
        let mut exhausted = false;
        for &(demonstrator, observer, bin) in &selected {
            let similarity =
                similarity_milli(&demonstration[&demonstrator], &observation[&observer]);
            summary.bins[bin].exposed_pairs += 1;
            summary.bins[bin].exposed_similarity_sum_milli += similarity;
            summary.exposed_pairs_total += 1;

            let conflicts = |pair: &(u64, u64)| {
                pair.0 == demonstrator
                    || pair.0 == observer
                    || pair.1 == demonstrator
                    || pair.1 == observer
            };
            let mut chosen = bin_queues[bin].iter().position(|pair| !conflicts(pair));
            while chosen.is_none() && scan_demo < demonstrators.len() {
                if !unlimited && budget == 0 {
                    exhausted = true;
                    break;
                }
                if scan_obs >= observers.len() {
                    scan_demo += 1;
                    scan_obs = 0;
                    continue;
                }
                let control_demo = demonstrators[scan_demo];
                let control_obs = observers[scan_obs];
                scan_obs += 1;
                if !unlimited {
                    budget -= 1;
                }
                if control_obs == control_demo || near(control_demo, control_obs) {
                    continue;
                }
                let pair_bin = relatedness_bin(pedigree.relatedness_q32(control_demo, control_obs));
                bin_queues[pair_bin].push_back((control_demo, control_obs));
                if pair_bin == bin && !conflicts(&(control_demo, control_obs)) {
                    chosen = Some(bin_queues[bin].len() - 1);
                }
            }
            match chosen {
                Some(position) => {
                    let (control_demo, control_obs) =
                        bin_queues[bin].remove(position).expect("position exists");
                    let control_similarity =
                        similarity_milli(&demonstration[&control_demo], &observation[&control_obs]);
                    summary.bins[bin].control_pairs += 1;
                    summary.bins[bin].control_similarity_sum_milli += control_similarity;
                    summary.control_pairs_total += 1;
                }
                None => summary.bins[bin].unmatched_exposed += 1,
            }
        }
        if exhausted {
            summary.control_budget_exhausted_windows += 1;
        }
    }

    let mut weighted_sum = 0_i64;
    let mut weight = 0_i64;
    for bin in &summary.bins {
        if bin.exposed_pairs > 0 && bin.control_pairs > 0 {
            let exposed_mean = bin.exposed_similarity_sum_milli / bin.exposed_pairs as i64;
            let control_mean = bin.control_similarity_sum_milli / bin.control_pairs as i64;
            let matched = (bin.exposed_pairs - bin.unmatched_exposed) as i64;
            weighted_sum += (exposed_mean - control_mean) * matched;
            weight += matched;
        }
    }
    summary.fidelity_delta_milli = if weight > 0 {
        Some(weighted_sum / weight)
    } else {
        None
    };
    Ok(summary)
}

/// Per spatial sample, entity ID -> position, by the arrival detector's
/// replay semantics (births in, deaths out at the sample tick). The
/// arrival census runs first purely for its fail-closed population check
/// - a mismatch anywhere refuses the whole join, for the reason recorded
/// there: a repaired join misassigns every later track silently.
pub(crate) fn spatial_positions_by_id(
    events: &[Event],
    spatial: &[SpatialSample],
    founder_count: u32,
) -> Result<Vec<(u64, BTreeMap<u64, (i32, i32)>)>, ArrivalError> {
    // ONE alive-set predicate, checked against every sample's own length
    // before any zip: the independent pass showed the earlier version
    // kept two hand-synchronized copies of the boundary rules (one in
    // the arrival census it called for its check, one inline) and then
    // zipped, which truncates silently the moment they disagree - the
    // exact misassignment the fail-closed rule exists to prevent.
    let mut born_at: BTreeMap<u64, u64> =
        (1..=u64::from(founder_count)).map(|id| (id, 0)).collect();
    let mut died_at: BTreeMap<u64, u64> = BTreeMap::new();
    for event in events {
        match event.kind {
            EventKind::Birth { id, .. } | EventKind::PairedBirth { id, .. } => {
                born_at.insert(id, event.tick);
            }
            EventKind::Death { id, .. } => {
                died_at.insert(id, event.tick);
            }
            _ => {}
        }
    }
    let mut result = Vec::with_capacity(spatial.len());
    for sample in spatial {
        let alive: Vec<u64> = born_at
            .iter()
            .filter(|(id, born)| {
                **born <= sample.tick && !died_at.get(id).is_some_and(|&at| at <= sample.tick)
            })
            .map(|(&id, _)| id)
            .collect();
        if alive.len() != sample.positions.len() {
            return Err(ArrivalError::PopulationMismatch {
                tick: sample.tick,
                reconstructed: alive.len(),
                sampled: sample.positions.len(),
            });
        }
        let positions: BTreeMap<u64, (i32, i32)> = alive
            .into_iter()
            .zip(sample.positions.iter().copied())
            .collect();
        result.push((sample.tick, positions));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::DeathCause;

    fn event(tick: u64, kind: EventKind) -> Event {
        Event { tick, kind }
    }

    fn paired(tick: u64, id: u64, a: u64, b: u64) -> Event {
        event(
            tick,
            EventKind::PairedBirth {
                id,
                parent_a: a,
                parent_b: b,
                genome_hash: 0,
                invest_a_milli: 0,
                invest_b_milli: 0,
                mutated_trait_genes: 0,
                mutated_neural_genes: 0,
            },
        )
    }

    /// The kinship table every genetics text pins: self 1/2,
    /// parent-offspring 1/4, full siblings 1/4, half siblings 1/8,
    /// grandparent 1/8, founders 0 - exact in Q32, no rounding anywhere,
    /// and the inbred case takes the f(a,a) branch.
    #[test]
    fn pedigree_kinship_matches_the_textbook_exactly() {
        // Founders 1..=4. 10 = child(1,2); 11 = full sib; 12 = child(1,3)
        // half sib of 10; 13 = child(10,11) inbred; 14 = child(10,3).
        let events = vec![
            paired(10, 10, 1, 2),
            paired(10, 11, 1, 2),
            paired(10, 12, 1, 3),
            paired(20, 13, 10, 11),
            paired(20, 14, 10, 3),
        ];
        let pedigree = Pedigree::from_events(&events);
        let q = |n: u64, d: u64| KINSHIP_ONE_Q32 * n / d;
        assert_eq!(pedigree.kinship_q32(1, 1), q(1, 2));
        assert_eq!(pedigree.kinship_q32(1, 2), 0);
        assert_eq!(pedigree.kinship_q32(1, 10), q(1, 4));
        assert_eq!(pedigree.kinship_q32(10, 11), q(1, 4));
        assert_eq!(pedigree.kinship_q32(10, 12), q(1, 8));
        assert_eq!(pedigree.kinship_q32(1, 14), q(1, 8));
        // The inbred child of full sibs: f(13,13) = (1 + 1/4) / 2 = 5/8.
        assert_eq!(pedigree.kinship_q32(13, 13), q(5, 8));
        assert_eq!(pedigree.relatedness_q32(10, 11), q(1, 2));
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

    fn spatial_sample(tick: u64, positions: Vec<(i32, i32)>) -> SpatialSample {
        SpatialSample { tick, positions }
    }

    const RADIUS: i32 = 1_000;

    fn only(class: usize, count: u32) -> [u32; 7] {
        let mut counts = [0_u32; 7];
        counts[class] = count;
        counts
    }

    /// Gate E scripted ground truth. Four unrelated founders: 1 and 2
    /// co-located (exposed), 3 and 4 far apart (the same-bin control
    /// material). Observer 2's post-exposure mix mostly copies 1's
    /// demonstration mix; observer 4's is disjoint from every
    /// demonstration. The bin table and the sign and identity of the
    /// delta are all predicted.
    #[test]
    fn the_detector_recovers_a_scripted_transmission_exactly() {
        let events: Vec<Event> = Vec::new();
        let actions = vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7]), (3, [0; 7]), (4, [0; 7])]),
            // Demonstration: 1 and 3 rest; 2 and 4 attack a little so
            // they carry demonstration mixes too.
            sample_set(
                1000,
                &[
                    (1, only(0, 10)),
                    (2, only(6, 1)),
                    (3, only(0, 10)),
                    (4, only(6, 1)),
                ],
            ),
            // Observation: 2 copies 1 (rest-heavy); 4 attacks on.
            sample_set(
                1500,
                &[
                    (1, only(0, 20)),
                    (2, {
                        let mut counts = only(6, 1);
                        counts[0] = 10;
                        counts
                    }),
                    (3, only(0, 20)),
                    (4, only(6, 11)),
                ],
            ),
        ];
        let positions = vec![(0, 0), (500, 0), (50_000, 0), (0, 50_000)];
        let spatial = vec![
            spatial_sample(500, positions.clone()),
            spatial_sample(750, positions.clone()),
            spatial_sample(1000, positions.clone()),
            spatial_sample(1500, positions),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 4,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        assert_eq!(summary.windows_analyzed, 1);
        // Exposure is symmetric and both 1 and 2 carry both mixes:
        // (1,2) and (2,1), both in the unrelated bin.
        assert_eq!(summary.exposed_pairs_total, 2);
        assert_eq!(summary.bins[0].exposed_pairs, 2);
        assert_eq!(summary.bins[0].control_pairs, 2);
        assert_eq!(summary.bins[0].unmatched_exposed, 0);
        // Exact: (1,2) has observer 2 copying 1's pure-rest mix (its one
        // attack count does not move between the bounding samples, so its
        // observation delta is pure rest) - similarity 1000; (2,1) has
        // demonstrator 2 pure-attack against observer 1 pure-rest -
        // similarity 0. Mean 500. Both controls pair a rest demonstrator
        // with the far attacker's pure-attack observation - 0.
        assert_eq!(summary.bins[0].exposed_similarity_sum_milli, 1000);
        assert_eq!(summary.bins[0].control_similarity_sum_milli, 0);
        assert_eq!(summary.fidelity_delta_milli, Some(500));
    }

    /// The exposure boundary is inclusive at exactly the radius and
    /// excludes one fixed-point unit past it.
    #[test]
    fn the_exposure_boundary_is_inclusive_and_one_unit_past_it_is_not() {
        for (distance, expect_exposed) in [(RADIUS, 2_u64), (RADIUS + 1, 0)] {
            let actions = vec![
                sample_set(500, &[(1, [0; 7]), (2, [0; 7])]),
                sample_set(1000, &[(1, only(0, 10)), (2, only(0, 10))]),
                sample_set(1500, &[(1, only(0, 20)), (2, only(0, 20))]),
            ];
            let spatial = vec![
                spatial_sample(500, vec![(0, 0), (distance, 0)]),
                spatial_sample(1000, vec![(0, 0), (distance, 0)]),
                spatial_sample(1500, vec![(0, 0), (distance, 0)]),
            ];
            let summary = world_fidelity(
                &Vec::new(),
                &actions,
                &spatial,
                &FidelityPlan {
                    exposure_radius_fp: RADIUS,
                    founder_count: 2,
                    exposed_cap_per_window: 0,
                    control_budget_per_window: 0,
                    window_stride: 0,
                },
            )
            .expect("join holds");
            assert_eq!(
                summary.exposed_pairs_total, expect_exposed,
                "at distance {distance}"
            );
        }
    }

    /// Kin pairs land in the pedigree's bin, never pooled with strangers
    /// - and with no same-bin control available they are counted
    /// unmatched and the delta refuses to exist rather than reading as
    /// zero.
    #[test]
    fn kin_pairs_are_binned_by_pedigree_not_pooled() {
        let events = vec![paired(100, 3, 1, 2)];
        let actions = vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7]), (3, [0; 7])]),
            sample_set(
                1000,
                &[(1, only(0, 10)), (2, only(0, 10)), (3, only(0, 10))],
            ),
            sample_set(
                1500,
                &[(1, only(0, 20)), (2, only(0, 20)), (3, only(0, 20))],
            ),
        ];
        // Parent 1 and child 3 co-located; founder 2 far from both.
        let spatial = vec![
            spatial_sample(500, vec![(0, 0), (90_000, 0), (100, 0)]),
            spatial_sample(1000, vec![(0, 0), (90_000, 0), (100, 0)]),
            spatial_sample(1500, vec![(0, 0), (90_000, 0), (100, 0)]),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 2,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        // (1,3) and (3,1) are r = 1/2: the fourth bin (index 3). The only
        // other organism is founder 2, unrelated to both, so no same-bin
        // control pair exists anywhere.
        assert_eq!(summary.bins[3].exposed_pairs, 2);
        assert_eq!(summary.bins[3].unmatched_exposed, 2);
        assert_eq!(summary.bins[3].control_pairs, 0);
        assert_eq!(summary.bins[0].exposed_pairs, 0);
        assert_eq!(summary.fidelity_delta_milli, None);
    }

    /// The control must come from the same kinship bin: with the only
    /// other available pair being close kin (bin 3) while the exposed
    /// pair is unrelated (bin 0), the exposed pair goes unmatched rather
    /// than borrowing a control from the wrong bin - the mutation that
    /// drops the bin check turns this exact ledger difference on.
    #[test]
    fn a_control_is_never_borrowed_from_another_kinship_bin() {
        // 3 is the parent of 4 (r = 1/2, bin 3); 1 and 2 are unrelated.
        let events = vec![paired(100, 4, 3, 3)];
        let actions = vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7]), (3, [0; 7]), (4, [0; 7])]),
            sample_set(
                1000,
                &[
                    (1, only(0, 10)),
                    (2, only(0, 10)),
                    (3, only(0, 10)),
                    (4, only(0, 10)),
                ],
            ),
            sample_set(
                1500,
                &[
                    (1, only(0, 20)),
                    (2, only(0, 20)),
                    (3, only(0, 20)),
                    (4, only(0, 20)),
                ],
            ),
        ];
        // 1,2 co-located (the exposed pair); 3 and 4 far from everyone
        // and from each other (never exposed, available as controls).
        let positions = vec![(0, 0), (100, 0), (50_000, 0), (0, 50_000)];
        let spatial = vec![
            spatial_sample(500, positions.clone()),
            spatial_sample(1000, positions.clone()),
            spatial_sample(1500, positions),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 3,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        // (1,2) and (2,1), unrelated: bin 0. The only non-exposed pairs
        // with both mixes involve 3 and 4, whose pair is bin 3 - and
        // pairs (3,1)-style are excluded for sharing a member with the
        // exposed pair or being near. No bin-0 control exists.
        assert_eq!(summary.bins[0].exposed_pairs, 2);
        assert_eq!(summary.bins[0].control_pairs, 0, "{summary:?}");
        assert_eq!(summary.bins[0].unmatched_exposed, 2);
    }

    /// An organism absent from a bounding action sample contributes no
    /// mix and no pair on that side: a demonstrator that died mid-window
    /// is not scored on a truncated window.
    #[test]
    fn a_missing_bounding_row_excludes_the_pair_not_the_window() {
        let actions = vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7])]),
            sample_set(1000, &[(2, only(0, 10))]),
            sample_set(1500, &[(2, only(0, 20))]),
        ];
        let events = vec![event(
            700,
            EventKind::Death {
                id: 1,
                cause: DeathCause::Starvation,
            },
        )];
        let spatial = vec![
            spatial_sample(500, vec![(0, 0), (100, 0)]),
            spatial_sample(1000, vec![(100, 0)]),
            spatial_sample(1500, vec![(100, 0)]),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 2,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        assert_eq!(summary.windows_analyzed, 1);
        assert_eq!(summary.exposed_pairs_total, 0);
    }

    /// Two-class counts, for the mixes that separate a real normalization
    /// from an accidental one.
    fn rest_and_attack(rest: u32, attack: u32) -> [u32; 7] {
        let mut counts = [0_u32; 7];
        counts[0] = rest;
        counts[6] = attack;
        counts
    }

    /// The pair of founders every arithmetic test below uses: co-located,
    /// unrelated, both carrying a demonstration and an observation mix.
    fn two_co_located_founders(actions: Vec<ActionSampleSet>) -> WorldFidelity {
        let together = vec![(0, 0), (100, 0)];
        let spatial = vec![
            spatial_sample(500, together.clone()),
            spatial_sample(1000, together.clone()),
            spatial_sample(1500, together),
        ];
        world_fidelity(
            &Vec::new(),
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 2,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds")
    }

    /// The action mix normalizes by the window's TOTAL action count, not by
    /// its largest class. Every other test in this module uses single-class
    /// deltas, where sum and maximum coincide; a two-class window is the
    /// smallest shape that tells them apart.
    #[test]
    fn the_action_mix_normalizes_by_the_window_total_not_its_largest_class() {
        let summary = two_co_located_founders(vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7])]),
            // Demonstration deltas: 1 is 6 rest + 2 attack, 2 the reverse.
            sample_set(
                1000,
                &[(1, rest_and_attack(6, 2)), (2, rest_and_attack(2, 6))],
            ),
            // Observation deltas: both 2 rest + 6 attack.
            sample_set(
                1500,
                &[(1, rest_and_attack(8, 8)), (2, rest_and_attack(4, 12))],
            ),
        ]);
        assert_eq!(summary.bins[0].exposed_pairs, 2, "{summary:?}");
        // Mixes over the total: demonstrations 750/250 and 250/750,
        // observations both 250/750. (1,2) scores 1000 - 1000/2 = 500 and
        // (2,1) scores 1000. Normalizing by the largest class instead would
        // read 1000/333 and 333/1000 and score 333 + 1000.
        assert_eq!(summary.bins[0].exposed_similarity_sum_milli, 1_500);
    }

    /// The similarity halves the L1 distance with truncation. A three-way
    /// mix makes L1 odd, which is the only case where truncating and
    /// rounding differ - and no other test in this module produces one.
    #[test]
    fn the_similarity_truncates_the_halved_l1_distance() {
        let three_ways = [1, 1, 1, 0, 0, 0, 0];
        let summary = two_co_located_founders(vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7])]),
            sample_set(1000, &[(1, three_ways), (2, only(0, 1))]),
            sample_set(1500, &[(1, [2, 1, 1, 0, 0, 0, 0]), (2, only(0, 6))]),
        ]);
        assert_eq!(summary.bins[0].exposed_pairs, 2, "{summary:?}");
        // (1,2): demonstration [333,333,333,..] against observation
        // [1000,0,..] is L1 1333, so 1000 - 666 = 334, not 1000 - 667.
        // (2,1) is two identical pure-rest mixes: 1000.
        assert_eq!(summary.bins[0].exposed_similarity_sum_milli, 1_334);
    }

    /// The exposure window is half-open at the demonstration window's
    /// closing tick: a sample taken exactly there belongs to the next
    /// window and cannot make a pair exposed.
    #[test]
    fn a_sample_at_the_windows_closing_tick_is_outside_the_window() {
        let actions = vec![
            sample_set(500, &[(1, [0; 7]), (2, [0; 7])]),
            sample_set(1000, &[(1, only(0, 10)), (2, only(0, 10))]),
            sample_set(1500, &[(1, only(0, 20)), (2, only(0, 20))]),
        ];
        let apart = vec![(0, 0), (50_000, 0)];
        let together = vec![(0, 0), (100, 0)];
        let spatial = vec![
            spatial_sample(500, apart.clone()),
            spatial_sample(750, apart),
            // The pair meets only at exactly the closing tick.
            spatial_sample(1000, together.clone()),
            spatial_sample(1500, together),
        ];
        let summary = world_fidelity(
            &Vec::new(),
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 2,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        assert_eq!(summary.windows_analyzed, 1);
        assert_eq!(summary.exposed_pairs_total, 0, "{summary:?}");
    }

    /// A control is a pair of two organisms, never a degenerate (d, d).
    /// Organism 99 carries action rows but no birth event, so it holds no
    /// position and is therefore not "near" even itself - the one shape in
    /// which a self-pair passes every other control test (same bin, not
    /// exposed, no member shared with the exposed pair).
    #[test]
    fn a_self_pair_is_never_accepted_as_the_matched_control() {
        let events = vec![
            paired(100, 10, 1, 2),
            paired(100, 11, 1, 2),
            paired(200, 13, 10, 11),
        ];
        let ids = [1_u64, 2, 10, 11, 13, 99];
        let rows = |counts: [u32; 7]| -> Vec<(u64, [u32; 7])> {
            ids.iter().map(|&id| (id, counts)).collect()
        };
        let actions = vec![
            sample_set(500, &rows([0; 7])),
            sample_set(1000, &rows(only(0, 10))),
            sample_set(1500, &rows(only(0, 20))),
        ];
        // Alive at every sample, ascending: 1, 2, 10, 11, 13. Only 10 and
        // 13 lie within the radius of each other; 99 has no position at all.
        let places = vec![(50_000, 0), (0, 50_000), (0, 0), (50_000, 50_000), (100, 0)];
        let spatial = vec![
            spatial_sample(500, places.clone()),
            spatial_sample(1000, places.clone()),
            spatial_sample(1500, places),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 2,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        // 13 is the child of full sibs 10 and 11, so r(10,13) = 3/4: the
        // closer-than-sibs bin, whose only other member pair is (99, 99).
        assert_eq!(summary.bins[4].exposed_pairs, 2, "{summary:?}");
        assert_eq!(summary.bins[4].control_pairs, 0, "{summary:?}");
        assert_eq!(summary.bins[4].unmatched_exposed, 2, "{summary:?}");
    }

    /// The weighted delta weights each bin by its **matched** exposed pairs,
    /// not by all of them. The world below separates the two: 1 and 2 are
    /// the only pair far enough apart to be a control, so every exposed
    /// unrelated pair containing one of them goes unmatched, while the
    /// sibling bin (12 and 13 exposed, 10 and 11 available) is fully
    /// matched. The expectation is recomputed from the published bin table
    /// so the test states the weight rule rather than a golden number.
    #[test]
    fn the_delta_weights_each_bin_by_its_matched_pairs_not_its_exposed_pairs() {
        // Founders 1..4 are unrelated; 10 and 11 are full sibs, and so are
        // 12 and 13, from parent pairs that never act.
        let events = vec![
            paired(100, 10, 6, 7),
            paired(100, 11, 6, 7),
            paired(100, 12, 8, 9),
            paired(100, 13, 8, 9),
        ];
        let ids = [1_u64, 2, 3, 4, 10, 11, 12, 13];
        let rows = |counts: [u32; 7]| -> Vec<(u64, [u32; 7])> {
            ids.iter().map(|&id| (id, counts)).collect()
        };
        let mut last = rows(only(0, 20));
        // 12 and 13 observe pure attack; everyone else observes pure rest,
        // which is what makes the two bins' deltas differ.
        for row in last.iter_mut() {
            if row.0 == 12 || row.0 == 13 {
                row.1 = rest_and_attack(10, 10);
            }
        }
        let actions = vec![
            sample_set(500, &rows([0; 7])),
            sample_set(1000, &rows(only(0, 10))),
            sample_set(1500, &last),
        ];
        // Ascending ids 1,2,3,4,10,11,12,13. Every pair is within the
        // radius except (1,2) and (10,11).
        let places = vec![
            (-800, 0),
            (800, 0),
            (0, 0),
            (0, 0),
            (0, 600),
            (0, -600),
            (0, 100),
            (0, -100),
        ];
        let spatial = vec![
            spatial_sample(500, places.clone()),
            spatial_sample(1000, places.clone()),
            spatial_sample(1500, places),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 4,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        assert!(
            summary.bins[0].unmatched_exposed > 0 && summary.bins[0].control_pairs > 0,
            "the unrelated bin must carry both matched and unmatched pairs: {summary:?}"
        );
        assert_eq!(
            summary.bins[3].unmatched_exposed, 0,
            "the sibling bin must be fully matched: {summary:?}"
        );
        let recompute = |weight_of: fn(&FidelityBin) -> i64| -> i64 {
            let mut weighted = 0_i64;
            let mut weight = 0_i64;
            for bin in &summary.bins {
                if bin.exposed_pairs > 0 && bin.control_pairs > 0 {
                    let exposed_mean = bin.exposed_similarity_sum_milli / bin.exposed_pairs as i64;
                    let control_mean = bin.control_similarity_sum_milli / bin.control_pairs as i64;
                    let w = weight_of(bin);
                    weighted += (exposed_mean - control_mean) * w;
                    weight += w;
                }
            }
            weighted / weight
        };
        let by_matched = recompute(|bin| (bin.exposed_pairs - bin.unmatched_exposed) as i64);
        let by_exposed = recompute(|bin| bin.exposed_pairs as i64);
        assert_ne!(
            by_matched, by_exposed,
            "the world must distinguish the two weightings: {summary:?}"
        );
        assert_eq!(
            summary.fidelity_delta_milli,
            Some(by_matched),
            "{summary:?}"
        );
    }

    /// The position join uses the arrival replay's own death boundary: a
    /// death at exactly a sample tick is absent from that sample. Founder 1
    /// dies at the first sample tick, so the two positions there belong to
    /// 2 and 3 - a join that kept 1 alive would hand 2's position to 1 and
    /// drop 3 entirely, and the fail-closed population check (which uses the
    /// correct boundary) would still balance and say nothing.
    #[test]
    fn the_position_join_drops_an_organism_that_dies_at_the_sample_tick() {
        let events = vec![event(
            500,
            EventKind::Death {
                id: 1,
                cause: DeathCause::Starvation,
            },
        )];
        let actions = vec![
            sample_set(500, &[(2, [0; 7]), (3, [0; 7])]),
            sample_set(1000, &[(2, only(0, 10)), (3, only(0, 10))]),
            sample_set(1500, &[(2, only(0, 20)), (3, only(0, 20))]),
        ];
        let survivors = vec![(0, 0), (100, 0)];
        let spatial = vec![
            spatial_sample(500, survivors.clone()),
            spatial_sample(1000, survivors.clone()),
            spatial_sample(1500, survivors),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 3,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        assert_eq!(summary.exposed_pairs_total, 2, "{summary:?}");
    }

    /// ...and the replay's birth boundary: an organism born at exactly a
    /// sample tick is present at it. The tick-500 birth below carries the
    /// small id 2, so dropping it would shift every later organism's
    /// position one place along while the population count still balances.
    #[test]
    fn the_position_join_admits_an_organism_born_at_the_sample_tick() {
        let events = vec![
            event(
                100,
                EventKind::Birth {
                    id: 3,
                    parent_id: 1,
                },
            ),
            event(
                100,
                EventKind::Birth {
                    id: 4,
                    parent_id: 1,
                },
            ),
            event(
                500,
                EventKind::Birth {
                    id: 2,
                    parent_id: 1,
                },
            ),
        ];
        let actions = vec![
            sample_set(500, &[(3, [0; 7]), (4, [0; 7])]),
            sample_set(1000, &[(3, only(0, 10)), (4, only(0, 10))]),
            sample_set(1500, &[(3, only(0, 20)), (4, only(0, 20))]),
        ];
        // Ascending ids 1, 2, 3, 4: only 3 and 4 are within the radius.
        let places = vec![(100_000, 0), (50_000, 0), (0, 0), (100, 0)];
        let spatial = vec![
            spatial_sample(500, places.clone()),
            spatial_sample(1000, places.clone()),
            spatial_sample(1500, places),
        ];
        let summary = world_fidelity(
            &events,
            &actions,
            &spatial,
            &FidelityPlan {
                exposure_radius_fp: RADIUS,
                founder_count: 1,
                exposed_cap_per_window: 0,
                control_budget_per_window: 0,
                window_stride: 0,
            },
        )
        .expect("join holds");
        assert_eq!(summary.exposed_pairs_total, 2, "{summary:?}");
    }
}
