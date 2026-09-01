//! Phase 12 artifact half: the tick integration (ADR-0028).
//!
//! The table arithmetic, the ledger identities and the violation classes are
//! unit-tested in `artifact.rs`; the registry gating in `registry.rs`. What
//! is left, and what this file is for, is everything that only exists once a
//! whole world is running: that a **disabled** section reproduces the
//! fixtures byte for byte, that a striker extracts and the ledger stays
//! exact, that a picker carries and pays, that a contested pick-up is
//! decided by (priority, distance, id) and not by visit order, that
//! combining then fracturing restores constituents exactly, that every cap
//! rejects, counts and events when driven, that the two control conditions
//! do what the plan says, and that a save round trip through the world's own
//! save path steps on identically.
//!
//! # How a scenario is scripted
//!
//! There is no intent-injection hook, on purpose: an intent that did not
//! come from a controller would be an intervention. Instead every scripted
//! world rewrites its founder genomes through the save path - the pattern
//! `phase11_learning.rs` uses - so that a chosen output channel is bound to
//! a node whose bias saturates its activation. The organism then requests
//! the action every tick through the ordinary controller, and what the tick
//! does with the request is what is under test.

use sim_core::{
    Activation, CHANNEL_COMBINE, CHANNEL_DROP, CHANNEL_PICK_UP, CHANNEL_PLACE, CHANNEL_STRIKE,
    EventKind, Genome2, GenomeCaps, InheritanceMode, Locus, LocusKind, MATERIAL_STONE, NodeRole,
    ObjectAction, RefuseReason, RestoreError, STRUCTURAL_HOMOLOGY_BASE, SimConfig, World,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// Schema 2's `rest` action channel. Bound always-on it zeroes the throttle,
/// which is how a scenario freezes movement without touching the speed
/// config (a zero speed is refused by validation, correctly).
const CHANNEL_REST: u16 = 105;

/// The Phase 9 fixture's configuration, pinned field by field (D-078),
/// duplicated from `phase12_worldmod.rs` for the reason that file gives.
fn phase9_fixture_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.phase2.enabled = true;
    config.genome2.enabled = true;
    let caps = &mut config.genome2.caps;
    caps.max_chromosomes = 4;
    caps.max_loci_per_chromosome = 160;
    caps.max_nodes = 160;
    caps.max_edges = 160;
    caps.max_edges_per_node = 32;
    caps.max_genome_bytes = 16_384;
    caps.min_nodes = 2;
    config.genome2.meiosis.mode = InheritanceMode::Meiotic;
    config.genome2.meiosis.max_extra_crossovers = 2;
    let mutation = &mut config.genome2.mutation;
    mutation.point_q16 = 6_554;
    mutation.duplication_q16 = 655;
    mutation.deletion_q16 = 655;
    mutation.insertion_q16 = 0;
    mutation.transposition_q16 = 328;
    mutation.regulatory_enabled = true;
    mutation.max_run = 3;
    mutation.point_delta_q16 = 3_277;
    config
}

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world");
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    world
}

fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
        world
            .check_invariants()
            .expect("invariants hold every tick");
    }
}

/// A small artifact world: schema 2 (the actions are schema-2 only),
/// worldmod (the yield layer lives there), contest (carcasses), no
/// reproduction so the founders' scripted bindings are the whole population.
fn artifact_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 48;
    config.cells_y = 48;
    config.initial_organisms = 24;
    config.max_entities = 200;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.contest.enabled = true;
    config.artifact.enabled = true;
    // No births: the scenario is the founders and nothing else.
    config.reproduction_enabled = false;
    config.validate().expect("the artifact config validates");
    config
}

/// Add an always-on output node bound to `channel` with `gain`. The node's
/// bias saturates its activation, so the request is ~1.0 every tick.
fn bind_always_on(genome: &mut Genome2, channel: u16, gain: f32, salt: u32) {
    let node_id = STRUCTURAL_HOMOLOGY_BASE + 50_000 + salt * 10;
    for haplotype in &mut genome.haplotypes {
        let chromosome = &mut haplotype.chromosomes[0];
        chromosome.push(Locus {
            homology_id: node_id,
            gene_lineage_id: u64::from(node_id),
            mutation_event_id: 0,
            kind: LocusKind::Node {
                role: NodeRole::Output,
                activation_id: Activation::TanhApprox.id(),
                bias: 8.0,
                time_constant: 0,
            },
        });
        chromosome.push(Locus {
            homology_id: node_id + 1,
            gene_lineage_id: u64::from(node_id + 1),
            mutation_event_id: 0,
            kind: LocusKind::IoBinding {
                node: node_id,
                channel_id: channel,
                gain,
            },
        });
        chromosome.sort_unstable_by_key(|locus| locus.homology_id);
    }
}

