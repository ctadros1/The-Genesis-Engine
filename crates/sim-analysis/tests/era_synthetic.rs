//! C17.4: the era detector's synthetic ground truth (ADR-0033).
//!
//! `synthetic_log` builds event logs with known, piecewise-constant
//! injected rates and known boundary windows; these tests recover those
//! boundaries through the public API alone (`world_era`, `segment`,
//! `render_world`) and check the exact segmenter against brute force on
//! small inputs, so a change to the DP or the cost formula has to keep
//! agreeing with an independent reference, not just with itself.
//!
//! Every scenario derives its own penalty from the true feature vector
//! (`segment` on the noiseless log, one segment versus the true count,
//! divided by four) rather than hard-coding a number picked to make the
//! test pass - the derivation is the thing under test as much as the
//! boundary recovery is, per ADR-0033's "the penalty is calibrated once,
//! on the null control and the synthetic fixtures".

use sim_analysis::{
    render_world, segment, synthetic_log, world_era, EraError, EraPlan, FeatureGates,
    SyntheticSpec, FEATURE_COUNT, FEATURE_NAMES,
};
use sim_core::{DeathCause, Event, EventKind};

const WINDOW_TICKS: u64 = 500;
const MAX_SEGMENTS: usize = 6;
const POPULATION: u32 = 500;
const MAX_ENTITIES: u32 = 4_000;
/// Baseline driven-rate value (milli per 1,000 organism-ticks): "hundreds
/// to thousands", per the assignment's fixture guidance.
const BASE: i64 = 400;
/// 4x `BASE` - comfortably over the required 3x step at an injected
/// boundary.
const BOOST: i64 = 1_600;

fn gates() -> FeatureGates {
    FeatureGates {
        contest: true,
        artifact: true,
        social: true,
        ontogeny: false,
        transition: false,
    }
}

fn plan(windows: u32, penalty_milli: i128) -> EraPlan {
    EraPlan {
        window_ticks: WINDOW_TICKS,
        burn_in_ticks: 0,
        penalty_milli,
        max_segments: MAX_SEGMENTS,
        initial_organisms: 0,
        max_entities: MAX_ENTITIES,
        run_ticks: u64::from(windows) * WINDOW_TICKS,
        gates: gates(),
    }
}

/// `[births, deaths_starvation, damage_events, signals_emitted,
/// objects_created]`. The `deaths_starvation` column always mirrors
/// `births`: `synthetic_log` derives every starvation death from a
/// same-tick birth match (see its doc comment), so setting it to anything
/// else would document a number the generator never reads.
fn rates_row(births: i64, damage: i64, signals: i64, objects: i64) -> [i64; 5] {
    [births, births, damage, signals, objects]
}

/// The penalty rule every scenario below uses: run the noiseless log
/// through `world_era` once (penalty is irrelevant to feature extraction),
/// take its window feature matrix, and compare `segment`'s best
/// `true_segments - 1`-segment cost against its best `true_segments`-segment
/// cost. The gap is the *weakest* of the true boundaries' own marginal SSD
/// reduction, not the total across all of them - exact DP adds boundaries
/// in decreasing order of benefit, so this is always the hardest one to
/// justify. A quarter of that is comfortably above the zero a spurious
/// split buys (splitting an already-flat sub-segment costs nothing to
/// begin with) and comfortably below every boundary's real signal,
/// including the weakest. Dividing the *total* reduction by four instead
/// would under-calibrate for a weak boundary sitting alongside stronger
/// ones - found the hard way: the three-boundary fixture below silently
/// dropped its smallest step until this was the comparison used.
///
/// Window 0 is excluded from this calibration. `synthetic_log` puts the
/// entire founder population's `Birth` events at tick 1 of window 0 (see
/// its doc comment); `world_era` cannot tell those apart from a driven
/// birth, and window 0's first tick is integrated at population 0 (before
/// those births land), so window 0 always carries its own large, one-time
/// step relative to every later window regardless of what the fixture
/// intends to test. That is a real, correctly-computed feature of *this
/// log*, not noise to average away - which is exactly why it is excluded
/// from calibration by slicing it off rather than by pretending it is
/// zero. Every test below tolerates (and does not count) the resulting
/// founder-settlement boundary at `window == 1` for the same reason
/// ADR-0033's own null control uses a burn-in: a founding transient is a
/// real regime, and the plan here is pinned to `burn_in_ticks == 0` by the
/// assignment, so it is filtered here instead of dropped there.
fn derive_penalty(events: &[Event], windows: u32, true_segments: usize) -> i128 {
    let probe = world_era(events, &plan(windows, 0)).expect("true features");
    let matrix: Vec<[Option<i64>; FEATURE_COUNT]> = probe.windows[1..]
        .iter()
        .map(|window| window.values)
        .collect();
    let (_, cost_weaker) = segment(&matrix, 0, true_segments - 1);
    let (_, cost_true) = segment(&matrix, 0, true_segments);
    (cost_weaker - cost_true) / 4
}

