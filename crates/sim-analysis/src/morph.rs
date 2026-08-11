//! Phase 10 morphological evolution: C10.3 and C10.6.
//!
//! C10.3 is **conjunctive over three clauses** and that structure is the
//! whole point of the criterion. ADR-0022 A13 - "novelty is not progress" -
//! is what promoted it: a diverging module count on its own is variance, and
//! variance is not evidence that morphology matters. So divergence,
//! consequence, and persistence must all hold in the same world.
//!
//! ## The consequence clause, and why it needs a permutation
//!
//! "Do organisms with different bodies do measurably better or worse?" looks
//! like a correlation between module count and offspring count, and it is -
//! but the naive correlation is confounded by **age**. An older organism has
//! had more chances to reproduce, and if body size correlates with age for
//! any reason at all (it does: bodies grow at birth from inherited programs,
//! and lineages that arrived later carry different programs), the
//! correlation reports the age structure rather than any fitness effect.
//!
//! The null used here is a **within-world permutation**: shuffle which body
//! goes with which reproductive record and recompute. That destroys the
//! morphology-outcome link while preserving *both* marginal distributions
//! and every age effect in them, which is exactly the confound an unpaired
//! comparison cannot remove. The observed correlation has to beat the 95th
//! percentile of 199 such shuffles.
//!
//! ## What a world with no morphological variance can and cannot say
//!
//! A world whose bodies never diverged has nothing to say about whether
//! morphology has consequence - the correlation is undefined, not zero.
//! Those worlds are reported as `no_variance` and are **not** counted as
//! consequence failures, because counting them would let a low effective
//! mutation rate masquerade as a refutation of the mechanism. That is D-079's
//! lesson applied before the fact rather than after it.

use crate::paired::{Direction, Pair, PairedResult, compare, median_milli};
use sim_core::{MorphologySample, named_random};
use std::fmt::Write as _;

pub const MORPH_ANALYSIS_VERSION: &str = "lifesim-morph-analysis-v1";

/// Permutations for the consequence null. Fixed rather than configurable so
/// two reports of the same data are comparable; 199 makes the 95th
/// percentile an exact order statistic (the 190th of 199).
pub const PERMUTATIONS: usize = 199;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphPlan {
    /// Founder body size, in modules.
    ///
    /// Three: gut, motor, sensor. It was 1 while the founder was a lone
    /// digestive module, and leaving it at 1 after the founder grew made the
    /// divergence clause trivially true for every world including the
    /// control. A criterion phrased as "differs from the founder" is only as
    /// good as its idea of what the founder is.
    pub founder_modules: u64,
    pub divergence_bar: usize,
    pub consequence_bar: usize,
    pub persistence_bar: usize,
    /// C10.6's equivalence bound on the relative scale, milli.
    pub stability_tolerance_milli: i64,
    pub stability_bar: usize,
    pub analysis_seed: u64,
}

impl Default for MorphPlan {
    fn default() -> Self {
        Self {
            founder_modules: 3,
            divergence_bar: 20,
            consequence_bar: 20,
            persistence_bar: 20,
            stability_tolerance_milli: 250,
            stability_bar: 20,
            analysis_seed: 0x9e3779b97f4a7c15,
        }
    }
}

/// One world's morphological outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldMorph {
    pub seed: u64,
    pub population: u64,
    pub extinct: bool,
    pub median_modules: u64,
    pub mean_modules_milli: u64,
    pub distinct_morphologies: u64,
    pub median_lifespan_ticks: u64,
    /// Mature organisms whose bodies differ from one another. Below three
    /// there is no correlation to compute.
    pub compared: usize,
    /// Spearman correlation between module count and offspring, milli.
    pub rho_milli: i64,
    /// The permutation null's 95th percentile, milli.
    pub null_p95_milli: i64,
    /// True when the world had no morphological variance among mature
    /// organisms, so consequence is undefined rather than absent.
    pub no_variance: bool,
    /// Divergence present at the halfway sample of the `.almo` series.
    pub diverged_at_halfway: bool,
    pub diverged_at_end: bool,
    pub series_samples: usize,
}

