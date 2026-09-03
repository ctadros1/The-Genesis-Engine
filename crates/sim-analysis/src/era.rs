//! Offline era detection (`lifesim-era-v1`), Phase 17, ADR-0033.
//!
//! **Analysis observes; it never instructs** (ADR-0016, restated concretely
//! for this detector in ADR-0033's "hard rule"). This module reads a
//! decoded event log and a run's plan, holds no world handle, draws from no
//! `RngSystem` stream, and computes a pure function of its inputs: the same
//! log and plan give byte-identical reports in two processes (C17.3). There
//! is no `era` field in world state and nothing here feeds a rule - a
//! `sim-core` -> `sim-analysis` dependency is a compile error, checked by
//! `tests/dependency_direction.rs`, not a review finding (C17.2).
//!
//! A segment is a segment (ADR-0033, "the hard rule, restated as
//! structure"). Nothing here names one after a human historical period.
//!
//! # What the log can supply
//!
//! The twenty-two features are exactly what the event schema can produce
//! today (ADR-0033's feature table), each a rate per 1,000 organism-ticks
//! in milli-units - a count divided by the window's integrated population,
//! so a crowded world and a sparse one are comparable - except the two
//! level features (`population`, a fraction of `max_entities`; and
//! `signal_energy`, milli per emission). A feature whose owning mechanism
//! is off in the run's config is reported `absent` (`None`), never `0`:
//! zero is a measured rate, absence is "this run's config could not have
//! produced this quantity at all", and conflating the two would make a
//! disabled mechanism look like a mechanism that fired zero times.
//!
//! # Segmentation
//!
//! [`segment`] is the exact bounded dynamic-programming change-point
//! partition ADR-0033 specifies: integer SSD in `i128`, a fixed per-segment
//! penalty, ties broken toward fewer segments and then toward the
//! lexicographically earliest boundary set, so two builds - or two
//! processes - agree on which partition is "the" optimum rather than
//! merely "an" optimum. "A segment above threshold" is a boundary the
//! penalty admits; a world whose best partition is one segment reports
//! `no segments above threshold`, C17.7's explicit negative result, printed
//! rather than left as silence.
//!
//! # Validation
//!
//! [`synthetic_log`] is the versioned generator (`SYNTHETIC_FIXTURE_VERSION`)
//! that builds ground-truth event logs with piecewise-constant rates and
//! known injected boundaries, keyed on a caller-supplied seed through a
//! tiny splitmix64 hash rather than any RNG stream or `HashMap` iteration -
//! C17.4's synthetic ground truth has to be exactly reproducible, not merely
//! plausible.

use sim_core::{DeathCause, Event, EventKind};
use std::collections::{BTreeMap, VecDeque};

/// This detector's identity, recorded in every report. Never in the config
/// hash (ADR-0033): an analysis version can never affect a world.
pub const ERA_VERSION: &str = "lifesim-era-v1";

/// Version of the synthetic ground-truth generator below. Bumped whenever
/// [`synthetic_log`]'s event shape or rate semantics change, so a fixture
/// built under an old version is never silently read as a new one.
pub const SYNTHETIC_FIXTURE_VERSION: u16 = 1;

/// Number of features in the vector every window reduces to.
pub const FEATURE_COUNT: usize = 22;

/// Feature names, in vector order. Used for both the machine-readable
/// `--features` tail and the boundary `delta` lines, so a report and this
/// array can never drift out of sync with each other.
pub const FEATURE_NAMES: [&str; FEATURE_COUNT] = [
    "births",
    "deaths_starvation",
    "deaths_old_age",
    "deaths_damage",
    "deaths_hazard",
    "population",
    "pair_rejections",
    "damage_events",
    "carcasses_consumed",
    "objects_created",
    "objects_destroyed",
    "objects_picked_up",
    "objects_placed",
    "objects_struck",
    "terrain_struck",
    "objects_combined",
    "objects_consumed",
    "object_actions_refused",
    "signals_emitted",
    "signal_energy",
    "growth_completions",
    "materializations",
];

