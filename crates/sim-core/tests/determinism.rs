//! Deterministic replay tests (same build, same process).
//! Clean-process determinism is covered by `scripts/verify-phase1-determinism.sh`
//! and the CLI integration tests, which spawn separate processes.

use sim_core::{SimConfig, World, WorldgenVersion};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn test_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = 200;
    config.max_entities = 2_000;
    config
}

#[test]
fn same_seed_and_config_repeat_exactly() {
    let mut first = World::new(test_config()).unwrap();
    let mut second = World::new(test_config()).unwrap();
    for tick in 0..500 {
        first.step();
        second.step();
        if tick % 100 == 0 {
            assert_eq!(
                first.state_checksum(),
                second.state_checksum(),
                "diverged at tick {tick}"
            );
        }
    }
    assert_eq!(first.state_checksum(), second.state_checksum());
    assert_eq!(first.population(), second.population());
    first.check_invariants().unwrap();
}

#[test]
fn different_seed_diverges() {
    let mut config = test_config();
    let mut first = World::new(config).unwrap();
    config.world_seed = SEED + 1;
    let mut second = World::new(config).unwrap();
    for _ in 0..200 {
        first.step();
        second.step();
    }
    assert_ne!(first.state_checksum(), second.state_checksum());
}

#[test]
fn config_change_changes_hash_and_trajectory() {
    let base = test_config();
    let mut changed = base;
    changed.growth_rate_q16_per_s += 1;
    assert_ne!(base.stable_hash(), changed.stable_hash());

    let mut first = World::new(base).unwrap();
    let mut second = World::new(changed).unwrap();
    for _ in 0..300 {
        first.step();
        second.step();
    }
    // Different config hash feeds the state checksum, so these must differ
    // even if trajectories happened to coincide.
    assert_ne!(first.state_checksum(), second.state_checksum());
}

#[test]
fn pause_and_resume_do_not_perturb_the_trajectory() {
    let mut uninterrupted = World::new(test_config()).unwrap();
    let mut interrupted = World::new(test_config()).unwrap();

    for _ in 0..100 {
        uninterrupted.step();
        interrupted.step();
    }
    interrupted.set_paused(true);
    for _ in 0..50 {
        interrupted.step(); // must be no-ops
    }
    interrupted.set_paused(false);
    for _ in 0..100 {
        uninterrupted.step();
        interrupted.step();
    }
    assert_eq!(uninterrupted.tick_number(), interrupted.tick_number());
    assert_eq!(uninterrupted.state_checksum(), interrupted.state_checksum());
}

#[test]
fn checksum_covers_paused_flag() {
    let mut world = World::new(test_config()).unwrap();
    let unpaused = world.state_checksum();
    world.set_paused(true);
    assert_ne!(world.state_checksum(), unpaused);
    world.set_paused(false);
    assert_eq!(world.state_checksum(), unpaused);
}

// --- Phase 6 climate ------------------------------------------------------

/// Seeds 1 and 7 are used below because they generate a world containing
/// every biome at this map size. That is not incidental: `Arid` requires an
/// interior far enough from water to be dry, so a small continent genuinely
/// has none, and C6.7 rejects such a world rather than producing a
/// six-biome map that claims to be a seven-biome one. Measured rejection
/// rates by map size are recorded in `research/performance-notes.md`.
fn phase6_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase6_default(seed);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = 80;
    config.max_entities = 800;
    config
}

/// C6.1: a climate-disabled world reproduces the Phase 1 and Phase 2 code
/// paths exactly, including both fixtures' config hashes.
#[test]
fn c6_1_disabled_climate_section_is_excluded_from_the_config_hash() {
    let phase1 = SimConfig::phase1_default(0x5eed_cafe_f00d_beef);
    let phase2 = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
    assert_eq!(phase1.stable_hash(), 0x918a_381c_7755_9236);
    assert_eq!(phase2.stable_hash(), 0xf83d_3981_bf7d_d189);
    assert!(!phase1.climate.enabled);
    // Changing a disabled section's parameters cannot move the hash.
    let mut fiddled = phase1;
    fiddled.climate.season_amplitude_milli += 1_234;
    fiddled.climate.highland_elevation_q16 = 12_345;
    assert_eq!(fiddled.stable_hash(), phase1.stable_hash());
}

/// Enabling climate is a new replay lineage, never a silent reinterpretation.
#[test]
fn c6_1_enabling_climate_starts_a_new_lineage() {
    let phase2 = SimConfig::phase2_default(7);
    let phase6 = SimConfig::phase6_default(7);
    assert_ne!(phase2.stable_hash(), phase6.stable_hash());
    assert_eq!(phase6.climate.worldgen_version, WorldgenVersion::V2);
    // The generator and the climate section must move together.
    let mut mismatched = phase6;
    mismatched.climate.worldgen_version = WorldgenVersion::V1;
    assert!(mismatched.validate().is_err());
}

