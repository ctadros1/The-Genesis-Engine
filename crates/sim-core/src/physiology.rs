//! Demography and life history (Phase 8, `lifesim-demography-v1`).
//!
//! The Phase 2 long run recorded 199,871 starvation deaths against 180
//! old-age deaths with population pinned on the `max_entities` guard. That
//! is a world in which food is the only brake, a process-safety limit is
//! the carrying capacity, and no organism can ever hold surplus energy. No
//! result about costly signalling, object manipulation, or social learning
//! is measurable in it, because every such result would be a null caused by
//! starvation (ADR-0025).
//!
//! This module supplies the four mechanisms that change that, each
//! independently config-gated so any subset can be disabled and the whole
//! section disabled reproduces the Phase 7 fixture exactly.
//!
//! Three implementation choices carry determinism, and each replaces
//! something a floating-point model would have reached for:
//!
//! - **Allometry uses exact integer roots, not `powf`.** The exponent is a
//!   rational with denominator four, so `x^(n/4)` is `isqrt(isqrt(x^n))` in
//!   fixed point. That covers 0.25 through 1.5 in quarter steps, including
//!   Kleiber's 0.75, with no transcendental and no platform variance.
//! - **Senescence is Weibull, not Gompertz.** A Gompertz hazard is
//!   exponential in age and would need `exp`. A Weibull hazard is a small
//!   integer power of scaled age, is the other standard senescence model,
//!   and is exact here.
//! - **Senescence and extrinsic mortality draw separately.** They are
//!   competing risks, so one draw each on distinct draw indices attributes
//!   every death to the hazard that actually caused it. A single combined
//!   draw would make the death-cause distribution -- which is C8.1's whole
//!   subject -- unattributable.
//!
//! Nothing here claims that any parameter corresponds to a real organism.
//! "Mass" is the body-scale phenotype in milli-units where 1000 is the
//! reference; it is not kilograms.

use crate::checksum::Fnv1a64;
use crate::config::{PhysiologyConfig, Q16_ONE};
use crate::rng::{RngSystem, named_random};

pub const PHYSIOLOGY_POLICY_VERSION: &str = "lifesim-demography-v1";

/// Per-organism physiology state plus the section's own counters.
///
/// `None` exactly when the section is disabled, so a disabled world takes
/// the Phase 7 code paths and reproduces its fixture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhysiologyState {
    /// Cumulative hazard survived, Q16-seconds. Fixed point per rule 7: it
    /// integrates over a lifetime. Reported so a campaign can show what the
    /// hazard model actually delivered rather than what it was configured
    /// to deliver.
    pub cumulative_hazard_q16: Vec<i64>,

    /// Counted here rather than in `Counters` so the Phase 1/2 checksum
    /// field list is untouched, exactly as Phase 7 did for damage deaths.
    pub deaths_senescence_total: u64,
    pub deaths_extrinsic_total: u64,
    pub deaths_juvenile_total: u64,
    pub thermal_cost_milli: i128,
    pub allometric_cost_milli: i128,
}

impl PhysiologyState {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cumulative_hazard_q16: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.cumulative_hazard_q16.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cumulative_hazard_q16.is_empty()
    }

    pub fn push_organism(&mut self) {
        self.cumulative_hazard_q16.push(0);
    }

    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for read in 0..self.cumulative_hazard_q16.len() {
            if !remove[read] {
                self.cumulative_hazard_q16[write] = self.cumulative_hazard_q16[read];
                write += 1;
            }
        }
        self.cumulative_hazard_q16.truncate(write);
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-physiology-state-v1");
        for &hazard in &self.cumulative_hazard_q16 {
            hasher.update_i64(hazard);
        }
        hasher.update_u64(self.deaths_senescence_total);
        hasher.update_u64(self.deaths_extrinsic_total);
        hasher.update_u64(self.deaths_juvenile_total);
        hasher.update_i128(self.thermal_cost_milli);
        hasher.update_i128(self.allometric_cost_milli);
    }
}

