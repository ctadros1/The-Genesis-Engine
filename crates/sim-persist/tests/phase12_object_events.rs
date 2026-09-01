//! Phase 12 artifact half: the object events (tags 14-23, event schema 6)
//! round-trip through the log file and reconstruct the world's object
//! counters class by class (ADR-0028; the A5.3 discipline extended to
//! `ObjectCounters`).
//!
//! The scenario is the `--artifact` trace's script in miniature: founders
//! rewritten through the save path (the `phase12_artifact.rs` pattern) so
//! that, by index, they strike; strike, pick up and place; strike, pick up
//! and combine; or pick up and drop, from tick one. What is asserted is not
//! that anything in particular happens - the trace fixture pins that - but
//! that whatever happened is told by the log exactly once: every created
//! object has one `ObjectCreated` under its cause, every object that is not
//! in the final table has one `ObjectDestroyed`, every success and every
//! refusal has its event, and every death in an artifact world has its
//! `ObjectExposure` record.

use sim_core::{
    Activation, CHANNEL_COMBINE, CHANNEL_DROP, CHANNEL_PICK_UP, CHANNEL_PLACE, CHANNEL_STRIKE,
    EventKind, Genome2, GenomeCaps, Locus, LocusKind, NodeRole, ObjectCounters,
    STRUCTURAL_HOMOLOGY_BASE, SimConfig, World,
};
use sim_persist::{EventLogInfo, EventLogRecorder, EventLogWriter, decode_log, decode_log_events};
use std::fs;
use std::path::PathBuf;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const CHANNEL_REST: u16 = 105;

fn scratch_dir(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "lifesim-object-events-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

fn log_info(world: &World) -> EventLogInfo {
    EventLogInfo {
        format_version: sim_persist::EVENT_LOG_FORMAT_VERSION,
        world_id: 1,
        seed: world.config().world_seed,
        config_hash: world.config_hash(),
        event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
        max_events_per_tick: sim_core::MAX_EVENTS_PER_TICK as u32,
        start_tick: 0,
        build_version: sim_persist::BUILD_VERSION.to_owned(),
    }
}

fn artifact_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 48;
    config.cells_y = 48;
    config.initial_organisms = 48;
    config.max_entities = 400;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.contest.enabled = true;
    config.artifact.enabled = true;
    // Cheap actions and a small per-cell cap so that both successes and cap
    // refusals appear inside the horizon. The world cap is left wide: at 40
    // it bound first and refused every carcass, which is a fact about caps
    // (a carcass at the cap is dust, ledgered) and not what this test is for.
    config.artifact.action_cost_milli = 6;
    config.artifact.strike_cost_milli = 12;
    config.artifact.max_objects = 400;
    config.artifact.max_objects_per_cell = 3;
    // The extrinsic hazard is the death that leaves energy behind (a
    // carcass object) inside a short horizon; starvation leaves none.
    config.physiology.enabled = true;
    config.physiology.extrinsic_hazard_q16_per_s = 60;
    config.reproduction_enabled = false;
    config.validate().expect("the artifact config validates");
    config
}

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