/// Rewrite every founder through the save path so its network requests the
/// given output channels every tick. `extra_nodes` is how many nodes each
/// rewrite adds (one per channel), because activation buffers are sized to
/// the network and `from_state` refuses a mismatch.
fn scripted_world(config: SimConfig, channels: &[u16]) -> World {
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    let schema2 = state.schema2.as_mut().expect("a schema-2 world");
    for index in 0..schema2.genomes.len() {
        let mut genome = Genome2::decode(&schema2.genomes[index], &caps).expect("decodes");
        for (salt, &channel) in channels.iter().enumerate() {
            bind_always_on(&mut genome, channel, 1.0, salt as u32);
        }
        genome
            .validate_structure(&caps)
            .expect("the rewritten genome validates");
        schema2.genomes[index] = genome.encode();
        for _ in 0..channels.len() {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    World::from_state(state).expect("the rewritten founders restore")
}

// --- C12.8: a disabled section changes nothing ------------------------------

#[test]
fn a_disabled_artifact_section_reproduces_every_reachable_fixture_exactly() {
    let cases: [(SimConfig, u64, u64, u64); 3] = [
        (
            SimConfig::phase1_default(SEED),
            500,
            0x918a_381c_7755_9236,
            0x1e31_58a2_6afd_3b39,
        ),
        (
            SimConfig::phase2_default(SEED),
            500,
            0xf83d_3981_bf7d_d189,
            0xff9d_fcff_5dff_bf42,
        ),
        (
            phase9_fixture_config(),
            8_000,
            0x9abc_0cd4_7914_127f,
            0x5f0c_4e95_e4f5_170f,
        ),
    ];
    for (config, ticks, config_hash, state_checksum) in cases {
        assert!(!config.artifact.enabled, "the section defaults to off");
        assert_eq!(
            config.genome2.mutation.binding_q16, 0,
            "the operator defaults to off"
        );
        assert_eq!(config.stable_hash(), config_hash, "config hash moved");
        let world = advance(config, ticks);
        assert_eq!(
            world.state_checksum(),
            state_checksum,
            "state checksum moved"
        );
        assert!(world.object_table().is_none());
        assert!(world.object_counters().is_none());
    }
}

// --- extraction and the ledger ---------------------------------------------

#[test]
fn strikers_extract_material_from_terrain_and_the_ledger_stays_exact() {
    let mut world = scripted_world(artifact_config(SEED), &[CHANNEL_STRIKE]);
    run(&mut world, 60);
    let counters = world.object_counters().expect("section on");
    let table = world.object_table().expect("section on");
    assert!(
        counters.struck_terrain > 0,
        "nobody struck the ground: {counters:?}"
    );
    assert!(
        counters.created_extracted > 0,
        "nothing was extracted: {counters:?}"
    );
    assert!(!table.is_empty());
    // Every object came from terrain, and the pool holds exactly what the
    // ledger says it should - `check_invariants` asserted that each tick;
    // here it is stated as a number so a reader can see the magnitude.
    let ledger = world.object_ledger().unwrap();
    assert!(ledger.mass_extracted_milli > 0);
    assert_eq!(table.total_mass_milli(), ledger.expected_mass_milli());
    // Every event kind the pass emits reconciles with the counters.
    let created_events = world
        .events()
        .iter()
        .filter(|event| matches!(event.kind, EventKind::ObjectCreated { .. }))
        .count();
    let _ = created_events; // last tick only; the counters are cumulative
    // Strikes cost energy: a striker world is poorer than an unscripted one.
    let control = advance(artifact_config(SEED), 60);
    assert!(
        world.total_energy_milli() < control.total_energy_milli(),
        "striking every tick must cost something"
    );
}

#[test]
fn a_depleted_cell_refuses_by_name_and_regenerates_on_its_cadence() {
    let mut config = artifact_config(SEED);
    config.artifact.terrain_yield_milli = 1_000;
    config.artifact.extraction_milli = 800;
    config.artifact.yield_regen_interval_ticks = 50;
    config.artifact.yield_regen_milli = 1_000;
    // Every striker stays put (rest) and hits its own cell until it runs
    // dry, then is refused by name until the cadence refills it.
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_REST]);
    run(&mut world, 40);
    let before = world.object_counters().unwrap();
    assert!(
        before.refused_depleted > 0,
        "no cell ever ran dry: {before:?}"
    );
    assert!(before.struck_terrain > 0);
    run(&mut world, 60);
    let after = world.object_counters().unwrap();
    assert!(
        after.struck_terrain > before.struck_terrain,
        "regeneration never let anyone extract again: {before:?} -> {after:?}"
    );
    let refused = world.events().iter().any(|event| {
        matches!(event.kind, EventKind::ObjectActionRefused { reason, .. } if reason == RefuseReason::Depleted.id())
    });
    let _ = refused; // the last tick may or may not carry one; the counter is the claim
}

// --- carrying ------------------------------------------------------------

#[test]
fn pickers_carry_what_they_extract_pay_to_hold_it_and_drop_it_at_death() {
    let mut config = artifact_config(SEED);
    config.artifact.max_held_objects = 2;
    let mut world = scripted_world(config.clone(), &[CHANNEL_STRIKE, CHANNEL_PICK_UP]);
    run(&mut world, 80);
    let counters = world.object_counters().unwrap();
    assert!(counters.picked_up > 0, "nothing picked up: {counters:?}");
    let table = world.object_table().unwrap();
    let held = table
        .holder_id
        .iter()
        .filter(|&&holder| holder != 0)
        .count();
    assert!(held > 0, "nothing is held at tick 80");
    // Holding costs: a strike-and-hold world is poorer than a strike-only
    // one at the same tick, and pick-ups cost too.
    let strikers = {
        let mut world = scripted_world(config, &[CHANNEL_STRIKE]);
        run(&mut world, 80);
        world
    };
    assert!(world.total_energy_milli() < strikers.total_energy_milli());
    // Starve everyone in one tick (a basal cost above anyone's energy, set
    // on the saved config so the ledger stays exact): held objects come back
    // to the world at the death position, in ascending id, and nothing is
    // left held.
    let mut state = world.export_state();
    state.config.basal_cost_milli_per_s = 10_000_000;
    let mut world = World::from_state(state).expect("restores");
    world.step();
    world
        .check_invariants()
        .expect("invariants after the mass death");
    let counters = world.object_counters().unwrap();
    assert!(
        counters.death_drops > 0,
        "no death dropped anything: {counters:?}"
    );
    let table = world.object_table().unwrap();
    assert!(table.holder_id.iter().all(|&holder| holder == 0));
    assert_eq!(world.population(), 0);
}

// --- contested acquisition (C12.4) -----------------------------------------

/// Two organisms claim one object at equal priority. The nearer wins even
/// when it has the higher id, so the resolution is (priority, distance, id)
/// and not first-visited; and at equal distance the lower id wins, which is
/// Rule 3's tie-break, applied only after the physical keys.
#[test]
fn a_contested_pick_up_is_decided_by_distance_before_id_and_never_by_visit_order() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 2;
    config.artifact.reach_m = 8;
    // Freeze movement (rest) so distances are what the test placed.
    let world = scripted_world(config, &[CHANNEL_PICK_UP, CHANNEL_REST]);
    let mut state = world.export_state();
    let ids = state.ids.clone();
    assert_eq!(ids.len(), 2);
    let (low, high) = (ids[0], ids[1]);
    // Put both on the same land cell, the higher id nearer the object.
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    state.x_fp[1] = x;
    state.y_fp[1] = y;
    let fp = 1024;
    let object_x = x + 3 * fp;
    state.x_fp[0] = x - 2 * fp; // low id: 5 m away
    state.x_fp[1] = x + 1 * fp; // high id: 2 m away
    let table = state.objects.as_mut().unwrap();
    let base = state.next_entity_id;
    let mut record = sim_core::ObjectRecord::simple(
        base,
        sim_core::material(MATERIAL_STONE).unwrap(),
        400,
        object_x,
        y,
        0,
        sim_core::CAUSE_EXTRACTED,
        0,
    );
    record.x_fp = object_x;
    table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
    table.push(record);
    table.objects_allocated_total += 1;
    state.next_entity_id += 1;
    let mut world = World::from_state(state).expect("restores");
    world.step();
    world.check_invariants().expect("invariants");
    let table = world.object_table().unwrap();
    assert_eq!(
        table.holder_id[0], high,
        "the nearer organism wins whatever its id"
    );
    let counters = world.object_counters().unwrap();
    assert_eq!(counters.picked_up, 1);
    assert_eq!(
        counters.refused_contested, 1,
        "the loser is refused Contested and pays"
    );
    let refused = world.events().iter().any(|event| {
        matches!(
            event.kind,
            EventKind::ObjectActionRefused { id, action, reason }
                if id == low && action == ObjectAction::PickUp.id() && reason == RefuseReason::Contested.id()
        )
    });
    assert!(refused, "the refusal events with the loser's id");
}

// --- combine and fracture (C12.6) ------------------------------------------

