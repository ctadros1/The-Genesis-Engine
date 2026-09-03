//! C17.4 at the LOCKED penalty (experiments/phase17-era-preregistration.md):
//! synthetic logs at the pilot's feature scale - population 330 of a
//! 10,000 cap, births around 1,000 and starvation deaths around 700 milli
//! per 1,000 organism-ticks - with injected boundaries of at least a
//! fourfold step, analyzed at window 1,000, penalty 2 x 10^8 and eight
//! segments at most. Precision and recall at +/- 1 window must both be
//! 1.0 on the fixture set, and the synthetic null must yield no boundary.
//!
//! The agent-written `era_synthetic.rs` proves the detector at a penalty
//! derived from each fixture's own step; this file proves it at the one
//! number the pre-registration locked, which is the number the campaign
//! is read at. Both are needed: the first says the machinery works, the
//! second says the locked threshold is neither blind nor trigger-happy at
//! the scale of real logs (trap 5's noise floor, checked rather than
//! assumed).

use sim_analysis::{EraPlan, FeatureGates, SyntheticSpec, synthetic_log, world_era};

const WINDOW: u64 = 1_000;
const PENALTY: i128 = 200_000_000;
const POPULATION: u32 = 330;
const MAX_ENTITIES: u32 = 10_000;

/// Rates as [births, deaths_starvation, damage_events, signals_emitted,
/// objects_created], milli per 1,000 organism-ticks.
const QUIET: [i64; 5] = [1_000, 700, 300, 800, 400];

fn plan(windows: u32) -> EraPlan {
    EraPlan {
        window_ticks: WINDOW,
        burn_in_ticks: 0,
        penalty_milli: PENALTY,
        max_segments: 8,
        initial_organisms: 0,
        max_entities: MAX_ENTITIES,
        run_ticks: u64::from(windows) * WINDOW,
        gates: FeatureGates {
            contest: true,
            artifact: true,
            social: true,
            ontogeny: false,
            transition: false,
        },
    }
}

fn spec(rates: Vec<[i64; 5]>) -> SyntheticSpec {
    SyntheticSpec {
        window_ticks: WINDOW,
        windows: rates.len() as u32,
        population: POPULATION,
        rates,
    }
}

fn boundaries_of(rates: Vec<[i64; 5]>, seed: u64) -> Vec<u32> {
    let spec = spec(rates);
    let events = synthetic_log(seed, &spec);
    let era = world_era(&events, &plan(spec.windows)).expect("analysis");
    era.boundaries.iter().map(|boundary| boundary.window).collect()
}

fn within_one(found: &[u32], truth: &[u32]) -> (usize, usize) {
    let hits = truth
        .iter()
        .filter(|t| found.iter().any(|f| f.abs_diff(**t) <= 1))
        .count();
    let precise = found
        .iter()
        .filter(|f| truth.iter().any(|t| f.abs_diff(*t) <= 1))
        .count();
    (hits, precise)
}

#[test]
fn a_fourfold_step_in_two_features_is_found_at_the_locked_penalty() {
    let mut rates = vec![QUIET; 40];
    // 1,000 -> 6,000 and 700 -> 4,200: over twenty windows a side the
    // reduction is 10 x (5,000^2 + 3,500^2) = 3.7 x 10^8, above the
    // locked 2 x 10^8; a lone step below ~4,500 would not clear it, which
    // the last test in this file pins from the other side.
    for row in rates.iter_mut().skip(20) {
        row[0] = 6_000;
        row[1] = 4_200;
    }
    let found = boundaries_of(rates, 0x17_01);
    let (hits, precise) = within_one(&found, &[20]);
    assert_eq!(hits, 1, "recall: boundaries found {found:?}");
    assert_eq!(precise, found.len(), "precision: extra boundaries in {found:?}");
    assert_eq!(found.len(), 1);
}

#[test]
fn three_steps_in_different_groups_are_found_with_precision_and_recall_one() {
    let mut rates = vec![QUIET; 60];
    for (index, row) in rates.iter_mut().enumerate() {
        // Fifteen windows a side: the reduction per boundary is
        // 7.5 x sum(step^2), so each boundary needs sum(step^2) above
        // 2.7 x 10^7.
        if index >= 15 {
            row[0] = 6_000; // demography: 5,000^2 = 2.5e7 ... with 7.5x = 1.9e8; the deaths co-move below
            row[1] = 2_500; // 1,800^2 = 3.2e6 -> total 2.1e8
        }
        if index >= 30 {
            row[2] = 5_000; // conflict
            row[4] = 5_000; // objects
        }
        if index >= 45 {
            row[3] = 6_000; // social
            row[1] = 5_000; // demography again
        }
    }
    let found = boundaries_of(rates, 0x17_02);
    let (hits, precise) = within_one(&found, &[15, 30, 45]);
    assert_eq!(hits, 3, "recall: boundaries found {found:?}");
    assert_eq!(precise, found.len(), "precision: extra boundaries in {found:?}");
}

#[test]
fn a_step_in_the_objects_group_alone_is_found() {
    let mut rates = vec![QUIET; 40];
    // 400 -> 6,400 alone: 10 x 6,000^2 = 3.6 x 10^8.
    for row in rates.iter_mut().skip(20) {
        row[4] = 6_400;
    }
    let found = boundaries_of(rates, 0x17_03);
    let (hits, precise) = within_one(&found, &[20]);
    assert_eq!(hits, 1, "recall: boundaries found {found:?}");
    assert_eq!(precise, found.len(), "precision: extra boundaries in {found:?}");
}

#[test]
fn the_synthetic_null_yields_no_boundary_at_the_locked_penalty() {
    let found = boundaries_of(vec![QUIET; 40], 0x17_04);
    assert!(found.is_empty(), "boundaries in a constant-rate log: {found:?}");
}

#[test]
fn a_twofold_step_alone_stays_below_the_locked_penalty() {
    // The pre-registration says what the threshold means at this scale:
    // a step of ~2,000 in one feature does not buy 2 x 10^8. A detector
    // that found it would be reading noise-sized shifts as regimes.
    let mut rates = vec![QUIET; 40];
    for row in rates.iter_mut().skip(20) {
        row[0] = 2_000;
    }
    let found = boundaries_of(rates, 0x17_05);
    assert!(found.is_empty(), "a twofold single-feature step was found: {found:?}");
}