impl WorldMorph {
    pub fn divergence(&self, plan: &MorphPlan) -> bool {
        self.distinct_morphologies > 1 && self.median_modules > plan.founder_modules
    }

    /// Consequence, and `false` when undefined. Callers separate
    /// `no_variance` worlds out before counting.
    pub fn consequence(&self) -> bool {
        !self.no_variance && self.compared >= 3 && self.rho_milli.abs() > self.null_p95_milli
    }

    pub fn persistence(&self) -> bool {
        self.diverged_at_halfway && self.diverged_at_end
    }
}

/// Spearman correlation between module count and offspring, in milli.
pub fn rho_of(samples: &[(i64, i64)]) -> i64 {
    crate::demography::spearman_milli(samples)
}

/// The permutation null: the 95th percentile of |rho| over shuffles that
/// break the pairing and keep both marginals.
pub fn permutation_p95_milli(samples: &[(i64, i64)], seed: u64) -> i64 {
    if samples.len() < 3 {
        return 0;
    }
    let bodies: Vec<i64> = samples.iter().map(|(body, _)| *body).collect();
    let outcomes: Vec<i64> = samples.iter().map(|(_, outcome)| *outcome).collect();
    let mut nulls = Vec::with_capacity(PERMUTATIONS);
    for round in 0..PERMUTATIONS {
        // Fisher-Yates driven by the named analysis stream, so a report is
        // reproducible from its seed alone.
        let mut shuffled = bodies.clone();
        for index in (1..shuffled.len()).rev() {
            let draw = named_random(
                seed,
                round as u64,
                sim_core::RngSystem::Analysis,
                index as u64,
                0,
            );
            let target = (draw % (index as u64 + 1)) as usize;
            shuffled.swap(index, target);
        }
        let paired: Vec<(i64, i64)> = shuffled
            .iter()
            .copied()
            .zip(outcomes.iter().copied())
            .collect();
        nulls.push(rho_of(&paired).abs());
    }
    nulls.sort_unstable();
    // 199 permutations: the 190th order statistic is the exact 95th
    // percentile, with no interpolation to argue about.
    nulls[(PERMUTATIONS * 95) / 100]
}