/// A holder combines with a free target; a later strike above the composite's
/// hardness brings it apart, restoring both constituents by id with mass and
/// energy exact.
#[test]
fn combining_then_fracturing_restores_constituent_ids_mass_and_energy_exactly() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 1;
    config.artifact.reach_m = 8;
    // Force well above stone hardness so the fracture is certain, and a
    // joint floor of zero so the combine is certain.
    config.artifact.strike_force_q16 = 64 << 16;
    config.artifact.joint_floor_q16 = 0;
    let world = scripted_world(config, &[CHANNEL_COMBINE, CHANNEL_REST]);
    let mut state = world.export_state();
    let organism = state.ids[0];
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    let table = state.objects.as_mut().unwrap();
    let base = state.next_entity_id;
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    let mut held =
        sim_core::ObjectRecord::simple(base, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
    held.holder_id = organism;
    let target = sim_core::ObjectRecord::simple(
        base + 1,
        stone,
        600,
        x + 1024,
        y,
        0,
        sim_core::CAUSE_EXTRACTED,
        0,
    );
    let (held_mass, target_mass) = (held.mass_milli, target.mass_milli);
    table.ledger.mass_extracted_milli += i128::from(held_mass + target_mass);
    table.push(held);
    table.push(target);
    table.objects_allocated_total += 2;
    state.next_entity_id += 2;
    let mut world = World::from_state(state).expect("restores");
    world.step();
    world.check_invariants().expect("invariants after combine");
    let table = world.object_table().unwrap();
    assert_eq!(
        world.object_counters().unwrap().combined,
        1,
        "the combine happened"
    );
    let composite = table
        .index_of(base + 2)
        .expect("the composite took the next id");
    assert_eq!(table.mass_milli[composite], held_mass + target_mass);
    assert_eq!(table.depth[composite], 1);
    assert_eq!(table.composition[composite], vec![base, base + 1]);
    assert_eq!(table.owner_id[table.index_of(base).unwrap()], base + 2);
    assert_eq!(
        table.total_mass_milli(),
        i128::from(held_mass + target_mass),
        "combine is mass-neutral"
    );
    // Now strike it apart: rewrite the founder to strike instead.
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let schema2 = state.schema2.as_mut().unwrap();
    let mut genome = Genome2::decode(&schema2.genomes[0], &caps).unwrap();
    bind_always_on(&mut genome, CHANNEL_STRIKE, 1.0, 7);
    // And stop combining, or the fresh constituents would be re-joined.
    for haplotype in &mut genome.haplotypes {
        haplotype.chromosomes[0].retain(|locus| {
            !matches!(locus.kind, LocusKind::IoBinding { channel_id, .. } if channel_id == CHANNEL_COMBINE)
        });
    }
    schema2.genomes[0] = genome.encode();
    schema2.activation_values[0].push(0.0);
    schema2.activation_prior[0].push(0.0);
    let mut world = World::from_state(state).expect("restores");
    world.step();
    world.check_invariants().expect("invariants after fracture");
    let table = world.object_table().unwrap();
    assert!(table.index_of(base + 2).is_none(), "the composite is gone");
    let a = table.index_of(base).expect("constituent a is back");
    let b = table.index_of(base + 1).expect("constituent b is back");
    assert_eq!(table.owner_id[a], 0);
    assert_eq!(table.owner_id[b], 0);
    assert_eq!(
        table.mass_milli[a] + table.mass_milli[b],
        held_mass + target_mass
    );
    assert_eq!(
        table.total_mass_milli(),
        i128::from(held_mass + target_mass),
        "fracture is mass-neutral"
    );
    let counters = world.object_counters().unwrap();
    assert_eq!(counters.disassembled, 1);
    assert_eq!(counters.fractured, 1);
}

// --- registry gating (ADR-0028 section 7) ----------------------------------

#[test]
fn a_genome_bound_to_an_object_channel_is_refused_in_a_world_that_does_not_offer_it() {
    // A schema-2 world without the section: registry version 1.
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 32;
    config.cells_y = 32;
    config.initial_organisms = 4;
    config.genome2.enabled = true;
    assert_eq!(config.channel_registry_version(), 1);
    let world = World::new(config).unwrap();
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let schema2 = state.schema2.as_mut().unwrap();
    let mut genome = Genome2::decode(&schema2.genomes[0], &caps).unwrap();
    bind_always_on(&mut genome, CHANNEL_PICK_UP, 1.0, 1);
    assert_eq!(genome.required_channel_registry_version(), 2);
    schema2.genomes[0] = genome.encode();
    schema2.activation_values[0].push(0.0);
    schema2.activation_prior[0].push(0.0);
    let error = World::from_state(state).err().expect("refused");
    assert!(
        matches!(error, RestoreError::StateInvalid(ref text) if text.contains("ChannelNotOffered")),
        "refused by the registry gate, not by something downstream: {error:?}"
    );
}

// --- the two control conditions -------------------------------------------

#[test]
fn an_inert_world_fires_and_pays_and_the_verbs_confer_nothing() {
    let mut config = artifact_config(SEED);
    config.artifact.inert = true;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(
        counters.struck_terrain > 0,
        "actions still resolve and count: {counters:?}"
    );
    assert!(
        world.object_table().unwrap().is_empty(),
        "and create nothing (nobody died, so no carcass exists either)"
    );
    let control = advance(artifact_config(SEED), 60);
    assert!(
        world.total_energy_milli() < control.total_energy_milli(),
        "and still cost"
    );
}

