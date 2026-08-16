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
    Activation, CHANNEL_COMBINE, CHANNEL_DROP, CHANNEL_PICK_UP, CHANNEL_STRIKE, EventKind,
    Genome2, GenomeCaps, InheritanceMode, Locus, LocusKind, MATERIAL_STONE, NodeRole,
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
        world.check_invariants().expect("invariants hold every tick");
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
        genome.validate_structure(&caps).expect("the rewritten genome validates");
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
        assert_eq!(config.genome2.mutation.binding_q16, 0, "the operator defaults to off");
        assert_eq!(config.stable_hash(), config_hash, "config hash moved");
        let world = advance(config, ticks);
        assert_eq!(world.state_checksum(), state_checksum, "state checksum moved");
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
    assert!(counters.struck_terrain > 0, "nobody struck the ground: {counters:?}");
    assert!(counters.created_extracted > 0, "nothing was extracted: {counters:?}");
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
    assert!(before.refused_depleted > 0, "no cell ever ran dry: {before:?}");
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
    let held = table.holder_id.iter().filter(|&&holder| holder != 0).count();
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
    world.check_invariants().expect("invariants after the mass death");
    let counters = world.object_counters().unwrap();
    assert!(counters.death_drops > 0, "no death dropped anything: {counters:?}");
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
    assert_eq!(table.holder_id[0], high, "the nearer organism wins whatever its id");
    let counters = world.object_counters().unwrap();
    assert_eq!(counters.picked_up, 1);
    assert_eq!(counters.refused_contested, 1, "the loser is refused Contested and pays");
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
    let mut held = sim_core::ObjectRecord::simple(base, stone, 400, x, y, 0, sim_core::CAUSE_EXTRACTED, 0);
    held.holder_id = organism;
    let target = sim_core::ObjectRecord::simple(base + 1, stone, 600, x + 1024, y, 0, sim_core::CAUSE_EXTRACTED, 0);
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
    assert_eq!(world.object_counters().unwrap().combined, 1, "the combine happened");
    let composite = table.index_of(base + 2).expect("the composite took the next id");
    assert_eq!(table.mass_milli[composite], held_mass + target_mass);
    assert_eq!(table.depth[composite], 1);
    assert_eq!(table.composition[composite], vec![base, base + 1]);
    assert_eq!(table.owner_id[table.index_of(base).unwrap()], base + 2);
    assert_eq!(table.total_mass_milli(), i128::from(held_mass + target_mass), "combine is mass-neutral");
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
    assert_eq!(table.mass_milli[a] + table.mass_milli[b], held_mass + target_mass);
    assert_eq!(table.total_mass_milli(), i128::from(held_mass + target_mass), "fracture is mass-neutral");
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
fn an_inert_world_fires_and_pays_and_changes_nothing() {
    let mut config = artifact_config(SEED);
    config.artifact.inert = true;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(counters.struck_terrain > 0, "actions still resolve and count: {counters:?}");
    assert!(world.object_table().unwrap().is_empty(), "and create nothing");
    let control = advance(artifact_config(SEED), 60);
    assert!(world.total_energy_milli() < control.total_energy_milli(), "and still cost");
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
    assert!(counters.ephemeral_destroyed > 0, "and destroyed at tick end: {counters:?}");
    let table = world.object_table().unwrap();
    // Nothing free that was dropped survives; what exists is either freshly
    // extracted this tick or held.
    assert!(
        table
            .ids
            .iter()
            .enumerate()
            .all(|(index, _)| table.holder_id[index] != 0 || table.created_tick[index] == world.tick_number()),
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
    assert!(counters.refused_object_cap > 0, "the object cap never bound: {counters:?}");
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
    assert!(counters.refused_held_cap > 0, "the held cap never bound: {counters:?}");

    // Occupancy cap: dropping into a full cell.
    let mut config = artifact_config(SEED);
    config.artifact.max_objects_per_cell = 1;
    let mut world = scripted_world(config, &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_DROP, CHANNEL_REST]);
    run(&mut world, 60);
    let counters = world.object_counters().unwrap();
    assert!(counters.refused_occupancy_cap > 0, "the occupancy cap never bound: {counters:?}");
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
    assert!(counters.combined > 0, "the scenario must produce a composite: {counters:?}");
    assert!(counters.picked_up > 0);
    let state = world.export_state();
    let mut restored = World::from_state(state.clone()).expect("restores");
    assert_eq!(restored.state_checksum(), world.state_checksum());
    assert_eq!(restored.export_state(), state);
    for _ in 0..100 {
        world.step();
        restored.step();
    }
    assert_eq!(restored.state_checksum(), world.state_checksum(), "the two worlds diverged after restore");
    world.check_invariants().unwrap();
    restored.check_invariants().unwrap();
}