/// Boundaries with the expected founder-settlement step (`window == 1`,
/// see [`derive_penalty`]) removed, so each scenario's assertions are
/// about the boundaries it actually injected.
fn injected_boundaries(era: &sim_analysis::WorldEra) -> Vec<&sim_analysis::EraBoundary> {
    era.boundaries.iter().filter(|b| b.window > 1).collect()
}

#[test]
fn a_single_injected_boundary_is_recovered_within_one_window() {
    let windows = 40u32;
    let rates: Vec<[i64; 5]> = (0..windows)
        .map(|w| {
            if w < 20 {
                rates_row(BASE, 300, 250, 200)
            } else {
                rates_row(BOOST, 300, 250, 200)
            }
        })
        .collect();
    let spec = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: POPULATION,
        rates,
    };
    let events = synthetic_log(1, &spec);

    let penalty = derive_penalty(&events, windows, 2);
    println!(
        "a_single_injected_boundary_is_recovered_within_one_window: derived penalty_milli = {penalty}"
    );
    assert!(
        penalty > 0,
        "derived penalty must be positive, got {penalty}"
    );

    let era = world_era(&events, &plan(windows, penalty)).expect("analysis");
    let found_boundaries = injected_boundaries(&era);
    assert_eq!(
        found_boundaries.len(),
        1,
        "expected exactly one injected boundary, got {:?} (all: {:?})",
        found_boundaries,
        era.boundaries
    );
    let found = i64::from(found_boundaries[0].window);
    assert!(
        (found - 20).abs() <= 1,
        "boundary at window {found}, expected within 1 of window 20"
    );
}

#[test]
fn three_boundaries_are_recovered_with_precision_and_recall_one() {
    let windows = 60u32;
    let rates: Vec<[i64; 5]> = (0..windows)
        .map(|w| {
            let (damage, signals, objects) = if w < 15 {
                (300, 250, 200)
            } else if w < 30 {
                (1_200, 250, 200)
            } else if w < 45 {
                (1_200, 1_000, 200)
            } else {
                (1_200, 1_000, 800)
            };
            rates_row(BASE, damage, signals, objects)
        })
        .collect();
    let spec = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: POPULATION,
        rates,
    };
    let events = synthetic_log(2, &spec);

    let penalty = derive_penalty(&events, windows, 4);
    println!(
        "three_boundaries_are_recovered_with_precision_and_recall_one: derived penalty_milli = {penalty}"
    );
    assert!(
        penalty > 0,
        "derived penalty must be positive, got {penalty}"
    );

    let era = world_era(&events, &plan(windows, penalty)).expect("analysis");
    let found_boundaries = injected_boundaries(&era);
    assert_eq!(
        found_boundaries.len(),
        3,
        "expected exactly three injected boundaries (precision), got {:?} (all: {:?})",
        found_boundaries,
        era.boundaries
    );
    let found: Vec<i64> = found_boundaries
        .iter()
        .map(|boundary| i64::from(boundary.window))
        .collect();
    for expected in [15_i64, 30, 45] {
        assert!(
            found.iter().any(|&window| (window - expected).abs() <= 1),
            "recall: no boundary within 1 of window {expected}, found {found:?}"
        );
    }
}

