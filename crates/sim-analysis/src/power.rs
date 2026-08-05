//! Simulation-based power for the C7.1 decision rule
//! (`lifesim-power-v1`).
//!
//! ADR-0022 A5 requires the seed count to come from a pilot-driven power
//! analysis with 30 as the floor. The decision rule here is not a t-test,
//! so there is no closed form for its power: C7.1 passes when at least `k`
//! of `n` worlds show a paired relative difference reaching the SESOI. That
//! is a binomial rule whose per-world success probability is unknown and
//! has to be estimated from pilot data.
//!
//! The method is a nonparametric bootstrap over the pilot's own paired
//! differences. Resampling the pilot pairs with replacement makes no
//! distributional assumption at all -- which matters, because a ratio
//! index pooled over samples has no reason to be normal, and assuming it
//! was would understate the seeds needed.
//!
//! What this estimates is the power of the rule *against the effect the
//! pilot saw*. If the pilot's effect is itself noise, the estimate is
//! noise; a pilot is a pilot. It is reported with the pilot's size so a
//! reader can weigh it.

use crate::paired::{Direction, Pair};
use sim_core::{RngSystem, named_random};

pub const POWER_VERSION: &str = "lifesim-power-v1";

/// Resamples per candidate seed count.
pub const POWER_TRIALS: u32 = 4_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PowerPoint {
    pub worlds: usize,
    /// Worlds that must reach the SESOI at this size, scaled from the
    /// criterion's 20-of-30 ratio.
    pub required: usize,
    /// Estimated probability the rule passes, milli-units.
    pub power_milli: u32,
}

/// Per-world probability of reaching the SESOI, estimated from the pilot,
/// in milli-units.
pub fn observed_success_rate_milli(pairs: &[Pair], sesoi_milli: i64, direction: Direction) -> u32 {
    if pairs.is_empty() {
        return 0;
    }
    let hits = success_flags(pairs, sesoi_milli, direction)
        .into_iter()
        .filter(|hit| *hit)
        .count();
    ((hits as u64 * 1_000) / pairs.len() as u64) as u32
}

fn success_flags(pairs: &[Pair], sesoi_milli: i64, direction: Direction) -> Vec<bool> {
    pairs
        .iter()
        .map(|pair| {
            pair.relative_milli()
                .is_some_and(|relative| match direction {
                    Direction::Decrease => relative <= -sesoi_milli,
                    Direction::Increase => relative >= sesoi_milli,
                    Direction::Either => relative.abs() >= sesoi_milli,
                })
        })
        .collect()
}

/// Estimate power for a range of candidate seed counts.
///
/// `required_of_thirty` is the criterion as written (20 of 30); at other
/// sizes the requirement scales by the same ratio, rounded up, so the bar
/// does not quietly soften as `n` grows.
pub fn power_curve(
    pilot: &[Pair],
    sesoi_milli: i64,
    direction: Direction,
    candidate_worlds: &[usize],
    required_of_thirty: usize,
    analysis_seed: u64,
) -> Vec<PowerPoint> {
    let successes: Vec<bool> = success_flags(pilot, sesoi_milli, direction);
    if successes.is_empty() {
        return candidate_worlds
            .iter()
            .map(|worlds| PowerPoint {
                worlds: *worlds,
                required: required_at(*worlds, required_of_thirty),
                power_milli: 0,
            })
            .collect();
    }

    candidate_worlds
        .iter()
        .map(|&worlds| {
            let required = required_at(worlds, required_of_thirty);
            let mut passes = 0_u32;
            for trial in 0..POWER_TRIALS {
                let mut hits = 0_usize;
                for draw in 0..worlds {
                    let random = named_random(
                        analysis_seed,
                        u64::from(trial),
                        RngSystem::Analysis,
                        worlds as u64,
                        draw as u32,
                    );
                    if successes[(random % successes.len() as u64) as usize] {
                        hits += 1;
                    }
                }
                if hits >= required {
                    passes += 1;
                }
            }
            PowerPoint {
                worlds,
                required,
                power_milli: ((u64::from(passes) * 1_000) / u64::from(POWER_TRIALS)) as u32,
            }
        })
        .collect()
}

/// The criterion's 20-of-30 bar carried to another size, rounded up.
pub fn required_at(worlds: usize, required_of_thirty: usize) -> usize {
    (worlds * required_of_thirty).div_ceil(30).min(worlds)
}

/// Smallest candidate whose estimated power reaches `target_milli`.
pub fn smallest_adequate(curve: &[PowerPoint], target_milli: u32) -> Option<PowerPoint> {
    curve
        .iter()
        .filter(|point| point.power_milli >= target_milli)
        .min_by_key(|point| point.worlds)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pilot(relative_hits: usize, total: usize) -> Vec<Pair> {
        (0..total)
            .map(|index| Pair {
                seed: index as u64,
                // A 30 percent shift clears a 10 percent SESOI; a 1 percent
                // one does not.
                treatment_milli: if index < relative_hits { 1_300 } else { 1_010 },
                control_milli: 1_000,
            })
            .collect()
    }

    #[test]
    fn the_bar_scales_with_size_and_never_exceeds_it() {
        assert_eq!(required_at(30, 20), 20);
        assert_eq!(required_at(60, 20), 40);
        assert_eq!(required_at(15, 20), 10);
        // Rounds up rather than down, so the bar never softens.
        assert_eq!(required_at(31, 20), 21);
        assert_eq!(required_at(1, 20), 1);
    }

    #[test]
    fn a_strong_pilot_effect_reaches_full_power_at_the_floor() {
        let curve = power_curve(&pilot(12, 12), 100, Direction::Increase, &[30], 20, 7);
        assert_eq!(curve[0].power_milli, 1_000);
    }

    #[test]
    fn a_null_pilot_gives_no_power_at_any_size() {
        let curve = power_curve(
            &pilot(0, 12),
            100,
            Direction::Increase,
            &[30, 60, 120],
            20,
            7,
        );
        assert!(curve.iter().all(|point| point.power_milli == 0));
        assert_eq!(smallest_adequate(&curve, 800), None);
    }

    #[test]
    fn power_rises_with_seed_count_for_a_marginal_effect() {
        // A per-world success rate just above the 2/3 bar: underpowered at
        // 30 worlds, better at 120. This is exactly the case the analysis
        // exists to detect before a campaign runs rather than after.
        let marginal = pilot(7, 10); // 70 percent
        let curve = power_curve(
            &marginal,
            100,
            Direction::Increase,
            &[30, 60, 120, 240],
            20,
            7,
        );
        assert_eq!(
            observed_success_rate_milli(&marginal, 100, Direction::Increase),
            700
        );
        for window in curve.windows(2) {
            assert!(
                window[1].power_milli >= window[0].power_milli,
                "power fell from {:?} to {:?}",
                window[0],
                window[1]
            );
        }
        assert!(curve[0].power_milli < 1_000);
        assert!(curve[3].power_milli > curve[0].power_milli);
    }

    #[test]
    fn the_curve_is_reproducible_from_its_seed() {
        let sample = pilot(8, 12);
        let a = power_curve(&sample, 100, Direction::Increase, &[30, 60], 20, 99);
        let b = power_curve(&sample, 100, Direction::Increase, &[30, 60], 20, 99);
        assert_eq!(a, b);
    }
}
