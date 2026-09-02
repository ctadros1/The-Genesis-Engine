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

// --- increment 2: the microbial field, in-world ------------------------------

fn microbial_config(seed: u64) -> SimConfig {
    let mut config = chemistry_config(seed);
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    // The default mutation rate (0.001 per neighbour) truncates to zero
    // below 993 milli of density; abiogenesis seeds at most 1000 with
    // death running first, so the default would leave the mutation term
    // untested. Still inside the Q16/8 validation cap.
    config.chemistry.mutation_q16 = 4_096;
    config
}

#[test]
fn a_microbial_world_runs_and_the_joint_identity_holds() {
    let mut world = World::new(microbial_config(SEED ^ 0x10)).expect("world");
    for tick in 1..=600 {
        world.step();
        if tick % 100 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    let state = world.export_state();
    let chemistry = state.chemistry.as_ref().expect("chemistry section");
    let microbial = state.microbial.as_ref().expect("microbial section");
    let chem_total: i128 = chemistry
        .concentrations
        .iter()
        .map(|&value| i128::from(value))
        .sum();
    let microbial_total: i128 = microbial
        .densities
        .iter()
        .map(|&value| i128::from(value))
        .sum();
    // Non-vacuity: every term of the joint identity must actually have
    // moved, or the equality below pins nothing about the microbial half.
    assert!(
        chemistry.abiogenesis_fired_total > 0,
        "abiogenesis never fired, so no density ever existed"
    );
    assert!(microbial_total > 0, "no microbial density at the end");
    assert!(microbial.grown_milli_total > 0, "growth never ran");
    assert!(microbial.died_milli_total > 0, "death never ran");
    assert!(microbial.mutated_milli_total > 0, "mutation never flowed");
    assert_eq!(
        chemistry.produced_milli + chemistry.deposited_milli,
        chem_total + microbial_total,
        "the joint C15.1 identity must hold to the milli-unit at the world level"
    );
}

#[test]
fn a_populated_microbial_field_survives_a_save_round_trip_with_the_same_future() {
    let mut world = World::new(microbial_config(SEED ^ 0x11)).expect("world");
    for _ in 0..300 {
        world.step();
    }
    let saved = world.export_state();
    assert!(
        saved
            .microbial
            .as_ref()
            .is_some_and(|microbial| microbial.densities.iter().any(|&value| value > 0)),
        "the saved field is empty, so the round trip would prove nothing"
    );
    // Mutation verification: the checksum equality below is only evidence
    // if the checksum actually covers the densities. Move one milli
    // between slots - conserving, so the restore's C15.1 invariant holds
    // and only the hash can object - and the restored world must hash
    // differently.
    let mut perturbed = saved.clone();
    let slot = perturbed
        .microbial
        .as_ref()
        .unwrap()
        .densities
        .iter()
        .position(|&value| value > 0)
        .unwrap();
    let densities = &mut perturbed.microbial.as_mut().unwrap().densities;
    densities[slot] -= 1;
    let other = (slot + 1) % densities.len();
    densities[other] += 1;
    let perturbed_world = World::from_state(perturbed).expect("perturbed restores");
    assert_ne!(
        perturbed_world.state_checksum(),
        world.state_checksum(),
        "a perturbed density hashed identically, so the checksum does not cover it"
    );
    let mut restored = World::from_state(saved).expect("restores");
    assert_eq!(restored.state_checksum(), world.state_checksum());
    for _ in 0..100 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored microbial field must advance identically"
    );
}

// --- increment 3: organism-to-field coupling (C15.6, v1 half) ---------------

/// The exchange test for the coupling this phase ships: excretion and
/// remains arrive in the field as counted deposits, the conservation
/// identity holds with organisms attached, and a world with the coupling
/// off deposits nothing. Materialization (field-to-organism) is Phase
/// 16's half, deferred by ADR-0031 and named in the findings.
#[test]
fn the_coupling_deposits_through_the_ledger_and_the_identity_holds() {
    let run = |excretion: u32, remains: u32| {
        let mut config = chemistry_config(SEED ^ 0x21);
        config.chemistry.excretion_fraction_q16 = excretion;
        config.chemistry.remains_fraction_q16 = remains;
        // Old-age deaths leave energy behind for the remains term to
        // deposit; starvation deaths carry zero and would leave that
        // term untested. Maturity must stay below the cutoff.
        config.maturity_age_ticks = 50;
        config.max_age_ticks = 200;
        let mut world = World::new(config).expect("world");
        for tick in 1..=400 {
            world.step();
            if tick % 100 == 0 {
                world
                    .check_invariants()
                    .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
            }
        }
        let state = world.export_state();
        let chemistry = state.chemistry.as_ref().expect("section");
        let total: i128 = chemistry
            .concentrations
            .iter()
            .map(|&value| i128::from(value))
            .sum();
        assert_eq!(
            chemistry.produced_milli + chemistry.deposited_milli - chemistry.seeded_out_milli,
            total,
            "the C15.1 identity must hold with the coupling at ({excretion}, {remains})"
        );
        let deaths =
            state.counters.deaths_starvation_total + state.counters.deaths_old_age_total;
        (world.state_checksum(), chemistry.deposited_milli, deaths)
    };
    let (control_hash, control_deposited, _) = run(0, 0);
    let (excretion_hash, excretion_deposited, _) = run(65_536, 0);
    let (_, remains_deposited, remains_deaths) = run(0, 65_536);
    assert_eq!(
        control_deposited, 0,
        "a world without the coupling must deposit nothing"
    );
    assert!(
        excretion_deposited > 0,
        "excretion never deposited, so the term is untested"
    );
    assert!(
        remains_deaths > 0,
        "nobody died, so the remains term is untested"
    );
    assert!(
        remains_deposited > 0,
        "remains never deposited despite {remains_deaths} deaths"
    );
    assert_ne!(
        control_hash, excretion_hash,
        "the coupling must actually change the world"
    );
}

#[test]
fn the_disabled_microbial_section_saves_nothing_and_is_absent() {
    let mut config = microbial_config(SEED ^ 0x12);
    config.chemistry.microbial_enabled = false;
    config.chemistry.abiogenesis_enabled = false;
    let mut world = World::new(config).expect("world");
    for _ in 0..50 {
        world.step();
    }
    let state = world.export_state();
    assert!(state.chemistry.is_some(), "chemistry itself stays on");
    assert!(state.microbial.is_none());
}