/// Which feature groups the run's config could have produced. A group that
/// is off is `absent` (`None`) in every window, never zero - ADR-0033's
/// distinction between "did not happen" and "could not happen".
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FeatureGates {
    /// Gates `deaths_damage`, `damage_events`, `carcasses_consumed`.
    pub contest: bool,
    /// Gates the nine object features (`objects_created` through
    /// `object_actions_refused`).
    pub artifact: bool,
    /// Gates `signals_emitted`, `signal_energy`.
    pub social: bool,
    /// Gates `growth_completions`.
    pub ontogeny: bool,
    /// Gates `materializations`.
    pub transition: bool,
}

/// Which feature index a group gates, mirroring the table in ADR-0033's
/// "Recorded deviations" section. Demography features and `pair_rejections`
/// have no gate: they are always present, regardless of config.
fn feature_enabled(index: usize, gates: FeatureGates) -> bool {
    match index {
        3 | 7 | 8 => gates.contest,
        9..=17 => gates.artifact,
        18 | 19 => gates.social,
        20 => gates.ontogeny,
        21 => gates.transition,
        _ => true,
    }
}

/// Parameters that must be fixed before a confirmatory read: window width,
/// burn-in, the segmentation penalty and segment bound, and the population
/// baseline the report reconstructs from (`initial_organisms`) and
/// normalizes against (`max_entities`). Every field is echoed verbatim in
/// the report so a reader can check none of them were chosen after seeing
/// the data (ADR-0033, "the threshold's meaning").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EraPlan {
    pub window_ticks: u64,
    /// Windows whose start tick is before this are dropped from
    /// segmentation and from the reported window list (but still counted,
    /// and still contribute to population continuity) - the founder
    /// settlement transient is a real regime, not noise, and a detector
    /// that found it would be right, not wrong (ADR-0033's null-control
    /// rationale, applied here to every plan).
    pub burn_in_ticks: u64,
    /// What an SSD reduction must exceed for a boundary to exist. This is
    /// the only threshold in the design; there is no second significance
    /// test layered on top of it.
    pub penalty_milli: i128,
    pub max_segments: usize,
    pub initial_organisms: u32,
    pub max_entities: u32,
    pub run_ticks: u64,
    pub gates: FeatureGates,
}

/// One window's feature vector. `index` is the window's position in the
/// *original* (pre-burn-in) sequence, so a boundary's window number always
/// means the same window across two reports built with different burn-ins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowFeatures {
    pub index: u32,
    pub start_tick: u64,
    pub values: [Option<i64>; FEATURE_COUNT],
}

/// One boundary the penalty admitted: the segment mean shift it produced,
/// per present feature. `window`/`tick` name the first window of the
/// segment that starts here, using the original (pre-burn-in) index so a
/// boundary can be located in the full log.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Boundary {
    pub window: u32,
    pub tick: u64,
    /// `mean(after) - mean(before)`, integer division, `None` where the
    /// feature is absent under this plan's gates.
    pub deltas: [Option<i64>; FEATURE_COUNT],
}

/// One world's era detection result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldEra {
    /// Windows after burn-in, in ascending original index.
    pub windows: Vec<WindowFeatures>,
    pub windows_dropped_burn_in: u32,
    pub boundaries: Vec<Boundary>,
    /// `boundaries.len() + 1`.
    pub segments: usize,
    /// The chosen partition's total objective: `sum(SSD) + penalty *
    /// (segments - 1)`, exact in `i128`.
    pub cost: i128,
}

/// Why a log and plan could not be reduced to an era result. Every variant
/// is a value, never a panic (fuzz-minded: a torn or foreign log is a typed
/// refusal, not undefined behaviour).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EraError {
    /// Zero windows survive burn-in (including the degenerate case of zero
    /// windows before it: `run_ticks == 0`).
    NoWindows,
    /// Population integration went negative at this tick - more deaths
    /// than the reconstructed population had to give, so the log is torn,
    /// foreign, or paired with the wrong `initial_organisms`.
    NegativePopulation {
        tick: u64,
    },
    ZeroWindow,
    ZeroMaxSegments,
    /// An event's tick exceeds the plan's `run_ticks`: the log and the plan
    /// do not describe the same run.
    EventPastHorizon {
        tick: u64,
    },
}

fn window_of(tick: u64, window_ticks: u64) -> usize {
    if tick == 0 {
        0
    } else {
        ((tick - 1) / window_ticks) as usize
    }
}

fn window_start(index: usize, window_ticks: u64) -> u64 {
    index as u64 * window_ticks + 1
}