/// Integer square root of a non-negative `i128`, exact and platform-free.
fn isqrt_i128(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }
    // Newton's method from a power-of-two seed; terminates and is exact.
    let mut guess = 1_i128 << ((128 - value.leading_zeros()).div_ceil(2));
    loop {
        let next = (guess + value / guess) / 2;
        if next >= guess {
            return guess;
        }
        guess = next;
    }
}

/// Square root of a milli-unit value, in milli-units.
fn sqrt_milli(value_milli: i128) -> i128 {
    isqrt_i128(value_milli * 1_000)
}

/// `base^(quarters/4)` for a milli-unit base, in milli-units.
///
/// Exact integer arithmetic: raise to the integer power, then take two
/// integer square roots. `quarters` is validated to 1..=6, covering 0.25
/// through 1.5, which spans every metabolic exponent anyone argues about.
pub fn pow_quarter_milli(base_milli: i64, quarters: u32) -> i64 {
    let base = i128::from(base_milli.max(0));
    if base == 0 {
        return 0;
    }
    let quarters = quarters.clamp(1, 6);
    let mut powered = 1_000_i128; // 1.0 in milli
    for _ in 0..quarters {
        powered = powered * base / 1_000;
    }
    // powered is now base^quarters; the two roots divide the exponent by 4.
    sqrt_milli(sqrt_milli(powered)) as i64
}

/// Allometric basal cost multiplier for a body scale, in milli-units.
///
/// Returns 1000 (a no-op multiplier) when allometry is disabled, so the
/// caller has one code path and the disabled world is bit-identical.
pub fn allometry_multiplier_milli(config: &PhysiologyConfig, body_scale_milli: i64) -> i64 {
    if !config.allometry_enabled {
        return 1_000;
    }
    pow_quarter_milli(body_scale_milli, config.basal_exponent_quarters)
}

/// The temperature an organism with this preference is neutral at,
/// milli-degrees.
pub fn preferred_temperature_milli(config: &PhysiologyConfig, thermal_pref_milli: i64) -> i32 {
    let low = i64::from(config.thermal_pref_low_milli);
    let high = i64::from(config.thermal_pref_high_milli);
    let pref = thermal_pref_milli.clamp(0, 1_000);
    (low + (high - low) * pref / 1_000) as i32
}

/// Thermoregulation cost for one tick, milli-EU.
///
/// Zero inside the neutral band, then linear in the excess deviation. A
/// quadratic penalty was considered and rejected: it makes the cost depend
/// on the square of a field the organism cannot fully perceive, which turns
/// a marginal habitat into an instantly lethal one and would collapse
/// populations for a reason unrelated to the mechanism being tested.
pub fn thermal_cost_milli(
    config: &PhysiologyConfig,
    thermal_pref_milli: i64,
    cell_temperature_milli: i32,
    dt_ms: u32,
) -> i64 {
    if !config.thermoregulation_enabled {
        return 0;
    }
    let preferred = preferred_temperature_milli(config, thermal_pref_milli);
    let deviation = i64::from((cell_temperature_milli - preferred).abs());
    let excess = (deviation - i64::from(config.thermal_neutral_band_milli)).max(0);
    // excess is in milli-degrees; the rate is milli-EU per second per degree.
    excess * config.thermal_cost_milli_per_s_per_degree * i64::from(dt_ms) / (1_000 * 1_000)
}