#[test]
fn c6_6_climate_world_replays_identically_from_the_same_seed() {
    let config = phase6_config(7);
    let mut first = World::new(config).unwrap();
    let mut second = World::new(config).unwrap();
    for _ in 0..600 {
        first.step();
        second.step();
    }
    assert_eq!(first.state_checksum(), second.state_checksum());
    first.check_invariants().unwrap();
}

/// C6.6: drift is stateless, so a world saved and restored mid-run continues
/// bit-identically — the property an accumulating climate could not have.
#[test]
fn c6_6_climate_survives_save_restore_and_continues_identically() {
    let config = phase6_config(7);
    let mut original = World::new(config).unwrap();
    for _ in 0..400 {
        original.step();
    }
    let checksum = original.state_checksum();
    let state = original.export_state();
    assert!(
        state.climate.is_some(),
        "a climate world must save its climate"
    );

    let mut restored = World::from_state(state).unwrap();
    assert_eq!(restored.state_checksum(), checksum);
    for _ in 0..300 {
        original.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        original.state_checksum(),
        "post-restore divergence with climate enabled"
    );
    restored.check_invariants().unwrap();
}

/// C6.8: moisture is conserved exactly across a long run inside a live world.
#[test]
fn c6_8_moisture_conserves_and_biomes_are_populated() {
    let config = phase6_config(7);
    let mut world = World::new(config).unwrap();
    let initial: i128 = world
        .moisture_cells()
        .iter()
        .map(|&value| i128::from(value))
        .sum();
    assert!(initial > 0);
    for tick in 1..=5_000_u64 {
        world.step();
        if tick % 500 == 0 {
            let total: i128 = world
                .moisture_cells()
                .iter()
                .map(|&value| i128::from(value))
                .sum();
            assert_eq!(total, initial, "moisture leaked by tick {tick}");
            world.check_invariants().unwrap();
        }
    }
    // C6.7: every biome is represented and none covers the map.
    let histogram = world.biome_histogram();
    let total: u32 = histogram.iter().sum();
    for (index, &count) in histogram.iter().enumerate() {
        assert!(count > 0, "biome {index} is empty");
        assert!(count < total, "biome {index} covers the whole map");
    }
}

/// A climate world must still be a *different* world from the phase2 one, or
/// the section is inert and the phase delivered nothing.
#[test]
fn c6_1_enabled_climate_changes_the_world_not_just_the_hash() {
    let mut phase2 = phase6_config(7);
    phase2.climate.enabled = false;
    phase2.climate.worldgen_version = WorldgenVersion::V1;
    let mut without = World::new(phase2).unwrap();
    let mut with = World::new(phase6_config(7)).unwrap();
    for _ in 0..500 {
        without.step();
        with.step();
    }
    assert_ne!(
        without.total_biomass_milli(),
        with.total_biomass_milli(),
        "biome-scaled capacity had no effect on the food field"
    );
    assert!(without.biome_cells().is_empty());
    assert_eq!(with.biome_cells().len(), with.biomass_cells().len());
}

/// Regression guard for the defect that made the first moisture model
/// worthless: a pure-diffusion update conserves the total and still erases
/// the field, because its only fixed point is a uniform one. A world that
/// generates with seven biomes and holds six by tick 20,000 would make every
/// Phase 6 result a result about the model flattening its own terrain.
#[test]
fn c6_8_the_moisture_gradient_survives_a_long_run() {
    let mut world = World::new(phase6_config(7)).unwrap();
    let spread = |world: &World| -> i64 {
        let moisture = world.moisture_cells();
        let high = moisture.iter().copied().max().unwrap_or(0);
        let low = moisture.iter().copied().min().unwrap_or(0);
        high - low
    };
    let initial_spread = spread(&world);
    assert!(initial_spread > 0);

    for _ in 0..20_000 {
        world.step();
    }

    // The field must still be a field, not an average.
    let final_spread = spread(&world);
    assert!(
        final_spread * 2 >= initial_spread,
        "moisture spread collapsed from {initial_spread} to {final_spread}; \
         the update is homogenizing the world"
    );
    // And every biome must still exist.
    let histogram = world.biome_histogram();
    for (index, &count) in histogram.iter().enumerate() {
        assert!(count > 0, "biome {index} was erased by tick 20,000");
    }
    world.check_invariants().unwrap();
}
