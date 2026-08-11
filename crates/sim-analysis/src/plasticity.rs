//! Phase 11 lifetime learning: C11.1 and C11.2.
//!
//! Modelled on `morph.rs`, which is the closest existing shape - per-individual
//! pairs reduced to one world statistic, with a within-world permutation null -
//! and it is reused rather than reimplemented: `morph::permutation_p95_milli`
//! is called verbatim, so a plasticity report is reproducible from its recorded
//! analysis seed alone exactly as a morphology report is.
//!
//! Everything here is integer arithmetic in milli-units. Nothing here can reach
//! a rule (ADR-0016).
//!
//! # C11.1: what "within-lifetime behavioural change" has to mean
//!
//! The criterion is about an **individual**. A population histogram cannot
//! answer it: births and deaths between two observations make selection a
//! complete explanation of any shift, so a population-level change cannot
//! distinguish "these organisms changed" from "different organisms are alive
//! now". The `.alac` artifact carries an entity id in every record precisely so
//! this analysis can key on the organism rather than on the array slot.
//!
//! The event to align on is a **patch relocation**, which is a pure function of
//! `(seed, config, tick)` and lands on ticks that are multiples of
//! `worldmod.relocate_interval_ticks`. For each such boundary `T` the analysis
//! takes two adjacent windows of the same length `W` from the same organism -
//! `[T-W, T]` and `[T, T+W]` - and measures how far apart the two action
//! histograms are.
//!
//! **That distance on its own is not evidence of anything.** Two adjacent
//! windows of any organism differ, because activations and energy move. So each
//! individual also contributes a matched *control* boundary at the epoch
//! midpoint `T + R/2`, whose two windows lie entirely inside one epoch with no
//! relocation between them. The per-individual quantity is therefore a pair of
//! distances - one across an event, one not - and the world statistic is the
//! rank correlation between "was there an event" and "how far apart were the
//! two windows".
//!
//! The null shuffles the event labels against the distances. That destroys the
//! event-distance link while preserving both marginals, which is the same
//! argument `morph.rs` makes for its permutation and the same code.
//!
//! Two honest asymmetries, both stated rather than smoothed:
//!
//! - The sample at tick `T` is taken **after** the relocation ran in that tick,
//!   so the pre-window contains exactly one post-relocation tick out of `W`.
//!   That dilutes the contrast rather than inflating it.
//! - A long-lived organism contributes a pair at several boundaries, so the
//!   pooled observations are not independent. The permutation is computed on
//!   the same pooled data, and free permutation of labels across a pooled set
//!   with real between-individual variance produces a *wider* null than a
//!   within-pair permutation would - conservative, again in the direction that
//!   matters.
//!
//! # The matched control is **not** age-matched, and that is a defect
//!
//! Recorded here rather than in a report because the code still carries it.
//! The control boundary is `event + relocate / 2`, so a control window pair
//! lies `relocate / 2` ticks later in **every** organism's life than the
//! event pair it is matched to, and it also sits at a different phase of the
//! epoch - immediately after the event pair's own post-window rather than
//! straddling anything. Any quantity that varies with age therefore enters
//! the correlation with the event label, and the pooled permutation null
//! cannot see it: the null shuffles labels against distances and every draw
//! carries the same age imbalance the observed statistic does.
//!
//! Measured rather than argued, in
//! `the_matched_control_is_not_age_matched_and_the_offset_alone_clears_c11_1`.
//! A stationary rolling cohort - one in which nothing happens at the event
//! tick, the population's age profile is identical at every sample, and
//! behaviour is a pure function of age - reports:
//!
//! - `rho = +158` against a null of `30`, and **passes
//!   [`WorldPlasticity::within_lifetime_shift`]**, when the window-to-window
//!   behavioural change decays with age;
//! - `rho = -154` when it grows with age, which is the sign the confirmatory
//!   campaign measured;
//! - `rho = 0` exactly when the change is age-dependent but has no curvature,
//!   so what the statistic reads is curvature in the age trend and not the
//!   cohort construction;
//! - `76, 158, 334, 700` at offsets of `500, 1_000, 2_000, 4_000` ticks -
//!   each doubling of the offset roughly doubles the correlation, at
//!   ratios of 2.08, 2.11 and 2.10. A dose-response over an eightfold
//!   range is what identifies the offset as the mechanism rather than
//!   leaving it an interpretation.
//!
//! The artifact is a fixed effect size while the null shrinks with the pooled
//! count, so it decides more worlds the more data there is: at the campaign's
//! ~27,000 pairs per world the null p95 fell as low as 6 milli and the measured
//! `rho` was -50 to -64 median **in all four arms, including `Bstat`**, where
//! plasticity is disabled and the relocation has zero magnitude. The directed
//! rule refused every one of them, so the recorded verdict does not change;
//! but the sign is a property of this substrate's age trend rather than of
//! the design, and the same defect with the opposite trend would have
//! produced a false pass. **The directed statistic is not identified.**
//!
//! The pre-registered decision rule is left exactly as it was - a threshold
//! is never changed after the data is seen - and the correction a future
//! pre-registration needs is specified in D-100 rather than applied here.
//! The one age-free reading available from the campaign as it stands is the
//! seed-paired contrast of `rho` between arms, which the report already
//! prints and which is null: Avar minus Bvar is +3 milli, 95 percent
//! interval [-1, +8], p = 0.707.
//!
//! # The observed statistic is signed and the null is two-sided, on purpose
//!
//! `morph.rs` compares `|rho|` against the p95 of `|rho|` because C10.3b
//! predicts no sign. C11.1 does: a relocation is predicted to make an
//! individual's action distribution move *more*, not less. So the observed
//! statistic here is signed and only a positive correlation can pass, while the
//! null is still the p95 of the absolute value. That bar is at least as strict
//! as a one-sided test at five percent, never weaker.
//!
//! # A world with no variance is undefined, not zero
//!
//! `WorldMorph::no_variance`'s discipline, and it matters more here than there.
//! Phase 11's own measurement substrate found that in a founder-dominated world
//! `Eat` and `Mate` are saturated and `Rest` and `TurnLeft` are empty, so only
//! two of seven columns carry any variation at all. A world whose recorded
//! distances are all identical cannot speak to whether behaviour changed, and
//! counting it as a failure would let a degenerate controller masquerade as a
//! refutation of lifetime learning. Those worlds are reported as `no_variance`
//! and counted separately, and the per-column occupancy is reported beside every
//! world so a null can be attributed to the right cause.
//!
//! # C11.2: the drift comparison
//!
//! `eta` and `EDGE_FLAG_PLASTIC` both start at the founder value - `0.0` and
//! clear - and both are reachable by point mutation. The neutral marker locus
//! starts at exactly the same two values, mutates on exactly the same draws with
//! exactly the same delta and clamp, sits between the two founder edges so it is
//! as tightly linked to each as they are to each other, and is never expressed.
//! It therefore experiences the same population size, the same variance in
//! reproductive success, the same linkage and the same mutation regime, and
//! **none** of the selection.
//!
//! So the drift control is empirical rather than analytic: the quantity is the
//! *excess* of the plasticity shift over the marker's shift in the same run. A
//! world where neither moved has nothing to say - the control itself never ran -
//! and is reported as `drift_no_variance`. A world where the marker moved and
//! `eta` did not is a different thing entirely: it is plasticity failing to keep
//! up with drift, which is a directed failure with a real reading, and it is
//! counted as one.

use crate::morph::permutation_p95_milli_stratified;
use crate::paired::{Direction, Pair, PairedResult, compare, median_milli};
use sim_core::{
    ACTION_CLASS_COUNT, EDGE_FLAG_PLASTIC, Genome2, LOCOMOTION_CLASS_COUNT, LocusKind,
    MARKER_FLAG_NEUTRAL,
};
use sim_persist::ActionLogScan;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Bumped to v2 when the permutation null became **stratified by the
/// organism's age at its boundary** (D-100). v1's unrestricted shuffle
/// destroyed the age balance along with the association, so an age artifact
/// sat in the observed value and not in the null it was compared against; on
/// a world where nothing happened at all it scored rho +158 against a p95 of
/// 30 and passed. The observed statistic is unchanged - only the reference
/// distribution moved - but a v1 and a v2 number are not comparable and the
/// version is what stops them being read side by side.
pub const PLASTICITY_ANALYSIS_VERSION: &str = "lifesim-plasticity-analysis-v2";

/// The analysis plan. Every bar is a named field and every field is echoed
/// verbatim into the report, so a reader can check it against the campaign
/// source rather than trusting a summary - `StructurePlan`'s contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlasticityPlan {
    /// Samples at or before this tick are discarded as the opening transient,
    /// while the founder population is still spreading from its placement.
    /// A whole number of relocation epochs, so the first usable boundary is a
    /// relocation like every other one.
    pub burn_in_ticks: u64,
    /// Distinct individuals a world must contribute before its correlation is
    /// read at all. Three is the theoretical floor for a permutation test with
    /// a binary label to reach a five percent tail; eight leaves headroom for
    /// ties.
    pub min_individuals: usize,
    /// C11.1's bar, out of the campaign's seed count.
    pub shift_bar: usize,
    /// The control arm must stay **strictly below** this. C11.1 is a claim
    /// that plasticity produced the change, and a control that shows the same
    /// thing refutes that claim however strong the treatment looks.
    pub control_ceiling: usize,
    /// C11.2's bar, out of the campaign's seed count.
    pub drift_bar: usize,
    /// Smallest excess over the neutral marker that counts, in milli of the
    /// allele's own `[0, 1]` range.
    ///
    /// Anchored rather than chosen: a point mutation's delta is uniform on
    /// `+/- point_delta_q16 / 65536` of the range, which at the pinned 3,277
    /// is `+/- 0.05`, so the expected absolute step is exactly 25 milli. An
    /// excess smaller than one typical mutational step is not an effect worth
    /// naming. The plastic-flag scale is matched to it by construction: the
    /// marker's flag toggles on the same draw as `EDGE_FLAG_PLASTIC`, so 25
    /// milli there is 2.5 percent of alleles on both sides of the comparison.
    pub drift_margin_milli: i64,
    /// Seed for the permutation. Recorded; the null is a function of the data
    /// and this value only.
    pub analysis_seed: u64,
}

impl Default for PlasticityPlan {
    fn default() -> Self {
        Self {
            burn_in_ticks: 6_000,
            min_individuals: 8,
            shift_bar: 20,
            control_ceiling: 20,
            drift_bar: 20,
            drift_margin_milli: 25,
            analysis_seed: 0x9e37_79b9_7f4a_7c15,
        }
    }
}

/// Distance between two action windows of one organism, all in milli.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WindowDistance {
    /// L1 over the milli-normalized counts of all [`ACTION_CLASS_COUNT`]
    /// columns. Range `[0, 5000]`: the locomotion block is a probability
    /// vector so it contributes at most 2,000, and each of the three
    /// indicators contributes at most 1,000.
    pub l1_milli: i64,
    /// Total variation over the locomotion partition alone, `[0, 1000]`.
    /// Reported because it is the interpretable half: the locomotion columns
    /// are mutually exclusive and sum to the window, so half their L1 is a
    /// genuine total-variation distance between two behaviour policies.
    pub locomotion_tv_milli: i64,
    /// L1 over the three independent indicators, `[0, 3000]`.
    pub indicator_l1_milli: i64,
}

/// Per-column milli rates of one window, normalized by the window's own
/// locomotion total.
///
/// The locomotion block is the denominator rather than the sum of every
/// column, and that is the only choice that keeps the two blocks comparable:
/// the locomotion columns partition the organism-ticks so their sum **is** the
/// window length, while the indicators co-occur freely and summing them in
/// would make the denominator depend on how often the organism happened to
/// want three things at once.
pub fn rates_milli(
    window: &[i64; ACTION_CLASS_COUNT],
    denominator: i64,
) -> [i64; ACTION_CLASS_COUNT] {
    let mut out = [0_i64; ACTION_CLASS_COUNT];
    if denominator <= 0 {
        return out;
    }
    for (slot, value) in window.iter().enumerate() {
        out[slot] = value * 1_000 / denominator;
    }
    out
}

/// Distance between two windows, or `None` when either window is unusable.
///
/// Unusable means an empty locomotion block - nothing was classified, so there
/// is no distribution to compare - or a negative delta, which cannot arise from
/// cumulative counts and therefore means the two records are not the same
/// organism's. Both are refusals rather than zeros.
pub fn window_distance(
    before: &[i64; ACTION_CLASS_COUNT],
    after: &[i64; ACTION_CLASS_COUNT],
) -> Option<WindowDistance> {
    if before.iter().chain(after.iter()).any(|value| *value < 0) {
        return None;
    }
    let denominator = |window: &[i64; ACTION_CLASS_COUNT]| -> i64 {
        window[..LOCOMOTION_CLASS_COUNT].iter().sum()
    };
    let (left, right) = (denominator(before), denominator(after));
    if left <= 0 || right <= 0 {
        return None;
    }
    let a = rates_milli(before, left);
    let b = rates_milli(after, right);
    let locomotion: i64 = (0..LOCOMOTION_CLASS_COUNT)
        .map(|slot| (a[slot] - b[slot]).abs())
        .sum();
    let indicator: i64 = (LOCOMOTION_CLASS_COUNT..ACTION_CLASS_COUNT)
        .map(|slot| (a[slot] - b[slot]).abs())
        .sum();
    Some(WindowDistance {
        l1_milli: locomotion + indicator,
        locomotion_tv_milli: locomotion / 2,
        indicator_l1_milli: indicator,
    })
}

/// One relocation boundary and the within-epoch boundary matched to it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Boundary {
    /// A relocation tick: a multiple of `relocate_interval_ticks`.
    pub event_tick: u64,
    /// The midpoint of the epoch that starts at `event_tick`. No relocation
    /// happens between its two windows, which is what makes it the control.
    pub control_tick: u64,
}