/// Senescence hazard at an age, Q16 per second.
///
/// Weibull: zero until `senescence_onset_ticks`, then rising as the
/// `senescence_power`-th power of elapsed age scaled by
/// `senescence_scale_ticks`. Unbounded in principle, clamped to certainty
/// at the point of use.
pub fn senescence_hazard_q16_per_s(config: &PhysiologyConfig, age_ticks: u64) -> u64 {
    if !config.senescence_enabled {
        return 0;
    }
    let elapsed = age_ticks.saturating_sub(config.senescence_onset_ticks);
    if elapsed == 0 {
        return 0;
    }
    let scale = u128::from(config.senescence_scale_ticks.max(1));
    let ratio_q16 = u128::from(elapsed) * u128::from(Q16_ONE) / scale;
    let mut powered = ratio_q16;
    for _ in 1..config.senescence_power.clamp(1, 4) {
        powered = powered * ratio_q16 / u128::from(Q16_ONE);
        if powered > u128::from(u32::MAX) * 65_536 {
            break;
        }
    }
    let hazard = powered * u128::from(config.senescence_hazard_q16_per_s) / u128::from(Q16_ONE);
    hazard.min(u128::from(u64::from(Q16_ONE) * 1_000)) as u64
}

/// Which competing risk, if any, kills this organism this tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HazardOutcome {
    Survives,
    Senescence,
    Extrinsic,
}