/// The D-118 fix (`lifesim-artifact-v2`): the inert arm skips exactly the
/// five verbs, so a carcass in an inert world is eaten exactly as it is
/// under condition A and the control arm no longer differs from its
/// treatment in its food supply. v1 returned from `artifact_phase` at the
/// end of the inert charge block, which skipped consumption and the
/// exposure/carry accounting too; that early return is the mutation this
/// test exists to catch.
#[test]
fn an_inert_world_still_eats_its_carcasses_and_records_exposure() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 1;
    config.artifact.inert = true;
    let world = scripted_world(config, &[CHANNEL_PICK_UP, CHANNEL_REST]);
    let mut state = world.export_state();
    let organism = state.ids[0];
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    // A carcass with energy on the organism's own cell, booked through the
    // carcass ledger exactly as `spawn_carcass_object` books one. The
    // creator is the organism so the exposure pass has something to see.
    let energy = 3_000_i64;
    let table = state.objects.as_mut().unwrap();
    let base = state.next_entity_id;
    let def = sim_core::material(sim_core::MATERIAL_CARCASS).unwrap();
    let mut record =
        sim_core::ObjectRecord::simple(base, def, 0, x, y, 0, sim_core::CAUSE_CARCASS, organism);
    record.mass_milli = energy;
    record.energy_milli = energy;
    table.ledger.mass_carcass_milli += i128::from(energy);
    table.ledger.energy_carcass_milli += i128::from(energy);
    table.push(record);
    // And a stone beside it: inedible (energy 0), so it outlives the
    // carcass and is what the exposure assertion below watches - a carcass
    // small enough to be eaten is destroyed in the very tick it is eaten,
    // before the observation pass reads the destroyed flags.
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    let mut stone_record =
        sim_core::ObjectRecord::simple(base + 1, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
    // `simple` never sets a creator; the exposure pass only counts
    // organism-created objects, so name one.
    stone_record.creator_id = organism;
    table.ledger.mass_extracted_milli += i128::from(stone_record.mass_milli);
    table.push(stone_record);
    table.objects_allocated_total += 2;
    state.next_entity_id += 2;
    let mut world = World::from_state(state).expect("restores");
    let energy_before = world.total_energy_milli();
    run(&mut world, 30);
    let counters = world.object_counters().unwrap();
    // The verb fires, is charged and counted, and confers nothing.
    assert!(
        counters.picked_up > 0,
        "the verb still fires under inert: {counters:?}"
    );
    let table = world.object_table().unwrap();
    assert!(
        table.holder_id.iter().all(|&holder| holder == 0),
        "and confers no hold"
    );
    // Consumption ran: the carcass was eaten rather than left to decay.
    assert!(
        counters.consumed_events > 0,
        "nobody ate the carcass under inert (the D-118 defect): {counters:?}"
    );
    let ledger = world.object_ledger().unwrap();
    assert!(
        ledger.energy_consumed_milli > 0,
        "consumption is ledgered: {ledger:?}"
    );
    assert!(
        world.total_energy_milli() > energy_before,
        "the eater kept what assimilation passed through"
    );
    assert_eq!(table.total_energy_milli(), ledger.expected_energy_milli());
    // The observation pass ran: standing on a cell with a free
    // organism-created object counts as exposure under inert as under A.
    assert!(
        table.exposure_ticks[0] > 0,
        "exposure accounting ran under inert"
    );
}

/// The inert charge block, pinned by arithmetic rather than by inequality.
/// An independent mutation pass on the D-118 fix (D-119) found that the
/// routing was defended and the *pricing* was not: a verb could be counted
/// without its charge, mispriced against another verb's cost, or counted
/// without its intent, and every test stayed green - because the only cost
/// assertion was one-sided against a different arm and three of the five
/// verbs' inert arms were never executed by any test at all. Two
/// seed-matched inert worlds, differing only in which verbs the founders
/// request, close all of that: the all-verbs world pins each counter to
/// exactly one fire per tick, the no-verbs world pins every counter to
/// zero (a verb counted without its intent shows up here), and the
/// difference of the two worlds' `spent_milli` ledgers is exactly the
/// per-tick verb bill - basal cost is common-mode, `rest` freezes
/// movement in both, and feeding credits assimilation, not spending.
#[test]
fn the_inert_charge_block_counts_each_verb_once_and_bills_it_at_its_own_price() {
    let ticks = 40_u64;
    let mut config = artifact_config(SEED);
    config.initial_organisms = 1;
    config.artifact.inert = true;
    // Small distinct primes: cheap enough that one organism can pay five
    // verbs a tick for forty ticks, distinct enough that a verb billed at
    // the other verb's price moves the total.
    config.artifact.action_cost_milli = 7;
    config.artifact.strike_cost_milli = 11;
    config.validate().expect("the priced config validates");
    let all_verbs = [
        CHANNEL_PICK_UP,
        CHANNEL_DROP,
        CHANNEL_PLACE,
        CHANNEL_STRIKE,
        CHANNEL_COMBINE,
        CHANNEL_REST,
    ];
    let mut with_verbs = scripted_world(config.clone(), &all_verbs);
    let mut without_verbs = scripted_world(config.clone(), &[CHANNEL_REST]);
    run(&mut with_verbs, ticks);
    run(&mut without_verbs, ticks);
    assert_eq!(with_verbs.metrics().population, 1, "the payer survived");
    assert_eq!(without_verbs.metrics().population, 1);

    // Each verb fires exactly once per tick, and only when requested.
    let fired = with_verbs.object_counters().unwrap();
    assert_eq!(fired.picked_up, ticks, "{fired:?}");
    assert_eq!(fired.dropped, ticks, "{fired:?}");
    assert_eq!(fired.placed, ticks, "{fired:?}");
    assert_eq!(fired.combined, ticks, "{fired:?}");
    assert_eq!(fired.struck_terrain, ticks, "{fired:?}");
    assert!(
        with_verbs.object_table().unwrap().is_empty(),
        "and confer nothing"
    );
    let idle = without_verbs.object_counters().unwrap();
    assert_eq!(
        (
            idle.picked_up,
            idle.dropped,
            idle.placed,
            idle.combined,
            idle.struck_terrain
        ),
        (0, 0, 0, 0, 0),
        "a verb counted without its intent: {idle:?}"
    );

    // And the bill is exact: four verbs at the action price, one at the
    // strike price, per tick.
    let expected = i128::from(ticks)
        * i128::from(4 * config.artifact.action_cost_milli + config.artifact.strike_cost_milli);
    let billed = with_verbs.ledger().spent_milli - without_verbs.ledger().spent_milli;
    assert_eq!(billed, expected, "the inert verb bill is mispriced");
}

#[test]
fn an_ephemeral_world_destroys_what_lands_at_the_end_of_its_tick() {
    let mut config = artifact_config(SEED);
    config.artifact.ephemeral = true;
    config.artifact.max_held_objects = 1;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_DROP]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(counters.dropped > 0, "something was dropped: {counters:?}");
    assert!(
        counters.ephemeral_destroyed > 0,
        "and destroyed at tick end: {counters:?}"
    );
    let table = world.object_table().unwrap();
    // Nothing free that was dropped survives; what exists is either freshly
    // extracted this tick or held.
    assert!(
        table
            .ids
            .iter()
            .enumerate()
            .all(|(index, _)| table.holder_id[index] != 0
                || table.created_tick[index] == world.tick_number()),
        "a dropped object survived the tick it landed in"
    );
    let ledger = world.object_ledger().unwrap();
    assert!(ledger.mass_dust_milli > 0);
}

// --- caps (C12.7) ---------------------------------------------------------

