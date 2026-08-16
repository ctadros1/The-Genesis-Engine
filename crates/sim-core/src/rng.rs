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
    /// Crossover counts and positions during gamete formation (Phase 9).
    /// Keyed on the prospective child and on the parent's haplotype slot, so
    /// the two parents' gametes are not drawn from the same sequence.
    Meiosis = 8,
    /// Structural mutation: duplication, deletion, insertion, transposition
    /// (Phase 9). Value mutation stays on `Recombination` (6), preserving
    /// the existing convention.
    StructuralMutation = 9,
    /// Initial plastic-state seeding at birth (Phase 11). Allocated now and
    /// **unused**: the default policy zeroes `learned_q16` and `trace_q16`
    /// at birth, and that zero is an invariant rather than a default - if
    /// learned state were inherited, a discovery would become a heritable
    /// trait and Phase 13's transmission question would be unaskable. The
    /// stream exists so that adopting the `lamarckian_fraction_q16`
    /// experimental condition later cannot renumber an existing stream.
    PlasticityInit = 10,
    /// Object physics (Phase 12 artifact half): fragment count on a fracture
    /// (subject: the struck object's ID, draw 0) and joint quality on a
    /// combination (subject: `pair_key(combiner, target)`, draw 0). Placement
    /// takes no draw in this version - `place` lands in the faced cell or is
    /// refused, never snapped - so the "placement site selection" the
    /// determinism spec reserved for this stream is unused and stays
    /// reserved.
    Artifact = 13,
    /// Extraction yield variance on a terrain strike (Phase 12 artifact
    /// half): subject the cell index, draw 0.
    MaterialYield = 14,
    /// Terrain modification schedules (Phase 12): today, the relocating
    /// resource patch's centre.
    ///
    /// **11 and 12 are skipped.** 13 and 14 were reserved for the artifact
    /// half before this stream was allocated, so it took 15 rather than the
    /// next free value; the artifact half has since taken them. Stream
    /// numbers are permanent - renumbering one silently changes every world
    /// that draws on it. 11 and 12 are left free deliberately, as spare
    /// capacity between the organism streams and the world streams.
    ///
    /// Keyed on the *epoch* (`tick / relocate_interval_ticks`) as the
    /// subject, not on a cell or an organism, which is what makes the patch a
    /// pure function of `(world_seed, epoch)`: the schedule needs no save
    /// section because any tick can recompute where the patch is and where it
    /// was.
    TerrainMod = 15,
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