/// The same null, with the shuffle **restricted to strata**.
///
/// Each sample is `(label, outcome, stratum)` and a round permutes labels
/// only among samples sharing a stratum. Everything else - the Fisher-Yates,
/// the named analysis stream, `PERMUTATIONS`, the exact 190th order
/// statistic - is identical to `permutation_p95_milli`, because the only
/// thing that should differ between the two is what the null is allowed to
/// destroy.
///
/// **Why a restricted randomisation.** A permutation null must preserve
/// everything about the data except the association under test. When the
/// label is confounded with a nuisance variable, the unrestricted shuffle
/// destroys the confound along with the association, so the observed value
/// keeps a bias the null it is compared against has thrown away. Stratifying
/// puts the nuisance structure into the null at the strength it has in the
/// data, where it cancels.
///
/// D-100 is the case that forced it. C11.1's control boundary sits later in
/// an organism's life than its event boundary, so the label was confounded
/// with age; on a world where nothing happens at all the unrestricted null
/// let an age artifact score rho +158 against a p95 of 30 and pass.
///
/// **This function on its own does not remove that artifact, and saying so is
/// the point.** Measured on D-100's own rolling cohort: stratifying the null
/// while leaving every observation in place moves the null from 30 to 162
/// against an observed value still at 158 - refused, but by four milli, and
/// at the 240-pair size the two are equal at 163 and the refusal has no
/// margin at all. What actually collapses the observed value to zero is the
/// caller's exclusion of single-label strata, described next. Any claim that
/// "only the reference distribution moved" is wrong about this pair of
/// changes: the observed statistic moved too, from 158 to 0, because it is
/// computed over fewer observations.
///
/// **A stratum holding one label is silently degenerate** - it contributes to
/// the observed statistic and cannot be shuffled, which reimports exactly the
/// bias being removed. This function does not filter: the caller must drop
/// single-label strata from the observed statistic and from these samples
/// alike, and report how many observations that cost. Filtering here would
/// hide the exclusion from the report.
///
/// Determinism: strata are walked in ascending key order through a
/// `BTreeMap`, so no unordered iteration reaches the draw, and the draw index
/// is a running counter over that walk (Rule 5). The result is a function of
/// the sample **sequence** and the seed, not of the sample *set*: reversing,
/// rotating or re-sorting `samples` gives a different p95 - measured, 26 of
/// 40 seeds moved on a 400-observation set, by one to two milli - because
/// Fisher-Yates over a permuted member list consumes the same draws in a
/// different order. `permutation_p95_milli` has the same property. It is not
/// reachable as nondeterminism, because `plasticity::world_shift` builds its
/// observations in a canonical order (`BTreeMap` by sample tick, then by
/// entity id, event before control) that no input ordering can disturb -
/// pinned by
/// `the_analysis_keys_on_the_entity_id_and_not_on_the_row_it_arrived_in`.
/// A caller that assembles samples in any other order owes itself that test.
pub fn permutation_p95_milli_stratified(samples: &[(i64, i64, u64)], seed: u64) -> i64 {
    if samples.len() < 3 {
        return 0;
    }
    let mut strata: std::collections::BTreeMap<u64, Vec<usize>> = std::collections::BTreeMap::new();
    for (index, (_, _, stratum)) in samples.iter().enumerate() {
        strata.entry(*stratum).or_default().push(index);
    }
    let labels: Vec<i64> = samples.iter().map(|(label, _, _)| *label).collect();
    let outcomes: Vec<i64> = samples.iter().map(|(_, outcome, _)| *outcome).collect();
    let mut nulls = Vec::with_capacity(PERMUTATIONS);
    for round in 0..PERMUTATIONS {
        let mut shuffled = labels.clone();
        let mut step = 0_u64;
        for members in strata.values() {
            for position in (1..members.len()).rev() {
                let draw = named_random(seed, round as u64, sim_core::RngSystem::Analysis, step, 0);
                step += 1;
                let target = (draw % (position as u64 + 1)) as usize;
                shuffled.swap(members[position], members[target]);
            }
        }
        let paired: Vec<(i64, i64)> = shuffled
            .iter()
            .copied()
            .zip(outcomes.iter().copied())
            .collect();
        nulls.push(rho_of(&paired).abs());
    }
    nulls.sort_unstable();
    nulls[(PERMUTATIONS * 95) / 100]
}

/// Reduce one world's census to its consequence statistics.
pub fn consequence_of(census: &[MorphologySample], seed: u64) -> (usize, i64, i64, bool) {
    let samples: Vec<(i64, i64)> = census
        .iter()
        .filter(|sample| sample.mature)
        .map(|sample| (i64::from(sample.modules), i64::from(sample.child_count)))
        .collect();
    let distinct_bodies = {
        let mut seen: Vec<i64> = samples.iter().map(|(body, _)| *body).collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    };
    // No variance in body size means the correlation is undefined, not zero.
    // Reporting it as zero would read as "morphology does not matter" when
    // the world simply never produced two different bodies to compare.
    if samples.len() < 3 || distinct_bodies < 2 {
        return (samples.len(), 0, 0, true);
    }
    let rho = rho_of(&samples);
    let null = permutation_p95_milli(&samples, seed);
    (samples.len(), rho, null, false)
}

/// One sample line of a `.almo` morphology series.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MorphSample {
    pub tick: u64,
    pub population: u64,
    pub mean_modules_milli: u64,
    pub median_modules: u64,
    pub distinct: u64,
}