fn window_end(index: usize, window_ticks: u64, run_ticks: u64) -> u64 {
    ((index as u64 + 1) * window_ticks).min(run_ticks)
}

/// Add `population * (end - start + 1)` to every window overlapping
/// `[start, end]`, splitting the range at window boundaries. `start` and
/// `end` are ticks, one-based; a no-op range (`start == 0` or `start >
/// end`) is silently ignored, since that is exactly what "nothing to
/// integrate yet" looks like at the head of the run.
fn add_organism_ticks(
    organism_ticks: &mut [i128],
    start: u64,
    end: u64,
    population: i64,
    window_ticks: u64,
    run_ticks: u64,
) {
    if start == 0 || start > end {
        return;
    }
    let mut cursor = start;
    let mut window = window_of(start, window_ticks);
    while cursor <= end && window < organism_ticks.len() {
        let w_end = window_end(window, window_ticks, run_ticks).min(end);
        if w_end >= cursor {
            let length = w_end - cursor + 1;
            organism_ticks[window] += i128::from(population) * i128::from(length);
            cursor = w_end + 1;
        }
        window += 1;
    }
}

/// Reduce an event log to its per-window feature vectors and change-point
/// segmentation (ADR-0033).
///
/// Population is reconstructed from the log itself - `initial_organisms`
/// plus every `Birth`, `PairedBirth` and `Materialized`, minus every
/// `Death` - because that is the only population series that is an
/// arithmetic fact about this exact log rather than an assumption about
/// it. A log that would drive it negative is refused rather than clamped:
/// clamping would silently hide a torn or foreign log as a sparse world.
pub fn world_era(events: &[Event], plan: &EraPlan) -> Result<WorldEra, EraError> {
    if plan.window_ticks == 0 {
        return Err(EraError::ZeroWindow);
    }
    if plan.max_segments == 0 {
        return Err(EraError::ZeroMaxSegments);
    }
    for event in events {
        if event.tick > plan.run_ticks {
            return Err(EraError::EventPastHorizon { tick: event.tick });
        }
    }

    let num_windows = plan.run_ticks.div_ceil(plan.window_ticks) as usize;
    if num_windows == 0 {
        // `run_ticks == 0`: no window can exist, burn-in or not.
        return Err(EraError::NoWindows);
    }

    // Population integration: every tick where the population *could*
    // change, in tick order, applying each tick's net delta once (Birth,
    // PairedBirth and Materialized are population entries; Death is an
    // exit). The population active "during" any tick between two such
    // change points is whatever the previous change point left it at.
    let mut delta_by_tick: BTreeMap<u64, i64> = BTreeMap::new();
    for event in events {
        match event.kind {
            EventKind::Birth { .. }
            | EventKind::PairedBirth { .. }
            | EventKind::Materialized { .. } => {
                *delta_by_tick.entry(event.tick).or_insert(0) += 1;
            }
            EventKind::Death { .. } => {
                *delta_by_tick.entry(event.tick).or_insert(0) -= 1;
            }
            _ => {}
        }
    }

    let mut organism_ticks = vec![0i128; num_windows];
    let mut population: i64 = i64::from(plan.initial_organisms);
    let mut cursor: u64 = 1;
    for (&tick, &delta) in &delta_by_tick {
        // Fill through `tick` itself (inclusive) at the pre-tick
        // population, matching "the population during tick t is the value
        // before tick t's events" exactly - `tick` is a change point, not
        // a gap between change points, and it still needs its own
        // organism-tick contribution counted once, even when its net
        // delta is zero (a same-tick birth matched by a death, say).
        // Filling only up to `tick - 1` here would silently drop every
        // tick that has any population-affecting event from the
        // integration entirely.
        if tick >= 1 && tick >= cursor {
            add_organism_ticks(
                &mut organism_ticks,
                cursor,
                tick.min(plan.run_ticks),
                population,
                plan.window_ticks,
                plan.run_ticks,
            );
        }
        population += delta;
        if population < 0 {
            return Err(EraError::NegativePopulation { tick });
        }
        if tick >= 1 {
            cursor = tick + 1;
        }
    }
    if cursor <= plan.run_ticks {
        add_organism_ticks(
            &mut organism_ticks,
            cursor,
            plan.run_ticks,
            population,
            plan.window_ticks,
            plan.run_ticks,
        );
    }

    // Raw per-window counts, straight from the log. `births` folds in
    // `Materialized` (ADR-0033's feature table: "births (paired + asexual
    // + materialized)") in addition to feeding `materializations` in its
    // own right - the field-to-individual transition is both a population
    // entry and, when the transition mechanism is on, its own signal.
    let mut raw = vec![[0i64; FEATURE_COUNT]; num_windows];
    let mut signal_cost_sum = vec![0i128; num_windows];
    for event in events {
        let window = window_of(event.tick, plan.window_ticks).min(num_windows - 1);
        let bucket = &mut raw[window];
        match event.kind {
            EventKind::Birth { .. } | EventKind::PairedBirth { .. } => bucket[0] += 1,
            EventKind::Materialized { .. } => {
                bucket[0] += 1;
                bucket[21] += 1;
            }
            EventKind::Death { cause, .. } => match cause {
                DeathCause::Starvation => bucket[1] += 1,
                DeathCause::OldAge => bucket[2] += 1,
                DeathCause::Damage => bucket[3] += 1,
                DeathCause::Senescence | DeathCause::Extrinsic => bucket[4] += 1,
            },
            EventKind::PairRejected { .. } => bucket[6] += 1,
            EventKind::Damage { .. } => bucket[7] += 1,
            EventKind::CarcassConsumed { .. } => bucket[8] += 1,
            EventKind::ObjectCreated { .. } => bucket[9] += 1,
            EventKind::ObjectDestroyed { .. } => bucket[10] += 1,
            EventKind::ObjectPickedUp { .. } => bucket[11] += 1,
            EventKind::ObjectReleased { placed, .. } => {
                if placed {
                    bucket[12] += 1;
                }
            }
            EventKind::ObjectStruck { .. } => bucket[13] += 1,
            EventKind::TerrainStruck { .. } => bucket[14] += 1,
            EventKind::ObjectCombined { .. } => bucket[15] += 1,
            EventKind::ObjectConsumed { .. } => bucket[16] += 1,
            EventKind::ObjectActionRefused { .. } => bucket[17] += 1,
            EventKind::SignalEmitted { cost_milli, .. } => {
                bucket[18] += 1;
                signal_cost_sum[window] += i128::from(cost_milli);
            }
            EventKind::GrowthCompleted { .. } => bucket[20] += 1,
            _ => {}
        }
    }

    let mut all_windows: Vec<WindowFeatures> = Vec::with_capacity(num_windows);
    for window in 0..num_windows {
        let organism_ticks_here = organism_ticks[window];
        let rate = |count: i64| -> i64 {
            if organism_ticks_here == 0 {
                0
            } else {
                ((i128::from(count) * 1_000 * 1_000) / organism_ticks_here) as i64
            }
        };
        let mut values = [None; FEATURE_COUNT];
        for (feature, slot) in values.iter_mut().enumerate() {
            if !feature_enabled(feature, plan.gates) {
                continue;
            }
            *slot = Some(match feature {
                5 => {
                    // Level feature: mean population over the window's
                    // ticks, as milli of `max_entities`. Two divisions in
                    // this order (mean first, then scale), per ADR-0033.
                    let start = window_start(window, plan.window_ticks);
                    let end = window_end(window, plan.window_ticks, plan.run_ticks);
                    let n_ticks = i128::from(end.saturating_sub(start) + 1);
                    let mean_population = if n_ticks == 0 {
                        0
                    } else {
                        organism_ticks_here / n_ticks
                    };
                    if plan.max_entities == 0 {
                        0
                    } else {
                        (mean_population * 1_000 / i128::from(plan.max_entities)) as i64
                    }
                }
                19 => {
                    // Level feature: milli per emission, not a rate.
                    let emissions = raw[window][18];
                    if emissions == 0 {
                        0
                    } else {
                        (signal_cost_sum[window] / i128::from(emissions)) as i64
                    }
                }
                other => rate(raw[window][other]),
            });
        }
        all_windows.push(WindowFeatures {
            index: window as u32,
            start_tick: window_start(window, plan.window_ticks),
            values,
        });
    }

    let windows_dropped_burn_in = all_windows
        .iter()
        .filter(|window| window.start_tick < plan.burn_in_ticks)
        .count() as u32;
    let kept: Vec<WindowFeatures> = all_windows
        .into_iter()
        .filter(|window| window.start_tick >= plan.burn_in_ticks)
        .collect();
    if kept.is_empty() {
        return Err(EraError::NoWindows);
    }

    let feature_matrix: Vec<[Option<i64>; FEATURE_COUNT]> =
        kept.iter().map(|window| window.values).collect();
    let (local_boundaries, cost) = segment(&feature_matrix, plan.penalty_milli, plan.max_segments);

    let mut segment_bounds = vec![0usize];
    segment_bounds.extend(local_boundaries.iter().copied());
    segment_bounds.push(kept.len());

    let mean_of = |range: std::ops::Range<usize>, feature: usize| -> i64 {
        let n = range.len() as i128;
        if n == 0 {
            return 0;
        }
        let sum: i128 = feature_matrix[range]
            .iter()
            .map(|window| i128::from(window[feature].unwrap_or(0)))
            .sum();
        (sum / n) as i64
    };

    let mut boundaries = Vec::with_capacity(local_boundaries.len());
    for (position, &local_index) in local_boundaries.iter().enumerate() {
        let before = segment_bounds[position]..segment_bounds[position + 1];
        let after = segment_bounds[position + 1]..segment_bounds[position + 2];
        let mut deltas = [None; FEATURE_COUNT];
        for (feature, slot) in deltas.iter_mut().enumerate() {
            if feature_enabled(feature, plan.gates) {
                *slot = Some(mean_of(after.clone(), feature) - mean_of(before.clone(), feature));
            }
        }
        boundaries.push(Boundary {
            window: kept[local_index].index,
            tick: kept[local_index].start_tick,
            deltas,
        });
    }

    Ok(WorldEra {
        windows: kept,
        windows_dropped_burn_in,
        segments: local_boundaries.len() + 1,
        boundaries,
        cost,
    })
}