/// Why a world's shift statistic could not be computed at all.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShiftRefusal {
    /// The condition runs no relocation schedule, so there is no event to
    /// align on. Reported, never treated as "no change observed".
    NoSchedule,
    /// The sampling interval does not divide half the relocation interval, so
    /// no matched within-epoch boundary lands on a sample tick.
    Misaligned { relocate: u64, window: u64 },
    /// The run is too short for one boundary and its control to fit inside the
    /// sampled series after burn-in.
    NoBoundaries,
    /// Fewer distinct individuals than the plan requires.
    TooFewIndividuals { individuals: usize, required: usize },
    /// No age stratum held both an event and a control observation, so the
    /// event label and the organism's age are perfectly confounded and no
    /// age-matched comparison exists in this population at all.
    ///
    /// **This is not "behaviour did not vary" and must never be reported as
    /// it.** One organism's own two observations are always `relocate / 2`
    /// ticks apart in age, so a stratum can only hold both labels when the
    /// population contains organisms born about that far apart. A
    /// birth-synchronised population - every founder alive, nothing born
    /// since - cannot supply that, and the honest answer for such a world is
    /// that C11.1 is unanswerable in it rather than answered in the negative.
    /// Reporting it as a null would let a demographic accident stand in for a
    /// refutation of lifetime learning, which is D-079's lesson.
    NoInformativeStrata {
        strata_total: usize,
        observations: usize,
    },
}

impl std::fmt::Display for ShiftRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSchedule => write!(formatter, "no_schedule"),
            Self::Misaligned { relocate, window } => {
                write!(formatter, "misaligned(relocate={relocate},window={window})")
            }
            Self::NoBoundaries => write!(formatter, "no_boundaries"),
            Self::TooFewIndividuals {
                individuals,
                required,
            } => write!(formatter, "too_few_individuals({individuals}<{required})"),
            Self::NoInformativeStrata {
                strata_total,
                observations,
            } => write!(
                formatter,
                "no_informative_strata(strata={strata_total},observations={observations})"
            ),
        }
    }
}

/// Every relocation boundary whose windows and matched control both fit inside
/// the sampled series.
///
/// `window` is the artifact's own sample interval, and the alignment condition
/// is exact: the control boundary sits at `relocate / 2` past the event, so
/// `relocate / 2` must be a whole number of windows or the control boundary
/// falls between two samples and cannot be read at all. Refusing is the only
/// honest answer - interpolating a sample would invent counts.
pub fn boundaries(
    relocate: u64,
    window: u64,
    burn_in: u64,
    last_sample_tick: u64,
) -> Result<Vec<Boundary>, ShiftRefusal> {
    if relocate == 0 {
        return Err(ShiftRefusal::NoSchedule);
    }
    if window == 0 || !relocate.is_multiple_of(2 * window) {
        return Err(ShiftRefusal::Misaligned { relocate, window });
    }
    let half = relocate / 2;
    let mut out = Vec::new();
    let mut event = relocate;
    while event + half + window <= last_sample_tick {
        if event >= burn_in.max(window) + window {
            out.push(Boundary {
                event_tick: event,
                control_tick: event + half,
            });
        }
        event += relocate;
    }
    if out.is_empty() {
        return Err(ShiftRefusal::NoBoundaries);
    }
    Ok(out)
}

/// One organism's record at one sample tick.
type Row = (u64, [u32; ACTION_CLASS_COUNT]);

/// C11.1's world-level result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShiftResult {
    pub boundaries: usize,
    /// Distinct organisms contributing at least one matched pair.
    pub individuals: usize,
    /// Matched pairs; each contributes one event observation and one control
    /// observation.
    pub pairs: usize,
    /// Rank correlation between the event label and the window distance, milli.
    pub rho_milli: i64,
    /// The permutation null's 95th percentile of `|rho|`, milli.
    pub null_p95_milli: i64,
    pub median_event_milli: i64,
    pub median_control_milli: i64,
    pub median_event_locomotion_tv_milli: i64,
    pub median_control_locomotion_tv_milli: i64,
    /// The paired picture, reported and **not** a second decision rule: how
    /// often the same organism's event window pair was further apart than its
    /// matched within-epoch pair, how often it was closer, and how often the
    /// two were identical. The distances here are heavily zero-inflated - most
    /// organisms hold one heading band for both windows - so a reader needs to
    /// see how much of the correlation is carried by the tail.
    pub event_wins: usize,
    pub control_wins: usize,
    pub ties: usize,
    /// Distinct distance values over the pooled observations. Below two the
    /// correlation is undefined rather than zero.
    pub distinct_distances: usize,
    pub no_variance: bool,
    /// Total increments per action column over every window used. The
    /// diagnostic that lets a null be attributed: a column that is empty or
    /// saturated in every window cannot carry an effect, and Phase 11's own
    /// substrate measurement found four of the seven columns in that state.
    pub column_totals: [u64; ACTION_CLASS_COUNT],
    /// Columns whose per-window milli rate was not the same in every window.
    pub varying_columns: usize,
    /// Window pairs discarded because a window was unusable.
    pub discarded: usize,
    /// Age strata over the pooled observations, and how many held both an
    /// event and a control observation. Only the informative ones are used;
    /// the rest are dropped from the observed statistic and the null alike.
    ///
    /// These are reported because the exclusion changes what was measured. A
    /// world where `strata_informative` is a small fraction of `strata_total`
    /// answered C11.1 over a minority of its own data, and a reader has to be
    /// able to see that without rerunning anything.
    pub strata_total: usize,
    pub strata_informative: usize,
    /// Observations dropped for sitting in a single-label stratum.
    pub observations_dropped: usize,
}

/// Reduce one world's `.alac` series to C11.1's statistics.
///
/// `seed` keys the permutation, so two reports of the same file agree exactly.
pub fn world_shift(
    scan: &ActionLogScan,
    relocate: u64,
    plan: &PlasticityPlan,
    seed: u64,
) -> Result<ShiftResult, ShiftRefusal> {
    let window = u64::from(scan.info.sample_interval_ticks);
    let last = scan.samples.last().map_or(0, |sample| sample.tick);
    let spec = boundaries(relocate, window, plan.burn_in_ticks, last)?;

    // Sample tick -> id -> (age, counts). A BTreeMap keeps the walk ordered
    // and therefore the observation order a function of the data alone.
    let mut by_tick: BTreeMap<u64, BTreeMap<u64, Row>> = BTreeMap::new();
    for sample in &scan.samples {
        let entry = by_tick.entry(sample.tick).or_default();
        for record in &sample.records {
            entry.insert(record.id, (record.age_ticks, record.counts));
        }
    }

    // `(label, distance, stratum)`.
    let mut observations: Vec<(i64, i64, u64)> = Vec::new();
    let mut event_distances: Vec<i64> = Vec::new();
    let mut control_distances: Vec<i64> = Vec::new();
    let mut event_locomotion: Vec<i64> = Vec::new();
    let mut control_locomotion: Vec<i64> = Vec::new();
    let mut individuals: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut column_totals = [0_u64; ACTION_CLASS_COUNT];
    let mut column_rates: [Vec<i64>; ACTION_CLASS_COUNT] = Default::default();
    let mut discarded = 0_usize;

    // One organism's window across `[centre - window, centre + window]`, or
    // `None` when it was not present at all three sample ticks with exactly
    // `window` ticks of age between each. Presence at both ends of a window
    // with the age arithmetic to match is what proves the organism lived the
    // whole window rather than being born inside it.
    let pair_at = |centre: u64,
                   id: u64|
     -> Option<(
        WindowDistance,
        [i64; ACTION_CLASS_COUNT],
        [i64; ACTION_CLASS_COUNT],
        u64,
    )> {
        let before = by_tick.get(&(centre - window))?.get(&id)?;
        let middle = by_tick.get(&centre)?.get(&id)?;
        let after = by_tick.get(&(centre + window))?.get(&id)?;
        if middle.0.checked_sub(before.0) != Some(window)
            || after.0.checked_sub(middle.0) != Some(window)
        {
            return None;
        }
        let mut pre = [0_i64; ACTION_CLASS_COUNT];
        let mut post = [0_i64; ACTION_CLASS_COUNT];
        for slot in 0..ACTION_CLASS_COUNT {
            pre[slot] = i64::from(middle.1[slot]) - i64::from(before.1[slot]);
            post[slot] = i64::from(after.1[slot]) - i64::from(middle.1[slot]);
        }
        // `middle.0` is the organism's age at the boundary tick, and it is the
        // stratifying variable. An event pair at boundary T and a control pair
        // at boundary T+half both cover ages `[a-window, a]` and
        // `[a, a+window]` for an organism of age `a` at its own boundary, so
        // two observations sharing a stratum cover identical age ranges. That
        // identity is what makes the stratified null absorb the age artifact
        // rather than merely dilute it (D-100).
        window_distance(&pre, &post).map(|distance| (distance, pre, post, middle.0))
    };

    for boundary in &spec {
        // Candidates are the organisms present at the earliest sample the
        // event window needs; everything else falls out of `pair_at`.
        let Some(candidates) = by_tick.get(&(boundary.event_tick - window)) else {
            continue;
        };
        for id in candidates.keys().copied() {
            let (Some(event), Some(control)) = (
                pair_at(boundary.event_tick, id),
                pair_at(boundary.control_tick, id),
            ) else {
                discarded += 1;
                continue;
            };
            individuals.insert(id);
            // Each observation carries the stratum it will be shuffled within:
            // the organism's own age at its own boundary, binned by the window
            // width. `window` is the resolution the windows are defined at, so
            // two observations in one stratum cannot differ by a whole window.
            observations.push((1, event.0.l1_milli, event.3 / window));
            observations.push((0, control.0.l1_milli, control.3 / window));
            event_distances.push(event.0.l1_milli);
            control_distances.push(control.0.l1_milli);
            event_locomotion.push(event.0.locomotion_tv_milli);
            control_locomotion.push(control.0.locomotion_tv_milli);
            for (_, pre, post, _) in [event, control] {
                let denominator = |w: &[i64; ACTION_CLASS_COUNT]| -> i64 {
                    w[..LOCOMOTION_CLASS_COUNT].iter().sum()
                };
                for (window_counts, total) in [(pre, denominator(&pre)), (post, denominator(&post))]
                {
                    let rates = rates_milli(&window_counts, total);
                    for slot in 0..ACTION_CLASS_COUNT {
                        column_totals[slot] += window_counts[slot].max(0) as u64;
                        column_rates[slot].push(rates[slot]);
                    }
                }
            }
        }
    }

    if individuals.len() < plan.min_individuals {
        return Err(ShiftRefusal::TooFewIndividuals {
            individuals: individuals.len(),
            required: plan.min_individuals,
        });
    }

    // Drop observations whose stratum holds only one label. Such a stratum
    // contributes to the observed correlation but cannot be shuffled, so
    // leaving it in would reimport the very age bias the stratification
    // exists to remove. The exclusion is reported rather than silent: a
    // design that quietly drops observations is a different design.
    let strata_index = {
        let mut index: BTreeMap<u64, (usize, usize)> = BTreeMap::new();
        for (label, _, stratum) in &observations {
            let entry = index.entry(*stratum).or_insert((0, 0));
            if *label == 1 {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
        index
    };
    let strata_total = strata_index.len();
    let strata_informative = strata_index
        .values()
        .filter(|(events, controls)| *events > 0 && *controls > 0)
        .count();
    let observations_seen = observations.len();
    observations.retain(|(_, _, stratum)| {
        strata_index
            .get(stratum)
            .is_some_and(|(events, controls)| *events > 0 && *controls > 0)
    });
    let observations_dropped = observations_seen - observations.len();
    if observations.is_empty() {
        return Err(ShiftRefusal::NoInformativeStrata {
            strata_total,
            observations: observations_seen,
        });
    }

    let distinct_distances = {
        let mut values: Vec<i64> = observations.iter().map(|(_, value, _)| *value).collect();
        values.sort_unstable();
        values.dedup();
        values.len()
    };
    let varying_columns = column_rates
        .iter()
        .filter(|rates| {
            let mut sorted = (*rates).clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted.len() > 1
        })
        .count();

    // No variance in the distances means the correlation is undefined, not
    // zero: the world produced no behavioural variation for an event to move.
    let no_variance = distinct_distances < 2;
    let (rho_milli, null_p95_milli) = if no_variance {
        (0, 0)
    } else {
        // Observed and null are computed over the SAME kept observations. The
        // statistic is unchanged from v1; only what the null is permitted to
        // shuffle has changed.
        let flat: Vec<(i64, i64)> = observations
            .iter()
            .map(|(label, value, _)| (*label, *value))
            .collect();
        (
            crate::demography::spearman_milli(&flat),
            permutation_p95_milli_stratified(&observations, seed),
        )
    };

    Ok(ShiftResult {
        boundaries: spec.len(),
        individuals: individuals.len(),
        pairs: event_distances.len(),
        rho_milli,
        null_p95_milli,
        median_event_milli: median_milli(&event_distances),
        median_control_milli: median_milli(&control_distances),
        median_event_locomotion_tv_milli: median_milli(&event_locomotion),
        median_control_locomotion_tv_milli: median_milli(&control_locomotion),
        event_wins: event_distances
            .iter()
            .zip(control_distances.iter())
            .filter(|(event, control)| event > control)
            .count(),
        control_wins: event_distances
            .iter()
            .zip(control_distances.iter())
            .filter(|(event, control)| event < control)
            .count(),
        ties: event_distances
            .iter()
            .zip(control_distances.iter())
            .filter(|(event, control)| event == control)
            .count(),
        distinct_distances,
        no_variance,
        column_totals,
        varying_columns,
        discarded,
        strata_total,
        strata_informative,
        observations_dropped,
    })
}

/// C11.2's allele census over one world's living genomes.
///
/// Both quantities are averaged **per organism first** and then across
/// organisms, so a genome that duplicated its way to forty edges does not
/// outvote thirty founders. ADR-0022 A5's rule applied one level down: the
/// allele is nested in the organism as the organism is nested in the world.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AlleleCensus {
    pub organisms: u64,
    pub edge_alleles: u64,
    pub plastic_alleles: u64,
    pub marker_alleles: u64,
    pub set_marker_alleles: u64,
    /// Mean over organisms of that organism's mean `eta`, milli.
    pub mean_eta_milli: i64,
    /// Mean over organisms of that organism's plastic-allele fraction, milli.
    pub plastic_fraction_milli: i64,
    /// The matched control for `mean_eta_milli`.
    pub marker_value_milli: i64,
    /// The matched control for `plastic_fraction_milli`.
    pub marker_set_fraction_milli: i64,
    /// Alleles that left the founder value. Zero on both sides means the
    /// mutational input never landed, so the comparison is undefined.
    pub moved_eta_alleles: u64,
    pub moved_marker_alleles: u64,
}

impl AlleleCensus {
    pub fn eta_excess_milli(&self) -> i64 {
        self.mean_eta_milli - self.marker_value_milli
    }

    pub fn plastic_excess_milli(&self) -> i64 {
        self.plastic_fraction_milli - self.marker_set_fraction_milli
    }

    /// True when neither the plasticity fields nor the marker ever moved, so
    /// the drift control itself never ran and the excess is undefined rather
    /// than zero.
    ///
    /// Deliberately **not** true when the marker moved and `eta` did not: that
    /// is plasticity failing to keep up with drift, which is a real directed
    /// result and is counted as a failure.
    pub fn no_variance(&self) -> bool {
        self.moved_eta_alleles == 0
            && self.moved_marker_alleles == 0
            && self.plastic_alleles == 0
            && self.set_marker_alleles == 0
    }
}

/// Convert an allele value in `[0, 1]` to milli, once, so every later step is
/// integer arithmetic.
fn value_milli(value: f32) -> i64 {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1_000.0) as i64
    } else {
        0
    }
}

