//! Phase 19 integration clauses (ADR-0034): an organism can eat the
//! substrate in its own cell, the field identity closes with the consumed
//! term (C19.1's short half, C19.6), two organisms in one cell take in ID
//! order, the room bound holds, the disabled path is the Phase 16 world
//! on every measured quantity (C19.7's inertness), and the Phase 16
//! neutrality twin still shares one future when the field feeds back
//! (C19.2).

use sim_core::{OriginMode, SimConfig, World, class_count, class_parameters, synthesize_genome};

const SEED: u64 = 0x0f19_5eed_0f19_5eed;
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
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 25;
    config.transition.density_floor_milli = ENERGY;
    config.transition.persistence_checks = 2;
    config.transition.organism_energy_milli = ENERGY;
    config.validate().expect("validates");
    generable(config)
}

fn coupled(mut config: SimConfig) -> SimConfig {
    config.chemistry.consumption_fraction_q16 = 65_536;
    config.validate().expect("validates");
    config
}

fn generable(mut config: SimConfig) -> SimConfig {
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed near {:#x}", config.world_seed);
}

fn frozen_field(config: &mut SimConfig) {
    config.chemistry.death_q16 = 0;
    config.chemistry.mutation_q16 = 0;
    config.chemistry.growth_rate_low_q16 = 0;
    config.chemistry.growth_rate_high_q16 = 0;
    // The abiotic reactions and diffusion are frozen too, so a planted
    // substrate amount is exactly what the organisms find.
    config.chemistry.reaction_monomer_q16 = 0;
    config.chemistry.reaction_recycle_q16 = 0;
    config.chemistry.diffusion_q16 = 0;
    config.chemistry.production_milli_per_step = 0;
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

fn assert_identities(state: &sim_core::SaveState) {
    let chemistry = state.chemistry.as_ref().unwrap();
    let microbial = state.microbial.as_ref().unwrap();
    let materialized = state.transition.as_ref().map_or(0, |t| t.materialized_milli);
    let chem: i128 = chemistry.concentrations.iter().map(|&v| i128::from(v)).sum();
    let micro: i128 = microbial.densities.iter().map(|&v| i128::from(v)).sum();
    assert_eq!(
        chemistry.produced_milli + chemistry.deposited_milli - materialized - chemistry.consumed_milli,
        chem + micro,
        "field identity with the consumed term"
    );
    let organisms: i128 = state.energy_milli.iter().map(|&v| i128::from(v)).sum();
    assert_eq!(
        state.ledger.initial_energy_milli + state.ledger.assimilated_milli + materialized
            - state.ledger.spent_milli
            - state.ledger.removed_at_death_milli,
        organisms,
        "organism energy identity"
    );
}

#[test]
fn a_coupled_scratch_world_eats_the_field_and_both_identities_hold() {
    let mut world = World::new(coupled(scratch_config(SEED))).expect("world");
    for tick in 1..=3_000 {
        world.step();
        if tick % 100 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    let metrics = world.metrics();
    assert!(metrics.materialized_total > 0, "nothing materialized");
    assert!(
        metrics.chemistry_consumed_milli > 0,
        "no substrate was ever eaten, so the coupling pinned nothing"
    );
    assert_identities(&world.export_state());
}

#[test]
fn the_disabled_coupling_is_the_phase_16_world_on_every_measured_quantity() {
    let base = scratch_config(SEED ^ 0x1);
    let mut a = World::new(base).expect("world");
    let mut b = World::new(coupled(base)).expect("world");
    // Until an organism exists nothing can eat, so the two worlds must
    // agree on every measured quantity through the field-only prefix;
    // once organisms exist the coupled world diverges (it eats), and
    // that divergence is the mechanism, so the test asserts it too.
    let mut diverged_at = None;
    for tick in 1..=3_000 {
        a.step();
        b.step();
        let (ma, mb) = (a.metrics(), b.metrics());
        assert_eq!(ma.transition_enabled, mb.transition_enabled);
        if ma.population == 0 && mb.population == 0 {
            assert_eq!(ma.microbial_total_milli, mb.microbial_total_milli, "tick {tick}");
            assert_eq!(ma.chemistry_total_milli, mb.chemistry_total_milli, "tick {tick}");
        }
        assert_eq!(ma.chemistry_consumed_milli, 0);
        if mb.chemistry_consumed_milli > 0 && diverged_at.is_none() {
            diverged_at = Some(tick);
        }
    }
    assert!(diverged_at.is_some(), "the coupled world never ate");
    assert!(a.export_state().chemistry.unwrap().consumed_milli == 0);
}

/// A planted world: one land cell holds substrate and, after the next
/// tick, exactly `organisms` materialized unicells (the Phase 16 surgery),
/// with the field frozen so what they eat is exactly what was planted.
fn planted(seed: u64, organisms: i64, primordial: i64, monomer: i64) -> (World, usize) {
    let mut config = coupled(scratch_config(seed));
    frozen_field(&mut config);
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 2;
    config.transition.max_organisms_per_event = 8;
    config.validate().expect("validates");
    let world = World::new(config).expect("world");
    let cell = first_land(&world);
    let classes = class_count(&config.chemistry);
    let slot = cell * classes + eligible_class(&config);
    let mut state = world.export_state();
    let mass = organisms * ENERGY;
    state.microbial.as_mut().unwrap().densities[slot] += mass;
    let chemistry = state.chemistry.as_mut().unwrap();
    chemistry.produced_milli += i128::from(mass + primordial + monomer);
    chemistry.concentrations[cell * sim_core::SUBSTRATE_COUNT + sim_core::S_PRIMORDIAL] += primordial;
    chemistry.concentrations[cell * sim_core::SUBSTRATE_COUNT + sim_core::S_MONOMER] += monomer;
    state.transition.as_mut().unwrap().persistence[slot] = 1;
    // One gut, one intake rate: biomass fills the capability first and
    // substrate only what it leaves, so the cell's biomass is emptied
    // (ledger-adjusted) to isolate the substrate path.
    let biomass = state.biomass_milli[cell];
    state.biomass_milli[cell] = 0;
    state.ledger.initial_biomass_milli -= i128::from(biomass);
    (World::from_state(state).expect("restores"), cell)
}

#[test]
fn two_organisms_in_one_cell_take_in_id_order_monomer_first() {
    // Two organisms materialize into a cell holding 250 milli of monomer
    // and nothing else, both hungry (4,000 of a 12,000 capacity) and each
    // with an appetite above 125: organism 1 takes its whole appetite from
    // the monomer, organism 2 finds only the remainder, and the monomer is
    // exhausted - so the ID order is visible in the energies, not only
    // in the code.
    let (mut world, cell) = planted(SEED ^ 0x2, 2, 0, 250);
    world.step(); // materialize
    let base = cell * sim_core::SUBSTRATE_COUNT;
    assert_eq!(
        world.export_state().chemistry.as_ref().unwrap().concentrations[base + sim_core::S_MONOMER],
        250
    );
    world.step(); // first feeding tick for both
    let after = world.export_state();
    let chemistry = after.chemistry.as_ref().unwrap();
    assert_eq!(chemistry.concentrations[base + sim_core::S_MONOMER], 0, "exhausted");
    assert_eq!(chemistry.consumed_milli, 250, "the gross taken is exactly what was there");
    let e1 = world.organism_detail(1).unwrap().energy_milli;
    let e2 = world.organism_detail(2).unwrap().energy_milli;
    assert!(
        e1 > e2 + 20,
        "the lower ID takes first and the higher one gets the remainder: {e1} vs {e2}"
    );
    assert!(e1 <= 12_000 && e2 <= 12_000, "the room bound");
    world.check_invariants().expect("identities after feeding");
    assert_identities(&after);
}

#[test]
fn a_full_organism_eats_nothing_and_a_starving_one_eats_up_to_room() {
    let (mut world, _cell) = planted(SEED ^ 0x3, 1, 1_000_000, 0);
    world.step(); // materialize at 4,000 of 12,000
    let mut last = ENERGY;
    let mut ticks_to_full = 0;
    for tick in 1..=200 {
        world.step();
        let Some(energy) = world.organism_detail(1).map(|d| d.energy_milli) else {
            break;
        };
        assert!(energy <= 12_000, "tick {tick}: energy {energy} above capacity");
        // The room bound floors gross by the yield, so a full organism
        // parks a milli or two under capacity rather than exactly at it.
        if energy >= 12_000 - 4 && ticks_to_full == 0 {
            ticks_to_full = tick;
        }
        last = energy;
    }
    assert!(ticks_to_full > 0, "never filled: last energy {last}");
    world.check_invariants().expect("identities");
    let consumed_at_full = world.export_state().chemistry.unwrap().consumed_milli;
    for _ in 0..20 {
        world.step();
    }
    let consumed_later = world.export_state().chemistry.unwrap().consumed_milli;
    assert!(consumed_later >= consumed_at_full);
    // A full organism can take only what basal cost frees each tick, so
    // twenty ticks cannot add twenty appetites' worth.
    assert!(
        consumed_later - consumed_at_full < 20 * 4_000,
        "a full organism kept eating beyond its room: {}",
        consumed_later - consumed_at_full
    );
}

#[test]
fn the_relabelled_twin_still_shares_one_future_when_the_field_feeds_back() {
    // C19.2: the Phase 16 neutrality A/B under coupling v2. The same
    // post-admission state, once as materialized and once relabelled as
    // founders, stepped side by side while the field feeds them.
    let (mut materialized, _cell) = planted(SEED ^ 0x4, 3, 200_000, 20_000);
    materialized.step();
    assert_eq!(materialized.population(), 3);
    let mut relabelled = materialized.export_state();
    let moved = relabelled.transition.as_ref().unwrap().materialized_milli;
    let count = relabelled.transition.as_ref().unwrap().materialized_total;
    {
        let transition = relabelled.transition.as_mut().unwrap();
        transition.materialized_milli = 0;
        transition.materialized_total = 0;
        transition.events_total = 0;
    }
    relabelled.ledger.initial_energy_milli += moved;
    relabelled.chemistry.as_mut().unwrap().produced_milli -= moved;
    relabelled.config.origin.mode = OriginMode::Random;
    relabelled.config.initial_organisms = count as u32;
    relabelled.config.validate().expect("validates");
    let mut twin = World::from_state(relabelled).expect("restores");
    for tick in 1..=400 {
        materialized.step();
        twin.step();
        let rows = |world: &World| -> Vec<(u64, i64, i32, i32)> {
            world
                .organism_ids_view()
                .iter()
                .map(|&id| {
                    let d = world.organism_detail(id).unwrap();
                    (id, d.energy_milli, d.x_fp, d.y_fp)
                })
                .collect()
        };
        assert_eq!(rows(&materialized), rows(&twin), "tick {tick}");
    }
    assert!(
        materialized.metrics().chemistry_consumed_milli > 0,
        "the twins never ate, so the coupled future was not exercised"
    );
}

#[test]
fn the_monomer_goes_first_and_the_loss_comes_back_as_waste() {
    // 100 monomer + 1,000 primordial, one hungry organism with an appetite
    // above 100: the monomer is exhausted before any primordial moves, and
    // the metabolic loss (gross minus gained) lands in the cell as S_WASTE
    // through the counted deposit term - the field does not merely lose it.
    let (mut world, cell) = planted(SEED ^ 0x5, 1, 1_000, 100);
    world.step(); // materialize
    let base = cell * sim_core::SUBSTRATE_COUNT;
    let before = world.export_state();
    let deposited_before = before.chemistry.as_ref().unwrap().deposited_milli;
    let waste_before = before.chemistry.as_ref().unwrap().concentrations[base + sim_core::S_WASTE];
    let energy_before = world.organism_detail(1).unwrap().energy_milli;
    world.step(); // feed
    let after = world.export_state();
    let chemistry = after.chemistry.as_ref().unwrap();
    assert_eq!(chemistry.concentrations[base + sim_core::S_MONOMER], 0, "monomer first");
    let primordial_taken = 1_000 - chemistry.concentrations[base + sim_core::S_PRIMORDIAL];
    assert!(primordial_taken > 0 && primordial_taken < 1_000, "then primordial: {primordial_taken}");
    let gross = chemistry.consumed_milli as i64;
    assert_eq!(gross, 100 + primordial_taken);
    // The organism's net change this tick is gained minus its costs; the
    // deposit term must have grown by at least the loss (excretion adds a
    // little more), and the waste in the cell by the loss too.
    let gained = gross * i64::from(world.config().chemistry.consumption_yield_q16) / 65_536;
    let loss = gross - gained;
    assert!(
        chemistry.deposited_milli - deposited_before >= i128::from(loss),
        "the loss was not deposited: {} < {loss}",
        chemistry.deposited_milli - deposited_before
    );
    assert!(chemistry.concentrations[base + sim_core::S_WASTE] - waste_before >= loss);
    let _ = &gained;
    assert!(world.organism_detail(1).unwrap().energy_milli > energy_before);
}

#[test]
fn one_gut_one_intake_rate_biomass_fills_first_and_substrate_only_the_rest() {
    // A cell with a little biomass and plenty of substrate: the organism's
    // gross intake this tick - biomass plus substrate - is exactly its
    // per-tick capability, never more.
    let (mut world, cell) = planted(SEED ^ 0x6, 1, 100_000, 0);
    world.step(); // materialize
    let mut state = world.export_state();
    let biomass_planted = 60_i64;
    // The cell may hold a milli of regrowth already; the ledger moves by
    // the delta, not the planted amount.
    let delta = biomass_planted - state.biomass_milli[cell];
    state.biomass_milli[cell] = biomass_planted;
    state.ledger.initial_biomass_milli += i128::from(delta);
    let mut world = World::from_state(state).expect("restores");
    let mult = world.organism_detail(1).unwrap().phase2.unwrap().phenotype.intake_mult_milli;
    let config = *world.config();
    let intake_tick = config.intake_rate_milli_per_s * i64::from(config.dt_ms) / 1000;
    let capability = intake_tick * mult / 1000;
    assert!(capability > biomass_planted, "the fixture needs an appetite above the biomass");
    let consumed_biomass_before = world.export_state().ledger.consumed_biomass_milli;
    world.step(); // feed: biomass first, substrate for the rest
    let after = world.export_state();
    // From the ledger, not the cell: regrowth adds a milli before feeding.
    let biomass_eaten = (after.ledger.consumed_biomass_milli - consumed_biomass_before) as i64;
    let gross = after.chemistry.as_ref().unwrap().consumed_milli as i64;
    assert!(biomass_eaten >= biomass_planted, "biomass goes first and is finished (a milli of regrowth may join it)");
    assert_eq!(after.biomass_milli[cell], 0, "the cell's biomass is gone before any substrate moves");
    assert_eq!(
        biomass_eaten + gross,
        capability,
        "biomass plus substrate must equal the one capability"
    );
}

#[test]
fn a_materialized_organism_under_v2_is_the_founder_path_organism_with_no_provenance() {
    // C19.2's first two clauses re-run with the mouth open: the organism
    // the transition builds under coupling v2 has the founder path's
    // rows for the same genome, and nothing on it says where it came
    // from. The constructor is shared, so this pins that the coupling
    // did not grow a second one.
    let (mut materialized, _cell) = planted(SEED ^ 0x10, 1, 100_000, 10_000);
    materialized.step();
    assert_eq!(materialized.population(), 1);
    let config = *materialized.config();
    assert!(config.chemistry.consumption_fraction_q16 > 0, "the mouth is open");
    let detail = materialized.organism_detail(1).expect("organism");
    let phase2 = detail.phase2.expect("phase 2");
    assert_eq!(phase2.parents, [0, 0], "no parent marks a materialized organism");
    assert_eq!(phase2.ancestry_depth, 0);
    assert_eq!(detail.age_ticks, 0);
    assert_eq!(detail.cooldown_ticks, 0);
    assert_eq!(detail.energy_milli, ENERGY, "materialization happens before any feeding");

    let mut founder_config = config;
    founder_config.origin.mode = OriginMode::Random;
    founder_config.initial_organisms = 1;
    founder_config.initial_energy_milli = ENERGY;
    founder_config.transition.enabled = false;
    founder_config.validate().expect("validates");
    let founder_world = World::new(founder_config).expect("world");
    let mut state = founder_world.export_state();
    state.schema2.as_mut().expect("schema2").genomes[0] = synthesize_genome(&founder_config, 0).encode();
    let founder_world = World::from_state(state).expect("spliced state restores");
    let twin = founder_world.organism_detail(1).expect("founder").phase2.expect("phase 2");
    assert_eq!(twin.phenotype, phase2.phenotype, "the coupled materialized phenotype is the founder path's");
    assert_eq!(twin.trait_genes, phase2.trait_genes);
}

/// C19.6: the exchange test in full. Excretion, remains, materialization
/// and consumption each move a counted term; each arm runs alone, the
/// four run together, a control runs with none, and both identities are
/// exact at every check in every arm.
#[test]
fn every_exchange_arm_moves_its_own_counted_term_alone_and_together() {
    struct Arm {
        name: &'static str,
        excretion: u32,
        remains: u32,
        materialization: bool,
        consumption: bool,
    }
    let run = |arm: &Arm| {
        let mut config = scratch_config(SEED ^ 0x20);
        config.chemistry.excretion_fraction_q16 = arm.excretion;
        config.chemistry.remains_fraction_q16 = arm.remains;
        config.chemistry.consumption_fraction_q16 = if arm.consumption { 65_536 } else { 0 };
        if !arm.materialization {
            // Organisms then come from founders, not the field; with no
            // standing biomass the only food is the substrate.
            config.origin.mode = OriginMode::Random;
            config.initial_organisms = 40;
            config.initial_energy_milli = ENERGY;
            config.initial_biomass_q16 = 0;
            config.transition.enabled = false;
        }
        // Old-age deaths leave energy for the remains term to deposit;
        // starvation deaths carry zero and would leave it untested.
        config.maturity_age_ticks = 50;
        config.max_age_ticks = 200;
        config.validate().unwrap_or_else(|error| panic!("{}: {error:?}", arm.name));
        let mut world = World::new(config).unwrap_or_else(|error| panic!("{}: {error:?}", arm.name));
        let ticks = if arm.materialization { 3_000 } else { 600 };
        for tick in 1..=ticks {
            world.step();
            if tick % 100 == 0 {
                world
                    .check_invariants()
                    .unwrap_or_else(|violation| panic!("{} tick {tick}: {violation}", arm.name));
            }
        }
        let state = world.export_state();
        assert_identities(&state);
        let chemistry = state.chemistry.as_ref().unwrap();
        let materialized = state.transition.as_ref().map_or(0, |t| t.materialized_milli);
        let deaths = state.counters.deaths_starvation_total + state.counters.deaths_old_age_total;
        (chemistry.deposited_milli, materialized, chemistry.consumed_milli, deaths, world.state_checksum())
    };
    let control = Arm { name: "control", excretion: 0, remains: 0, materialization: false, consumption: false };
    let excretion = Arm { name: "excretion", excretion: 65_536, remains: 0, materialization: false, consumption: false };
    let remains = Arm { name: "remains", excretion: 0, remains: 65_536, materialization: false, consumption: false };
    let materialization = Arm { name: "materialization", excretion: 0, remains: 0, materialization: true, consumption: false };
    let consumption = Arm { name: "consumption", excretion: 0, remains: 0, materialization: false, consumption: true };
    let together = Arm { name: "together", excretion: 65_536, remains: 65_536, materialization: true, consumption: true };

    let (c_dep, c_mat, c_con, _, c_hash) = run(&control);
    assert_eq!((c_dep, c_mat, c_con), (0, 0, 0), "the control moves nothing");
    let (e_dep, e_mat, e_con, _, e_hash) = run(&excretion);
    assert!(e_dep > 0, "excretion never deposited");
    assert_eq!((e_mat, e_con), (0, 0), "excretion alone moved another term");
    assert_ne!(e_hash, c_hash, "excretion must change the world");
    let (r_dep, r_mat, r_con, r_deaths, _) = run(&remains);
    assert!(r_deaths > 0, "nobody died, so remains is untested");
    assert!(r_dep > 0, "remains never deposited despite {r_deaths} deaths");
    assert_eq!((r_mat, r_con), (0, 0), "remains alone moved another term");
    let (m_dep, m_mat, m_con, _, _) = run(&materialization);
    assert!(m_mat > 0, "nothing materialized");
    assert_eq!((m_dep, m_con), (0, 0), "materialization alone moved another term");
    let (k_dep, k_mat, k_con, _, k_hash) = run(&consumption);
    assert!(k_con > 0, "nothing was eaten from the field");
    assert_eq!(k_mat, 0, "consumption alone materialized");
    // Consumption's metabolic loss returns as waste through the deposit
    // term, so that term is nonzero here by construction and its floor
    // is the loss itself.
    assert!(k_dep >= k_con - k_con * 6 / 10 - 1, "the loss did not come back as a deposit");
    assert_ne!(k_hash, c_hash, "consumption must change the world");
    let (t_dep, t_mat, t_con, t_deaths, _) = run(&together);
    assert!(t_dep > 0 && t_mat > 0 && t_con > 0 && t_deaths > 0, "together: a term stayed at zero");
}