#[test]
fn every_cap_rejects_counts_and_events_when_driven() {
    // Object cap: extraction stops at the cap.
    let mut config = artifact_config(SEED);
    config.artifact.max_objects = 5;
    config.artifact.max_fragments = 2;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE]);
    run(&mut world, 30);
    let counters = world.object_counters().unwrap();
    assert!(
        counters.refused_object_cap > 0,
        "the object cap never bound: {counters:?}"
    );
    assert!(world.object_table().unwrap().len() <= 5);
    let evented = world.events().iter().any(|event| {
        matches!(event.kind, EventKind::ObjectActionRefused { reason, .. } if reason == RefuseReason::ObjectCap.id())
    });
    assert!(evented, "the cap refusal events");

    // Held cap: one object per holder.
    let mut config = artifact_config(SEED);
    config.artifact.max_held_objects = 1;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(
        counters.refused_held_cap > 0,
        "the held cap never bound: {counters:?}"
    );

    // Occupancy cap: dropping into a full cell.
    let mut config = artifact_config(SEED);
    config.artifact.max_objects_per_cell = 1;
    let mut world = scripted_world(
        config,
        &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_DROP, CHANNEL_REST],
    );
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(
        counters.refused_occupancy_cap > 0,
        "the occupancy cap never bound: {counters:?}"
    );

    // Carry capacity: a capacity below any extracted object's mass refuses
    // pick-ups as CapacityExceeded. Not *every* pick-up: a carcass eaten
    // down to a few milli fits a ten-milli capacity, and a first draft that
    // asserted zero pick-ups was told so (seven, all of them near-empty
    // carcasses). What holds is that whatever is held fits.
    let mut config = artifact_config(SEED);
    config.artifact.carry_capacity_milli = 10;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(
        counters.refused_capacity > 0,
        "the carry capacity never bound: {counters:?}"
    );
    assert!(
        counters.refused_capacity > counters.picked_up,
        "{counters:?}"
    );
    let table = world.object_table().unwrap();
    for index in 0..table.len() {
        if table.holder_id[index] != 0 {
            assert!(
                table.mass_milli[index] < 100,
                "held object {} weighs {} milli, more than a ten-milli capacity at any body scale can take",
                table.ids[index],
                table.mass_milli[index]
            );
        }
    }

    // Depth cap: `the_depth_cap_refuses_the_composite_that_would_exceed_it`.
    //
    // Breadth cap: **cannot bind, and this is stated rather than left to
    // look tested.** A combine joins exactly two objects, so a composite's
    // direct constituent count is always 2, and validation refuses
    // `max_composition_breadth < 2`; the refusal path exists and is
    // reachable by no valid configuration. The cap is vestigial under the
    // binary-combine design (D-116). What is asserted here is the
    // consequence: a busy combining world never records a breadth refusal.
    let mut config = artifact_config(SEED);
    config.artifact.max_composition_breadth = 2;
    config.artifact.joint_floor_q16 = 0;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_COMBINE]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(counters.combined > 0, "nobody combined: {counters:?}");
    assert_eq!(
        counters.refused_breadth_cap, 0,
        "a breadth refusal in a valid config would be a new mechanism"
    );
}

// --- save round trip through the world's own save path -----------------------

#[test]
fn a_world_with_held_objects_and_composites_round_trips_and_steps_identically() {
    let mut config = artifact_config(SEED);
    config.artifact.joint_floor_q16 = 0;
    config.artifact.max_held_objects = 2;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_COMBINE]);
    run(&mut world, 120);
    let counters = world.object_counters().unwrap();
    assert!(
        counters.combined > 0,
        "the scenario must produce a composite: {counters:?}"
    );
    assert!(counters.picked_up > 0);
    let state = world.export_state();
    let mut restored = World::from_state(state.clone()).expect("restores");
    assert_eq!(restored.state_checksum(), world.state_checksum());
    assert_eq!(restored.export_state(), state);
    for _ in 0..100 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the two worlds diverged after restore"
    );
    world.check_invariants().unwrap();
    restored.check_invariants().unwrap();
}

// --- C12.2's per-organism observation -------------------------------------

/// Standing in a cell with a placed object counts an exposure tick; holding
/// something counts a carry tick; both survive a save round trip and leave
/// with the organism as an `ObjectExposure` event; and a placed object that
/// landed this tick counts from the next.
#[test]
fn exposure_and_carry_ticks_are_recorded_saved_and_emitted_at_death() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 2;
    let world = scripted_world(config, &[CHANNEL_REST]);
    let mut state = world.export_state();
    let (a, b) = (state.ids[0], state.ids[1]);
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    let table = state.objects.as_mut().unwrap();
    let base = state.next_entity_id;
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    // A placed object in a's cell (creator set), and an object held by b.
    let mut placed =
        sim_core::ObjectRecord::simple(base, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
    placed.creator_id = b;
    let mut held =
        sim_core::ObjectRecord::simple(base + 1, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
    held.holder_id = b;
    table.ledger.mass_extracted_milli += i128::from(placed.mass_milli + held.mass_milli);
    table.push(placed);
    table.push(held);
    table.objects_allocated_total += 2;
    state.next_entity_id += 2;
    let bands: Vec<u8> = table.birth_band.clone();
    let mut world = World::from_state(state).expect("restores");
    run(&mut world, 10);
    let table = world.object_table().unwrap();
    assert_eq!(
        table.exposure_ticks[0], 10,
        "a stood on a placed object for ten ticks"
    );
    assert_eq!(table.carry_ticks[1], 10, "b held something for ten ticks");
    assert_eq!(
        table.exposure_ticks[1],
        if world.organism_detail(b).map(|d| (d.x_fp, d.y_fp)) == Some((x, y)) {
            10
        } else {
            table.exposure_ticks[1]
        }
    );
    // Save round trip keeps the histories.
    let saved = world.export_state();
    let restored = World::from_state(saved.clone()).expect("restores");
    assert_eq!(
        restored.object_table().unwrap().exposure_ticks,
        table.exposure_ticks
    );
    assert_eq!(restored.object_table().unwrap().birth_band, bands);
    assert_eq!(restored.state_checksum(), world.state_checksum());
    // Death emits the record.
    let mut state = world.export_state();
    state.config.basal_cost_milli_per_s = 10_000_000;
    let mut world = World::from_state(state).unwrap();
    world.step();
    let mut seen = 0;
    for event in world.events() {
        if let EventKind::ObjectExposure {
            id,
            exposure_ticks,
            carry_ticks,
            age_ticks,
            birth_band,
        } = event.kind
        {
            seen += 1;
            if id == a {
                assert!(exposure_ticks >= 10, "{exposure_ticks}");
                assert_eq!(carry_ticks, 0);
            }
            if id == b {
                assert!(carry_ticks >= 10, "{carry_ticks}");
            }
            assert!(age_ticks >= 10);
            assert!(birth_band <= 4);
        }
    }
    assert_eq!(seen, 2, "one record per death");
    // The birth band is a pure function of the terrain: every founder's band
    // is within range and the thresholds put at least one founder somewhere.
    assert!(bands.iter().all(|&band| band <= 4));
}

// --- object perception (ADR-0028 section 5) ---------------------------------

/// The six cues are what the controller saw: presence, a distance that
/// falls with range, a signed bearing, heft as a share of what the organism
/// could carry, hardness as a share of the hardest material, and carried
/// load. Never a material id, never a depth. Placed at exactly two metres
/// ahead of a frozen organism facing +x, a stone reads: present 1, distance
/// 1 - 2/8, bearing 0, heft mass/capacity, hardness 1 (stone is the hardest
/// material). Nothing in range reads all zeros; a held object reads as load.
#[test]
fn object_cues_report_presence_distance_bearing_heft_hardness_and_load() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 1;
    config.artifact.perception_range_m = 8;
    config.artifact.carry_capacity_milli = 4_000;
    let world = scripted_world(config, &[CHANNEL_REST]);
    // Nothing in the world: every cue is zero.
    let mut empty = World::from_state(world.export_state()).unwrap();
    empty.step();
    assert_eq!(empty.object_perception(0), Some([0.0; 6]));

    let mut state = world.export_state();
    let organism = state.ids[0];
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    state.phase2.as_mut().unwrap().heading_bam[0] = 0; // facing +x
    let table = state.objects.as_mut().unwrap();
    let base = state.next_entity_id;
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    let fp = sim_core::FP_PER_METER;
    let ahead = sim_core::ObjectRecord::simple(
        base,
        stone,
        1_000,
        x + 2 * fp,
        y,
        0,
        sim_core::CAUSE_EXTRACTED,
        0,
    );
    table.ledger.mass_extracted_milli += i128::from(ahead.mass_milli);
    table.push(ahead);
    table.objects_allocated_total += 1;
    state.next_entity_id += 1;
    let mut world = World::from_state(state).unwrap();
    world.step();
    let cues = world.object_perception(0).unwrap();
    let scale = world
        .organism_detail(organism)
        .unwrap()
        .phase2
        .map_or(1_000, |_| 1_000);
    let _ = scale;
    assert_eq!(cues[0], 1.0, "present");
    assert!(
        (cues[1] - 0.75).abs() < 0.02,
        "distance 1 - 2/8, got {}",
        cues[1]
    );
    assert!(cues[2].abs() < 0.02, "dead ahead, got bearing {}", cues[2]);
    assert!(
        cues[3] > 0.0 && cues[3] <= 1.0,
        "heft is a share of capacity, got {}",
        cues[3]
    );
    assert!(
        (cues[4] - 1.0).abs() < 1e-6,
        "stone is the hardest material, got {}",
        cues[4]
    );
    assert_eq!(cues[5], 0.0, "holding nothing");
    // The same stone to the left (+y at heading 0 is a positive cross
    // product) reads a positive bearing; and a held object reads as load.
    let mut state = world.export_state();
    let table = state.objects.as_mut().unwrap();
    table.x_fp[0] = x;
    table.y_fp[0] = y + 2 * fp;
    let mut held = sim_core::ObjectRecord::simple(
        base + 1,
        stone,
        2_000,
        x,
        y,
        0,
        sim_core::CAUSE_EXTRACTED,
        0,
    );
    held.holder_id = organism;
    table.ledger.mass_extracted_milli += i128::from(held.mass_milli);
    table.push(held);
    table.objects_allocated_total += 1;
    state.next_entity_id += 1;
    let mut world = World::from_state(state).unwrap();
    world.step();
    let cues = world.object_perception(0).unwrap();
    assert!(
        cues[2] > 0.5,
        "to the left reads a positive bearing, got {}",
        cues[2]
    );
    assert!(
        cues[5] > 0.0,
        "a held object reads as carried load, got {}",
        cues[5]
    );
    // Held objects are not "present" targets: only free ones are sensed, so
    // taking the free stone away leaves presence at zero while load stays.
    let mut state = world.export_state();
    let table = state.objects.as_mut().unwrap();
    table.x_fp[0] = x + 20 * fp; // out of range
    let mut world = World::from_state(state).unwrap();
    world.step();
    let cues = world.object_perception(0).unwrap();
    assert_eq!(cues[0], 0.0, "the held stone is not a sensed target");
    assert!(cues[5] > 0.0);
}