pub fn allele_census(genomes: &[Genome2]) -> AlleleCensus {
    let mut census = AlleleCensus {
        organisms: genomes.len() as u64,
        ..AlleleCensus::default()
    };
    let mut eta_means: Vec<i64> = Vec::with_capacity(genomes.len());
    let mut plastic_fractions: Vec<i64> = Vec::with_capacity(genomes.len());
    let mut marker_means: Vec<i64> = Vec::with_capacity(genomes.len());
    let mut marker_fractions: Vec<i64> = Vec::with_capacity(genomes.len());

    for genome in genomes {
        let (mut eta_sum, mut edges, mut plastic) = (0_i64, 0_u64, 0_u64);
        let (mut marker_sum, mut markers, mut set) = (0_i64, 0_u64, 0_u64);
        for locus in genome.loci() {
            match locus.kind {
                LocusKind::Edge {
                    flags, plasticity, ..
                } => {
                    edges += 1;
                    let eta = value_milli(plasticity.eta);
                    eta_sum += eta;
                    if eta != 0 {
                        census.moved_eta_alleles += 1;
                    }
                    if flags & EDGE_FLAG_PLASTIC != 0 {
                        plastic += 1;
                    }
                }
                LocusKind::Marker { value, flags } => {
                    markers += 1;
                    let level = value_milli(value);
                    marker_sum += level;
                    if level != 0 {
                        census.moved_marker_alleles += 1;
                    }
                    if flags & MARKER_FLAG_NEUTRAL != 0 {
                        set += 1;
                    }
                }
                _ => {}
            }
        }
        census.edge_alleles += edges;
        census.plastic_alleles += plastic;
        census.marker_alleles += markers;
        census.set_marker_alleles += set;
        if edges > 0 {
            eta_means.push(eta_sum / edges as i64);
            plastic_fractions.push((plastic as i64 * 1_000) / edges as i64);
        }
        if markers > 0 {
            marker_means.push(marker_sum / markers as i64);
            marker_fractions.push((set as i64 * 1_000) / markers as i64);
        }
    }

    let mean = |values: &[i64]| -> i64 {
        if values.is_empty() {
            return 0;
        }
        let total: i128 = values.iter().map(|value| i128::from(*value)).sum();
        (total / values.len() as i128) as i64
    };
    census.mean_eta_milli = mean(&eta_means);
    census.plastic_fraction_milli = mean(&plastic_fractions);
    census.marker_value_milli = mean(&marker_means);
    census.marker_set_fraction_milli = mean(&marker_fractions);
    census
}

/// One world's Phase 11 outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldPlasticity {
    pub condition: String,
    pub seed: u64,
    pub population: u64,
    pub extinct: bool,
    /// C11.1's statistics, or the typed reason there are none.
    pub shift: Result<ShiftResult, ShiftRefusal>,
    /// C11.2's allele census.
    pub alleles: AlleleCensus,
    /// Learned state actually accumulated over the run, from the manifest.
    /// Reported beside the criterion because "plasticity was selected for" and
    /// "plasticity ever did anything" are different claims and a null in the
    /// second explains a null in the first.
    pub plastic_edges_total: u64,
    pub plasticity_updates_total: u64,
    pub mean_abs_learned_milli: u64,
}

impl WorldPlasticity {
    /// C11.1: did an individual's action distribution move more across a
    /// relocation than across a matched within-epoch boundary?
    ///
    /// **`min_individuals` is not re-checked here.** `world_shift` already
    /// refuses a thin world by name, and a second copy of the same bar in
    /// this predicate would be a guard no test could ever distinguish from
    /// the first - delete either one and everything still passes. One guard,
    /// pinned by its own diagnostic in
    /// `a_condition_with_no_relocation_schedule_is_refused_rather_than_
    /// scored_zero`, is strictly better than two that alibi each other.
    /// `plan` stays in the signature because the bar it carries is the
    /// criterion's, and a caller reading this should be looking at the plan.
    pub fn within_lifetime_shift(&self, _plan: &PlasticityPlan) -> bool {
        match &self.shift {
            Ok(shift) => !shift.no_variance && shift.rho_milli > shift.null_p95_milli,
            Err(_) => false,
        }
    }

    /// The **two-sided** count: an association between the event and the
    /// individual's behaviour in either direction.
    ///
    /// Reported, never decisive. C11.1's claim is that plasticity produces
    /// behavioural change in response to the event; a relocation that makes an
    /// individual's behaviour *more* stable is an association but it is not
    /// that claim. Reporting it is what stops a sign-reversed result being
    /// invisible in a report whose decision rule is directed.
    pub fn behaviour_associated(&self, _plan: &PlasticityPlan) -> bool {
        match &self.shift {
            Ok(shift) => !shift.no_variance && shift.rho_milli.abs() > shift.null_p95_milli,
            Err(_) => false,
        }
    }

    pub fn shift_no_variance(&self) -> bool {
        matches!(&self.shift, Ok(shift) if shift.no_variance)
    }

    /// C11.2 on the `eta` scale.
    pub fn eta_over_drift(&self, plan: &PlasticityPlan) -> bool {
        !self.alleles.no_variance() && self.alleles.eta_excess_milli() >= plan.drift_margin_milli
    }

    /// C11.2 on the plastic-flag scale.
    pub fn plastic_over_drift(&self, plan: &PlasticityPlan) -> bool {
        !self.alleles.no_variance()
            && self.alleles.plastic_excess_milli() >= plan.drift_margin_milli
    }

