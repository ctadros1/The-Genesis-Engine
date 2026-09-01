//! Seed-paired comparison of two conditions (`lifesim-paired-stats-v1`).
//!
//! Conditions in this project are matched on seed, so a world under A and
//! the same-seeded world under B share terrain, founder draws, and every
//! other input except the treatment. The correct analysis is therefore
//! paired, and ADR-0022 A5 already fixes the unit: the world is the
//! replicate, per-organism quantities having been aggregated to a
//! world-level statistic before anything here runs.
//!
//! C7.1 asks for three things and this module produces exactly those:
//!
//! - a **count of worlds** whose paired difference reaches the prespecified
//!   smallest effect of interest, with an exact binomial p-value, because
//!   the criterion is written as "in at least 20 of 30 worlds";
//! - an **interval** on the mean paired difference;
//! - an **equivalence result**, so that a null is interpretable as evidence
//!   of no effect rather than as absence of evidence.
//!
//! Everything is deterministic. The bootstrap draws from the kernel's own
//! named RNG streams keyed on a recorded analysis seed, so re-running a
//! report reproduces it bit-for-bit, and the interval is a percentile
//! interval over integer milli-unit differences rather than a normal
//! approximation this data has no reason to satisfy.
//!
//! The equivalence test is TOST at alpha, implemented through its exact
//! identity with a `(1 - 2*alpha)` interval: the two one-sided tests both
//! reject when the `(1 - 2*alpha)` interval lies entirely inside the
//! equivalence bounds. At alpha = 0.05 that is the 90% interval.

use sim_core::{RngSystem, named_random};

pub const PAIRED_STATS_VERSION: &str = "lifesim-paired-stats-v1";

/// The direction a prespecified effect is expected to take.
///
/// This exists because the criterion's literal wording -- worlds whose
/// index "differs by at least the SESOI" -- turns out not to discriminate.
/// In the pilot, condition D (perception live, attack impossible, mean
/// effect +1.4 percent on aggregation) still crossed a 10 percent SESOI in
/// 12 of 16 worlds, because per-world seed variation is itself larger than
/// the SESOI. Counting only worlds that cross **in a direction fixed in
/// advance** is a strictly stronger bar than the criterion as written, and
/// it makes the 20-of-30 count an exact one-sided sign test at a null rate
/// of one half.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Decrease,
    Increase,
    /// Magnitude only: the criterion as literally written. Retained so a
    /// report can show both and a reader can see the difference.
    Either,
}

impl Direction {
    fn counts(self, relative_milli: i64, sesoi_milli: i64) -> bool {
        match self {
            Direction::Decrease => relative_milli <= -sesoi_milli,
            Direction::Increase => relative_milli >= sesoi_milli,
            Direction::Either => relative_milli.abs() >= sesoi_milli,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::Decrease => "decrease",
            Direction::Increase => "increase",
            Direction::Either => "either",
        }
    }
}

/// Bootstrap resamples. Fixed rather than configurable so two reports of
/// the same data are always comparable.
pub const BOOTSTRAP_RESAMPLES: u32 = 20_000;

/// One seed's matched pair of world-level statistics, in milli-units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pair {
    pub seed: u64,
    pub treatment_milli: i64,
    pub control_milli: i64,
}

impl Pair {
    pub fn difference_milli(&self) -> i64 {
        self.treatment_milli - self.control_milli
    }