/// Exact optimal change-point partitioning (ADR-0033). Minimizes
/// `sum(SSD) + penalty_milli * (segments - 1)` over partitions of at most
/// `max_segments` segments, `SSD` computed per present feature in `i128` as
/// `sum(x^2) - floor(sum(x)^2 / n)` and summed across features - a feature
/// that is `None` in every window (an absent group) contributes nothing,
/// so a gate a run never enabled can never move a boundary (mutation
/// obligation (v)).
///
/// Ties are broken toward fewer segments, then toward the
/// lexicographically earliest boundary set, both stated so two builds
/// agree on which optimum is "the" answer. Returns the boundary window
/// indices - the first window of each segment after the first - and the
/// winning objective.
#[allow(clippy::needless_range_loop)]
// Every range loop below indexes two DP tables at once (the loop variable
// plus a `k`, `k - 1` or fixed endpoint from the enclosing scope) -
// `dpf[k - 1][i]` and `dpf[k][j]` in the same body, say - which is exactly
// the shape `.iter().enumerate()` cannot express; clippy's mechanical
// rewrite for this lint indexes the wrong axis for every instance here.
pub fn segment(
    features: &[[Option<i64>; FEATURE_COUNT]],
    penalty_milli: i128,
    max_segments: usize,
) -> (Vec<usize>, i128) {
    let windows = features.len();
    if windows == 0 || max_segments == 0 {
        return (Vec::new(), 0);
    }
    let max_k = max_segments.min(windows);

    // Prefix sums per feature, `None` entries excluded from count, sum and
    // sum-of-squares alike so an absent feature's segments carry `n == 0`
    // and contribute zero cost, never a spurious pull toward the mean of
    // whatever `unwrap_or` would have supplied.
    let mut prefix_count = vec![[0i64; FEATURE_COUNT]; windows + 1];
    let mut prefix_sum = vec![[0i128; FEATURE_COUNT]; windows + 1];
    let mut prefix_sumsq = vec![[0i128; FEATURE_COUNT]; windows + 1];
    for window in 0..windows {
        prefix_count[window + 1] = prefix_count[window];
        prefix_sum[window + 1] = prefix_sum[window];
        prefix_sumsq[window + 1] = prefix_sumsq[window];
        for feature in 0..FEATURE_COUNT {
            if let Some(value) = features[window][feature] {
                let value = i128::from(value);
                prefix_count[window + 1][feature] += 1;
                prefix_sum[window + 1][feature] += value;
                prefix_sumsq[window + 1][feature] += value * value;
            }
        }
    }

    let cost = |i: usize, j: usize| -> i128 {
        let mut total = 0i128;
        for feature in 0..FEATURE_COUNT {
            let n = prefix_count[j][feature] - prefix_count[i][feature];
            if n == 0 {
                continue;
            }
            let sum = prefix_sum[j][feature] - prefix_sum[i][feature];
            let sumsq = prefix_sumsq[j][feature] - prefix_sumsq[i][feature];
            total += sumsq - (sum * sum) / i128::from(n);
        }
        total
    };

    const INF: i128 = i128::MAX / 4;

    // Forward DP: `dpf[k][j]` is the minimal cost of splitting the first
    // `j` windows into exactly `k` segments.
    let mut dpf = vec![vec![INF; windows + 1]; max_k + 1];
    dpf[0][0] = 0;
    for k in 1..=max_k {
        for j in k..=windows {
            let mut best = INF;
            for i in (k - 1)..j {
                if dpf[k - 1][i] >= INF {
                    continue;
                }
                let candidate = dpf[k - 1][i] + cost(i, j);
                if candidate < best {
                    best = candidate;
                }
            }
            dpf[k][j] = best;
        }
    }

    // Backward DP: `dpb[k][i]` is the minimal cost of splitting `[i,
    // windows)` into exactly `k` segments. Used only for lexicographic
    // reconstruction below, never for the objective itself.
    let mut dpb = vec![vec![INF; windows + 1]; max_k + 1];
    dpb[0][windows] = 0;
    for k in 1..=max_k {
        for i in (0..=windows.saturating_sub(k)).rev() {
            let mut best = INF;
            for j in (i + 1)..=(windows - (k - 1)) {
                if dpb[k - 1][j] >= INF {
                    continue;
                }
                let candidate = cost(i, j) + dpb[k - 1][j];
                if candidate < best {
                    best = candidate;
                }
            }
            dpb[k][i] = best;
        }
    }

    // Minimal objective first; among ties, the smallest segment count.
    let mut best_objective = INF;
    let mut best_k = 1usize;
    for k in 1..=max_k {
        if dpf[k][windows] >= INF {
            continue;
        }
        let objective = dpf[k][windows] + penalty_milli * (k as i128 - 1);
        if objective < best_objective || (objective == best_objective && k < best_k) {
            best_objective = objective;
            best_k = k;
        }
    }
    if best_objective >= INF {
        // `k = 1` always covers `windows >= 1`, so this is unreachable;
        // kept as a fail-closed value rather than a panic.
        return (Vec::new(), 0);
    }

    // Reconstruct the lexicographically earliest boundary set achieving
    // `best_objective` at `best_k` segments: at each step, take the
    // smallest split point consistent with an optimal completion (checked
    // against the backward DP), which is exactly what "earliest boundary
    // set" means read left to right.
    let mut boundaries = Vec::with_capacity(best_k.saturating_sub(1));
    let mut start = 0usize;
    let mut remaining = best_k;
    for _ in 0..best_k.saturating_sub(1) {
        let target = dpb[remaining][start];
        let last_split = windows - (remaining - 1);
        let mut chosen = None;
        for candidate in (start + 1)..=last_split {
            if dpb[remaining - 1][candidate] >= INF {
                continue;
            }
            if cost(start, candidate) + dpb[remaining - 1][candidate] == target {
                chosen = Some(candidate);
                break;
            }
        }
        let split = chosen.expect("DP invariant: an optimal split exists at every step");
        boundaries.push(split);
        start = split;
        remaining -= 1;
    }

    (boundaries, best_objective)
}