#[test]
fn a_boundary_in_one_feature_group_alone_is_found() {
    let windows = 30u32;
    let rates: Vec<[i64; 5]> = (0..windows)
        .map(|w| {
            if w < 15 {
                rates_row(BASE, 300, 250, 200)
            } else {
                rates_row(BASE, 300, 250, 800) // objects_created alone changes.
            }
        })
        .collect();
    let spec = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: POPULATION,
        rates,
    };
    let events = synthetic_log(3, &spec);

    let penalty = derive_penalty(&events, windows, 2);
    println!("a_boundary_in_one_feature_group_alone_is_found: derived penalty_milli = {penalty}");
    assert!(
        penalty > 0,
        "derived penalty must be positive, got {penalty}"
    );

    let era = world_era(&events, &plan(windows, penalty)).expect("analysis");
    let found_boundaries = injected_boundaries(&era);
    assert_eq!(
        found_boundaries.len(),
        1,
        "expected exactly one injected boundary, got {:?} (all: {:?})",
        found_boundaries,
        era.boundaries
    );
    let found = i64::from(found_boundaries[0].window);
    assert!(
        (found - 15).abs() <= 1,
        "boundary at window {found}, expected within 1 of window 15"
    );
}

#[test]
fn the_synthetic_null_yields_no_boundary() {
    let windows = 40u32;
    let rates: Vec<[i64; 5]> = (0..windows)
        .map(|_| rates_row(BASE, 300, 250, 200))
        .collect();
    let spec = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: POPULATION,
        rates,
    };
    let events = synthetic_log(4, &spec);

    // Windows 1..40 hold exactly constant rates - the actual null - so any
    // partition of them alone has zero SSD and a positive penalty strictly
    // favours fewer segments, no tie-break needed. Window 0 is not part of
    // that null (see `derive_penalty`'s comment: the founder population's
    // Birth events make it its own one-time step), so the penalty here is
    // derived from the one real structure this log has - the gap between
    // one segment and the best two-segment split, which is exactly the
    // founder step - doubled so that splitting it off is never worth it.
    let probe = world_era(&events, &plan(windows, 0)).expect("true features");
    let matrix: Vec<[Option<i64>; FEATURE_COUNT]> =
        probe.windows.iter().map(|window| window.values).collect();
    let (_, cost_one) = segment(&matrix, 0, 1);
    let (_, cost_two) = segment(&matrix, 0, 2);
    let founder_step = cost_one - cost_two;
    let penalty = founder_step * 2 + 1;
    println!(
        "the_synthetic_null_yields_no_boundary: founder_step={founder_step} derived penalty_milli = {penalty}"
    );

    let era = world_era(&events, &plan(windows, penalty)).expect("analysis");
    assert_eq!(era.segments, 1);
    assert!(era.boundaries.is_empty());

    let rendered_first = render_world("null", 4, 0xdead_beef_cafe_1234, 13, &era, false);
    let rendered_second = render_world("null", 4, 0xdead_beef_cafe_1234, 13, &era, false);
    assert_eq!(
        rendered_first, rendered_second,
        "rendering the same result twice must be byte-identical"
    );
    assert!(rendered_first.contains("no segments above threshold"));
}