    /// Relative difference against the control, in milli-units. This is the
    /// scale the smallest effect of interest is expressed on, because a
    /// fixed absolute step means different things to an index at 1.1 and an
    /// index at 40.
    pub fn relative_milli(&self) -> Option<i64> {
        if self.control_milli == 0 {
            return None;
        }
        Some(
            ((i128::from(self.treatment_milli - self.control_milli) * 1_000)
                / i128::from(self.control_milli).abs()) as i64,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedResult {
    pub pairs: usize,
    /// Worlds whose relative difference reaches the SESOI in magnitude.
    /// The criterion as literally written; reported but not decisive.
    pub reaching_sesoi: usize,
    /// Worlds reaching the SESOI **in the prespecified direction**. This is
    /// the count the decision rule uses.
    pub reaching_sesoi_directed: usize,
    /// Worlds whose ABSOLUTE paired difference reaches the SESOI in the
    /// prespecified direction. Defined for every pair, including those
    /// whose control is exactly zero (where the relative form is not);
    /// the decision rule for a normalized-fraction quantity uses this
    /// count, per its own pre-registration (D-120's blindness lesson).
    pub reaching_absolute_directed: usize,
    pub direction: Direction,
    pub positive_differences: usize,
    /// Exact one-sided binomial p-value for `reaching_sesoi_directed`
    /// successes out of `pairs` under a null probability of
    /// `null_rate_milli / 1000`.
    pub sesoi_p_value_milli: i64,
    pub mean_difference_milli: i64,
    pub median_difference_milli: i64,
    /// Percentile bootstrap interval on the mean paired difference.
    pub ci_low_milli: i64,
    pub ci_high_milli: i64,
    /// Confidence level of the reported interval, in milli-units.
    pub ci_level_milli: u32,
    /// Mean relative difference and its interval, milli-units.
    pub mean_relative_milli: i64,
    pub relative_ci_low_milli: i64,
    pub relative_ci_high_milli: i64,
    /// TOST at the analysis alpha against `+/- sesoi_milli` on the relative
    /// scale: true when the whole interval sits inside the bounds.
    pub equivalent: bool,
    pub sesoi_milli: i64,
}

/// Median of a slice of milli-unit values. Even counts take the lower of
/// the two central values rather than their mean, keeping every reported
/// quantity an exact integer.
pub fn median_milli(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[(sorted.len() - 1) / 2]
}

fn mean_milli(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let total: i128 = values.iter().map(|value| i128::from(*value)).sum();
    (total / values.len() as i128) as i64
}

/// Exact upper-tail binomial probability `P(X >= successes)` for `trials`
/// Bernoulli draws at rate `rate_milli / 1000`, returned in milli-units.
///
/// Computed with `i128` rationals scaled by a power of ten rather than in
/// floating point, so the p-value is exact and reproducible.
pub fn binomial_upper_tail_milli(successes: usize, trials: usize, rate_milli: u32) -> i64 {
    if successes == 0 {
        return 1_000;
    }
    if successes > trials {
        return 0;
    }
    // Work in Q60 fixed point: probabilities are in [0, 1], and 2^60 gives
    // ample headroom against the binomial coefficients at n <= 200.
    const ONE: i128 = 1_i128 << 60;
    let p = (i128::from(rate_milli) * ONE) / 1_000;
    let q = ONE - p;

    // P(X = k) computed iteratively: term_{k+1} = term_k * (n-k)/(k+1) * p/q.
    let mut term = ONE;
    for _ in 0..trials {
        term = (term * q) >> 60;
    }
    let mut total = 0_i128;
    for k in 0..=trials {
        if k >= successes {
            total += term;
        }
        if k < trials {
            if q == 0 {
                term = 0;
            } else {
                term = term * (trials - k) as i128 / (k + 1) as i128;
                term = (term * p) / q;
            }
        }
    }
    ((total * 1_000) >> 60) as i64
}

/// Percentile bootstrap interval on the mean of `values`.
///
/// `analysis_seed` is recorded in the report: the interval is a function of
/// the data and that seed and of nothing else.
fn bootstrap_mean_interval(
    values: &[i64],
    level_milli: u32,
    analysis_seed: u64,
    stream_subject: u64,
) -> (i64, i64) {
    if values.is_empty() {
        return (0, 0);
    }
    if values.len() == 1 {
        return (values[0], values[0]);
    }
    let mut means = Vec::with_capacity(BOOTSTRAP_RESAMPLES as usize);
    for resample in 0..BOOTSTRAP_RESAMPLES {
        let mut total = 0_i128;
        for draw in 0..values.len() {
            let random = named_random(
                analysis_seed,
                u64::from(resample),
                RngSystem::Analysis,
                stream_subject,
                draw as u32,
            );
            let pick = (random % values.len() as u64) as usize;
            total += i128::from(values[pick]);
        }
        means.push((total / values.len() as i128) as i64);
    }
    means.sort_unstable();
    let tail_milli = (1_000 - level_milli) / 2;
    let low_index = ((u64::from(tail_milli) * BOOTSTRAP_RESAMPLES as u64) / 1_000) as usize;
    let high_index = (BOOTSTRAP_RESAMPLES as usize - 1)
        .min((((1_000 - u64::from(tail_milli)) * BOOTSTRAP_RESAMPLES as u64) / 1_000) as usize);
    (means[low_index], means[high_index])
}

/// Compare a treatment condition against its seed-matched control.
///
/// `sesoi_milli` is the smallest effect of interest on the **relative**
/// scale (100 = a 10 percent change). `null_rate_milli` is the per-world
/// probability of reaching it under the null, used only for the binomial
/// p-value; it must be prespecified alongside the SESOI.
pub fn compare(
    pairs: &[Pair],
    sesoi_milli: i64,
    null_rate_milli: u32,
    direction: Direction,
    analysis_seed: u64,
) -> PairedResult {
    let differences: Vec<i64> = pairs.iter().map(Pair::difference_milli).collect();
    let relatives: Vec<i64> = pairs.iter().filter_map(Pair::relative_milli).collect();

    let reaching = pairs
        .iter()
        .filter(|pair| {
            pair.relative_milli()
                .is_some_and(|relative| relative.abs() >= sesoi_milli)
        })
        .count();
    let reaching_directed = pairs
        .iter()
        .filter(|pair| {
            pair.relative_milli()
                .is_some_and(|relative| direction.counts(relative, sesoi_milli))
        })
        .count();
    // The absolute-scale counterpart: for quantities that are already
    // normalized fractions (Phase 13's naive arrival fraction), the
    // relative form is undefined whenever the control is exactly zero -
    // which for an expected-null control arm is most pairs, and a SESOI
    // count that silently excludes them is an outcome measure blind to
    // its own factor (D-120). The absolute count is defined for every
    // pair; which form a criterion uses is fixed in its pre-registration.
    let reaching_absolute_directed = differences
        .iter()
        .filter(|&&difference| direction.counts(difference, sesoi_milli))
        .count();
    let positive_differences = differences.iter().filter(|value| **value > 0).count();

    // Alpha is 0.05, so the equivalence interval is the 90% one and the
    // estimation interval is the 95% one. Both are reported; only the 90%
    // one decides equivalence, which is what makes TOST exact.
    let (ci_low, ci_high) = bootstrap_mean_interval(&differences, 950, analysis_seed, 1);
    let (relative_low, relative_high) = bootstrap_mean_interval(&relatives, 900, analysis_seed, 2);

    PairedResult {
        pairs: pairs.len(),
        reaching_sesoi: reaching,
        reaching_sesoi_directed: reaching_directed,
        reaching_absolute_directed,
        direction,
        positive_differences,
        sesoi_p_value_milli: binomial_upper_tail_milli(
            reaching_directed,
            pairs.len(),
            null_rate_milli,
        ),
        mean_difference_milli: mean_milli(&differences),
        median_difference_milli: median_milli(&differences),
        ci_low_milli: ci_low,
        ci_high_milli: ci_high,
        ci_level_milli: 950,
        mean_relative_milli: mean_milli(&relatives),
        relative_ci_low_milli: relative_low,
        relative_ci_high_milli: relative_high,
        equivalent: !relatives.is_empty()
            && relative_low > -sesoi_milli
            && relative_high < sesoi_milli,
        sesoi_milli,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs_from(values: &[(i64, i64)]) -> Vec<Pair> {
        values
            .iter()
            .enumerate()
            .map(|(index, (treatment, control))| Pair {
                seed: index as u64 + 1,
                treatment_milli: *treatment,
                control_milli: *control,
            })
            .collect()
    }

    #[test]
    fn the_binomial_tail_matches_values_computable_by_hand() {
        // P(X >= 0) = 1 always.
        assert_eq!(binomial_upper_tail_milli(0, 30, 100), 1_000);
        // P(X >= n) = p^n. At p = 0.5, n = 4 that is 1/16 = 0.0625.
        let all_four = binomial_upper_tail_milli(4, 4, 500);
        assert!((all_four - 62).abs() <= 1, "got {all_four}");
        // P(X >= 1) = 1 - (1-p)^n. At p = 0.1, n = 10 that is 0.6513.
        let at_least_one = binomial_upper_tail_milli(1, 10, 100);
        assert!((at_least_one - 651).abs() <= 1, "got {at_least_one}");
        // The criterion's own shape: 20 of 30 at a 10% null rate is
        // vanishingly unlikely.
        assert_eq!(binomial_upper_tail_milli(20, 30, 100), 0);
        // ...and impossible successes are impossible.
        assert_eq!(binomial_upper_tail_milli(31, 30, 100), 0);
    }

    #[test]
    fn a_real_shift_is_detected_and_is_not_declared_equivalent() {
        // Every world moves by +30%, far beyond a 10% SESOI.
        let pairs = pairs_from(&[(1_300, 1_000); 30]);
        let result = compare(&pairs, 100, 500, Direction::Increase, 42);
        assert_eq!(result.reaching_sesoi, 30);
        assert_eq!(result.reaching_sesoi_directed, 30);
        assert_eq!(result.mean_relative_milli, 300);
        assert_eq!(result.sesoi_p_value_milli, 0);
        assert!(!result.equivalent);
        assert!(result.ci_low_milli > 0);
    }

    #[test]
    fn a_true_null_is_declared_equivalent_rather_than_merely_unproven() {
        // The property that makes a null interpretable: identical
        // conditions must come back *equivalent*, not just non-significant.
        let pairs = pairs_from(&[(1_000, 1_000); 30]);
        let result = compare(&pairs, 100, 500, Direction::Increase, 42);
        assert_eq!(result.reaching_sesoi, 0);
        assert_eq!(result.mean_relative_milli, 0);
        assert!(result.equivalent);
        assert_eq!((result.ci_low_milli, result.ci_high_milli), (0, 0));
    }

    #[test]
    fn noise_smaller_than_the_sesoi_is_equivalent_and_noise_larger_is_not() {
        // Alternating +-2% around zero: real variation, all of it below a
        // 10% SESOI, so the correct answer is equivalence.
        let small: Vec<(i64, i64)> = (0..30)
            .map(|k| {
                if k % 2 == 0 {
                    (1_020, 1_000)
                } else {
                    (980, 1_000)
                }
            })
            .collect();
        let result = compare(&pairs_from(&small), 100, 500, Direction::Increase, 7);
        assert_eq!(result.reaching_sesoi, 0);
        assert!(result.equivalent, "{result:?}");

        // Alternating +-40%: every world reaches the SESOI, but they
        // disagree on direction, so the direction-matched count is half.
        let large: Vec<(i64, i64)> = (0..30)
            .map(|k| {
                if k % 2 == 0 {
                    (1_400, 1_000)
                } else {
                    (600, 1_000)
                }
            })
            .collect();
        let result = compare(&pairs_from(&large), 100, 500, Direction::Increase, 7);
        assert_eq!(result.reaching_sesoi, 30);
        // Half move up and half move down, so a rule that fixes the
        // direction in advance counts only half of them -- which is the
        // whole point of fixing it.
        assert_eq!(result.reaching_sesoi_directed, 15);
        assert!(!result.equivalent);
    }

    #[test]
    fn the_bootstrap_interval_is_reproducible_and_widens_with_spread() {
        let tight = pairs_from(&[(1_010, 1_000); 20]);
        let spread: Vec<(i64, i64)> = (0..20)
            .map(|k| (1_000 + (k as i64 - 10) * 60, 1_000))
            .collect();
        let a = compare(&tight, 100, 500, Direction::Increase, 99);
        let b = compare(&tight, 100, 500, Direction::Increase, 99);
        assert_eq!(a, b, "the same seed must reproduce the same interval");
        let wide = compare(&pairs_from(&spread), 100, 500, Direction::Increase, 99);
        assert!(
            (wide.ci_high_milli - wide.ci_low_milli) > (a.ci_high_milli - a.ci_low_milli),
            "a spread sample must produce a wider interval"
        );
    }

    #[test]
    fn a_zero_control_is_dropped_from_the_relative_scale_rather_than_dividing_by_zero() {
        let pairs = pairs_from(&[(500, 0), (1_200, 1_000)]);
        let result = compare(&pairs, 100, 500, Direction::Increase, 1);
        // Only the second pair has a defined relative difference.
        assert_eq!(result.mean_relative_milli, 200);
        assert_eq!(result.pairs, 2);
        assert_eq!(result.reaching_sesoi, 1);
    }

    /// The absolute directed count is defined at zero controls (where the
    /// relative form is None) and inclusive at exactly the SESOI - the
    /// two properties the Phase 13 primary depends on.
    #[test]
    fn the_absolute_directed_count_includes_zero_control_pairs_inclusively() {
        let pairs = [
            Pair {
                seed: 1,
                treatment_milli: 100,
                control_milli: 0,
            },
            Pair {
                seed: 2,
                treatment_milli: 99,
                control_milli: 0,
            },
            Pair {
                seed: 3,
                treatment_milli: 0,
                control_milli: 100,
            },
        ];
        let result = compare(&pairs, 100, 500, Direction::Increase, 7);
        assert_eq!(result.reaching_absolute_directed, 1);
        assert_eq!(
            result.reaching_sesoi_directed, 0,
            "the relative form is blind at zero controls; the absolute one is not"
        );
    }
}