/// Founders scripted by index, as the `--artifact` trace scripts them.
fn scripted_trace_world(config: SimConfig) -> World {
    const SCRIPTS: [&[u16]; 4] = [
        &[CHANNEL_STRIKE],
        &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_PLACE],
        &[CHANNEL_STRIKE, CHANNEL_PICK_UP, CHANNEL_COMBINE],
        &[CHANNEL_PICK_UP, CHANNEL_DROP],
    ];
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    let schema2 = state.schema2.as_mut().expect("a schema-2 world");
    for index in 0..schema2.genomes.len() {
        let script = SCRIPTS[index % SCRIPTS.len()];
        let mut genome = Genome2::decode(&schema2.genomes[index], &caps).expect("decodes");
        // Movers and stayers alternate so the placers have somewhere to face
        // and the pickers something to walk to.
        let freeze = index % 2 == 0;
        let mut salt = 0;
        for &channel in script {
            bind_always_on(&mut genome, channel, 1.0, salt);
            salt += 1;
        }
        if freeze {
            bind_always_on(&mut genome, CHANNEL_REST, 1.0, salt);
            salt += 1;
        }
        genome
            .validate_structure(&caps)
            .expect("the rewritten genome validates");
        schema2.genomes[index] = genome.encode();
        for _ in 0..salt {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    World::from_state(state).expect("the rewritten founders restore")
}

#[test]
fn every_object_event_round_trips_and_the_log_reconstructs_the_object_counters() {
    let directory = scratch_dir("reconcile");
    let path = directory.join("run.alev");
    let mut world = scripted_trace_world(artifact_config(SEED));
    let writer = EventLogWriter::create(&path, &log_info(&world)).expect("create log");
    let mut recorder = EventLogRecorder::new(writer);
    // Long enough for the founders to age out (deaths, so `ObjectExposure`
    // and death drops appear) and for carcasses to decay.
    for _ in 0..6_000 {
        world.step();
        world
            .check_invariants()
            .expect("invariants hold every tick");
        recorder.record(&world).expect("record tick");
    }
    recorder.writer_mut().sync().expect("sync");
    let bytes = fs::read(&path).expect("read log");

    let (scan, events) = decode_log_events(&bytes).expect("log decodes");
    assert_eq!(scan.bytes_consumed, bytes.len(), "trailing bytes");
    assert_eq!(
        scan.dropped, 0,
        "a dropped event would make every equality below approximate"
    );
    assert_eq!(
        scan.counters,
        decode_log(&bytes).expect("streaming decode").counters
    );

    let table = world.object_table().expect("an artifact world");
    let counters: ObjectCounters = world.object_counters().expect("counters");
    let recon = &scan.counters;

    // The scenario is not a control: every class of event this test claims
    // to reconcile must have happened at least once.
    for (name, value) in [
        ("created_extracted", counters.created_extracted),
        ("created_fractured", counters.created_fractured),
        ("created_combined", counters.created_combined),
        ("created_carcass", counters.created_carcass),
        ("picked_up", counters.picked_up),
        ("dropped", counters.dropped),
        ("placed", counters.placed),
        ("struck_objects", counters.struck_objects),
        ("struck_terrain", counters.struck_terrain),
        ("combined", counters.combined),
        ("consumed_events", counters.consumed_events),
        ("decayed_away", counters.decayed_away),
        ("death_drops", counters.death_drops),
        ("refusals", counters.refusals()),
        ("cap_refusals", counters.cap_refusals()),
    ] {
        assert!(
            value > 0,
            "the scenario never exercised `{name}`, so this test proves nothing about it"
        );
    }
    let metrics = world.metrics();
    let deaths = metrics.deaths_starvation_total
        + metrics.deaths_old_age_total
        + metrics.deaths_by_damage_total
        + metrics.deaths_senescence_total
        + metrics.deaths_extrinsic_total;
    assert!(
        deaths > 0,
        "nobody died, so exposure records and death drops were never exercised"
    );

    // Creation, per cause (wire codes 1..=4).
    assert_eq!(
        recon.objects_created_by_cause[0], counters.created_extracted,
        "extracted"
    );
    assert_eq!(
        recon.objects_created_by_cause[1], counters.created_fractured,
        "fractured"
    );
    assert_eq!(
        recon.objects_created_by_cause[2], counters.created_combined,
        "combined"
    );
    assert_eq!(
        recon.objects_created_by_cause[3], counters.created_carcass,
        "carcass"
    );
    let created: u64 = recon.objects_created_by_cause.iter().sum();
    assert_eq!(
        created, table.objects_allocated_total,
        "every allocated object has one ObjectCreated"
    );

    // Destruction: every object that was created and is not in the table
    // was destroyed by exactly one event, and per cause the counters agree.
    let destroyed: u64 = recon.objects_destroyed_by_cause.iter().sum();
    assert_eq!(
        destroyed + table.len() as u64,
        created,
        "created = destroyed + live"
    );
    assert_eq!(
        recon.objects_destroyed_by_cause[0], counters.decayed_away,
        "Decayed"
    );
    assert_eq!(
        recon.objects_destroyed_by_cause[2], counters.disassembled,
        "Disassembled"
    );
    // `Fractured` is the simple-object end of the fracture path: a struck
    // simple object over threshold (`fractured`) or worn to nothing
    // (`worn_away`). A struck composite over threshold counts as `fractured`
    // *and* comes apart under `Disassembled` (so does a composite worn to
    // nothing, under `disassembled` alone). The two event classes together
    // therefore cover `fractured + worn_away` exactly when no composite was
    // worn out, and exceed it by the worn composites otherwise - which is
    // why the equality below is on the sum, with the composite term bounded
    // rather than reconstructed.
    let fractured_events = recon.objects_destroyed_by_cause[1];
    let disassembled_events = recon.objects_destroyed_by_cause[2];
    assert!(
        fractured_events <= counters.fractured + counters.worn_away,
        "Fractured events exceed the counters"
    );
    assert!(
        fractured_events + disassembled_events >= counters.fractured + counters.worn_away,
        "a fracture or a wear-out left no destruction event"
    );
    let composite_fractures = counters.fractured + counters.worn_away - fractured_events;
    assert!(
        composite_fractures <= counters.disassembled,
        "more composite fractures than disassemblies"
    );
    // `consumed_events` counts bites; `Consumed` is the bite that empties
    // an object, so it is bounded by the bites and pinned by the sum above.
    assert!(
        recon.objects_destroyed_by_cause[3] <= counters.consumed_events,
        "Consumed"
    );
    assert_eq!(
        recon.objects_destroyed_by_cause[5], counters.ephemeral_destroyed,
        "Ephemeral (none under A)"
    );

    // Actions.
    assert_eq!(recon.objects_picked_up_total, counters.picked_up);
    assert_eq!(recon.objects_placed_total, counters.placed);
    assert_eq!(
        recon.objects_dropped_total,
        counters.dropped + counters.death_drops,
        "released unplaced = drops + death drops"
    );
    assert_eq!(recon.objects_struck_total, counters.struck_objects);
    assert_eq!(recon.terrain_struck_total, counters.struck_terrain);
    assert_eq!(recon.objects_combined_total, counters.combined);
    assert_eq!(recon.objects_consumed_total, counters.consumed_events);

    // Refusals, per reason (wire codes 1..=13) and in total.
    assert_eq!(recon.object_refusals_total, counters.refusals());
    let by_reason = [
        counters.refused_no_target,
        counters.refused_capacity,
        counters.refused_held_cap,
        counters.refused_contested,
        counters.refused_nothing_held,
        counters.refused_occupancy_cap,
        counters.refused_invalid_cell,
        counters.refused_depleted,
        counters.refused_no_yield,
        counters.refused_object_cap,
        counters.refused_depth_cap,
        counters.refused_breadth_cap,
        counters.refused_joint_failed,
    ];
    assert_eq!(recon.object_refusals_by_reason, by_reason);

    // One exposure record per death, and each is a well-formed record.
    assert_eq!(
        recon.object_exposure_records, deaths,
        "one ObjectExposure per death"
    );
    let exposure_records = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::ObjectExposure { .. }))
        .count() as u64;
    assert_eq!(exposure_records, deaths);
    for event in &events {
        if let EventKind::ObjectExposure {
            exposure_ticks,
            carry_ticks,
            age_ticks,
            birth_band,
            ..
        } = event.kind
        {
            assert!(
                exposure_ticks <= age_ticks && carry_ticks <= age_ticks,
                "ticks of a life bound its history"
            );
            assert!(birth_band < 5, "a capacity quintile");
        }
    }
    let _ = fs::remove_dir_all(&directory);
}