/// Render the fixed preamble: analysis identity and every plan parameter,
/// verbatim, so a reader never has to trust that a report used the
/// parameters it claims to.
pub fn render_header(campaign: &str, plan: &EraPlan) -> String {
    format!(
        "era-report 1 campaign {campaign} detector {ERA_VERSION} window {window} penalty {penalty} max_segments {max_segments} burn_in {burn_in} features {feature_count}\n",
        window = plan.window_ticks,
        penalty = plan.penalty_milli,
        max_segments = plan.max_segments,
        burn_in = plan.burn_in_ticks,
        feature_count = FEATURE_COUNT,
    )
}

/// Render one world's result. No floats, no `HashMap` iteration - every
/// value is an integer already, and the only ordering used (feature index)
/// is an array, so the same [`WorldEra`] renders to the same bytes in any
/// process (C17.3).
pub fn render_world(
    condition: &str,
    seed: u64,
    config_hash: u64,
    event_schema: u32,
    era: &WorldEra,
    with_features: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "world condition={condition} seed={seed:#018x} config={config_hash:#018x} schema={event_schema} windows={windows} dropped={dropped} segments={segments} cost={cost}\n",
        windows = era.windows.len(),
        dropped = era.windows_dropped_burn_in,
        segments = era.segments,
        cost = era.cost,
    ));

    if era.segments == 1 {
        // C17.7: the explicit negative result, printed rather than implied
        // by an empty boundary list.
        out.push_str("no segments above threshold\n");
    } else {
        for boundary in &era.boundaries {
            out.push_str(&format!(
                "boundary tick={} window={}\n",
                boundary.tick, boundary.window
            ));
            let mut present: Vec<(usize, i64)> = boundary
                .deltas
                .iter()
                .enumerate()
                .filter_map(|(feature, delta)| delta.map(|value| (feature, value)))
                .collect();
            // Largest |delta| first; ties broken by feature index, so the
            // five printed lines are a deterministic function of the
            // deltas rather than of sort stability.
            present.sort_by(|a, b| b.1.abs().cmp(&a.1.abs()).then(a.0.cmp(&b.0)));
            for (feature, value) in present.into_iter().take(5) {
                out.push_str(&format!("delta {}={}\n", FEATURE_NAMES[feature], value));
            }
        }
    }

    if with_features {
        for window in &era.windows {
            out.push_str(&format!(
                "features window={} start={}",
                window.index, window.start_tick
            ));
            for (feature, name) in FEATURE_NAMES.iter().enumerate() {
                match window.values[feature] {
                    Some(value) => out.push_str(&format!(" {name}={value}")),
                    None => out.push_str(&format!(" {name}=absent")),
                }
            }
            out.push('\n');
        }
    }

    out
}

