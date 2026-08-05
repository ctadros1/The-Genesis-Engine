//! Health, damage, contest, and carcasses (Phase 7, `contest-behavior-v1`).
//!
//! Phase 7 delivers the **physics of damage and contest** and nothing more.
//! The original justification for scheduling it here also claimed organized
//! violence "tends to fall out of scarcity plus kin-biased grouping"; the
//! commissioned social-organization review rejects that shortcut and lists
//! roughly eleven further dependencies, almost all of which land in Phases 10
//! to 12 (ADR-0022 A1). So kin recognition, directed inter-group aggression,
//! and coalition formation are **not** implemented and **not** claimed here.
//!
//! There is no group, tribe, faction, alliance, or territory object anywhere
//! in this module. Grouping, if it occurs, is a spatial and genetic statistic
//! measured after the fact.
//!
//! Two determinism rules do the load-bearing work:
//!
//! - **Rule 1, the canonical pair key.** Every draw whose outcome concerns
//!   two organisms is keyed on `pair_key(p, q)`, which is symmetric, so the
//!   outcome cannot depend on which combatant the tick happened to visit
//!   first. That is the difference between a contest system that replays and
//!   one that merely usually replays.
//! - **Rule 7, fixed-point accumulators.** Health and accumulated damage
//!   integrate over a lifetime, so both are fixed point. Float accumulation
//!   over 10^5 ticks is exactly what ADR-0011 exists to avoid.

use crate::checksum::Fnv1a64;
use crate::config::ContestConfig;
use crate::rng::{RngSystem, named_random};

pub const CONTEST_POLICY_VERSION: &str = "contest-behavior-v1";
pub const PAIR_KEY_POLICY_VERSION: &str = "lifesim-pairkey-v1";

/// Canonical pair key (determinism rule 1, `lifesim-pairkey-v1`).
///
/// `pair_key(p, q) == pair_key(q, p)` by construction, so a draw about two
/// organisms is a function of the pair and never of traversal order.
pub fn pair_key(left: u64, right: u64) -> u64 {
    let low = left.min(right);
    let high = left.max(right);
    mix64(low) ^ mix64(high).rotate_left(32)
}

/// The splitmix-style finalizer `sim-core::rng` uses, duplicated here rather
/// than exported so the RNG module's internals stay private.
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// One carcass: the transferable energy a dead organism left behind.
///
/// Carcasses close the gap `docs/06-organism-model.md` has documented since
/// Phase 1. The ID is the dead organism's entity ID, which is unique and
/// never reused, so no new counter is introduced and Phase 11's shared
/// object-ID space stays free to be designed on its own terms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Carcass {
    pub id: u64,
    pub x_fp: i32,
    pub y_fp: i32,
    pub energy_milli: i64,
    pub created_tick: u64,
}

/// Per-organism contest state plus the carcass table.
///
/// `None` exactly when the contest section is disabled, so a disabled world
/// executes the Phase 2 code paths and reproduces its fixture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContestState {
    /// Current health, milli-units. Fixed point per rule 7.
    pub health_milli: Vec<i64>,
    /// Decaying accumulator of damage taken, milli-units. Fixed point.
    pub recent_damage_milli: Vec<i64>,
    /// Carcasses, kept sorted by ID exactly as organisms are.
    pub carcasses: Vec<Carcass>,

    /// Ledger terms for the carcass pool. Kept here rather than in the base
    /// `Ledger` so a contest-disabled world's checksum is untouched.
    pub carcass_created_milli: i128,
    pub carcass_consumed_milli: i128,
    pub carcass_decayed_milli: i128,

    pub attacks_total: u64,
    pub damage_dealt_milli: i128,
    pub deaths_by_damage_total: u64,
    pub healed_milli: i128,

    /// Per-tick intent buffers; rebuilt every tick, never logical state.
    pub intent_attack: Vec<bool>,
}

impl ContestState {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            health_milli: Vec::with_capacity(capacity),
            recent_damage_milli: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.health_milli.len()
    }

    pub fn is_empty(&self) -> bool {
        self.health_milli.is_empty()
    }

    /// Maximum health for a given body scale. Larger bodies hold more.
    pub fn health_max_milli(contest: &ContestConfig, body_scale_milli: i64) -> i64 {
        (contest.base_health_milli * body_scale_milli / 1_000).max(1)
    }

    pub fn push_organism(&mut self, health_milli: i64) {
        self.health_milli.push(health_milli);
        self.recent_damage_milli.push(0);
    }

    /// Compact with the same removal flags the world applies to its primary
    /// arrays, so the parallel arrays stay in lockstep.
    pub fn retain(&mut self, remove: &[bool]) {
        retain_by_flags(&mut self.health_milli, remove);
        retain_by_flags(&mut self.recent_damage_milli, remove);
    }

    pub fn total_carcass_energy_milli(&self) -> i128 {
        self.carcasses
            .iter()
            .map(|carcass| i128::from(carcass.energy_milli))
            .sum()
    }

    /// Damage one attack does, before clamping to the target's health.
    ///
    /// The stochastic component is keyed on the canonical pair key, so
    /// swapping which combatant is visited first cannot change the number.
    pub fn damage_milli(
        contest: &ContestConfig,
        world_seed: u64,
        tick: u64,
        attacker_id: u64,
        target_id: u64,
        attacker_scale_milli: i64,
        target_scale_milli: i64,
    ) -> i64 {
        if contest.damage_base_milli <= 0 {
            return 0;
        }
        // Larger attackers hit harder; larger targets absorb more.
        let scaled = contest.damage_base_milli * attacker_scale_milli / target_scale_milli.max(1);
        if contest.damage_variance_q16 == 0 {
            return scaled.max(0);
        }
        let draw = named_random(
            world_seed,
            tick,
            RngSystem::Contest,
            pair_key(attacker_id, target_id),
            0,
        );
        // Uniform in [1 - v, 1 + v] around the scaled damage, Q16.
        let unit = i64::from((draw & 0xffff) as u32);
        let swing = (unit - 32_768) * i64::from(contest.damage_variance_q16) / 32_768;
        let factor = 65_536 + swing;
        (scaled * factor / 65_536).max(0)
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-contest-state-v1");
        for &health in &self.health_milli {
            hasher.update_i64(health);
        }
        for &damage in &self.recent_damage_milli {
            hasher.update_i64(damage);
        }
        // Carcasses are sorted by ID, so this is an order-free hash of a
        // canonically ordered table.
        hasher.update_u32(self.carcasses.len() as u32);
        for carcass in &self.carcasses {
            hasher.update_u64(carcass.id);
            hasher.update_i32(carcass.x_fp);
            hasher.update_i32(carcass.y_fp);
            hasher.update_i64(carcass.energy_milli);
            hasher.update_u64(carcass.created_tick);
        }
        hasher.update_i128(self.carcass_created_milli);
        hasher.update_i128(self.carcass_consumed_milli);
        hasher.update_i128(self.carcass_decayed_milli);
        hasher.update_u64(self.attacks_total);
        hasher.update_i128(self.damage_dealt_milli);
        hasher.update_u64(self.deaths_by_damage_total);
        hasher.update_i128(self.healed_milli);
    }
}

