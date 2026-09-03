//! Phase 21's fact test (ADR-0036, C21.4): the elder eats first. Both
//! feeding passes visit organisms in entity-ID order and a cell's biomass
//! and substrate are taken in that order, so a newborn - the highest ID in
//! its cell - eats after every older occupant. Two organisms materialize
//! into one cell thirty ticks apart; substrate for one appetite is planted
//! and the elder takes its whole appetite while the newcomer gets the
//! remainder; then the same with biomass. A statement about the shipped
//! physics, pinned so the phase reasons from what runs.

use sim_core::{OriginMode, SimConfig, World, class_count, class_parameters};

const SEED: u64 = 0x0f21_5eed_0f21_5eed;
const ENERGY: i64 = 4_000;

fn scratch_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 24;
    config.cells_y = 24;
    config.initial_organisms = 0;
    config.max_entities = 400;
    config.origin.mode = OriginMode::Scratch;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.chemistry.enabled = true;
    config.chemistry.field_steps_per_tick = 2;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config.chemistry.mutation_q16 = 4_096;
    config.chemistry.production_milli_per_step = 20;
    config.chemistry.excretion_fraction_q16 = 32_768;
    config.chemistry.remains_fraction_q16 = 32_768;
    config.chemistry.consumption_fraction_q16 = 65_536;
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 2;
    config.transition.density_floor_milli = ENERGY;
    config.transition.organism_energy_milli = ENERGY;
    config.transition.max_organisms_per_event = 8;
    // Freeze the field so a planted amount is exactly what is found.
    config.chemistry.death_q16 = 0;
    config.chemistry.mutation_q16 = 0;
    config.chemistry.growth_rate_low_q16 = 0;
    config.chemistry.growth_rate_high_q16 = 0;
    config.chemistry.reaction_monomer_q16 = 0;
    config.chemistry.reaction_recycle_q16 = 0;
    config.chemistry.diffusion_q16 = 0;
    config.chemistry.production_milli_per_step = 0;
    config.validate().expect("validates");
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed");
}

fn eligible_class(config: &SimConfig) -> usize {
    (0..class_count(&config.chemistry))
        .find(|&class| class_parameters(&config.chemistry, class).aggregation_step >= 1)
        .expect("eligible class")
}

fn first_land(world: &World) -> usize {
    let terrain = world.terrain();
    (0..terrain.cell_count())
        .find(|&cell| terrain.capacity_milli[cell] > 0)
        .expect("land")
}

/// Plant one organism's worth of density in `cell`, empty its biomass, and
/// arm the persistence window so the next step materializes exactly one.
fn arm_one(world: World, cell: usize, config: &SimConfig) -> World {
    let classes = class_count(&config.chemistry);
    let slot = cell * classes + eligible_class(config);
    let mut state = world.export_state();
    state.microbial.as_mut().unwrap().densities[slot] += ENERGY;
    state.chemistry.as_mut().unwrap().produced_milli += i128::from(ENERGY);
    state.transition.as_mut().unwrap().persistence[slot] = 1;
    let biomass = state.biomass_milli[cell];
    state.biomass_milli[cell] = 0;
    state.ledger.initial_biomass_milli -= i128::from(biomass);
    World::from_state(state).expect("restores")
}

fn plant_substrate(world: World, cell: usize, monomer: i64) -> World {
    let mut state = world.export_state();
    let chemistry = state.chemistry.as_mut().unwrap();
    chemistry.produced_milli += i128::from(monomer);
    chemistry.concentrations[cell * sim_core::SUBSTRATE_COUNT + sim_core::S_MONOMER] += monomer;
    World::from_state(state).expect("restores")
}

fn plant_biomass(world: World, cell: usize, amount: i64) -> World {
    let mut state = world.export_state();
    state.biomass_milli[cell] += amount;
    state.ledger.initial_biomass_milli += i128::from(amount);
    World::from_state(state).expect("restores")
}

/// An elder (ID 1, aged thirty ticks) and a newcomer (ID 2, aged zero) in
/// one cell, the cell's biomass emptied, both hungry.
fn elder_and_newcomer(seed: u64) -> (World, usize) {
    let config = scratch_config(seed);
    let world = World::new(config).expect("world");
    let cell = first_land(&world);
    let mut world = arm_one(world, cell, &config);
    world.step();
    assert_eq!(world.population(), 1, "the elder materializes");
    for _ in 0..30 {
        world.step();
    }
    let mut world = arm_one(world, cell, &config);
    world.step();
    assert_eq!(world.population(), 2, "the newcomer materializes into the same cell");
    let elder = world.organism_detail(1).expect("elder");
    let newcomer = world.organism_detail(2).expect("newcomer");
    assert!(elder.age_ticks > newcomer.age_ticks, "the elder is older");
    // Both hungry, well under capacity (12,000), and in the same cell.
    assert!(elder.energy_milli < 12_000 - 4_000 && newcomer.energy_milli <= ENERGY);
    let mut state = world.export_state();
    state.biomass_milli[cell] = 0; // the newcomer's admission may have left none anyway
    let world = World::from_state(state).expect("restores");
    (world, cell)
}

#[test]
fn the_elder_takes_the_substrate_first_and_the_newcomer_gets_the_remainder() {
    let (world, cell) = elder_and_newcomer(SEED);
    let e1 = world.organism_detail(1).unwrap().energy_milli;
    let e2 = world.organism_detail(2).unwrap().energy_milli;
    // Substrate for one appetite and a bit: each wants up to ~1,000 per
    // tick of capability; 250 is exhausted by the first taker.
    let mut world = plant_substrate(world, cell, 250);
    world.step();
    let after = world.export_state();
    let base = cell * sim_core::SUBSTRATE_COUNT;
    assert_eq!(after.chemistry.as_ref().unwrap().concentrations[base + sim_core::S_MONOMER], 0, "exhausted");
    let d1 = world.organism_detail(1).unwrap().energy_milli - e1;
    let d2 = world.organism_detail(2).unwrap().energy_milli - e2;
    assert!(d1 > d2 + 20, "the elder gains more from the shared substrate: elder {d1}, newcomer {d2}");
    world.check_invariants().expect("identities");
}

#[test]
fn the_elder_takes_the_biomass_first_and_the_newcomer_gets_the_remainder() {
    let (world, cell) = elder_and_newcomer(SEED ^ 0x1);
    let e1 = world.organism_detail(1).unwrap().energy_milli;
    let e2 = world.organism_detail(2).unwrap().energy_milli;
    // Biomass for a little more than one intake; the intake rate per tick
    // is the capability (1,000 milli) times the tick length.
    let mut world = plant_biomass(world, cell, 120);
    world.step();
    let d1 = world.organism_detail(1).unwrap().energy_milli - e1;
    let d2 = world.organism_detail(2).unwrap().energy_milli - e2;
    assert!(d1 > d2, "the elder gains more from the shared biomass: elder {d1}, newcomer {d2}");
    world.check_invariants().expect("identities");
}
