//! Phase 15 increment 1: the chemistry field runs inside a world, its
//! conservation identity holds at the world level, and a populated field
//! survives a save round trip with an identical future. The stencil-level
//! adversarial conservation tests live in `chemistry.rs`'s unit tests;
//! these are the integration clauses.

use sim_core::{SimConfig, World};

const SEED: u64 = 0x0f15_5eed_0f15_5eed;

fn chemistry_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 32;
    config.cells_y = 32;
    config.initial_organisms = 40;
    config.max_entities = 1_000;
    config.chemistry.enabled = true;
    config.chemistry.field_steps_per_tick = 2;
    config
}

#[test]
fn a_chemistry_world_runs_and_the_identity_holds() {
    let mut world = World::new(chemistry_config(SEED)).expect("world");
    for tick in 1..=400 {
        world.step();
        if tick % 100 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    let state = world.export_state();
    let chemistry = state.chemistry.as_ref().expect("chemistry section");
    let total: i128 = chemistry
        .concentrations
        .iter()
        .map(|&value| i128::from(value))
        .sum();
    assert!(
        chemistry.produced_milli > 0,
        "production never ran, so the field pins nothing"
    );
    assert_eq!(
        chemistry.produced_milli + chemistry.deposited_milli - chemistry.seeded_out_milli,
        total,
        "the C15.1 identity must hold to the milli-unit at the world level"
    );
    // The field spread: production is map-wide, so after diffusion more
    // than one cell holds mass.
    let occupied = chemistry
        .concentrations
        .iter()
        .filter(|&&value| value > 0)
        .count();
    assert!(occupied > 100, "the field never spread ({occupied} cells)");
}

#[test]
fn a_populated_field_survives_a_save_round_trip_with_the_same_future() {
    let mut world = World::new(chemistry_config(SEED ^ 0x1)).expect("world");
    for _ in 0..150 {
        world.step();
    }
    let saved = world.export_state();
    let mut restored = World::from_state(saved).expect("restores");
    assert_eq!(restored.state_checksum(), world.state_checksum());
    for _ in 0..100 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored field must advance identically"
    );
}

#[test]
fn the_disabled_section_saves_nothing_and_is_absent() {
    let mut config = chemistry_config(SEED ^ 0x2);
    config.chemistry.enabled = false;
    let mut world = World::new(config).expect("world");
    for _ in 0..50 {
        world.step();
    }
    assert!(world.export_state().chemistry.is_none());
}

#[test]
fn the_scaffold_arm_is_a_distinct_lineage_with_the_same_production_total() {
    let run = |radius: u32, contrast: u32| {
        let mut config = chemistry_config(SEED ^ 0x3);
        config.chemistry.scaffold_patch_radius_cells = radius;
        config.chemistry.scaffold_patch_contrast_q16 = contrast;
        let mut world = World::new(config).expect("world");
        for _ in 0..200 {
            world.step();
        }
        let state = world.export_state();
        let chemistry = state.chemistry.expect("section");
        (world.state_checksum(), chemistry.produced_milli)
    };
    let (neutral_hash, neutral_produced) = run(0, 65_536);
    let (scaffold_hash, scaffold_produced) = run(3, 4 * 65_536);
    assert_ne!(
        neutral_hash, scaffold_hash,
        "the scaffold must actually change the world"
    );
    assert_eq!(
        neutral_produced, scaffold_produced,
        "the scaffold redistributes production and must never add to it"
    );
}