/// Exhaustive reference partitioner: every boundary-position subset with
/// at most `max_segments` segments, scored with the same cost formula
/// ADR-0033 specifies, tie-broken toward fewer segments and then toward
/// the lexicographically earliest boundary vector. `segment` is checked
/// against this on every trial below - there is no other source of truth
/// for "the exact optimum" than trying all of them.
fn brute_force_segment(
    matrix: &[[Option<i64>; FEATURE_COUNT]],
    penalty_milli: i128,
    max_segments: usize,
) -> (Vec<usize>, i128) {
    let windows = matrix.len();
    let cost = |start: usize, end: usize| -> i128 {
        let mut total = 0i128;
        for feature in 0..FEATURE_COUNT {
            let mut n = 0i128;
            let mut sum = 0i128;
            let mut sumsq = 0i128;
            for window in matrix.iter().take(end).skip(start) {
                if let Some(value) = window[feature] {
                    let value = i128::from(value);
                    n += 1;
                    sum += value;
                    sumsq += value * value;
                }
            }
            if n > 0 {
                total += sumsq - (sum * sum) / n;
            }
        }
        total
    };

    let position_count = windows.saturating_sub(1);
    let mut best: Option<(i128, Vec<usize>)> = None;
    for mask in 0u32..(1u32 << position_count) {
        let boundaries: Vec<usize> = (0..position_count)
            .filter(|position| mask & (1 << position) != 0)
            .map(|position| position + 1)
            .collect();
        let segments = boundaries.len() + 1;
        if segments > max_segments {
            continue;
        }
        let mut bounds = vec![0usize];
        bounds.extend(boundaries.iter().copied());
        bounds.push(windows);
        let mut total_cost = 0i128;
        for window in 0..segments {
            total_cost += cost(bounds[window], bounds[window + 1]);
        }
        let objective = total_cost + penalty_milli * (segments as i128 - 1);
        let is_better = match &best {
            None => true,
            Some((best_objective, best_boundaries)) => {
                objective < *best_objective
                    || (objective == *best_objective && boundaries.len() < best_boundaries.len())
                    || (objective == *best_objective
                        && boundaries.len() == best_boundaries.len()
                        && boundaries < *best_boundaries)
            }
        };
        if is_better {
            best = Some((objective, boundaries));
        }
    }
    let (objective, boundaries) = best.expect("at least the whole-range partition exists");
    (boundaries, objective)
}