/// One tick's mortality draw for one organism.
///
/// Senescence and extrinsic hazard are **separate draws on separate draw
/// indices**, because they are competing risks and C8.1 is a claim about
/// which of them killed what. A single combined draw would be cheaper and
/// would make the death-cause distribution unattributable.
///
/// The juvenile multiplier applies to both, so a pre-reproductive organism
/// faces the same causes at a raised rate rather than a distinct cause.
/// Returns the outcome and the total hazard applied, Q16 per tick, which
/// the caller accumulates.
pub fn hazard_draw(
    config: &PhysiologyConfig,
    world_seed: u64,
    tick: u64,
    organism_id: u64,
    age_ticks: u64,
    maturity_ticks: u64,
    dt_ms: u32,
) -> (HazardOutcome, i64) {
    let juvenile_q16 = if age_ticks < maturity_ticks {
        u64::from(config.juvenile_hazard_multiplier_q16)
    } else {
        u64::from(Q16_ONE)
    };

    let per_tick = |per_second_q16: u64| -> u64 {
        let scaled = per_second_q16 * juvenile_q16 / u64::from(Q16_ONE);
        (scaled * u64::from(dt_ms) / 1_000).min(u64::from(Q16_ONE))
    };

    let senescence = per_tick(senescence_hazard_q16_per_s(config, age_ticks));
    let extrinsic = per_tick(u64::from(config.extrinsic_hazard_q16_per_s));
    let total = (senescence + extrinsic).min(u64::from(Q16_ONE)) as i64;

    // Draw index 0 is senescence, 1 is extrinsic. Permanent, like an RNG
    // stream value: renumbering them would silently change every world.
    let outcome = if senescence > 0
        && (named_random(world_seed, tick, RngSystem::Mortality, organism_id, 0) & 0xffff)
            < senescence
    {
        HazardOutcome::Senescence
    } else if extrinsic > 0
        && (named_random(world_seed, tick, RngSystem::Mortality, organism_id, 1) & 0xffff)
            < extrinsic
    {
        HazardOutcome::Extrinsic
    } else {
        HazardOutcome::Survives
    };
    (outcome, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PhysiologyConfig {
        PhysiologyConfig::physiology_default()
    }

    #[test]
    fn the_quarter_power_matches_values_computable_by_hand() {
        // 1.0 to any power is 1.0, exactly.
        for quarters in 1..=6 {
            assert_eq!(pow_quarter_milli(1_000, quarters), 1_000);
        }
        // 1.6^0.75 = 1.42247...; 0.6^0.75 = 0.68182...
        assert!((pow_quarter_milli(1_600, 3) - 1_422).abs() <= 1);
        assert!((pow_quarter_milli(600, 3) - 682).abs() <= 1);
        // Exponent 4/4 is the identity, which is the check that the two
        // roots really do divide the exponent by four.
        for base in [600_i64, 900, 1_000, 1_300, 1_600] {
            assert!(
                (pow_quarter_milli(base, 4) - base).abs() <= 2,
                "base {base}"
            );
        }
        // 2/4 is a square root: 1.44^0.5 = 1.2.
        assert!((pow_quarter_milli(1_440, 2) - 1_200).abs() <= 1);
    }

    #[test]
    fn allometry_is_sublinear_and_monotonic() {
        let mut config = config();
        config.allometry_enabled = true;
        config.basal_exponent_quarters = 3; // 0.75
        let small = allometry_multiplier_milli(&config, 600);
        let reference = allometry_multiplier_milli(&config, 1_000);
        let large = allometry_multiplier_milli(&config, 1_600);
        assert!(small < reference && reference < large);
        assert_eq!(reference, 1_000);
        // Sublinear: doubling mass must less than double the cost. That is
        // the entire content of allometry and the thing C8.4 measures.
        let ratio_milli = large * 1_000 / small;
        let mass_ratio_milli = 1_600 * 1_000 / 600;
        assert!(
            ratio_milli < mass_ratio_milli,
            "cost ratio {ratio_milli} should be below mass ratio {mass_ratio_milli}"
        );
    }

    #[test]
    fn disabled_allometry_is_exactly_a_no_op() {
        let mut config = config();
        config.allometry_enabled = false;
        for scale in [600_i64, 1_000, 1_600] {
            assert_eq!(allometry_multiplier_milli(&config, scale), 1_000);
        }
    }

    #[test]
    fn thermal_cost_is_zero_in_the_band_and_rises_outside_it() {
        let mut config = config();
        config.thermoregulation_enabled = true;
        // Preference 0.5 maps to the midpoint of the configured range.
        let preferred = preferred_temperature_milli(&config, 500);
        assert_eq!(
            preferred,
            (config.thermal_pref_low_milli + config.thermal_pref_high_milli) / 2
        );
        // Inside the band: free.
        assert_eq!(
            thermal_cost_milli(&config, 500, preferred, 100),
            0,
            "at the preferred temperature"
        );
        let edge = preferred + config.thermal_neutral_band_milli;
        assert_eq!(thermal_cost_milli(&config, 500, edge, 100), 0);
        // Outside: positive, symmetric, and monotonic.
        let hot = thermal_cost_milli(&config, 500, edge + 10_000, 100);
        let cold = thermal_cost_milli(
            &config,
            500,
            preferred - config.thermal_neutral_band_milli - 10_000,
            100,
        );
        assert!(hot > 0);
        assert_eq!(hot, cold, "the penalty must be symmetric about preference");
        assert!(thermal_cost_milli(&config, 500, edge + 20_000, 100) > hot);
    }

    #[test]
    fn disabled_thermoregulation_is_exactly_a_no_op() {
        let mut config = config();
        config.thermoregulation_enabled = false;
        assert_eq!(thermal_cost_milli(&config, 0, 100_000, 100), 0);
    }

    #[test]
    fn senescence_hazard_is_zero_before_onset_and_monotonic_after() {
        let mut config = config();
        config.senescence_enabled = true;
        for age in [0_u64, 100, config.senescence_onset_ticks] {
            assert_eq!(senescence_hazard_q16_per_s(&config, age), 0, "age {age}");
        }
        let mut previous = 0;
        for step in 1..=20_u64 {
            let age = config.senescence_onset_ticks + step * 500;
            let hazard = senescence_hazard_q16_per_s(&config, age);
            assert!(hazard >= previous, "hazard fell at age {age}");
            previous = hazard;
        }
        assert!(previous > 0, "hazard never rose above zero");
    }

    #[test]
    fn the_hazard_probability_never_leaves_its_bounds() {
        // Adversarial config: everything at its maximum. The per-tick
        // probability must still be a probability.
        let mut config = config();
        config.senescence_enabled = true;
        config.extrinsic_hazard_q16_per_s = u32::MAX;
        config.senescence_hazard_q16_per_s = u32::MAX;
        config.juvenile_hazard_multiplier_q16 = u32::MAX;
        config.senescence_power = 4;
        for age in [0_u64, 1, 1_000, 100_000, u64::from(u32::MAX)] {
            let (_, total) = hazard_draw(&config, 1, 1, 1, age, 600, 100);
            assert!(
                (0..=i64::from(Q16_ONE)).contains(&total),
                "total hazard {total} out of bounds at age {age}"
            );
        }
    }

    #[test]
    fn a_zero_hazard_configuration_never_kills() {
        let mut config = config();
        config.senescence_enabled = false;
        config.extrinsic_hazard_q16_per_s = 0;
        for tick in 0..2_000_u64 {
            let (outcome, total) = hazard_draw(&config, 5, tick, 7, 50_000, 600, 100);
            assert_eq!(outcome, HazardOutcome::Survives);
            assert_eq!(total, 0);
        }
    }

    #[test]
    fn extrinsic_mortality_kills_at_about_its_configured_rate() {
        // The check that the draw means what the config says. At 0.001 per
        // second and dt = 100 ms the per-tick probability is 1e-4, so over
        // 200,000 organism-ticks expect about 20 deaths.
        let mut config = config();
        config.senescence_enabled = false;
        config.extrinsic_hazard_q16_per_s = (0.001 * 65_536.0) as u32;
        let mut deaths = 0;
        for id in 0..200_u64 {
            for tick in 0..1_000_u64 {
                if hazard_draw(&config, 9, tick, id, 5_000, 600, 100).0 == HazardOutcome::Extrinsic
                {
                    deaths += 1;
                }
            }
        }
        assert!(
            (10..=35).contains(&deaths),
            "expected roughly 20 deaths at the configured rate, got {deaths}"
        );
    }

    #[test]
    fn the_juvenile_multiplier_raises_the_rate_for_the_immature_only() {
        let mut config = config();
        config.senescence_enabled = false;
        config.extrinsic_hazard_q16_per_s = (0.002 * 65_536.0) as u32;
        config.juvenile_hazard_multiplier_q16 = 4 * Q16_ONE;
        let count = |age: u64| {
            (0..200_u64)
                .flat_map(|id| (0..1_000_u64).map(move |tick| (id, tick)))
                .filter(|(id, tick)| {
                    hazard_draw(&config, 3, *tick, *id, age, 600, 100).0 == HazardOutcome::Extrinsic
                })
                .count()
        };
        let juvenile = count(100);
        let adult = count(5_000);
        assert!(
            juvenile > adult * 2,
            "juvenile deaths {juvenile} should far exceed adult deaths {adult}"
        );
    }

    #[test]
    fn the_two_risks_are_attributed_separately() {
        // Both hazards at once: every death must still name exactly one
        // cause, and both causes must actually occur. A single combined
        // draw would make this test impossible to write.
        let mut config = config();
        config.senescence_enabled = true;
        config.senescence_onset_ticks = 100;
        config.senescence_scale_ticks = 1_000;
        config.senescence_hazard_q16_per_s = (0.01 * 65_536.0) as u32;
        config.extrinsic_hazard_q16_per_s = (0.01 * 65_536.0) as u32;
        let mut senescence = 0;
        let mut extrinsic = 0;
        for id in 0..500_u64 {
            for tick in 0..200_u64 {
                match hazard_draw(&config, 11, tick, id, 3_000, 600, 100).0 {
                    HazardOutcome::Senescence => senescence += 1,
                    HazardOutcome::Extrinsic => extrinsic += 1,
                    HazardOutcome::Survives => {}
                }
            }
        }
        assert!(
            senescence > 0 && extrinsic > 0,
            "{senescence} / {extrinsic}"
        );
    }

    #[test]
    fn retain_keeps_the_hazard_array_in_lockstep() {
        let mut state = PhysiologyState::with_capacity(4);
        for _ in 0..4 {
            state.push_organism();
        }
        state.cumulative_hazard_q16[2] = 77;
        state.retain(&[false, true, false, true]);
        assert_eq!(state.cumulative_hazard_q16, vec![0, 77]);
        assert_eq!(state.len(), 2);
    }
}
