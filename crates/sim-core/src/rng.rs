//! Named deterministic random streams.
//!
//! Every draw is a pure function of `(world_seed, tick, system, subject,
//! draw_index)`. No system reads shared mutable RNG state, so adding a draw
//! in one system can never shift another system's sequence.

/// Identifier recorded in world metadata and deterministic fixtures.
/// Changing the derivation below requires bumping this version and creates a
/// new replay lineage.
pub const RNG_ALGORITHM_VERSION: &str = "lifesim-rng-v1";

/// Bounded enumeration of random-consuming systems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum RngSystem {
    WorldGen = 1,
    Spawn = 2,
    Movement = 3,
    Reproduction = 4,
    /// Founder genome creation (Phase 2).
    GenomeInit = 5,
    /// Two-source recombination and bounded variation (Phase 2).
    Recombination = 6,
    /// Contest tie lottery, damage variance, and retreat resolution
    /// (Phase 7). Every draw is keyed on the canonical pair key.
    Contest = 7,
    /// Age-dependent and extrinsic mortality hazard draws (Phase 8).
    /// Senescence uses draw index 0 and extrinsic hazard draw index 1;
    /// those indices are as permanent as the stream value itself, because
    /// renumbering them would silently change every world.
    Mortality = 16,
    /// Optional bounded stochastic climate component (Phase 6). Allocated
    /// now and unused under the default deterministic policy, so adopting a
    /// stochastic policy later cannot renumber an existing stream.
    ClimateDrift = 21,
    /// Archetype selection, deme centre choice, and founder placement
    /// (Phase 6). Deliberately separate from `GenomeInit` (5) so adding
    /// origin modes cannot shift an existing `random` world's founder
    /// sequence.
    FounderSeed = 22,
    /// Offline analysis resampling: bootstrap intervals and simulation-based
    /// power (Phase 7). **No tick ever draws on this stream.** It lives here
    /// only so that offline resampling uses the same audited derivation as
    /// everything else and a report reproduces bit-for-bit; ADR-0016 still
    /// forbids anything computed from it reaching world state.
    Analysis = 41,
}

/// Derive one deterministic 64-bit value for a named draw.
pub fn named_random(
    world_seed: u64,
    tick: u64,
    system: RngSystem,
    subject_id: u64,
    draw_index: u32,
) -> u64 {
    let mut value = world_seed ^ 0x9e37_79b9_7f4a_7c15;
    value = mix64(value ^ tick.rotate_left(17));
    value = mix64(value ^ (system as u64).rotate_left(31));
    value = mix64(value ^ subject_id.rotate_left(7));
    mix64(value ^ u64::from(draw_index).rotate_left(43))
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_are_pure_and_independent() {
        let before = named_random(7, 11, RngSystem::Movement, 99, 0);
        let _unrelated = named_random(7, 11, RngSystem::Movement, 12, 900);
        let _other_system = named_random(7, 11, RngSystem::Reproduction, 99, 0);
        let after = named_random(7, 11, RngSystem::Movement, 99, 0);
        assert_eq!(before, after);
    }

    #[test]
    fn inputs_change_output() {
        let base = named_random(1, 2, RngSystem::Movement, 3, 4);
        assert_ne!(base, named_random(2, 2, RngSystem::Movement, 3, 4));
        assert_ne!(base, named_random(1, 3, RngSystem::Movement, 3, 4));
        assert_ne!(base, named_random(1, 2, RngSystem::Reproduction, 3, 4));
        assert_ne!(base, named_random(1, 2, RngSystem::Movement, 4, 4));
        assert_ne!(base, named_random(1, 2, RngSystem::Movement, 3, 5));
    }
}