/// A tiny splitmix64-style hash keyed on `(seed, window, feature, k)`. Not
/// a simulation RNG stream - ADR-0033 draws from none - just enough spread
/// to give [`synthetic_log`]'s placeholder identifiers seed-dependent
/// variety without touching the piecewise-constant rates that are the
/// fixture's actual ground truth. No `HashMap`, so no iteration-order
/// nondeterminism can leak in here either.
fn keyed_hash(seed: u64, window: u32, feature: u32, k: u32) -> u64 {
    let mut z = seed
        ^ u64::from(window).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(feature).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ u64::from(k).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Tags disambiguating [`keyed_hash`] call sites within [`synthetic_log`],
/// so the same `(window, k)` pair used for two different placeholder
/// fields never draws the same value by accident.
const HASH_TAG_DAMAGE_ATTACKER: u32 = 1;
const HASH_TAG_DAMAGE_TARGET: u32 = 2;
const HASH_TAG_SIGNAL_ID: u32 = 3;

/// A window's driven rates, in milli per 1,000 organism-ticks, for the
/// five features [`synthetic_log`] can inject a known regime change into:
/// `births`, `deaths_starvation`, `damage_events`, `signals_emitted`,
/// `objects_created` (feature indices 0, 1, 7, 18, 9).
///
/// `rates[w][1]` (`deaths_starvation`) is deliberately never read as an
/// independent generator: [`synthetic_log`] matches every synthetic birth
/// with a same-tick starvation death of the oldest living organism so the
/// population returns to `population` at each window's end (a documented
/// generator behaviour, not a free parameter - an independently driven
/// extra death would drain the population out from under later windows
/// with no compensating entry). The column is kept in the row shape so a
/// caller can record the value it expects that feature to measure - which,
/// by this construction, is always the same count as `births` - without
/// the generator silently inventing a sixth mechanism to spend it on.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticSpec {
    pub window_ticks: u64,
    pub windows: u32,
    pub population: u32,
    pub rates: Vec<[i64; 5]>,
}