    /// C11.2's world decision.
    ///
    /// **Either quantity, not both.** `eta` and `EDGE_FLAG_PLASTIC` are two
    /// independently reachable fields of one mechanism, and requiring both
    /// would turn the criterion into a test of which mutation target happened
    /// to be hit - the same reasoning C9.1 uses for "nodes or edges".
    pub fn selected_over_drift(&self, plan: &PlasticityPlan) -> bool {
        self.eta_over_drift(plan) || self.plastic_over_drift(plan)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlasticityOutcome {
    pub condition: String,
    pub worlds: usize,
    pub extinct: usize,
    /// C11.1
    pub shifted: usize,
    /// The two-sided count, reported beside the directed one.
    pub associated: usize,
    pub shift_no_variance: usize,
    pub shift_refused: usize,
    pub median_rho_milli: i64,
    pub median_null_milli: i64,
    pub median_event_distance_milli: i64,
    pub median_control_distance_milli: i64,
    pub median_event_locomotion_tv_milli: i64,
    pub median_control_locomotion_tv_milli: i64,
    pub median_individuals: i64,
    pub median_varying_columns: i64,
    pub column_totals: [u64; ACTION_CLASS_COUNT],
    /// C11.2
    pub selected: usize,
    pub eta_selected: usize,
    pub plastic_selected: usize,
    pub drift_no_variance: usize,
    pub median_eta_milli: i64,
    pub median_marker_value_milli: i64,
    pub median_plastic_fraction_milli: i64,
    pub median_marker_set_fraction_milli: i64,
    pub median_eta_excess_milli: i64,
    pub median_plastic_excess_milli: i64,
    pub total_moved_eta_alleles: u64,
    pub total_moved_marker_alleles: u64,
    pub total_plastic_alleles: u64,
    pub total_set_marker_alleles: u64,
    pub median_population: i64,
    pub total_plasticity_updates: u64,
}

pub fn summarise(
    condition: &str,
    worlds: &[WorldPlasticity],
    plan: &PlasticityPlan,
) -> PlasticityOutcome {
    let pick = |values: Vec<i64>| median_milli(&values);
    let shifts: Vec<&ShiftResult> = worlds
        .iter()
        .filter_map(|w| w.shift.as_ref().ok())
        .collect();
    let mut column_totals = [0_u64; ACTION_CLASS_COUNT];
    for shift in &shifts {
        for (slot, total) in column_totals.iter_mut().enumerate() {
            *total += shift.column_totals[slot];
        }
    }
    PlasticityOutcome {
        condition: condition.to_owned(),
        worlds: worlds.len(),
        extinct: worlds.iter().filter(|w| w.extinct).count(),
        shifted: worlds
            .iter()
            .filter(|w| w.within_lifetime_shift(plan))
            .count(),
        associated: worlds
            .iter()
            .filter(|w| w.behaviour_associated(plan))
            .count(),
        shift_no_variance: worlds.iter().filter(|w| w.shift_no_variance()).count(),
        shift_refused: worlds.iter().filter(|w| w.shift.is_err()).count(),
        median_rho_milli: pick(shifts.iter().map(|s| s.rho_milli).collect()),
        median_null_milli: pick(shifts.iter().map(|s| s.null_p95_milli).collect()),
        median_event_distance_milli: pick(shifts.iter().map(|s| s.median_event_milli).collect()),
        median_control_distance_milli: pick(
            shifts.iter().map(|s| s.median_control_milli).collect(),
        ),
        median_event_locomotion_tv_milli: pick(
            shifts
                .iter()
                .map(|s| s.median_event_locomotion_tv_milli)
                .collect(),
        ),
        median_control_locomotion_tv_milli: pick(
            shifts
                .iter()
                .map(|s| s.median_control_locomotion_tv_milli)
                .collect(),
        ),
        median_individuals: pick(shifts.iter().map(|s| s.individuals as i64).collect()),
        median_varying_columns: pick(shifts.iter().map(|s| s.varying_columns as i64).collect()),
        column_totals,
        selected: worlds
            .iter()
            .filter(|w| w.selected_over_drift(plan))
            .count(),
        eta_selected: worlds.iter().filter(|w| w.eta_over_drift(plan)).count(),
        plastic_selected: worlds.iter().filter(|w| w.plastic_over_drift(plan)).count(),
        drift_no_variance: worlds.iter().filter(|w| w.alleles.no_variance()).count(),
        median_eta_milli: pick(worlds.iter().map(|w| w.alleles.mean_eta_milli).collect()),
        median_marker_value_milli: pick(
            worlds
                .iter()
                .map(|w| w.alleles.marker_value_milli)
                .collect(),
        ),
        median_plastic_fraction_milli: pick(
            worlds
                .iter()
                .map(|w| w.alleles.plastic_fraction_milli)
                .collect(),
        ),
        median_marker_set_fraction_milli: pick(
            worlds
                .iter()
                .map(|w| w.alleles.marker_set_fraction_milli)
                .collect(),
        ),
        median_eta_excess_milli: pick(
            worlds
                .iter()
                .map(|w| w.alleles.eta_excess_milli())
                .collect(),
        ),
        median_plastic_excess_milli: pick(
            worlds
                .iter()
                .map(|w| w.alleles.plastic_excess_milli())
                .collect(),
        ),
        total_moved_eta_alleles: worlds.iter().map(|w| w.alleles.moved_eta_alleles).sum(),
        total_moved_marker_alleles: worlds.iter().map(|w| w.alleles.moved_marker_alleles).sum(),
        total_plastic_alleles: worlds.iter().map(|w| w.alleles.plastic_alleles).sum(),
        total_set_marker_alleles: worlds.iter().map(|w| w.alleles.set_marker_alleles).sum(),
        median_population: pick(worlds.iter().map(|w| w.population as i64).collect()),
        total_plasticity_updates: worlds.iter().map(|w| w.plasticity_updates_total).sum(),
    }
}

/// Seed-matched pairs of a world-level quantity.
pub fn pairs_of(
    treatment: &[WorldPlasticity],
    control: &[WorldPlasticity],
    quantity: impl Fn(&WorldPlasticity) -> i64,
) -> Vec<Pair> {
    let mut pairs = Vec::new();
    for world in treatment {
        if let Some(peer) = control.iter().find(|peer| peer.seed == world.seed) {
            pairs.push(Pair {
                seed: world.seed,
                treatment_milli: quantity(world),
                control_milli: quantity(peer),
            });
        }
    }
    pairs.sort_by_key(|pair| pair.seed);
    pairs
}

/// A seed-paired contrast of one quantity between two conditions.
pub fn contrast(pairs: &[Pair], plan: &PlasticityPlan) -> PairedResult {
    compare(
        pairs,
        plan.drift_margin_milli,
        500,
        Direction::Increase,
        plan.analysis_seed,
    )
}

/// One criterion's decision, with the counts that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verdict {
    pub criterion: String,
    pub treatment: String,
    pub control: String,
    pub treatment_count: usize,
    pub control_count: usize,
    pub treatment_worlds: usize,
    pub control_worlds: usize,
    pub bar: usize,
    pub ceiling: usize,
    /// Worlds excluded from the treatment count because the statistic was
    /// undefined, reported beside the decision because the bar is against the
    /// campaign's declared seed count and not against the usable subset.
    pub treatment_undefined: usize,
    pub met: bool,
}

impl Verdict {
    #[allow(clippy::too_many_arguments)]
    pub fn decide(
        criterion: &str,
        treatment: &PlasticityOutcome,
        control: &PlasticityOutcome,
        treatment_count: usize,
        control_count: usize,
        treatment_undefined: usize,
        bar: usize,
        ceiling: usize,
    ) -> Self {
        Self {
            criterion: criterion.to_owned(),
            treatment: treatment.condition.clone(),
            control: control.condition.clone(),
            treatment_count,
            control_count,
            treatment_worlds: treatment.worlds,
            control_worlds: control.worlds,
            bar,
            ceiling,
            treatment_undefined,
            met: treatment_count >= bar && control_count < ceiling,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    campaign_id: &str,
    plan: &PlasticityPlan,
    per_world: &[(String, Vec<WorldPlasticity>)],
    outcomes: &[PlasticityOutcome],
    contrasts: &[(String, String, String, PairedResult)],
    verdicts: &[Verdict],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "plasticity-report 1 campaign {campaign_id}");
    let _ = writeln!(out, "analysis-version {PLASTICITY_ANALYSIS_VERSION}");
    let _ = writeln!(
        out,
        "census-policy {}",
        sim_core::ACTION_CENSUS_POLICY_VERSION
    );
    let _ = writeln!(
        out,
        "plan burn_in_ticks={} min_individuals={} shift_bar={} control_ceiling={} \
         drift_bar={} drift_margin_milli={} permutations={} analysis_seed={:#018x}",
        plan.burn_in_ticks,
        plan.min_individuals,
        plan.shift_bar,
        plan.control_ceiling,
        plan.drift_bar,
        plan.drift_margin_milli,
        crate::morph::PERMUTATIONS,
        plan.analysis_seed,
    );
    let column_names: Vec<&str> = sim_core::ActionClass::ALL
        .iter()
        .map(|c| c.name())
        .collect();
    let _ = writeln!(out, "columns {}", column_names.join(","));

    for (condition, worlds) in per_world {
        for world in worlds {
            match &world.shift {
                Ok(shift) => {
                    let totals: Vec<String> = shift
                        .column_totals
                        .iter()
                        .map(|value| value.to_string())
                        .collect();
                    let _ = writeln!(
                        out,
                        "world condition={condition} seed={:#018x} population={} extinct={} \
                         boundaries={} individuals={} pairs={} rho_milli={} null_p95_milli={} \
                         median_event_milli={} median_control_milli={} \
                         median_event_loco_tv_milli={} median_control_loco_tv_milli={} \
                         event_wins={} control_wins={} ties={} \
                         distinct_distances={} varying_columns={} discarded={} \
                         strata_total={} strata_informative={} observations_dropped={} \
                         column_totals={} no_variance={} shift={} associated={}",
                        world.seed,
                        world.population,
                        world.extinct,
                        shift.boundaries,
                        shift.individuals,
                        shift.pairs,
                        shift.rho_milli,
                        shift.null_p95_milli,
                        shift.median_event_milli,
                        shift.median_control_milli,
                        shift.median_event_locomotion_tv_milli,
                        shift.median_control_locomotion_tv_milli,
                        shift.event_wins,
                        shift.control_wins,
                        shift.ties,
                        shift.distinct_distances,
                        shift.varying_columns,
                        shift.discarded,
                        shift.strata_total,
                        shift.strata_informative,
                        shift.observations_dropped,
                        totals.join(","),
                        shift.no_variance,
                        world.within_lifetime_shift(plan),
                        world.behaviour_associated(plan),
                    );
                }
                Err(refusal) => {
                    let _ = writeln!(
                        out,
                        "world condition={condition} seed={:#018x} population={} extinct={} \
                         refused={refusal}",
                        world.seed, world.population, world.extinct,
                    );
                }
            }
            let _ = writeln!(
                out,
                "drift condition={condition} seed={:#018x} organisms={} edge_alleles={} \
                 plastic_alleles={} marker_alleles={} set_marker_alleles={} \
                 mean_eta_milli={} marker_value_milli={} plastic_fraction_milli={} \
                 marker_set_fraction_milli={} eta_excess_milli={} plastic_excess_milli={} \
                 moved_eta_alleles={} moved_marker_alleles={} plastic_edges_total={} \
                 plasticity_updates={} mean_abs_learned_milli={} no_variance={} selected={}",
                world.seed,
                world.alleles.organisms,
                world.alleles.edge_alleles,
                world.alleles.plastic_alleles,
                world.alleles.marker_alleles,
                world.alleles.set_marker_alleles,
                world.alleles.mean_eta_milli,
                world.alleles.marker_value_milli,
                world.alleles.plastic_fraction_milli,
                world.alleles.marker_set_fraction_milli,
                world.alleles.eta_excess_milli(),
                world.alleles.plastic_excess_milli(),
                world.alleles.moved_eta_alleles,
                world.alleles.moved_marker_alleles,
                world.plastic_edges_total,
                world.plasticity_updates_total,
                world.mean_abs_learned_milli,
                world.alleles.no_variance(),
                world.selected_over_drift(plan),
            );
        }
    }

    for outcome in outcomes {
        let totals: Vec<String> = outcome
            .column_totals
            .iter()
            .map(|value| value.to_string())
            .collect();
        let _ = writeln!(
            out,
            "condition {} worlds={} extinct={} shifted={} associated={} shift_no_variance={} \
             shift_refused={} med_rho_milli={} med_null_milli={} med_event_milli={} \
             med_control_milli={} med_event_loco_tv_milli={} med_control_loco_tv_milli={} \
             med_individuals={} med_varying_columns={} column_totals={} \
             selected={} eta_selected={} plastic_selected={} drift_no_variance={} \
             med_eta_milli={} med_marker_value_milli={} med_plastic_fraction_milli={} \
             med_marker_set_fraction_milli={} med_eta_excess_milli={} \
             med_plastic_excess_milli={} moved_eta_alleles={} moved_marker_alleles={} \
             plastic_alleles={} set_marker_alleles={} med_population={} plasticity_updates={}",
            outcome.condition,
            outcome.worlds,
            outcome.extinct,
            outcome.shifted,
            outcome.associated,
            outcome.shift_no_variance,
            outcome.shift_refused,
            outcome.median_rho_milli,
            outcome.median_null_milli,
            outcome.median_event_distance_milli,
            outcome.median_control_distance_milli,
            outcome.median_event_locomotion_tv_milli,
            outcome.median_control_locomotion_tv_milli,
            outcome.median_individuals,
            outcome.median_varying_columns,
            totals.join(","),
            outcome.selected,
            outcome.eta_selected,
            outcome.plastic_selected,
            outcome.drift_no_variance,
            outcome.median_eta_milli,
            outcome.median_marker_value_milli,
            outcome.median_plastic_fraction_milli,
            outcome.median_marker_set_fraction_milli,
            outcome.median_eta_excess_milli,
            outcome.median_plastic_excess_milli,
            outcome.total_moved_eta_alleles,
            outcome.total_moved_marker_alleles,
            outcome.total_plastic_alleles,
            outcome.total_set_marker_alleles,
            outcome.median_population,
            outcome.total_plasticity_updates,
        );
    }

    for (treatment, control, quantity, result) in contrasts {
        let _ = writeln!(
            out,
            "contrast treatment={treatment} control={control} quantity={quantity} pairs={} \
             reaching_sesoi_directed={} mean_diff_milli={} median_diff_milli={} \
             ci95_milli=[{},{}] mean_rel_milli={} ci90_rel_milli=[{},{}] equivalent={} \
             p_milli={}",
            result.pairs,
            result.reaching_sesoi_directed,
            result.mean_difference_milli,
            result.median_difference_milli,
            result.ci_low_milli,
            result.ci_high_milli,
            result.mean_relative_milli,
            result.relative_ci_low_milli,
            result.relative_ci_high_milli,
            result.equivalent,
            result.sesoi_p_value_milli,
        );
    }

    for verdict in verdicts {
        let _ = writeln!(
            out,
            "criterion {} treatment={} control={} treatment_count={} treatment_worlds={} \
             control_count={} control_worlds={} bar={} ceiling={} treatment_undefined={} met={}",
            verdict.criterion,
            verdict.treatment,
            verdict.control,
            verdict.treatment_count,
            verdict.treatment_worlds,
            verdict.control_count,
            verdict.control_worlds,
            verdict.bar,
            verdict.ceiling,
            verdict.treatment_undefined,
            verdict.met,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{Locus, PlasticityGenes};

    fn counts(values: [i64; ACTION_CLASS_COUNT]) -> [i64; ACTION_CLASS_COUNT] {
        values
    }

    #[test]
    fn two_identical_windows_are_at_distance_zero_and_a_reversal_is_at_the_maximum() {
        // Locomotion entirely in one column against entirely in another: the
        // two probability vectors are disjoint, so the total variation is 1
        // and the L1 over the partition is 2.
        let left = counts([500, 0, 0, 0, 0, 0, 0]);
        let right = counts([0, 0, 0, 500, 0, 0, 0]);
        assert_eq!(window_distance(&left, &left).unwrap().l1_milli, 0);
        let moved = window_distance(&left, &right).unwrap();
        assert_eq!(moved.locomotion_tv_milli, 1_000);
        assert_eq!(moved.l1_milli, 2_000);
        assert_eq!(moved.indicator_l1_milli, 0);
    }

    #[test]
    fn a_saturated_indicator_contributes_nothing_and_does_not_mask_locomotion() {
        // `Mate` is saturated in every world Phase 11 has measured. It must
        // neither create a distance where there is none nor hide one that
        // exists, or the instrument's known degeneracy becomes the result.
        let left = counts([500, 0, 0, 0, 500, 500, 0]);
        let right = counts([0, 500, 0, 0, 500, 500, 0]);
        let distance = window_distance(&left, &right).unwrap();
        assert_eq!(distance.indicator_l1_milli, 0);
        assert_eq!(distance.locomotion_tv_milli, 1_000);
        // And the same locomotion move with no indicators at all gives the
        // same locomotion answer: the blocks are independent.
        let bare_left = counts([500, 0, 0, 0, 0, 0, 0]);
        let bare_right = counts([0, 500, 0, 0, 0, 0, 0]);
        assert_eq!(
            window_distance(&bare_left, &bare_right)
                .unwrap()
                .locomotion_tv_milli,
            distance.locomotion_tv_milli
        );
    }

    #[test]
    fn an_indicator_that_moves_alone_is_visible_even_with_identical_locomotion() {
        let left = counts([500, 0, 0, 0, 500, 0, 0]);
        let right = counts([500, 0, 0, 0, 0, 0, 0]);
        let distance = window_distance(&left, &right).unwrap();
        assert_eq!(distance.locomotion_tv_milli, 0);
        assert_eq!(distance.indicator_l1_milli, 1_000);
        assert_eq!(distance.l1_milli, 1_000);
    }

    #[test]
    fn an_empty_or_negative_window_is_refused_rather_than_scored_zero() {
        let empty = counts([0; ACTION_CLASS_COUNT]);
        let full = counts([500, 0, 0, 0, 0, 0, 0]);
        assert_eq!(window_distance(&empty, &full), None);
        assert_eq!(window_distance(&full, &empty), None);
        let negative = counts([-1, 501, 0, 0, 0, 0, 0]);
        assert_eq!(window_distance(&negative, &full), None);
    }

    #[test]
    fn boundaries_land_on_sample_ticks_and_a_misaligned_window_is_refused() {
        // R = 2000, W = 500: the control boundary at T + 1000 is a sample tick.
        let found = boundaries(2_000, 500, 6_000, 20_000).expect("aligned");
        assert_eq!(
            found[0],
            Boundary {
                event_tick: 8_000,
                control_tick: 9_000
            }
        );
        for boundary in &found {
            assert_eq!(boundary.event_tick % 2_000, 0);
            assert_eq!(boundary.control_tick % 500, 0);
            assert!(boundary.control_tick + 500 <= 20_000);
        }
        // Burn-in excludes 2000..6000 but not 8000; the first usable boundary
        // must be strictly past burn-in plus a window.
        assert!(found.iter().all(|b| b.event_tick >= 6_500));

        // R = 2000, W = 750: T + 1000 is not a multiple of 750, so there is no
        // control boundary to read. Refused, never interpolated.
        assert_eq!(
            boundaries(2_000, 750, 0, 20_000),
            Err(ShiftRefusal::Misaligned {
                relocate: 2_000,
                window: 750
            })
        );
        assert_eq!(boundaries(0, 500, 0, 20_000), Err(ShiftRefusal::NoSchedule));
        assert_eq!(
            boundaries(2_000, 500, 0, 2_000),
            Err(ShiftRefusal::NoBoundaries)
        );

        // R = 1500, W = 500: the interval **is** a whole number of windows,
        // so a guard that checked only that would accept it - and the
        // control at T + 750 would then fall between two samples, every pair
        // would be discarded, and the world would come back refused as
        // `too_few_individuals`. Two layers refusing the same input means
        // neither is pinned by the other's test (D-097), so the alignment
        // guard is pinned here by the diagnostic only it prints.
        assert_eq!(
            boundaries(1_500, 500, 0, 20_000),
            Err(ShiftRefusal::Misaligned {
                relocate: 1_500,
                window: 500
            })
        );

        // The horizon bound is inclusive: a boundary whose control window
        // ends exactly on the last sample is readable and must be kept. A
        // strict comparison here silently drops a whole boundary from every
        // world whose run length lands on the bound.
        let flush = boundaries(2_000, 500, 0, 19_500).expect("aligned");
        assert_eq!(
            flush.last().copied(),
            Some(Boundary {
                event_tick: 18_000,
                control_tick: 19_000
            })
        );
    }

    // --- C11.1: a synthetic `.alac` series -------------------------------
    //
    // Built one tick at a time from a per-tick locomotion column, so the
    // cumulative counts, the ages and the one-tick contamination of the
    // pre-window are all exactly what the kernel would have written rather
    // than what would be convenient to assert against.

    const W: u64 = 500;
    const R: u64 = 2_000;
    const HORIZON: u64 = 20_000;

    fn scan_from(
        organisms: u64,
        horizon: u64,
        column: impl Fn(u64, u64) -> usize,
        alive: impl Fn(u64, u64) -> bool,
    ) -> ActionLogScan {
        let mut cumulative: BTreeMap<u64, [u32; ACTION_CLASS_COUNT]> = BTreeMap::new();
        let mut age: BTreeMap<u64, u64> = BTreeMap::new();
        let mut samples = Vec::new();
        for tick in 1..=horizon {
            for id in 1..=organisms {
                if !alive(tick, id) {
                    continue;
                }
                let row = cumulative.entry(id).or_insert([0; ACTION_CLASS_COUNT]);
                row[column(tick, id)] += 1;
                // `Mate` saturated, as it is in every world Phase 11 has
                // measured; `Eat` and `Attack` silent.
                row[sim_core::ActionClass::Mate as usize] += 1;
                *age.entry(id).or_insert(0) += 1;
            }
            if tick.is_multiple_of(W) {
                let records = (1..=organisms)
                    .filter(|id| alive(tick, *id))
                    .map(|id| sim_persist::ActionRecord {
                        id,
                        age_ticks: age[&id],
                        counts: cumulative[&id],
                    })
                    .collect();
                samples.push(sim_persist::ActionSampleSet { tick, records });
            }
        }
        scan_of(samples)
    }

    /// The provenance block every synthetic series in this module shares.
    fn scan_of(samples: Vec<sim_persist::ActionSampleSet>) -> ActionLogScan {
        ActionLogScan {
            info: sim_persist::ActionLogInfo {
                format_version: sim_persist::ACTION_LOG_FORMAT_VERSION,
                world_id: 1,
                seed: 1,
                config_hash: 0,
                terrain_checksum: 0,
                class_count: ACTION_CLASS_COUNT as u32,
                sample_interval_ticks: W as u32,
                max_organisms: 100_000,
                policy_hash: sim_persist::action_policy_hash(),
                build_version: "test".to_owned(),
            },
            samples,
            bytes_consumed: 0,
            truncated_at: None,
        }
    }

    /// Locomotion column fixed for the whole epoch and redrawn at each
    /// relocation: the planted within-lifetime response.
    fn epoch_locked(tick: u64, id: u64) -> usize {
        let epoch = tick / R;
        ((epoch * 3 + id) % LOCOMOTION_CLASS_COUNT as u64) as usize
    }

    /// Births staggered one window apart, so the population holds organisms
    /// of many different ages at once.
    ///
    /// **Every age-matched design needs this and a synchronised population
    /// cannot supply it.** One organism's event and control observations are
    /// always `R / 2` ticks apart in age, so an age stratum holds both labels
    /// only when the population contains organisms born about that far apart.
    /// A world where everything is alive from tick 1 confounds the event
    /// label with age perfectly, and `world_shift` refuses it rather than
    /// scoring it - see
    /// `a_birth_synchronised_population_is_refused_rather_than_scored`.
    ///
    /// Staggering is also the realistic case by a wide margin: the
    /// confirmatory campaign's worlds recorded tens of thousands of births
    /// each, against a founder cohort of a few hundred.
    fn staggered(tick: u64, id: u64) -> bool {
        tick >= (id - 1) * W
    }

    /// Behavioural change of the same size, redrawn every window from a mixer
    /// that knows nothing about epochs. Adjacent windows land in the same
    /// column about a quarter of the time, so the distances genuinely vary -
    /// they are simply uncorrelated with the relocation schedule.
    fn window_locked(tick: u64, id: u64) -> usize {
        let window = (tick - 1) / W;
        let mut mixed = window
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(id.wrapping_mul(1_442_695_040_888_963_407));
        mixed ^= mixed >> 33;
        mixed = mixed.wrapping_mul(0xff51_afd7_ed55_8ccd);
        mixed ^= mixed >> 33;
        (mixed % LOCOMOTION_CLASS_COUNT as u64) as usize
    }

    #[test]
    fn a_planted_within_lifetime_response_beats_its_permutation_null() {
        // Behaviour that changes at each relocation and nowhere else. The
        // event windows straddle a change; the matched control windows lie
        // inside one epoch and do not.
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, epoch_locked, staggered);
        let shift = world_shift(&scan, R, &plan, 11).expect("computable");
        assert_eq!(shift.individuals, 24);
        assert!(shift.boundaries >= 4, "{shift:?}");
        assert!(!shift.no_variance);
        // **This is the half of the age-matching change that stops it being a
        // fix that breaks the instrument.** A straightened ruler that can no
        // longer measure anything satisfies "the fake world scores zero" just
        // as well as a correct one does, so the stratified null has to be
        // shown clearing a planted effect as well as refusing an age
        // artifact. It does, and by a wide margin: a perfect correlation
        // against a null that the stratification has actually widened
        // (140 milli here, against 8-13 in the campaign's pooled worlds).
        assert_eq!((shift.rho_milli, shift.null_p95_milli), (1_000, 140));
        assert!(
            shift.rho_milli > shift.null_p95_milli,
            "planted response {} did not beat its null {}",
            shift.rho_milli,
            shift.null_p95_milli
        );
        // The exclusion is small here and is asserted rather than assumed:
        // four strata at the extremes of the age range hold only one label,
        // and the eight observations in them are dropped from the observed
        // statistic and the null alike.
        assert_eq!(
            (
                shift.strata_total,
                shift.strata_informative,
                shift.observations_dropped
            ),
            (38, 34, 8)
        );
        // The event windows sit at the top of the distance scale and the
        // control windows at the bottom, which is what the correlation is
        // reporting. Not exactly 2,000 and 0: the sample at the relocation
        // tick is taken *after* the relocation ran, so the pre-window carries
        // one post-relocation tick out of 500.
        assert_eq!(shift.median_event_milli, 1_996);
        assert_eq!(shift.median_control_milli, 0);
        assert_eq!(shift.median_event_locomotion_tv_milli, 998);
        // The paired view agrees with the pooled one: every organism's event
        // pair is further apart than its own control pair.
        assert_eq!(shift.control_wins, 0);
        assert_eq!(shift.event_wins + shift.ties, shift.pairs);
        assert!(shift.event_wins > shift.ties);
        // The saturated indicator contributed nothing, exactly as designed.
        assert_eq!(
            shift.column_totals[sim_core::ActionClass::Mate as usize],
            shift.column_totals[..LOCOMOTION_CLASS_COUNT]
                .iter()
                .sum::<u64>()
        );
    }

    /// The column changes at the epoch **midpoint** instead of the
    /// relocation: an association of the same size running the other way.
    fn midpoint_locked(tick: u64, id: u64) -> usize {
        let key = (tick + R / 2) / R;
        ((key * 3 + id) % LOCOMOTION_CLASS_COUNT as u64) as usize
    }

    #[test]
    fn a_reversed_association_is_reported_and_does_not_count_as_a_within_lifetime_shift() {
        // The whole reason the decision rule is directed. Here behaviour is
        // *more* stable across a relocation than across a matched boundary
        // inside one epoch - a real association, and the opposite of the claim
        // C11.1 makes. It must be visible in the report and must not pass.
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, midpoint_locked, staggered);
        let shift = world_shift(&scan, R, &plan, 37).expect("computable");
        assert_eq!(shift.median_event_milli, 0);
        assert_eq!(shift.median_control_milli, 1_996);
        assert_eq!(shift.event_wins, 0);
        assert!(shift.rho_milli < -shift.null_p95_milli, "{shift:?}");
        let world = WorldPlasticity {
            shift: Ok(shift),
            ..world_for(AlleleCensus::default())
        };
        assert!(
            !world.within_lifetime_shift(&plan),
            "a reversed association passed a directed criterion"
        );
        assert!(
            world.behaviour_associated(&plan),
            "the reversed association was invisible in the report"
        );
    }

    #[test]
    fn the_same_amount_of_change_uncorrelated_with_the_event_does_not_beat_its_null() {
        // The control for the test above, and the assertion that makes the
        // bar discriminating: behaviour that changes just as much, but on a
        // schedule of its own, must fail. Without this an analysis that
        // simply measured "did anything change" would pass C11.1 in every
        // world with a living population.
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, window_locked, staggered);
        let shift = world_shift(&scan, R, &plan, 13).expect("computable");
        assert!(!shift.no_variance, "the distances must actually vary");
        assert!(shift.distinct_distances > 1);
        // **The null has to be doing the work.** A correlation of exactly
        // zero would clear any bar, and a null of exactly zero would be
        // cleared by any correlation, so both are asserted to be nonzero
        // before the comparison between them means anything.
        assert!(
            shift.null_p95_milli > 0,
            "the permutation null collapsed to zero, so the bar is vacuous"
        );
        assert_ne!(
            shift.rho_milli, 0,
            "the observed correlation is exactly zero, so it would clear any null"
        );
        assert!(
            shift.rho_milli.abs() <= shift.null_p95_milli,
            "an unrelated schedule beat its null: rho {} vs {}",
            shift.rho_milli,
            shift.null_p95_milli
        );
    }