/// Splitmix64-style keyed hash, local to this test file (the crate's own
/// copy in `era.rs` is private, and this one only needs to disagree with
/// it, not match it - any deterministic keyed generator will do for
/// building adversarial small inputs).
fn test_hash(a: u64, b: u32, c: u32, d: u32) -> u64 {
    let mut z = a
        ^ u64::from(b).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(c).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ u64::from(d).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[test]
fn segmentation_matches_brute_force_on_small_inputs() {
    for trial in 0..20u64 {
        let window_count = 6 + (trial % 3) as usize; // 6..=8
        let max_segments = 2 + (trial % 3) as usize;
        // Never zero: a zero penalty would let the DP's fewer-segments
        // tie-break alone decide most of these small, low-cardinality
        // cases, which would make the comparison weak evidence that the
        // penalty term itself is wired in correctly.
        let penalty_milli: i128 = [1, 3, 7, 15][(trial % 4) as usize];

        let mut matrix = vec![[None; FEATURE_COUNT]; window_count];
        for (window, slot) in matrix.iter_mut().enumerate() {
            for feature in 0..2 {
                // A small value range (0..=5) makes coincidental ties
                // between candidate partitions common, which is exactly
                // what exercises the tie-break rule rather than only the
                // objective comparison.
                let value = (test_hash(trial, window as u32, feature as u32, 0) % 6) as i64;
                slot[feature] = Some(value);
            }
        }

        let expected = brute_force_segment(&matrix, penalty_milli, max_segments);
        let actual = segment(&matrix, penalty_milli, max_segments);
        assert_eq!(
            actual, expected,
            "trial {trial}: window_count={window_count} max_segments={max_segments} penalty={penalty_milli}"
        );
    }
}

#[test]
fn absent_features_do_not_move_the_cost() {
    let windows = 20u32;
    let rates: Vec<[i64; 5]> = (0..windows)
        .map(|w| {
            if w < 10 {
                rates_row(BASE, 300, 250, 200)
            } else {
                // Births step up; objects_created (200) stays constant
                // throughout, present or not.
                rates_row(BOOST, 300, 250, 200)
            }
        })
        .collect();
    let spec = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: POPULATION,
        rates,
    };
    let events = synthetic_log(6, &spec);

    let mut gates_on = gates();
    gates_on.artifact = true;
    let mut gates_off = gates();
    gates_off.artifact = false;

    let penalty = 10_000;
    let plan_on = EraPlan {
        gates: gates_on,
        ..plan(windows, penalty)
    };
    let plan_off = EraPlan {
        gates: gates_off,
        ..plan(windows, penalty)
    };

    let era_on = world_era(&events, &plan_on).expect("analysis, artifact gate on");
    let era_off = world_era(&events, &plan_off).expect("analysis, artifact gate off");

    let boundaries_on: Vec<u32> = era_on.boundaries.iter().map(|b| b.window).collect();
    let boundaries_off: Vec<u32> = era_off.boundaries.iter().map(|b| b.window).collect();
    assert_eq!(
        boundaries_on, boundaries_off,
        "gating a feature whose rate never moves must not move the boundaries"
    );

    let rendered_off = render_world("off", 6, 0, 1, &era_off, true);
    assert!(
        rendered_off.contains("objects_created=absent"),
        "artifact gate off must print the object features as absent:\n{rendered_off}"
    );
    let rendered_on = render_world("on", 6, 0, 1, &era_on, true);
    assert!(
        !rendered_on.contains("objects_created=absent"),
        "artifact gate on must print a measured value:\n{rendered_on}"
    );
}