/// `round(rate * organism_ticks / 1_000_000)`, the event count a milli
/// rate implies over a window - the inverse of the rate formula
/// [`world_era`] uses, so a fixture's injected rate and the rate the
/// detector later measures agree by construction (round-half-up; rates are
/// never negative in a well-formed spec, and a negative one clamps to 0
/// rather than producing a negative event count).
fn milli_rate_to_count(rate_milli: i64, organism_ticks: i128) -> u32 {
    if rate_milli <= 0 || organism_ticks <= 0 {
        return 0;
    }
    let numerator = i128::from(rate_milli) * organism_ticks;
    ((numerator + 500_000) / 1_000_000) as u32
}

/// Build a deterministic event log with known, injected piecewise-constant
/// rates (C17.4's synthetic ground truth). `population` organisms are born
/// at tick 1 of window 0; every subsequent synthetic birth is matched, in
/// the same tick, by a starvation death of the oldest still-living
/// organism, holding the population at `population` for the rest of the
/// run so that `organism_ticks` per window is exactly `population *
/// window_ticks` and the injected milli rates translate to event counts
/// without drift. See [`SyntheticSpec`] for why `deaths_starvation`'s rate
/// column is not an independent driver.
pub fn synthetic_log(seed: u64, spec: &SyntheticSpec) -> Vec<Event> {
    let mut events = Vec::new();
    let mut alive: VecDeque<u64> = VecDeque::with_capacity(spec.population as usize);
    let mut next_organism_id: u64 = 1;
    let mut next_object_id: u64 = 1;

    for _ in 0..spec.population {
        let id = next_organism_id;
        next_organism_id += 1;
        alive.push_back(id);
        events.push(Event {
            tick: 1,
            kind: EventKind::Birth { id, parent_id: 0 },
        });
    }

    let organism_ticks = i128::from(spec.population) * i128::from(spec.window_ticks);

    for window in 0..spec.windows {
        let start = u64::from(window) * spec.window_ticks + 1;
        let rates = spec.rates.get(window as usize).copied().unwrap_or([0; 5]);

        let births = milli_rate_to_count(rates[0], organism_ticks);
        for k in 0..births {
            let tick = start + (u64::from(k) * spec.window_ticks) / u64::from(births);
            let id = next_organism_id;
            next_organism_id += 1;
            alive.push_back(id);
            events.push(Event {
                tick,
                kind: EventKind::Birth { id, parent_id: 0 },
            });
            if let Some(oldest) = alive.pop_front() {
                events.push(Event {
                    tick,
                    kind: EventKind::Death {
                        id: oldest,
                        cause: DeathCause::Starvation,
                    },
                });
            }
        }

        let damage = milli_rate_to_count(rates[2], organism_ticks);
        for k in 0..damage {
            let tick = start + (u64::from(k) * spec.window_ticks) / u64::from(damage);
            let attacker = 1 + keyed_hash(seed, window, HASH_TAG_DAMAGE_ATTACKER, k)
                % u64::from(spec.population.max(1));
            let target = 1 + keyed_hash(seed, window, HASH_TAG_DAMAGE_TARGET, k)
                % u64::from(spec.population.max(1));
            events.push(Event {
                tick,
                kind: EventKind::Damage {
                    attacker,
                    target,
                    raw_milli: 100,
                    applied_milli: 100,
                    health_milli: 1_000,
                },
            });
        }

        let signals = milli_rate_to_count(rates[3], organism_ticks);
        for k in 0..signals {
            let tick = start + (u64::from(k) * spec.window_ticks) / u64::from(signals);
            let id = 1 + keyed_hash(seed, window, HASH_TAG_SIGNAL_ID, k)
                % u64::from(spec.population.max(1));
            events.push(Event {
                tick,
                kind: EventKind::SignalEmitted {
                    id,
                    channel_mask: 1,
                    peak_amplitude_q16: 1,
                    cost_milli: 10,
                },
            });
        }

        let objects = milli_rate_to_count(rates[4], organism_ticks);
        for k in 0..objects {
            let tick = start + (u64::from(k) * spec.window_ticks) / u64::from(objects);
            let id = next_object_id;
            next_object_id += 1;
            events.push(Event {
                tick,
                kind: EventKind::ObjectCreated {
                    id,
                    material_id: 0,
                    cause: 0,
                    mass_milli: 0,
                    energy_milli: 0,
                    parent_id: 0,
                },
            });
        }
    }

    events
}