    #[test]
    fn a_world_whose_behaviour_never_varies_is_undefined_rather_than_a_failure() {
        // Phase 11's own substrate measurement found four of the seven
        // columns empty or saturated in a founder-dominated world. A world in
        // which nothing at all varies cannot speak to whether behaviour
        // changed, and counting it as a refutation would let a degenerate
        // controller stand in for a result.
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, |_, _| 1, staggered);
        let shift = world_shift(&scan, R, &plan, 17).expect("computable");
        assert!(shift.no_variance);
        assert_eq!(shift.distinct_distances, 1);
        assert_eq!((shift.rho_milli, shift.null_p95_milli), (0, 0));
        assert_eq!(shift.varying_columns, 0);
        // Every pair is a tie, and a tie is neither side winning. This is the
        // case that tells a fold-ties-into-wins bug from a real majority: in
        // the planted world above there are no ties at all, so only a world
        // made entirely of them can pin the boundary.
        assert_eq!((shift.event_wins, shift.control_wins), (0, 0));
        assert_eq!(shift.ties, shift.pairs);
        assert!(shift.pairs > 0);
        let world = WorldPlasticity {
            shift: Ok(shift),
            ..world_for(AlleleCensus::default())
        };
        assert!(!world.within_lifetime_shift(&plan));
        assert!(world.shift_no_variance());
    }

    #[test]
    fn an_undefined_world_cannot_pass_even_with_a_correlation_attached_to_it() {
        // `world_shift` zeroes both statistics whenever it reports
        // `no_variance`, so in a real report the two guards always agree -
        // and that agreement is exactly why this test has to exist. Without
        // it, deleting the `no_variance` check from the decision changes
        // nothing observable and no test notices, and the criterion quietly
        // stops distinguishing "behaviour did not change" from "there was no
        // behavioural variance for an event to change". The record is built
        // in the inconsistent state on purpose: the decision must respect the
        // reported fact, not the arithmetic that usually accompanies it.
        let plan = PlasticityPlan::default();
        let world = WorldPlasticity {
            shift: Ok(ShiftResult {
                no_variance: true,
                rho_milli: 900,
                null_p95_milli: 10,
                individuals: 50,
                pairs: 50,
                ..ShiftResult::default()
            }),
            ..world_for(AlleleCensus::default())
        };
        assert!(
            !world.within_lifetime_shift(&plan),
            "an undefined world passed a criterion it has nothing to say about"
        );
        assert!(!world.behaviour_associated(&plan));
        assert!(world.shift_no_variance());
    }

    #[test]
    fn an_organism_missing_from_any_of_the_six_samples_contributes_no_pair() {
        // The property the entity id is in the file for. An organism that is
        // not present at every sample a matched pair needs is dropped, rather
        // than compared against whoever now occupies its row.
        let plan = PlasticityPlan::default();
        // Half the population dies at tick 9,000, inside the first usable
        // boundary's control window.
        let scan = scan_from(24, HORIZON, epoch_locked, |tick, id| {
            staggered(tick, id) && (id % 2 == 1 || tick < 9_000)
        });
        let shift = world_shift(&scan, R, &plan, 19).expect("computable");
        assert_eq!(shift.individuals, 12, "a dead organism contributed a pair");
        assert!(shift.discarded > 0, "the drops were not counted");
    }