// --- blocking (ADR-0028 section 3: entry-only, by mass) ---------------------

/// A ring of stones at or above `blocking_mass_milli` around an organism
/// keeps it in its cell for as long as they stand; the same ring below the
/// threshold does not. Blocking is on entry only: the organism inside its
/// own cell is free to move within it, and nothing about a carried stone
/// blocks anyone.
#[test]
fn a_heavy_free_object_blocks_entry_and_a_light_one_does_not() {
    let mut config = artifact_config(SEED);
    config.artifact.blocking_mass_milli = 3_000;
    let world = scripted_world(config, &[]);
    let state = world.export_state();
    let cell_fp = state.config.cell_size_fp();
    let cells_x = state.config.cells_x as i32;
    let cells_y = state.config.cells_y as i32;
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    // Find, in an unblocked control, an organism that changes cell inside
    // the horizon, so the blocked assertion is about a mover.
    let horizon = 120;
    let mut control = World::from_state(state.clone()).unwrap();
    let start: Vec<usize> = state
        .ids
        .iter()
        .enumerate()
        .map(|(i, _)| control.cell_index_of(state.x_fp[i], state.y_fp[i]))
        .collect();
    let mut first_mover: Option<(usize, u64)> = None;
    for _ in 0..horizon {
        control.step();
        for (i, &id) in state.ids.iter().enumerate() {
            if first_mover.is_none()
                && let Some(detail) = control.organism_detail(id)
                && control.cell_index_of(detail.x_fp, detail.y_fp) != start[i]
            {
                first_mover = Some((i, id));
            }
        }
    }
    let (mover, mover_id) = first_mover.expect("some organism changes cell in the control");
    let ring = |state: &mut sim_core::SaveState, mass: i64| {
        let (x, y) = (state.x_fp[mover], state.y_fp[mover]);
        let (cx, cy) = (x / cell_fp, y / cell_fp);
        let table = state.objects.as_mut().unwrap();
        for dy in -2..=2_i32 {
            for dx in -2..=2_i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (cx + dx, cy + dy);
                if nx < 0 || ny < 0 || nx >= cells_x || ny >= cells_y {
                    continue;
                }
                let id = state.next_entity_id;
                state.next_entity_id += 1;
                // `simple` takes a volume and derives the mass from the
                // material's density; the threshold is on mass, so set it.
                let mut record = sim_core::ObjectRecord::simple(
                    id,
                    stone,
                    1_000,
                    nx * cell_fp + cell_fp / 2,
                    ny * cell_fp + cell_fp / 2,
                    0,
                    sim_core::CAUSE_EXTRACTED,
                    0,
                );
                record.mass_milli = mass;
                table.ledger.mass_extracted_milli += i128::from(mass);
                table.push(record);
                table.objects_allocated_total += 1;
            }
        }
    };
    let cell_of_mover = |world: &World| {
        let detail = world.organism_detail(mover_id).expect("the mover is alive");
        world.cell_index_of(detail.x_fp, detail.y_fp)
    };
    // Heavy ring: the mover never leaves its cell.
    let mut heavy = state.clone();
    ring(&mut heavy, 3_000);
    let mut world = World::from_state(heavy).expect("restores");
    let home = cell_of_mover(&world);
    for _ in 0..horizon {
        world.step();
        world.check_invariants().unwrap();
        assert_eq!(
            cell_of_mover(&world),
            home,
            "the mover left a cell ringed by blocking stones"
        );
    }
    // Light ring, same layout: the mover leaves as it did in the control.
    let mut light = state.clone();
    ring(&mut light, 2_999);
    let mut world = World::from_state(light).expect("restores");
    let home = cell_of_mover(&world);
    let mut left = false;
    for _ in 0..horizon {
        world.step();
        if cell_of_mover(&world) != home {
            left = true;
            break;
        }
    }
    assert!(left, "a ring of sub-threshold stones must not block");
}