fn retain_by_flags(values: &mut Vec<i64>, remove: &[bool]) {
    let mut write = 0_usize;
    for read in 0..values.len() {
        if !remove[read] {
            values[write] = values[read];
            write += 1;
        }
    }
    values.truncate(write);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ContestConfig;

    #[test]
    fn the_pair_key_is_symmetric() {
        for (left, right) in [(1_u64, 2_u64), (7, 7), (0, u64::MAX), (99, 3)] {
            assert_eq!(
                pair_key(left, right),
                pair_key(right, left),
                "pair key is not symmetric for ({left}, {right})"
            );
        }
        // ...and still distinguishes different pairs.
        assert_ne!(pair_key(1, 2), pair_key(1, 3));
        assert_ne!(pair_key(1, 2), pair_key(2, 3));
    }

    #[test]
    fn damage_does_not_depend_on_visit_order() {
        // The whole point of rule 1: the same two organisms produce the same
        // number regardless of which one the tick reached first.
        let contest = ContestConfig::contest_default();
        let forward = ContestState::damage_milli(&contest, 42, 100, 7, 9, 1_000, 1_000);
        let backward = ContestState::damage_milli(&contest, 42, 100, 9, 7, 1_000, 1_000);
        assert_eq!(forward, backward);
    }

    #[test]
    fn damage_scales_with_relative_body_size_and_stays_non_negative() {
        let mut contest = ContestConfig::contest_default();
        contest.damage_variance_q16 = 0;
        let even = ContestState::damage_milli(&contest, 1, 1, 1, 2, 1_000, 1_000);
        let big_attacker = ContestState::damage_milli(&contest, 1, 1, 1, 2, 2_000, 1_000);
        let big_target = ContestState::damage_milli(&contest, 1, 1, 1, 2, 1_000, 2_000);
        assert!(big_attacker > even);
        assert!(big_target < even);
        assert!(even >= 0 && big_target >= 0);
    }

    #[test]
    fn zero_damage_configuration_yields_exactly_zero() {
        // Condition C of the phase's design: the action fires and costs
        // energy without consequence. It has to be exactly zero, not small.
        let mut contest = ContestConfig::contest_default();
        contest.damage_base_milli = 0;
        for tick in 0..64_u64 {
            assert_eq!(
                ContestState::damage_milli(&contest, 5, tick, 1, 2, 1_500, 900),
                0
            );
        }
    }

    #[test]
    fn damage_variance_stays_inside_its_configured_band() {
        let contest = ContestConfig::contest_default();
        let mut low = i64::MAX;
        let mut high = i64::MIN;
        for tick in 0..4_000_u64 {
            let value = ContestState::damage_milli(&contest, 9, tick, 3, 4, 1_000, 1_000);
            low = low.min(value);
            high = high.max(value);
        }
        let centre = contest.damage_base_milli;
        let bound = centre * i64::from(contest.damage_variance_q16) / 65_536 + 1;
        assert!(low >= centre - bound, "{low} below the band");
        assert!(high <= centre + bound, "{high} above the band");
        // The variance must actually vary, or the band proves nothing.
        assert!(high > low);
    }

    #[test]
    fn health_maximum_scales_with_body() {
        let contest = ContestConfig::contest_default();
        let small = ContestState::health_max_milli(&contest, 600);
        let large = ContestState::health_max_milli(&contest, 1_600);
        assert!(large > small);
        assert!(small >= 1);
    }

    #[test]
    fn retain_keeps_parallel_arrays_in_lockstep() {
        let mut state = ContestState::with_capacity(4);
        for health in [10_i64, 20, 30, 40] {
            state.push_organism(health);
        }
        state.recent_damage_milli[2] = 7;
        state.retain(&[false, true, false, true]);
        assert_eq!(state.health_milli, vec![10, 30]);
        assert_eq!(state.recent_damage_milli, vec![0, 7]);
        assert_eq!(state.len(), 2);
    }
}