    #[test]
    fn a_break_in_the_age_series_drops_the_pair_rather_than_spanning_it() {
        // Presence at both ends of a window is not enough: the ages have to
        // account for every tick between them, or the two records are not one
        // continuous life. This is what an id reuse would look like, and it
        // must not produce a window.
        let plan = PlasticityPlan::default();
        let mut scan = scan_from(24, HORIZON, epoch_locked, staggered);
        let full = world_shift(&scan, R, &plan, 23).expect("computable").pairs;
        for sample in scan.samples.iter_mut() {
            if sample.tick >= 8_000 {
                for record in sample.records.iter_mut() {
                    if record.id == 1 {
                        record.age_ticks -= 1;
                    }
                }
            }
        }
        let broken = world_shift(&scan, R, &plan, 23).expect("computable");
        assert!(
            broken.pairs < full,
            "a discontinuous age series still produced every pair"
        );
        assert!(broken.discarded > 0, "the broken pairs were not counted");
    }

    #[test]
    fn a_condition_with_no_relocation_schedule_is_refused_rather_than_scored_zero() {
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, epoch_locked, staggered);
        assert_eq!(
            world_shift(&scan, 0, &plan, 29),
            Err(ShiftRefusal::NoSchedule)
        );
        // ...and a population too small to support a permutation is refused
        // by name rather than reported as a null.
        let thin = scan_from(2, HORIZON, epoch_locked, |_, _| true);
        assert_eq!(
            world_shift(&thin, R, &plan, 31),
            Err(ShiftRefusal::TooFewIndividuals {
                individuals: 2,
                required: plan.min_individuals
            })
        );
        // ...and exactly `min_individuals` is enough. The bar is "fewer
        // than", so the world at the bar is read rather than refused, and
        // the boundary is the only place the two readings differ.
        let exact = scan_from(
            plan.min_individuals as u64,
            HORIZON,
            epoch_locked,
            staggered,
        );
        let shift = world_shift(&exact, R, &plan, 33).expect("the bar itself is enough");
        assert_eq!(shift.individuals, plan.min_individuals);
    }

    // --- The matched control is offset in age, and that is enough --------
    //
    // `boundaries` puts the control at `event + relocate / 2`, so a control
    // window pair lies `relocate / 2` ticks later in **every** organism's
    // life than that organism's own event window pair. Nothing else about
    // the two pairs differs. The series below makes the age offset the only
    // difference there is: no relocation is simulated, nothing whatever
    // happens at the event tick, and the world is stationary in absolute
    // time, so any statistic the analysis reports here is the offset and
    // nothing else.
    //
    // The construction is a rolling cohort. `AGE_LANES` organisms are born
    // every `W` ticks and each lives exactly `AGE_LIFESPAN`, so at every
    // sample tick the population holds exactly the same multiset of ages -
    // `AGE_COHORT` of them, `W` apart - and no world-level quantity moves
    // with the tick at all. Each organism spends `turn_ticks(k)` of its
    // `k`-th window of life turning right and the rest moving ahead, so
    // `window_distance` is exactly `4 * |turn_ticks(k) - turn_ticks(k + 1)|`
    // and a pure function of age.

    const AGE_LANES: u64 = 8;
    const AGE_COHORT: u64 = 24;
    const AGE_LIFESPAN: u64 = AGE_COHORT * W;
    const AGE_HORIZON: u64 = 40_000;
    const AGE_BURN_IN: u64 = 14_000;

    /// Ticks of `TurnRight` in the `k`-th window of one organism's life.
    type TurnSchedule = fn(u64, u64) -> u64;

    fn rolling_cohort_scan(turn_ticks: TurnSchedule, lanes: u64) -> ActionLogScan {
        let mut samples = Vec::new();
        for step in 1..=(AGE_HORIZON / W) {
            let tick = step * W;
            let mut records = Vec::new();
            for birth in step.saturating_sub(AGE_COHORT)..step {
                let age = tick - birth * W;
                if age > AGE_LIFESPAN {
                    continue;
                }
                for lane in 0..lanes {
                    let mut counts = [0_u32; ACTION_CLASS_COUNT];
                    for k in 0..(age / W) {
                        let turn = turn_ticks(k, lane);
                        counts[sim_core::ActionClass::TurnRight as usize] += turn as u32;
                        counts[sim_core::ActionClass::MoveAhead as usize] += (W - turn) as u32;
                    }
                    // `Mate` saturated, as in every world Phase 11 measured.
                    counts[sim_core::ActionClass::Mate as usize] += age as u32;
                    records.push(sim_persist::ActionRecord {
                        id: birth * lanes + lane + 1,
                        age_ticks: age,
                        counts,
                    });
                }
            }
            records.sort_by_key(|record| record.id);
            samples.push(sim_persist::ActionSampleSet { tick, records });
        }
        scan_of(samples)
    }

    fn age_plan() -> PlasticityPlan {
        PlasticityPlan {
            burn_in_ticks: AGE_BURN_IN,
            ..PlasticityPlan::default()
        }
    }

    /// Window-to-window change that **decays** with age: the organism
    /// settles down. `t(k) = W * K / (K + k)`.
    fn settling(k: u64, lane: u64) -> u64 {
        let scale = 20 + lane;
        W * scale / (scale + k)
    }

    /// The same curve run backwards: change that **grows** with age.
    fn unsettling(k: u64, lane: u64) -> u64 {
        settling(30_u64.saturating_sub(k), lane)
    }

    /// An age trend with no curvature: the turn fraction moves by a fixed
    /// number of ticks every window for the whole of life, so behaviour
    /// depends on age but the window-to-window change does not.
    fn linear_in_age(k: u64, lane: u64) -> u64 {
        480 - (lane + 2) * k
    }

    #[test]
    fn the_age_offset_alone_no_longer_reaches_the_statistic_under_a_stratified_null() {
        // **The permanent tripwire for D-100.** A world in which nothing
        // whatever happens at the event tick must not score. Before the null
        // was stratified this construction scored `rho = +158` against a p95
        // of 30 and **passed C11.1's own directed decision rule** - the
        // criterion cleared in a world with no event in it. Those numbers are
        // kept here as the record of what the unstratified null did; the
        // assertions below are what the stratified one does.
        //
        // Why the artifact vanishes rather than shrinks: an event pair at
        // boundary `T` and a control pair at boundary `T + R/2` both cover
        // ages `[a - W, a]` and `[a, a + W]` for an organism of age `a` at
        // its own boundary. Two observations sharing an age stratum therefore
        // cover identical age ranges, and in a world where behaviour is a
        // pure function of age they carry identical distances. The
        // within-stratum association is exactly zero by construction, not
        // small by luck.
        //
        // If a future pre-registration changes the pairing again, this test is
        // **expected to fail** and these numbers are what the change must be
        // measured against. Do not silence it; re-measure it.
        let plan = age_plan();

        // 1. Behaviour that settles with age. Under the old null the younger
        //    event windows moved more and the correlation went positive; the
        //    stratified null reads exactly nothing.
        let scan = rolling_cohort_scan(settling, AGE_LANES);
        let settled = world_shift(&scan, R, &plan, 101).expect("computable");
        assert_eq!((settled.rho_milli, settled.null_p95_milli), (0, 6));
        let world = WorldPlasticity {
            shift: Ok(settled.clone()),
            ..world_for(AlleleCensus::default())
        };
        assert!(
            !world.within_lifetime_shift(&plan),
            "the age offset alone still reproduces the criterion's pass condition"
        );
        // Not vacuous: the world still contains plenty of behavioural
        // variation and plenty of usable strata. It is the *association* that
        // is gone, not the data.
        assert!(!settled.no_variance);
        assert!(settled.distinct_distances > 1);
        assert!(settled.strata_informative > 0);

        // 2. The same construction with the age trend reversed. Under the old
        //    null this was `-154` against 28 and produced the sign-reversed
        //    association the campaign measured in all four arms. It is now
        //    also nothing, which is what says the statistic reads the event
        //    and not the trend in either direction.
        let scan = rolling_cohort_scan(unsettling, AGE_LANES);
        let unsettled = world_shift(&scan, R, &plan, 101).expect("computable");
        assert_eq!(unsettled.rho_milli, 0);
        let world = WorldPlasticity {
            shift: Ok(unsettled),
            ..world_for(AlleleCensus::default())
        };
        assert!(!world.behaviour_associated(&plan));
        assert!(!world.within_lifetime_shift(&plan));

        // 3. The control that makes the first two mean something. Behaviour
        //    still depends on age, and still changes every window of every
        //    life - but by the *same amount* at every age. The offset then
        //    has nothing to read and the correlation is exactly zero. So the
        //    rolling cohort does not manufacture an association by itself:
        //    what the statistic reports is curvature in the age trend.
        let scan = rolling_cohort_scan(linear_in_age, AGE_LANES);
        let flat = world_shift(&scan, R, &plan, 101).expect("computable");
        assert_eq!(flat.rho_milli, 0);
        assert!(!flat.no_variance, "the distances must actually vary");
        assert!(flat.distinct_distances > 1);
        assert_eq!(flat.ties, flat.pairs);
        let world = WorldPlasticity {
            shift: Ok(flat),
            ..world_for(AlleleCensus::default())
        };
        assert!(!world.behaviour_associated(&plan));

        // 4. Dose-response on the offset itself, which is what identified the
        //    age gap as the mechanism in the first place. The control sits at
        //    `relocate / 2`, so widening the relocation interval widens the
        //    age gap and nothing else. Under the unstratified null the
        //    correlation scaled with it - 76 / 158 / 334 / 700 at gaps of
        //    500 / 1,000 / 2,000 / 4,000, about doubling per doubling. The
        //    stratified null must be flat at zero across the whole dose
        //    range: a fix that only worked at the gap it was tuned on would
        //    show up here as a residual that grows with the dose.
        let scan = rolling_cohort_scan(settling, AGE_LANES);
        let dose: Vec<(u64, i64)> = [1_000_u64, 2_000, 4_000, 8_000]
            .into_iter()
            .map(|relocate| {
                let shift = world_shift(&scan, relocate, &plan, 101).expect("computable");
                (relocate / 2, shift.rho_milli)
            })
            .collect();
        assert_eq!(dose, vec![(500, 0), (1_000, 0), (2_000, 0), (4_000, 0)]);

        // 5. The scaling argument, which is why the defect decided worlds
        //    rather than being lost in the noise. Under the unstratified null
        //    the artifact was a *fixed* effect size while the null shrank
        //    with the number of pooled observations - 163/85, 160/59, 158/30,
        //    139/15 at 240, 480, 1,920 and 7,680 pairs - so pooling bought
        //    significance for a bias. The campaign pooled about 27,000 pairs
        //    per world and reported nulls as low as 6 milli.
        //
        //    Stratified, the effect is zero at every scale, so there is
        //    nothing for a larger sample to certify. Asserting this across
        //    four population sizes is what rules out "the fix works at the
        //    size I happened to test".
        let scale: Vec<(usize, i64)> = [1_u64, 2, 8, 32]
            .into_iter()
            .map(|lanes| {
                let scan = rolling_cohort_scan(settling, lanes);
                let shift = world_shift(&scan, R, &plan, 101).expect("computable");
                (shift.pairs, shift.rho_milli)
            })
            .collect();
        assert_eq!(scale, vec![(240, 0), (480, 0), (1_920, 0), (7_680, 0)]);
    }

    #[test]
    fn a_birth_synchronised_population_is_refused_rather_than_scored() {
        // The cost of age-matching, stated as a refusal instead of hidden in
        // a number. One organism's event and control observations are always
        // `R / 2` ticks apart in age, so an age stratum can hold both labels
        // only if the population contains organisms born about that far
        // apart. When everything is alive from tick 1, the event label and
        // age are perfectly confounded and **no age-matched comparison exists
        // in that population at all**.
        //
        // The honest answer is that C11.1 is unanswerable in such a world,
        // not that it was answered in the negative. Scoring it would let a
        // demographic accident stand in for a refutation of lifetime
        // learning - D-079's lesson - so it is a typed refusal, pinned here
        // by the diagnostic only it prints.
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, epoch_locked, |_, _| true);
        let refusal = world_shift(&scan, R, &plan, 11).expect_err("must refuse");
        assert!(
            matches!(refusal, ShiftRefusal::NoInformativeStrata { .. }),
            "{refusal:?}"
        );
        assert!(
            refusal.to_string().starts_with("no_informative_strata("),
            "{refusal}"
        );

        // And the guard is not firing for want of data: the very same world
        // with births staggered one window apart is richly computable, and
        // carries the strongest possible planted signal. The difference
        // between the two is demography and nothing else - same organisms,
        // same behaviour rule, same horizon.
        let staggered_scan = scan_from(24, HORIZON, epoch_locked, staggered);
        let shift = world_shift(&staggered_scan, R, &plan, 11).expect("computable");
        assert_eq!(shift.rho_milli, 1_000);
        assert!(shift.strata_informative > 0);
    }

    #[test]
    fn the_analysis_keys_on_the_entity_id_and_not_on_the_row_it_arrived_in() {
        // The property the module's own documentation says the entity id is
        // in the file for, and which nothing tested: every synthetic series
        // here writes records in ascending id order with identical ages, so
        // keying on the array slot and keying on the id agree everywhere.
        //
        // Reversing the records inside *alternate* samples changes no id, no
        // age and no count - it changes only which row an organism arrived
        // in - and a slot-keyed analysis then compares one organism's
        // pre-window with another's post-window without any age check being
        // able to notice, because in this series every organism is exactly
        // as old as every other.
        let plan = PlasticityPlan::default();
        let ordered = scan_from(12, HORIZON, epoch_locked, |_, _| true);
        let mut shuffled = ordered.clone();
        for sample in shuffled.samples.iter_mut() {
            if sample.tick.is_multiple_of(2 * W) {
                sample.records.reverse();
            }
        }
        assert!(
            shuffled
                .samples
                .iter()
                .zip(ordered.samples.iter())
                .any(|(left, right)| left
                    .records
                    .iter()
                    .map(|record| record.id)
                    .ne(right.records.iter().map(|record| record.id))),
            "the reordering did not reorder anything, so the comparison is vacuous"
        );
        assert_eq!(
            world_shift(&ordered, R, &plan, 41),
            world_shift(&shuffled, R, &plan, 41),
            "the result moved when only the row order moved"
        );
    }

    /// Locomotion pinned to one column for every organism at every tick,
    /// with the `Eat` indicator held for a whole epoch and redrawn at each
    /// relocation. The behavioural change is real and is carried entirely by
    /// an indicator column.
    fn indicator_only_scan(organisms: u64, horizon: u64) -> ActionLogScan {
        let mut cumulative: BTreeMap<u64, [u32; ACTION_CLASS_COUNT]> = BTreeMap::new();
        let mut samples = Vec::new();
        for tick in 1..=horizon {
            for id in 1..=organisms {
                if !staggered(tick, id) {
                    continue;
                }
                let row = cumulative.entry(id).or_insert([0; ACTION_CLASS_COUNT]);
                row[sim_core::ActionClass::MoveAhead as usize] += 1;
                row[sim_core::ActionClass::Mate as usize] += 1;
                if (tick / R + id).is_multiple_of(2) {
                    row[sim_core::ActionClass::Eat as usize] += 1;
                }
            }
            if tick.is_multiple_of(W) {
                samples.push(sim_persist::ActionSampleSet {
                    tick,
                    records: (1..=organisms)
                        .filter(|id| staggered(tick, *id))
                        .map(|id| sim_persist::ActionRecord {
                            id,
                            age_ticks: tick - (id - 1) * W,
                            counts: cumulative[&id],
                        })
                        .collect(),
                });
            }
        }
        scan_of(samples)
    }

    #[test]
    fn a_change_carried_entirely_by_an_indicator_column_still_reaches_the_statistic() {
        // The pooled statistic correlates the **full** L1, not the
        // locomotion half of it. Every other series in this module leaves
        // the indicator block constant, so `l1_milli` there is exactly twice
        // `locomotion_tv_milli` - a monotone transform, identical ranks, and
        // therefore a world in which correlating on either one gives the
        // same answer. Here locomotion never moves at all, so an analysis
        // that dropped the indicators would report a world with no
        // behavioural variance rather than a planted response.
        let plan = PlasticityPlan::default();
        let scan = indicator_only_scan(12, HORIZON);
        let shift = world_shift(&scan, R, &plan, 47).expect("computable");
        assert!(!shift.no_variance);
        assert_eq!(shift.median_event_milli, 998);
        assert_eq!(shift.median_control_milli, 0);
        assert_eq!(shift.median_event_locomotion_tv_milli, 0);
        assert!(shift.rho_milli > shift.null_p95_milli, "{shift:?}");

        // And the per-column occupancy is normalized by the locomotion block
        // alone, which is what keeps a constant locomotion column reading as
        // constant while a co-occurring indicator moves. Normalized by every
        // column instead, `move_ahead` and `mate` would both start varying
        // because the denominator itself would move with `Eat`.
        assert_eq!(
            shift.varying_columns, 1,
            "only `eat` varies; a denominator that moves with the indicators \
             would make the constant columns vary too"
        );
    }

    #[test]
    fn the_permutation_null_is_a_function_of_the_recorded_analysis_seed() {
        // The plan records `analysis_seed` and the report echoes it, on the
        // stated ground that the null is a function of the data and that
        // value only. Nothing checked that the value reached the shuffle:
        // a null computed from a constant seed is equally reproducible and
        // makes the recorded number a decoration.
        let plan = PlasticityPlan::default();
        let scan = scan_from(24, HORIZON, window_locked, staggered);
        let read = |seed: u64| world_shift(&scan, R, &plan, seed).expect("computable");
        let nulls: Vec<i64> = (1..=11_u64).map(|seed| read(seed).null_p95_milli).collect();
        let observed: Vec<i64> = (1..=11_u64).map(|seed| read(seed).rho_milli).collect();
        assert!(
            observed.iter().all(|rho| *rho == observed[0]),
            "the observed statistic moved with the permutation seed: {observed:?}"
        );
        assert!(
            nulls.iter().any(|null| *null != nulls[0]),
            "the null never moved across eleven seeds, so the recorded seed is not \
             the one the shuffle used: {nulls:?}"
        );
        assert_eq!(
            read(1),
            world_shift(&scan, R, &plan, 1).expect("computable"),
            "the same seed did not reproduce the same report"
        );
    }

    #[test]
    fn a_correlation_exactly_at_the_null_does_not_clear_it() {
        // The pre-registration says the correlation "must exceed the 95th
        // percentile of its own permutation null". Equality is the boundary
        // and it is not reachable from any synthetic series, so it needs a
        // constructed record - the same argument as
        // `an_undefined_world_cannot_pass_even_with_a_correlation_attached`.
        let plan = PlasticityPlan::default();
        let at = |rho: i64| WorldPlasticity {
            shift: Ok(ShiftResult {
                rho_milli: rho,
                null_p95_milli: 120,
                individuals: 50,
                pairs: 50,
                distinct_distances: 9,
                ..ShiftResult::default()
            }),
            ..world_for(AlleleCensus::default())
        };
        assert!(
            !at(120).within_lifetime_shift(&plan),
            "equality passed a bar it must exceed"
        );
        assert!(!at(120).behaviour_associated(&plan));
        assert!(
            at(121).within_lifetime_shift(&plan),
            "one milli over the null did not pass"
        );
    }

    #[test]
    fn the_condition_summary_counts_the_directed_shift_and_reports_the_association_beside_it() {
        // `summarise` had no test at all, and `outcome.shifted` is exactly
        // what `Verdict::decide` reads as C11.1's treatment count. So the
        // directed decision rule, which `within_lifetime_shift` defends and
        // which two tests pin there, is re-decided one layer up where
        // nothing was watching: counting `behaviour_associated` here would
        // have turned the campaign's 0 of 30 into 29 of 30.
        let plan = PlasticityPlan::default();
        let with = |rho: i64| WorldPlasticity {
            shift: Ok(ShiftResult {
                rho_milli: rho,
                null_p95_milli: 10,
                individuals: 40,
                pairs: 40,
                distinct_distances: 9,
                ..ShiftResult::default()
            }),
            ..world_for(AlleleCensus::default())
        };
        let outcome = summarise("Avar", &[with(-900), with(900), with(3)], &plan);
        assert_eq!(outcome.worlds, 3);
        assert_eq!(
            outcome.shifted, 1,
            "the reversed association was counted as a within-lifetime shift"
        );
        assert_eq!(
            outcome.associated, 2,
            "the two-sided count must see both directions"
        );
        assert_eq!(outcome.shift_no_variance, 0);
        assert_eq!(outcome.shift_refused, 0);
    }

    #[test]
    fn a_treatment_world_with_no_seed_matched_control_contributes_no_pair() {
        // `pairs_of` had no test. The contrasts it feeds are reported rather
        // than decisive, but a pairing that falls through to the nearest
        // available seed silently compares two different worlds - and the
        // case it has to survive is the real one, a control arm with a world
        // missing.
        let with = |seed: u64, population: u64| WorldPlasticity {
            seed,
            population,
            ..world_for(AlleleCensus::default())
        };
        let treatment = vec![with(3, 300), with(1, 100), with(2, 200)];
        let control = vec![with(1, 11), with(3, 33)];
        let pairs = pairs_of(&treatment, &control, |world| world.population as i64);
        assert_eq!(
            pairs.iter().map(|pair| pair.seed).collect::<Vec<_>>(),
            vec![1, 3],
            "an unmatched treatment seed was paired with somebody else's control"
        );
        assert_eq!(
            (pairs[0].treatment_milli, pairs[0].control_milli),
            (100, 11)
        );
        assert_eq!(
            (pairs[1].treatment_milli, pairs[1].control_milli),
            (300, 33)
        );
    }

    fn edge(homology_id: u32, eta: f32, flags: u8) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Edge {
                source: 1,
                target: 2,
                weight: 1.0,
                flags,
                plasticity: PlasticityGenes {
                    eta,
                    ..PlasticityGenes::inert()
                },
            },
        }
    }

    fn marker(homology_id: u32, value: f32, flags: u8) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Marker { value, flags },
        }
    }

    fn genome(loci: Vec<Locus>) -> Genome2 {
        let build = |loci: &Vec<Locus>| sim_core::Haplotype {
            chromosomes: vec![loci.clone()],
        };
        Genome2 {
            haplotypes: [build(&loci), build(&loci)],
        }
    }

    #[test]
    fn a_founder_population_has_no_variance_on_either_side_of_the_drift_comparison() {
        // Every allele at the founder value: the marker never moved, so the
        // control never ran and the excess is undefined rather than zero.
        // Counting this as a failure of C11.2 would let a low effective
        // mutation rate masquerade as a refutation of the mechanism.
        let founders: Vec<Genome2> = (0..20)
            .map(|_| {
                genome(vec![
                    edge(4_000, 0.0, 0),
                    marker(4_500, 0.0, 0),
                    edge(5_000, 0.0, 0),
                ])
            })
            .collect();
        let census = allele_census(&founders);
        assert!(census.no_variance());
        assert_eq!(census.eta_excess_milli(), 0);
        assert_eq!(census.edge_alleles, 80);
        assert_eq!(census.marker_alleles, 40);
    }

    #[test]
    fn a_marker_that_moved_while_eta_did_not_is_a_failure_and_not_undefined() {
        // The phase's named most-likely outcome. Drift moved the control and
        // plasticity did not follow, which is a real directed result: it must
        // be counted as a failure of C11.2 rather than excluded as undefined.
        let worlds: Vec<Genome2> = (0..20)
            .map(|_| {
                genome(vec![
                    edge(4_000, 0.0, 0),
                    marker(4_500, 0.2, 0),
                    edge(5_000, 0.0, 0),
                ])
            })
            .collect();
        let census = allele_census(&worlds);
        assert!(!census.no_variance());
        assert_eq!(census.mean_eta_milli, 0);
        assert_eq!(census.marker_value_milli, 200);
        assert_eq!(census.eta_excess_milli(), -200);
        let plan = PlasticityPlan::default();
        let world = world_for(census);
        assert!(!world.selected_over_drift(&plan));
    }

    fn world_for(alleles: AlleleCensus) -> WorldPlasticity {
        WorldPlasticity {
            condition: "A".to_owned(),
            seed: 1,
            population: 100,
            extinct: false,
            shift: Err(ShiftRefusal::NoSchedule),
            alleles,
            plastic_edges_total: 0,
            plasticity_updates_total: 0,
            mean_abs_learned_milli: 0,
        }
    }

    #[test]
    fn eta_above_its_matched_marker_by_more_than_a_mutational_step_counts() {
        let plan = PlasticityPlan::default();
        // eta at 0.30, marker at 0.20: a 100-milli excess, four expected
        // mutational steps.
        let selected: Vec<Genome2> = (0..20)
            .map(|_| {
                genome(vec![
                    edge(4_000, 0.3, 0),
                    marker(4_500, 0.2, 0),
                    edge(5_000, 0.3, 0),
                ])
            })
            .collect();
        let world = world_for(allele_census(&selected));
        assert_eq!(world.alleles.eta_excess_milli(), 100);
        assert!(world.eta_over_drift(&plan));
        assert!(world.selected_over_drift(&plan));

        // ...and an excess of exactly one expected step short of the margin
        // does not. The bar is the bar.
        let marginal: Vec<Genome2> = (0..20)
            .map(|_| {
                genome(vec![
                    edge(4_000, 0.224, 0),
                    marker(4_500, 0.2, 0),
                    edge(5_000, 0.224, 0),
                ])
            })
            .collect();
        let world = world_for(allele_census(&marginal));
        assert_eq!(world.alleles.eta_excess_milli(), 24);
        assert!(!world.eta_over_drift(&plan));
    }

    #[test]
    fn the_plastic_flag_is_compared_against_the_marker_flag_and_not_against_zero() {
        let plan = PlasticityPlan::default();
        // Both flags set on every allele: plasticity is exactly as common as
        // the neutral marker's flag, so the excess is zero and the world does
        // not count - even though the plastic fraction is 100 percent. A bar
        // stated against zero rather than against the control would pass here
        // and would be measuring the mutation rate.
        let drifted: Vec<Genome2> = (0..20)
            .map(|_| {
                genome(vec![
                    edge(4_000, 0.0, EDGE_FLAG_PLASTIC),
                    marker(4_500, 0.0, MARKER_FLAG_NEUTRAL),
                    edge(5_000, 0.0, EDGE_FLAG_PLASTIC),
                ])
            })
            .collect();
        let world = world_for(allele_census(&drifted));
        assert_eq!(world.alleles.plastic_fraction_milli, 1_000);
        assert_eq!(world.alleles.marker_set_fraction_milli, 1_000);
        assert_eq!(world.alleles.plastic_excess_milli(), 0);
        assert!(!world.plastic_over_drift(&plan));
        assert!(!world.selected_over_drift(&plan));
    }

    #[test]
    fn a_genome_with_many_edges_does_not_outvote_one_with_few() {
        // ADR-0022 A5 one level down. One organism carrying forty plastic
        // edges and nineteen carrying none is not a world where plasticity
        // reached the population, and an allele-weighted mean would say it
        // was.
        let mut population: Vec<Genome2> = (0..19)
            .map(|_| genome(vec![edge(4_000, 0.0, 0), marker(4_500, 0.0, 0)]))
            .collect();
        let mut heavy = Vec::new();
        for index in 0..20_u32 {
            heavy.push(edge(4_000 + index, 1.0, EDGE_FLAG_PLASTIC));
        }
        heavy.push(marker(4_500, 0.0, 0));
        population.push(genome(heavy));
        let census = allele_census(&population);
        // Allele-weighted the mean would be 40/(38+40) = 513 milli; per
        // organism it is 1000/20 = 50.
        assert_eq!(census.mean_eta_milli, 50);
        assert_eq!(census.plastic_fraction_milli, 50);
        assert_eq!(census.edge_alleles, 38 + 40);
    }

    /// A population of `count` identical genomes built from `loci`.
    fn population(count: usize, loci: Vec<Locus>) -> Vec<Genome2> {
        (0..count).map(|_| genome(loci.clone())).collect()
    }

    #[test]
    fn each_of_the_four_signals_makes_the_comparison_defined_on_its_own() {
        // `no_variance` is a four-way conjunction and each term guards a
        // different world. Deleting any one of them leaves the other three
        // true in every case the other tests build, so this walks the four
        // one-signal worlds - the only shape that can tell the terms apart.
        //
        // The consequence is not cosmetic. A world wrongly called undefined
        // is not counted for C11.2 however far its excess is above the bar,
        // and `drift_no_variance` is what a reader uses to tell "the control
        // never ran" from "plasticity failed to keep up with drift", which
        // the module doc says are different results.
        let plan = PlasticityPlan::default();
        let quiet = vec![
            edge(4_000, 0.0, 0),
            marker(4_500, 0.0, 0),
            edge(5_000, 0.0, 0),
        ];
        assert!(allele_census(&population(20, quiet.clone())).no_variance());

        // 1. Only `eta` moved. Defined, and selected: this is what C11.2
        //    passing looks like, and a `no_variance` missing its `eta` term
        //    would report it as undefined instead.
        let census = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.05, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.05, 0),
            ],
        ));
        assert!(!census.no_variance(), "moved eta alone read as undefined");
        assert_eq!(census.eta_excess_milli(), 50);
        assert!(world_for(census).selected_over_drift(&plan));

        // 2. Only `EDGE_FLAG_PLASTIC` set.
        let census = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.0, EDGE_FLAG_PLASTIC),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.0, EDGE_FLAG_PLASTIC),
            ],
        ));
        assert!(
            !census.no_variance(),
            "a set plastic flag read as undefined"
        );
        assert_eq!(census.plastic_excess_milli(), 1_000);
        assert!(world_for(census).selected_over_drift(&plan));

        // 3. Only the marker's value moved - the directed failure.
        let census = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.0, 0),
                marker(4_500, 0.05, 0),
                edge(5_000, 0.0, 0),
            ],
        ));
        assert!(!census.no_variance(), "a moved marker read as undefined");
        assert_eq!(census.eta_excess_milli(), -50);
        assert!(!world_for(census).selected_over_drift(&plan));

        // 4. Only the marker's flag set. Also a failure and also defined:
        //    the control moved and plasticity did not follow it.
        let census = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.0, 0),
                marker(4_500, 0.0, MARKER_FLAG_NEUTRAL),
                edge(5_000, 0.0, 0),
            ],
        ));
        assert!(
            !census.no_variance(),
            "a set marker flag read as undefined, so the control's own drift is invisible"
        );
        assert_eq!(census.plastic_excess_milli(), -1_000);
        assert!(!world_for(census).selected_over_drift(&plan));
    }

    #[test]
    fn an_excess_of_exactly_the_margin_counts_on_both_scales() {
        // The bar is `>=`, and 25 milli is not an arbitrary number: it is the
        // expected absolute size of one point mutation's step at the pinned
        // `point_delta_q16`. A world that moved by exactly one step is the
        // smallest world the criterion was written to count, and `>` would
        // silently drop it. No other test evaluates either predicate at the
        // bar itself.
        let plan = PlasticityPlan::default();

        // `eta` at 25 milli against a marker still at the founder value.
        // 0.025 and 0.024 are chosen so the f32 -> milli truncation lands on
        // 25 and 24 exactly rather than near them.
        let at_bar = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.025, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.025, 0),
            ],
        ));
        assert_eq!(at_bar.eta_excess_milli(), 25);
        assert!(world_for(at_bar).eta_over_drift(&plan));
        let below = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.024, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.024, 0),
            ],
        ));
        assert_eq!(below.eta_excess_milli(), 24);
        assert!(!world_for(below).eta_over_drift(&plan));

        // The plastic-flag scale at exactly the same bar. One organism in
        // twenty carrying one flagged edge of two is 500 milli for that
        // organism and 25 for the world.
        let mut flagged = population(
            19,
            vec![
                edge(4_000, 0.0, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.0, 0),
            ],
        );
        flagged.push(genome(vec![
            edge(4_000, 0.0, EDGE_FLAG_PLASTIC),
            marker(4_500, 0.0, 0),
            edge(5_000, 0.0, 0),
        ]));
        let census = allele_census(&flagged);
        assert_eq!(census.plastic_fraction_milli, 25);
        assert_eq!(census.plastic_excess_milli(), 25);
        let world = world_for(census);
        assert!(world.plastic_over_drift(&plan));
        assert!(world.selected_over_drift(&plan));
    }

    #[test]
    fn a_world_with_no_variance_is_undefined_even_at_a_zero_margin() {
        // The `!no_variance()` guard in the two predicates is unreachable at
        // the default 25-milli margin: a world where nothing moved has an
        // excess of exactly zero, which is already below the bar. It becomes
        // load-bearing the moment the margin is zero - and the margin is a
        // plan field the `lifesim plasticity --sesoi` flag sets with no lower
        // bound, so this is a reachable configuration and not a hypothetical.
        //
        // Without the guard a founder-frozen world reports `0 >= 0` and is
        // counted as a world where plasticity beat drift, which is the exact
        // inversion of what it is.
        let plan = PlasticityPlan {
            drift_margin_milli: 0,
            ..PlasticityPlan::default()
        };
        let census = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.0, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.0, 0),
            ],
        ));
        assert!(census.no_variance());
        assert_eq!(census.eta_excess_milli(), 0);
        assert_eq!(census.plastic_excess_milli(), 0);
        let world = world_for(census);
        assert!(!world.eta_over_drift(&plan));
        assert!(!world.plastic_over_drift(&plan));
        assert!(!world.selected_over_drift(&plan));

        // ...and the same plan does count a world that actually moved, so the
        // three refusals above are the guard firing rather than the margin
        // being unreachable.
        let moved = allele_census(&population(
            20,
            vec![
                edge(4_000, 0.001, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.001, 0),
            ],
        ));
        assert!(!moved.no_variance());
        assert!(world_for(moved).selected_over_drift(&plan));
    }

    #[test]
    fn the_marker_is_averaged_per_organism_before_it_is_averaged_over_the_world() {
        // `a_genome_with_many_edges_does_not_outvote_one_with_few` states this
        // for the treatment side only, and the two sides are averaged by
        // separate lines. A control weighted by alleles against a treatment
        // weighted by organisms is a mismatched comparison in exactly the
        // worlds where it matters - the ones where duplication has moved the
        // locus counts apart, which is every world in the campaign: the Avar
        // arm ended at 4.12 edge alleles and 2.21 marker alleles per organism
        // against a founder's 4 and 2.
        let mut population: Vec<Genome2> = (0..19)
            .map(|_| genome(vec![edge(4_000, 0.0, 0), marker(4_500, 0.0, 0)]))
            .collect();
        let mut heavy = vec![edge(4_000, 0.0, 0)];
        for index in 0..20_u32 {
            heavy.push(marker(4_500 + index, 1.0, MARKER_FLAG_NEUTRAL));
        }
        population.push(genome(heavy));
        let census = allele_census(&population);
        // Allele-weighted both quantities would be 40/(38+40) = 512 milli;
        // per organism they are 1000/20 = 50.
        assert_eq!(census.marker_value_milli, 50);
        assert_eq!(census.marker_set_fraction_milli, 50);
        assert_eq!(census.marker_alleles, 38 + 40);
        assert_eq!(census.set_marker_alleles, 40);
    }

    #[test]
    fn the_summary_reads_each_c11_2_column_from_its_own_predicate() {
        // Every C11.2 number a reader sees comes out of `summarise`, and the
        // campaign's headline reading - "eight worlds, all of them on the
        // plastic-flag scale and none on the `eta` scale" - is the two counts
        // being distinguishable. Nothing tested these columns at all, so a
        // report that filled `eta_selected` from the plastic predicate would
        // have printed a clean, wrong sentence.
        let plan = PlasticityPlan::default();
        let quiet = vec![
            edge(4_000, 0.0, 0),
            marker(4_500, 0.0, 0),
            edge(5_000, 0.0, 0),
        ];
        let by_eta = world_for(allele_census(&population(
            20,
            vec![
                edge(4_000, 0.05, 0),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.05, 0),
            ],
        )));
        let by_flag = world_for(allele_census(&population(
            20,
            vec![
                edge(4_000, 0.0, EDGE_FLAG_PLASTIC),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.0, EDGE_FLAG_PLASTIC),
            ],
        )));
        // **Two worlds on the flag scale and one on the `eta` scale, not one
        // of each.** With one of each the two counts are both 1 and a column
        // filled from the other predicate is indistinguishable - which is
        // exactly what the campaign's `eta_selected=0 plastic_selected=8`
        // would have been read against.
        let by_flag_again = world_for(allele_census(&population(
            20,
            vec![
                edge(4_000, 0.0, EDGE_FLAG_PLASTIC),
                marker(4_500, 0.0, 0),
                edge(5_000, 0.0, 0),
            ],
        )));
        let undefined = world_for(allele_census(&population(20, quiet)));

        let outcome = summarise("A", &[by_eta, by_flag, by_flag_again, undefined], &plan);
        assert_eq!(outcome.worlds, 4);
        assert_eq!(outcome.selected, 3);
        assert_eq!(outcome.eta_selected, 1);
        assert_eq!(outcome.plastic_selected, 2);
        assert_eq!(outcome.drift_no_variance, 1);
        // The medians are read off the same four worlds, so a column wired to
        // the wrong census field shows here too. `median_milli` takes the
        // lower of the two middle values, so with two zero worlds these are
        // all zero and the totals below are what separate the columns.
        assert_eq!(outcome.median_eta_milli, 0);
        assert_eq!(outcome.median_eta_excess_milli, 0);
        assert_eq!(outcome.median_plastic_fraction_milli, 0);
        assert_eq!(outcome.median_plastic_excess_milli, 0);
        assert_eq!(outcome.total_moved_eta_alleles, 80);
        assert_eq!(outcome.total_plastic_alleles, 80 + 40);
        assert_eq!(outcome.total_moved_marker_alleles, 0);
        assert_eq!(outcome.total_set_marker_alleles, 0);
    }

    #[test]
    fn the_verdict_needs_the_bar_and_a_control_that_stays_below_the_ceiling() {
        let plan = PlasticityPlan::default();
        let empty = PlasticityOutcome {
            condition: "T".to_owned(),
            worlds: 30,
            extinct: 0,
            shifted: 0,
            associated: 0,
            shift_no_variance: 0,
            shift_refused: 0,
            median_rho_milli: 0,
            median_null_milli: 0,
            median_event_distance_milli: 0,
            median_control_distance_milli: 0,
            median_event_locomotion_tv_milli: 0,
            median_control_locomotion_tv_milli: 0,
            median_individuals: 0,
            median_varying_columns: 0,
            column_totals: [0; ACTION_CLASS_COUNT],
            selected: 0,
            eta_selected: 0,
            plastic_selected: 0,
            drift_no_variance: 0,
            median_eta_milli: 0,
            median_marker_value_milli: 0,
            median_plastic_fraction_milli: 0,
            median_marker_set_fraction_milli: 0,
            median_eta_excess_milli: 0,
            median_plastic_excess_milli: 0,
            total_moved_eta_alleles: 0,
            total_moved_marker_alleles: 0,
            total_plastic_alleles: 0,
            total_set_marker_alleles: 0,
            median_population: 0,
            total_plasticity_updates: 0,
        };
        let mut control = empty.clone();
        control.condition = "C".to_owned();

        // Bar met, control silent: met.
        let met = Verdict::decide(
            "C11.1",
            &empty,
            &control,
            22,
            3,
            0,
            plan.shift_bar,
            plan.control_ceiling,
        );
        assert!(met.met);
        // Bar met and the control shows the same thing: **not** met. The
        // claim is that plasticity produced the change, and a control that
        // reproduces it refutes that however strong the treatment looks.
        let confounded = Verdict::decide(
            "C11.1",
            &empty,
            &control,
            28,
            25,
            0,
            plan.shift_bar,
            plan.control_ceiling,
        );
        assert!(!confounded.met);
        // Bar missed: not met, whatever the control did.
        let missed = Verdict::decide(
            "C11.1",
            &empty,
            &control,
            19,
            0,
            0,
            plan.shift_bar,
            plan.control_ceiling,
        );
        assert!(!missed.met);

        // Both thresholds at their exact boundary, which is the only place
        // the pre-registration's wording is load-bearing: "at least 20 of
        // 30" includes 20, and the control "must stay strictly below" the
        // ceiling, so a control at exactly 20 refutes the claim.
        let at_bar = Verdict::decide(
            "C11.1",
            &empty,
            &control,
            plan.shift_bar,
            0,
            0,
            plan.shift_bar,
            plan.control_ceiling,
        );
        assert!(at_bar.met, "a treatment exactly at the bar was rejected");
        let at_ceiling = Verdict::decide(
            "C11.1",
            &empty,
            &control,
            30,
            plan.control_ceiling,
            0,
            plan.shift_bar,
            plan.control_ceiling,
        );
        assert!(
            !at_ceiling.met,
            "a control exactly at the ceiling was tolerated"
        );
    }
}