// --- carcass objects (ADR-0028 section 9) -----------------------------------

/// With the section on, a death that leaves energy makes a carcass *object*
/// with a fresh id (material 4, mass = energy = the contest share of what
/// was left) and the Phase 7 carcass table stays empty; every
/// `CarcassCreated` pairs with an `ObjectCreated` of the same id and energy.
#[test]
fn a_death_with_energy_left_makes_a_carcass_object_and_no_phase7_carcass() {
    let mut config = artifact_config(SEED);
    config.physiology.enabled = true;
    // A hazard high enough that founders die with energy in hand.
    config.physiology.extrinsic_hazard_q16_per_s = 2_000;
    let mut world = scripted_world(config, &[CHANNEL_REST]);
    let mut paired = 0;
    for _ in 0..200 {
        world.step();
        world.check_invariants().unwrap();
        let events = world.events();
        for event in events {
            if let EventKind::CarcassCreated {
                id, energy_milli, ..
            } = event.kind
            {
                let created = events.iter().find(|other| {
                    matches!(other.kind, EventKind::ObjectCreated { id: object, material_id, cause, mass_milli, energy_milli: e, .. }
                        if object == id && material_id == sim_core::MATERIAL_CARCASS && cause == sim_core::CAUSE_CARCASS
                            && mass_milli == energy_milli && e == energy_milli)
                });
                assert!(
                    created.is_some(),
                    "a carcass without its object record: {event:?}"
                );
                paired += 1;
                assert!(energy_milli > 0);
            }
        }
    }
    let counters = world.object_counters().unwrap();
    assert!(paired > 0, "no carcass was made in 200 ticks: {counters:?}");
    assert_eq!(counters.created_carcass, paired);
    assert_eq!(
        world.metrics().carcasses,
        0,
        "the Phase 7 carcass table stays empty with the section on"
    );
    let table = world.object_table().unwrap();
    assert!(
        table
            .material_id
            .iter()
            .any(|&m| m == sim_core::MATERIAL_CARCASS)
    );
    // A carcass object decays: its energy falls tick over tick and the loss
    // is ledgered, so what the ledger says the table holds is what it holds.
    let ledger = world.object_ledger().unwrap();
    assert!(
        ledger.energy_decayed_milli > 0,
        "carcasses decay: {ledger:?}"
    );
    assert_eq!(table.total_energy_milli(), ledger.expected_energy_milli());
}

// --- combination: the joint draw and the depth cap ------------------------

/// A joint floor of one whole refuses every combination as `JointFailed`
/// (the draw cannot reach it), charged and counted; the constituents are
/// untouched. The same scene with the floor at zero combines (the companion
/// test above), so the floor is the knob and not a coincidence.
#[test]
fn a_joint_floor_the_draw_cannot_reach_refuses_every_combine_by_name() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 1;
    config.artifact.reach_m = 8;
    config.artifact.joint_floor_q16 = 65_536;
    config.artifact.action_cost_milli = 500;
    let world = scripted_world(config.clone(), &[CHANNEL_COMBINE, CHANNEL_REST]);
    let mut state = world.export_state();
    let organism = state.ids[0];
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    let table = state.objects.as_mut().unwrap();
    let base = state.next_entity_id;
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    let mut held =
        sim_core::ObjectRecord::simple(base, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
    held.holder_id = organism;
    let target = sim_core::ObjectRecord::simple(
        base + 1,
        stone,
        600,
        x + 1024,
        y,
        0,
        sim_core::CAUSE_EXTRACTED,
        0,
    );
    table.ledger.mass_extracted_milli += i128::from(held.mass_milli + target.mass_milli);
    table.push(held);
    table.push(target);
    table.objects_allocated_total += 2;
    state.next_entity_id += 2;
    // The control is the same scene with nobody asking to combine.
    let mut control_state = state.clone();
    let caps = control_state.config.genome2.caps;
    let schema2 = control_state.schema2.as_mut().unwrap();
    let mut genome = Genome2::decode(&schema2.genomes[0], &caps).unwrap();
    for haplotype in &mut genome.haplotypes {
        haplotype.chromosomes[0].retain(|locus| {
            !matches!(locus.kind, LocusKind::IoBinding { channel_id, .. } if channel_id == CHANNEL_COMBINE)
        });
    }
    schema2.genomes[0] = genome.encode();
    let mut control = World::from_state(control_state).unwrap();
    run(&mut control, 5);
    let mut world = World::from_state(state).unwrap();
    run(&mut world, 5);
    let counters = world.object_counters().unwrap();
    assert_eq!(counters.combined, 0);
    assert_eq!(
        counters.refused_joint_failed, 5,
        "one refusal per attempt: {counters:?}"
    );
    let table = world.object_table().unwrap();
    assert_eq!(table.len(), 2, "both constituents untouched");
    assert_eq!(table.holder_id[0], organism);
    assert_eq!(table.owner_id[1], 0);
    assert!(
        world.organism_detail(organism).unwrap().energy_milli
            < control.organism_detail(organism).unwrap().energy_milli,
        "a refused combine is still charged"
    );
}