/// Parse a `.almo` series. Unknown lines are ignored; a malformed sample is
/// skipped rather than defaulted, so a truncated file reports fewer samples
/// instead of inventing them.
pub fn parse_series(text: &str) -> Vec<MorphSample> {
    let mut samples = Vec::new();
    for line in text.lines() {
        if !line.starts_with("sample ") {
            continue;
        }
        let fields: std::collections::BTreeMap<&str, &str> = line
            .split_whitespace()
            .filter_map(|part| part.split_once('='))
            .collect();
        let get = |key: &str| fields.get(key).and_then(|value| value.parse::<u64>().ok());
        if let (Some(tick), Some(population), Some(mean), Some(median), Some(distinct)) = (
            get("tick"),
            get("population"),
            get("mean_modules_milli"),
            get("median_modules"),
            get("distinct"),
        ) {
            samples.push(MorphSample {
                tick,
                population,
                mean_modules_milli: mean,
                median_modules: median,
                distinct,
            });
        }
    }
    samples
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphOutcome {
    pub condition: String,
    pub worlds: usize,
    pub extinct: usize,
    pub diverged: usize,
    pub consequential: usize,
    pub persistent: usize,
    pub all_three: usize,
    pub no_variance: usize,
    pub median_modules: i64,
    pub median_distinct: i64,
    pub median_population: i64,
}

pub fn summarise(condition: &str, worlds: &[WorldMorph], plan: &MorphPlan) -> MorphOutcome {
    let pick = |values: Vec<i64>| median_milli(&values);
    MorphOutcome {
        condition: condition.to_owned(),
        worlds: worlds.len(),
        extinct: worlds.iter().filter(|world| world.extinct).count(),
        diverged: worlds.iter().filter(|w| w.divergence(plan)).count(),
        consequential: worlds.iter().filter(|w| w.consequence()).count(),
        persistent: worlds.iter().filter(|w| w.persistence()).count(),
        all_three: worlds
            .iter()
            .filter(|w| w.divergence(plan) && w.consequence() && w.persistence())
            .count(),
        no_variance: worlds.iter().filter(|w| w.no_variance).count(),
        median_modules: pick(worlds.iter().map(|w| w.median_modules as i64).collect()),
        median_distinct: pick(
            worlds
                .iter()
                .map(|w| w.distinct_morphologies as i64)
                .collect(),
        ),
        median_population: pick(worlds.iter().map(|w| w.population as i64).collect()),
    }
}

pub fn pairs_of(
    treatment: &[WorldMorph],
    control: &[WorldMorph],
    quantity: impl Fn(&WorldMorph) -> i64,
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

pub fn stability(pairs: &[Pair], plan: &MorphPlan) -> (usize, PairedResult) {
    let within = pairs
        .iter()
        .filter(|pair| match pair.relative_milli() {
            Some(relative) => relative >= -plan.stability_tolerance_milli,
            None => false,
        })
        .count();
    (
        within,
        compare(
            pairs,
            plan.stability_tolerance_milli,
            500,
            Direction::Either,
            plan.analysis_seed,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    campaign_id: &str,
    plan: &MorphPlan,
    per_world: &[(String, Vec<WorldMorph>)],
    outcomes: &[MorphOutcome],
    stabilities: &[(String, String, usize, PairedResult)],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "morph-report 1 campaign {campaign_id}");
    let _ = writeln!(out, "analysis-version {MORPH_ANALYSIS_VERSION}");
    let _ = writeln!(
        out,
        "plan founder_modules={} divergence_bar={} consequence_bar={} persistence_bar={} \
         stability_tolerance_milli={} stability_bar={} permutations={} analysis_seed={:#018x}",
        plan.founder_modules,
        plan.divergence_bar,
        plan.consequence_bar,
        plan.persistence_bar,
        plan.stability_tolerance_milli,
        plan.stability_bar,
        PERMUTATIONS,
        plan.analysis_seed,
    );
    for (condition, worlds) in per_world {
        for world in worlds {
            let _ = writeln!(
                out,
                "world condition={condition} seed={:#018x} population={} extinct={} \
                 median_modules={} mean_modules_milli={} distinct={} median_lifespan={} \
                 compared={} rho_milli={} null_p95_milli={} no_variance={} \
                 diverged_halfway={} diverged_end={} series_samples={} \
                 divergence={} consequence={} persistence={}",
                world.seed,
                world.population,
                world.extinct,
                world.median_modules,
                world.mean_modules_milli,
                world.distinct_morphologies,
                world.median_lifespan_ticks,
                world.compared,
                world.rho_milli,
                world.null_p95_milli,
                world.no_variance,
                world.diverged_at_halfway,
                world.diverged_at_end,
                world.series_samples,
                world.divergence(plan),
                world.consequence(),
                world.persistence(),
            );
        }
    }
    for outcome in outcomes {
        let _ = writeln!(
            out,
            "condition {} worlds={} extinct={} diverged={} consequential={} persistent={} \
             all_three={} no_variance={} med_modules={} med_distinct={} med_population={}",
            outcome.condition,
            outcome.worlds,
            outcome.extinct,
            outcome.diverged,
            outcome.consequential,
            outcome.persistent,
            outcome.all_three,
            outcome.no_variance,
            outcome.median_modules,
            outcome.median_distinct,
            outcome.median_population,
        );
    }
    for (treatment, quantity, within, paired) in stabilities {
        let _ = writeln!(
            out,
            "stability treatment={treatment} quantity={quantity} pairs={} within_or_better={within} \
             mean_relative_milli={} relative_ci_low_milli={} relative_ci_high_milli={} \
             equivalent={}",
            paired.pairs,
            paired.mean_relative_milli,
            paired.relative_ci_low_milli,
            paired.relative_ci_high_milli,
            paired.equivalent,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(modules: u32, children: u32) -> MorphologySample {
        MorphologySample {
            modules,
            child_count: children,
            age_ticks: 5_000,
            mature: true,
        }
    }

    #[test]
    fn a_world_with_one_body_size_reports_undefined_rather_than_zero() {
        // The distinction the criterion turns on: "no morphological variance"
        // and "morphology does not matter" are opposite conclusions, and
        // reporting the first as the second would let a low effective
        // mutation rate masquerade as a refutation of the mechanism.
        let flat: Vec<MorphologySample> = (0..40).map(|i| sample(1, i % 5)).collect();
        let (compared, rho, null, no_variance) = consequence_of(&flat, 7);
        assert!(no_variance, "a single body size must be undefined");
        assert_eq!((rho, null), (0, 0));
        assert_eq!(compared, 40);
    }

    #[test]
    fn a_planted_relationship_beats_its_own_permutation_null() {
        // Offspring count made a strict function of module count: the
        // observed correlation must clear the shuffled distribution.
        let planted: Vec<MorphologySample> = (0..60)
            .map(|i| {
                let modules = 1 + (i % 6);
                sample(modules, modules * 3)
            })
            .collect();
        let (compared, rho, null, no_variance) = consequence_of(&planted, 11);
        assert!(!no_variance);
        assert!(compared >= 3);
        assert!(rho > 900, "a strict relationship should be near +1: {rho}");
        assert!(
            rho.abs() > null,
            "planted relationship {rho} did not beat its null {null}"
        );
    }

    #[test]
    fn an_unrelated_pairing_does_not_beat_its_null() {
        // The control for the test above. Offspring counts that have nothing
        // to do with body size must fail the same bar, or the bar is not
        // discriminating and every world would pass C10.3b.
        let unrelated: Vec<MorphologySample> = (0..60)
            .map(|i| sample(1 + (i % 6), (i * 7 + 3) % 5))
            .collect();
        let (_, rho, null, no_variance) = consequence_of(&unrelated, 13);
        assert!(!no_variance);
        assert!(
            rho.abs() <= null,
            "an unrelated pairing beat its null: rho {rho} vs {null}"
        );
    }

    #[test]
    fn juveniles_are_excluded_because_they_have_had_no_chance_to_breed() {
        let mut census: Vec<MorphologySample> = (0..30).map(|i| sample(1 + (i % 4), 2)).collect();
        for entry in census.iter_mut().take(20) {
            entry.mature = false;
        }
        let (compared, _, _, _) = consequence_of(&census, 17);
        assert_eq!(compared, 10, "immature organisms were counted");
    }

    #[test]
    fn a_truncated_series_reports_fewer_samples_rather_than_inventing_them() {
        let text = "morphology-series 1 policy lifesim-morphology-v1\n\
                    sample tick=3000 population=10 mean_modules_milli=1200 median_modules=1 distinct=2\n\
                    sample tick=6000 population=11 mean_modules_milli=\n";
        let samples = parse_series(text);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].tick, 3_000);
        assert_eq!(samples[0].distinct, 2);
    }
}