#[test]
fn population_is_reconstructed_and_a_negative_one_is_refused() {
    let bare_plan = EraPlan {
        window_ticks: 10,
        burn_in_ticks: 0,
        penalty_milli: 0,
        max_segments: 1,
        initial_organisms: 0,
        max_entities: 100,
        run_ticks: 10,
        gates: FeatureGates::default(),
    };

    // 3 births, then 4 deaths at a later tick: negative under any tick
    // ordering, the plain case.
    let events = vec![
        Event {
            tick: 1,
            kind: EventKind::Birth {
                id: 1,
                parent_id: 0,
            },
        },
        Event {
            tick: 1,
            kind: EventKind::Birth {
                id: 2,
                parent_id: 0,
            },
        },
        Event {
            tick: 1,
            kind: EventKind::Birth {
                id: 3,
                parent_id: 0,
            },
        },
        Event {
            tick: 5,
            kind: EventKind::Death {
                id: 1,
                cause: DeathCause::Starvation,
            },
        },
        Event {
            tick: 5,
            kind: EventKind::Death {
                id: 2,
                cause: DeathCause::Starvation,
            },
        },
        Event {
            tick: 5,
            kind: EventKind::Death {
                id: 3,
                cause: DeathCause::Starvation,
            },
        },
        Event {
            tick: 5,
            kind: EventKind::Death {
                id: 4,
                cause: DeathCause::Starvation,
            },
        },
    ];
    assert_eq!(
        world_era(&events, &bare_plan),
        Err(EraError::NegativePopulation { tick: 5 })
    );

    // A same-tick mix of births and deaths is one net delta, not deaths
    // applied before births: 3 alive, then +2/-4 in the same tick nets to
    // -2 (population 1, never negative), even though the deaths alone
    // would outnumber who is there before the births land.
    let mixed_events = vec![
        Event {
            tick: 1,
            kind: EventKind::Birth {
                id: 1,
                parent_id: 0,
            },
        },
        Event {
            tick: 1,
            kind: EventKind::Birth {
                id: 2,
                parent_id: 0,
            },
        },
        Event {
            tick: 1,
            kind: EventKind::Birth {
                id: 3,
                parent_id: 0,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::Birth {
                id: 4,
                parent_id: 0,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::Birth {
                id: 5,
                parent_id: 0,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::Death {
                id: 10,
                cause: DeathCause::Starvation,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::Death {
                id: 11,
                cause: DeathCause::Starvation,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::Death {
                id: 12,
                cause: DeathCause::Starvation,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::Death {
                id: 13,
                cause: DeathCause::Starvation,
            },
        },
    ];
    let mixed_result = world_era(&mixed_events, &bare_plan);
    assert!(
        mixed_result.is_ok(),
        "a same-tick net delta must not be treated as sequential deaths-then-births: {mixed_result:?}"
    );

    // Burn-in of 2 windows: window_ticks=10, burn_in_ticks=21 drops
    // windows 0 (start 1) and 1 (start 11), keeps window 2 (start 21)
    // onward, with original indices preserved.
    let mut events_five_windows = Vec::new();
    for window in 0..5u64 {
        events_five_windows.push(Event {
            tick: window * 10 + 1,
            kind: EventKind::Birth {
                id: window + 1,
                parent_id: 0,
            },
        });
    }
    let burn_plan = EraPlan {
        window_ticks: 10,
        burn_in_ticks: 21,
        penalty_milli: 0,
        max_segments: 1,
        initial_organisms: 0,
        max_entities: 100,
        run_ticks: 50,
        gates: FeatureGates::default(),
    };
    let era = world_era(&events_five_windows, &burn_plan).expect("analysis");
    assert_eq!(era.windows_dropped_burn_in, 2);
    assert_eq!(era.windows.len(), 3);
    assert_eq!(era.windows[0].index, 2);
    assert_eq!(era.windows[1].index, 3);
    assert_eq!(era.windows[2].index, 4);
}

#[test]
fn rates_are_per_organism_tick_not_per_window() {
    let windows = 10u32;
    // Every rate here is a multiple of 8: with `window_ticks = 500`,
    // `organism_ticks` is `population * 500`, which is exactly divisible
    // by `1_000_000 / gcd(500, 1_000_000)` for both `population` values
    // below only when the implied event count has no fractional part to
    // round - 300 and 250 do not clear that bar and reproduce a different
    // rounding remainder at population 250 versus 500, which looks like a
    // per-window rate but is really `milli_rate_to_count`'s round-half-up
    // landing on different integers. The property under test is about
    // organism-tick normalization, not about rounding, so the fixture
    // avoids the ambiguity instead of asserting through it.
    let rates: Vec<[i64; 5]> = (0..windows)
        .map(|_| rates_row(BASE, 320, 256, 200))
        .collect();

    let spec_small = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: 250,
        rates: rates.clone(),
    };
    let spec_big = SyntheticSpec {
        window_ticks: WINDOW_TICKS,
        windows,
        population: 500,
        rates,
    };

    let events_small = synthetic_log(7, &spec_small);
    let events_big = synthetic_log(7, &spec_big);

    let era_small = world_era(&events_small, &plan(windows, 0)).expect("small population");
    let era_big = world_era(&events_big, &plan(windows, 0)).expect("big population");

    assert_eq!(era_small.windows.len(), era_big.windows.len());
    // Window 0 is skipped: it carries the founder population's one-time
    // `Birth` dump (see `derive_penalty`'s comment above), which is a
    // fixed *count* of events, not a rate scaled by population - so unlike
    // every later window, its measured rate is population-size-dependent
    // by construction and is not what this property is about.
    for (window_small, window_big) in era_small
        .windows
        .iter()
        .zip(era_big.windows.iter())
        .filter(|(window, _)| window.index != 0)
    {
        for (feature, name) in FEATURE_NAMES.iter().enumerate() {
            if feature == 5 {
                // `population`, the one level feature that scales with
                // population itself rather than being normalized by it.
                continue;
            }
            assert_eq!(
                window_small.values[feature], window_big.values[feature],
                "feature {name} differs between population sizes at window {}",
                window_small.index
            );
        }
    }
}

/// Added during the mutation-testing pass (AGENTS.md standing rule 1):
/// mutation (iii) - flipping the segment-count tie rule from "fewer" to
/// "more" - survived every test above unmutated, because none of their
/// derived or fixed penalties happen to land on an exact objective tie
/// across different segment counts (a much rarer coincidence than a tie
/// between two boundary sets of the *same* length, which the brute-force
/// trials do hit). This test manufactures both kinds of tie by hand so
/// the rule itself, not just its usual practical irrelevance, is checked.
#[test]
fn the_tie_rule_prefers_fewer_segments_then_the_earliest_boundary_set() {
    let one_feature = |value: i64| {
        let mut row = [None; FEATURE_COUNT];
        row[0] = Some(value);
        row
    };

    // Three windows at 0, three at 10: the whole-range SSD is exactly
    // 150 (mean 5, sum 30, sum-sq 300, 300 - 30^2/6 = 150) and the best
    // two-segment split has SSD 0, so at penalty 150 the one- and
    // two-segment objectives are both exactly 150. Only "fewer segments"
    // breaks the tie.
    let stepped = vec![
        one_feature(0),
        one_feature(0),
        one_feature(0),
        one_feature(10),
        one_feature(10),
        one_feature(10),
    ];
    let outcome = segment(&stepped, 150, 2);
    assert_eq!(
        outcome,
        (Vec::new(), 150),
        "an exact objective tie between one and two segments must resolve to fewer segments"
    );

    // 0, 10, 10, 0: splitting after window 1 ([0] | [10,10,0]) and after
    // window 3 ([0,10,10] | [0]) both leave a within-segment SSD of 67
    // (sum 20, sum-sq 200, 200 - 20^2/3 = 67, by hand on either side).
    // Only "earliest boundary set" breaks the tie.
    let symmetric = vec![
        one_feature(0),
        one_feature(10),
        one_feature(10),
        one_feature(0),
    ];
    let outcome = segment(&symmetric, 0, 2);
    assert_eq!(
        outcome,
        (vec![1], 67),
        "an exact tie between two same-length boundary sets must resolve to the \
         lexicographically earliest one"
    );
}

/// Added during the mutation-testing pass: mutation (v) - treating an
/// absent (`None`) reading as a measured zero - survives every test above
/// unmutated, because `world_era` only ever hands `segment` a feature that
/// is `None` in *every* window or `Some` in every window (gates are
/// constant for the whole plan), and a feature absent everywhere
/// contributes nothing either way. `segment`'s own contract is more
/// general than that - a caller may hand it a feature that is present in
/// some windows and not others - so this test manufactures exactly that
/// mixed case by hand, bypassing `world_era` entirely.
#[test]
fn a_partly_absent_feature_column_does_not_manufacture_a_boundary() {
    // Windows 0-2 have no reading at all for feature 0; windows 3-5 all
    // read exactly 100. Excluding the absent windows leaves feature 0
    // perfectly flat wherever it is defined (zero SSD for any partition,
    // so the one-segment tie-break wins at any positive penalty).
    // Treating the absent windows as a measured zero turns this into a
    // real step from 0 to 100 and manufactures a boundary the data never
    // supports.
    let mut matrix = vec![[None; FEATURE_COUNT]; 6];
    for window in matrix.iter_mut().skip(3) {
        window[0] = Some(100);
    }
    let outcome = segment(&matrix, 1_000, 2);
    assert_eq!(
        outcome,
        (Vec::new(), 0),
        "an absent reading must be excluded from the cost, not scored as a measured zero"
    );
}