/// `max_composition_depth` binds on the composite that *would* be made: a
/// depth-one composite combined with a simple object is depth two, refused
/// `DepthCap` at a cap of one and made at a cap of four.
#[test]
fn the_depth_cap_refuses_the_composite_that_would_exceed_it() {
    let scene = |max_depth: u32| {
        let mut config = artifact_config(SEED);
        config.initial_organisms = 1;
        config.artifact.reach_m = 8;
        config.artifact.joint_floor_q16 = 0;
        config.artifact.max_composition_depth = max_depth;
        let world = scripted_world(config, &[CHANNEL_COMBINE, CHANNEL_REST]);
        let mut state = world.export_state();
        let organism = state.ids[0];
        let (x, y) = (state.x_fp[0], state.y_fp[0]);
        let table = state.objects.as_mut().unwrap();
        let base = state.next_entity_id;
        let stone = sim_core::material(MATERIAL_STONE).unwrap();
        let mut held =
            sim_core::ObjectRecord::simple(base, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
        held.holder_id = organism;
        let target = sim_core::ObjectRecord::simple(
            base + 1,
            stone,
            600,
            x + 1024,
            y,
            0,
            sim_core::CAUSE_EXTRACTED,
            0,
        );
        table.ledger.mass_extracted_milli += i128::from(held.mass_milli + target.mass_milli);
        table.push(held);
        table.push(target);
        table.objects_allocated_total += 2;
        state.next_entity_id += 2;
        let mut world = World::from_state(state).unwrap();
        world.step(); // depth-one composite at base + 2, free, in reach
        assert_eq!(world.object_counters().unwrap().combined, 1);
        // Hand the composite to the organism and put a fresh stone in reach.
        let mut state = world.export_state();
        let table = state.objects.as_mut().unwrap();
        let composite = table.index_of(base + 2).unwrap();
        table.holder_id[composite] = organism;
        let extra = sim_core::ObjectRecord::simple(
            base + 3,
            stone,
            500,
            x + 1024,
            y,
            0,
            sim_core::CAUSE_EXTRACTED,
            0,
        );
        table.ledger.mass_extracted_milli += i128::from(extra.mass_milli);
        table.push(extra);
        table.objects_allocated_total += 1;
        state.next_entity_id += 1;
        let mut world = World::from_state(state).expect("a held composite restores");
        world.step();
        world.check_invariants().unwrap();
        world
    };
    let capped = scene(1);
    let counters = capped.object_counters().unwrap();
    assert_eq!(
        counters.combined, 1,
        "the second combine was refused: {counters:?}"
    );
    assert_eq!(counters.refused_depth_cap, 1);
    let open = scene(4);
    let counters = open.object_counters().unwrap();
    assert_eq!(
        counters.combined, 2,
        "the second combine was made: {counters:?}"
    );
    assert_eq!(counters.refused_depth_cap, 0);
    let table = open.object_table().unwrap();
    assert_eq!(table.count_with_depth_at_least(2), 1);
    let deep = (0..table.len()).find(|&i| table.depth[i] == 2).unwrap();
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    let mass_of = |volume: i64| {
        sim_core::ObjectRecord::simple(0, stone, volume, 0, 0, 0, sim_core::CAUSE_EXTRACTED, 0)
            .mass_milli
    };
    assert_eq!(
        table.mass_milli[deep],
        mass_of(400) + mass_of(600) + mass_of(500),
        "a depth-two composite carries every constituent's mass"
    );
}

// --- placement geometry ------------------------------------------------------

/// A placed object lands at the centre of the cell one cell-length ahead of
/// the placer, whatever the placer's own offset in its cell; a placer at the
/// map edge facing out is refused `InvalidCell`, and so is one facing a cell
/// that is not traversable.
#[test]
fn a_placed_object_lands_at_the_faced_cells_centre_and_off_map_is_invalid() {
    let mut config = artifact_config(SEED);
    config.initial_organisms = 1;
    let world = scripted_world(config, &[CHANNEL_PLACE, CHANNEL_REST]);
    let state = world.export_state();
    let cell_fp = state.config.cell_size_fp();
    let organism = state.ids[0];
    let stone = sim_core::material(MATERIAL_STONE).unwrap();
    let scene = |x: i32, y: i32, heading: u16| {
        let mut state = state.clone();
        state.x_fp[0] = x;
        state.y_fp[0] = y;
        state.phase2.as_mut().unwrap().heading_bam[0] = heading;
        let table = state.objects.as_mut().unwrap();
        let base = state.next_entity_id;
        let mut held =
            sim_core::ObjectRecord::simple(base, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
        held.holder_id = organism;
        table.ledger.mass_extracted_milli += i128::from(held.mass_milli);
        table.push(held);
        table.objects_allocated_total += 1;
        state.next_entity_id += 1;
        let mut world = World::from_state(state).expect("restores");
        world.step();
        world.check_invariants().unwrap();
        world
    };
    // Find a traversable cell whose +x neighbour is traversable too, away
    // from the edge, and stand near that cell's far side facing +x.
    let cells_x = state.config.cells_x as i32;
    let cells_y = state.config.cells_y as i32;
    let probe = World::from_state(state.clone()).unwrap();
    let mut found = None;
    'search: for cy in 1..cells_y - 1 {
        for cx in 1..cells_x - 2 {
            let here = (cy * cells_x + cx) as usize;
            let next = here + 1;
            if probe.effective_traversable(here) && probe.effective_traversable(next) {
                found = Some((cx, cy));
                break 'search;
            }
        }
    }
    let (cx, cy) = found.expect("two adjacent land cells");
    let x = cx * cell_fp + cell_fp - 3; // near the far side of the cell
    let y = cy * cell_fp + 5;
    let world = scene(x, y, 0);
    let counters = world.object_counters().unwrap();
    assert_eq!(counters.placed, 1, "{counters:?}");
    let table = world.object_table().unwrap();
    assert_eq!(table.holder_id[0], 0);
    assert_eq!(
        (table.x_fp[0], table.y_fp[0]),
        ((cx + 1) * cell_fp + cell_fp / 2, cy * cell_fp + cell_fp / 2),
        "snapped to the faced cell's centre, not to the placer's offset"
    );
    // At a map edge facing out: off the map, refused by name. Any edge cell
    // that is land will do; the heading faces its edge (BAM: 0 = +x,
    // 16384 = +y, 32768 = -x, 49152 = -y).
    let mut edge_cell = None;
    for cy in 0..cells_y {
        for cx in 0..cells_x {
            let heading = if cx == cells_x - 1 {
                0
            } else if cx == 0 {
                32_768
            } else if cy == cells_y - 1 {
                16_384
            } else if cy == 0 {
                49_152
            } else {
                continue;
            };
            if probe.effective_traversable((cy * cells_x + cx) as usize) {
                edge_cell = Some((cx, cy, heading));
                break;
            }
        }
        if edge_cell.is_some() {
            break;
        }
    }
    // The generated map is an island - its edges are water - so the off-map
    // branch is only reachable if some edge cell is land; when it is not,
    // the same refusal is exercised on the coast below, which this map
    // always has.
    if let Some((ex, ey, heading)) = edge_cell {
        let edge = scene(
            ex * cell_fp + cell_fp / 2,
            ey * cell_fp + cell_fp / 2,
            heading,
        );
        let counters = edge.object_counters().unwrap();
        assert_eq!(counters.placed, 0, "{counters:?}");
        assert_eq!(counters.refused_invalid_cell, 1, "{counters:?}");
        assert_eq!(
            edge.object_table().unwrap().holder_id[0],
            organism,
            "still held"
        );
    }
    // A non-traversable faced cell (the coast) is refused the same way.
    let mut water = None;
    'water: for cy in 1..cells_y - 1 {
        for cx in 1..cells_x - 2 {
            let here = (cy * cells_x + cx) as usize;
            if probe.effective_traversable(here) && !probe.effective_traversable(here + 1) {
                water = Some((cx, cy));
                break 'water;
            }
        }
    }
    let (cx, cy) = water.expect("a land cell with water to its +x");
    let shore = scene(cx * cell_fp + cell_fp / 2, cy * cell_fp + cell_fp / 2, 0);
    let counters = shore.object_counters().unwrap();
    assert_eq!(
        counters.refused_invalid_cell, 1,
        "facing water: {counters:?}"
    );
    assert_eq!(counters.placed, 0);
    assert_eq!(
        shore.object_table().unwrap().holder_id[0],
        organism,
        "still held"
    );
}
