//! Phase 13 social channel: the tick integration (ADR-0029).
//!
//! The table arithmetic and the registry gating are unit-tested beside their
//! modules; what this file owns is everything that only exists once a whole
//! world is running: that a **disabled** section reproduces the fixtures
//! byte for byte, that an emitter is charged and stamped and a receiver
//! reads the committed field exactly one tick late (Rule 4 made
//! observable), that the field decays to exactly zero, that the condition
//! gates are distinct replay lineages that actually draw, that the
//! neighbour cues read committed prior state, and that a social world save
//! round-trips and steps identically.
//!
//! Scenarios are scripted the way `phase12_artifact.rs` scripts them: no
//! intent-injection hook exists, on purpose; founder genomes are rewritten
//! through the save path so a chosen output channel is bound always-on.

use sim_core::{
    Activation, CHANNEL_SIGNAL_EMIT_BASE, Genome2, GenomeCaps, Locus, LocusKind, NodeRole,
    STRUCTURAL_HOMOLOGY_BASE, SimConfig, World,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// Schema 2's `rest` action channel: bound always-on it freezes movement.
const CHANNEL_REST: u16 = 105;

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

/// A small social world: everything the section requires (schema 2,
/// worldmod, contest, artifact) plus the section itself, with no births so
/// the founders' scripted bindings are the whole population.
fn social_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 48;
    config.cells_y = 48;
    config.initial_organisms = 8;
    config.max_entities = 200;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.contest.enabled = true;
    config.artifact.enabled = true;
    config.social.enabled = true;
    config.reproduction_enabled = false;
    config.validate().expect("the social config validates");
    config
}

fn bind_always_on(genome: &mut Genome2, channel: u16, gain: f32, salt: u32) {
    let node_id = STRUCTURAL_HOMOLOGY_BASE + 60_000 + salt * 10;
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

// --- C13.12: a disabled section changes nothing -----------------------------

#[test]
fn a_disabled_social_section_reproduces_every_reachable_fixture_exactly() {
    let cases: [(SimConfig, u64, u64); 2] = [
        (SimConfig::phase1_default(SEED), 500, 0x1e31_58a2_6afd_3b39),
        (SimConfig::phase2_default(SEED), 500, 0xff9d_fcff_5dff_bf42),
    ];
    for (config, ticks, state_checksum) in cases {
        assert!(!config.social.enabled, "the section defaults to off");
        let world = advance(config, ticks);
        assert_eq!(
            world.state_checksum(),
            state_checksum,
            "state checksum moved"
        );
        assert!(world.social_table().is_none());
        assert!(world.social_counters().is_none());
    }
}

// --- emission, reception, and the one-tick lag ------------------------------

/// The heart of the two-phase design (Rule 4, C13.13): an emission at tick t
/// is charged and staged at t, committed at t's finalize, and **readable at
/// t+1 and never earlier**. The perception buffer built by t's own sense
/// phase - before the emission ran - reads zero; the buffer built at t+1
/// reads the committed value. Nothing an organism does mid-tick is
/// observable by any other organism in the same tick.
#[test]
fn an_emission_is_charged_at_t_and_readable_at_t_plus_one_never_earlier() {
    let mut config = social_config(SEED);
    config.initial_organisms = 2;
    let world = scripted_world(config, &[CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_REST]);
    // Co-locate the pair so the receiver reads the emitter's cell region.
    let mut state = world.export_state();
    state.x_fp[1] = state.x_fp[0];
    state.y_fp[1] = state.y_fp[0];
    let mut world = World::from_state(state).expect("restores");

    world.step();
    world.check_invariants().expect("invariants");
    let counters = world.social_counters().expect("section on");
    assert!(counters.signals_emitted_total > 0, "{counters:?}");
    assert!(counters.signal_cost_milli_total > 0, "emission is charged");
    let table = world.social_table().expect("section on");
    assert!(
        table.committed_field_q16.iter().any(|&value| value > 0),
        "the field committed at finalize"
    );
    // The perception buffer this tick's sense built predates the emission.
    let received_at_t = world.social_perception_of(1).expect("row exists")[36];
    assert_eq!(
        received_at_t, 0.0,
        "an emission must not be observable in the tick it was made"
    );

    world.step();
    world.check_invariants().expect("invariants");
    let received_at_t1 = world.social_perception_of(1).expect("row exists")[36];
    assert!(
        received_at_t1 > 0.0,
        "the committed field is readable exactly one tick later"
    );
}

/// An injected committed field with no emitter decays to exactly zero: the
/// integer decay-and-clamp has no residue, so a signal is a transient local
/// event and not a permanent world marking.
#[test]
fn the_committed_field_decays_to_exactly_zero_without_emission() {
    let config = social_config(SEED);
    let world = scripted_world(config, &[CHANNEL_REST]);
    let mut state = world.export_state();
    let social = state.social.as_mut().expect("section on");
    let len = social.committed_field_q16.len();
    social.committed_field_q16[0] = 65_536;
    social.committed_field_q16[len / 2] = 12_345;
    social.committed_field_q16[len - 1] = 1;
    let mut world = World::from_state(state).expect("a nonzero field restores");
    run(&mut world, 200);
    let table = world.social_table().expect("section on");
    assert!(
        table.committed_field_q16.iter().all(|&value| value == 0),
        "a field with no emitter must decay to exactly zero"
    );
}

// --- neighbour cues ---------------------------------------------------------

/// Two near-adjacent organisms perceive each other: presence, distance,
/// scale and health are live; with perception disabled (condition C's
/// shape) the same cues read zero while the registry width is unchanged.
#[test]
fn neighbour_cues_read_committed_state_and_a_perception_off_world_reads_zero() {
    let mut config = social_config(SEED);
    config.initial_organisms = 2;
    let world = scripted_world(config, &[CHANNEL_REST]);
    let mut state = world.export_state();
    state.x_fp[1] = state.x_fp[0] + 1024;
    state.y_fp[1] = state.y_fp[0];
    let mut world = World::from_state(state).expect("restores");
    world.step();
    let cues = world.social_perception_of(0).expect("row exists");
    assert_eq!(cues[0], 1.0, "slot 0 occupied: {cues:?}");
    assert!(cues[1] > 0.0, "distance cue live: {cues:?}");
    assert!(cues[7] > 0.0, "scale cue live: {cues:?}");
    assert!(cues[8] > 0.0, "health cue live: {cues:?}");
    assert_eq!(cues[9], 0.0, "slot 1 empty: only one neighbour exists");

    let mut config_off = social_config(SEED);
    config_off.initial_organisms = 2;
    config_off.social.perception_enabled = false;
    config_off.validate().expect("valid");
    let world_off = scripted_world(config_off, &[CHANNEL_REST]);
    let mut state = world_off.export_state();
    state.x_fp[1] = state.x_fp[0] + 1024;
    state.y_fp[1] = state.y_fp[0];
    let mut world_off = World::from_state(state).expect("restores");
    world_off.step();
    let cues_off = world_off.social_perception_of(0).expect("row exists");
    assert!(
        cues_off[..36].iter().all(|&value| value == 0.0),
        "perception off reads zero: {cues_off:?}"
    );
}

/// Feeding is contact: after a tick in which an organism ate, its committed
/// one-tick contact record is set, and a neighbour's contact cue reads it
/// the following tick.
#[test]
fn contact_records_commit_at_finalize_and_are_cues_one_tick_later() {
    let mut config = social_config(SEED);
    config.initial_organisms = 2;
    let world = scripted_world(config, &[CHANNEL_REST]);
    let mut state = world.export_state();
    state.x_fp[1] = state.x_fp[0] + 1024;
    state.y_fp[1] = state.y_fp[0];
    let mut world = World::from_state(state).expect("restores");
    run(&mut world, 3);
    let table = world.social_table().expect("section on");
    assert!(
        table.prior_contact.iter().any(|&flag| flag),
        "feeding never recorded contact: {table:?}"
    );
    let cues = world.social_perception_of(0).expect("row exists");
    assert_eq!(
        cues[4], 1.0,
        "the neighbour's contact cue is live: {cues:?}"
    );
}

// --- the condition gates are distinct lineages that draw ---------------------

#[test]
fn scramble_and_corruption_are_distinct_lineages_and_actually_draw() {
    let base = {
        let mut config = social_config(SEED);
        config.initial_organisms = 4;
        config
    };
    let mut scrambled = base.clone();
    scrambled.social.scramble_delivery = true;
    scrambled.validate().expect("valid");
    let mut corrupted = base.clone();
    corrupted.social.signal_corruption_q16 = 16_384;
    corrupted.validate().expect("valid");

    assert_ne!(base.stable_hash(), scrambled.stable_hash());
    assert_ne!(base.stable_hash(), corrupted.stable_hash());
    assert_ne!(scrambled.stable_hash(), corrupted.stable_hash());

    let channels = [CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_REST];
    let mut world_a = scripted_world(base, &channels);
    let mut world_d = scripted_world(scrambled, &channels);
    let mut world_c = scripted_world(corrupted, &channels);
    run(&mut world_a, 30);
    run(&mut world_d, 30);
    run(&mut world_c, 30);

    let a = world_a.social_counters().unwrap();
    let d = world_d.social_counters().unwrap();
    let c = world_c.social_counters().unwrap();
    assert_eq!(a.scrambled_deliveries_total, 0);
    assert!(d.scrambled_deliveries_total > 0, "{d:?}");
    assert_eq!(a.corruption_draws_total, 0, "zero corruption takes no draw");
    assert!(c.corruption_draws_total > 0, "{c:?}");
    // Cost is identical across A and D by construction - the scramble moves
    // the stamp, never the bill.
    assert_eq!(
        a.signal_cost_milli_total, d.signal_cost_milli_total,
        "condition D must keep the emission bill byte-identical"
    );
}

// --- save round trip --------------------------------------------------------

/// A social world with a live nonzero field and committed cue records saves
/// through its own save path, restores, and steps on identically.
#[test]
fn a_social_world_round_trips_and_steps_identically_with_a_nonzero_field() {
    let mut config = social_config(SEED);
    config.initial_organisms = 4;
    let mut world = scripted_world(config, &[CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_REST]);
    run(&mut world, 20);
    let table = world.social_table().expect("section on");
    assert!(
        table.committed_field_q16.iter().any(|&value| value > 0),
        "the fixture must carry a live field or the round trip pins nothing"
    );
    let state = world.export_state();
    let mut restored = World::from_state(state).expect("restores");
    assert_eq!(world.state_checksum(), restored.state_checksum());
    for _ in 0..50 {
        world.step();
        restored.step();
    }
    assert_eq!(
        world.state_checksum(),
        restored.state_checksum(),
        "a restored social world diverged"
    );
}

/// A tampered restore fails closed by the named violation: a field value
/// above one whole, a remainder at a whole milli, and a length mismatch are
/// each refused.
#[test]
fn a_tampered_social_table_is_refused_by_name() {
    let config = social_config(SEED);
    let world = scripted_world(config, &[CHANNEL_REST]);

    let mut state = world.export_state();
    state.social.as_mut().unwrap().committed_field_q16[0] = 65_537;
    assert!(
        World::from_state(state)
            .err()
            .map(|error| format!("{error:?}"))
            .is_some_and(|text| text.contains("field_value_range")),
        "an out-of-range field value must be refused by name"
    );

    let mut state = world.export_state();
    state.social.as_mut().unwrap().emission_remainder_milli[0] = 65_536;
    assert!(
        World::from_state(state)
            .err()
            .map(|error| format!("{error:?}"))
            .is_some_and(|text| text.contains("emission_remainder_range")),
        "a whole uncharged milli must be refused, not normalized (D-094)"
    );

    let mut state = world.export_state();
    state.social.as_mut().unwrap().prior_contact.pop();
    assert!(
        World::from_state(state)
            .err()
            .map(|error| format!("{error:?}"))
            .is_some_and(|text| text.contains("per_organism_len")),
        "a short per-organism array must be refused by name"
    );
}

// --- validation gates -------------------------------------------------------

#[test]
fn the_social_validation_refuses_what_the_design_forbids() {
    // Social without artifact: the registry total order.
    let mut config = SimConfig::phase2_default(SEED);
    config.genome2.enabled = true;
    config.social.enabled = true;
    assert!(config.validate().is_err(), "social requires artifact");

    // A sub-gate moved with the section off.
    let mut config = SimConfig::phase2_default(SEED);
    config.social.observational_enabled = true;
    assert!(
        config.validate().is_err(),
        "a condition set while disabled is refused, not ignored"
    );

    // A non-decaying field: a permanent marking is what artifacts are for.
    let mut config = social_config(SEED);
    config.social.signal_retain_q16 = 65_536;
    assert!(config.validate().is_err(), "retain must be below one whole");

    // Scramble without a signal half is condition B wearing a D name.
    let mut config = social_config(SEED);
    config.social.signal_enabled = false;
    config.social.scramble_delivery = true;
    assert!(config.validate().is_err());

    // K and channel counts beyond what the registry can express.
    let mut config = social_config(SEED);
    config.social.perception_k = 5;
    assert!(config.validate().is_err());
    let mut config = social_config(SEED);
    config.social.signal_channels = 0;
    assert!(config.validate().is_err());
}

// --- clean-process-shape determinism ----------------------------------------

/// Two worlds from one config agree bit for bit across 200 ticks with
/// emission, corruption and scramble all live. The clean-process version is
/// the Phase 13 verify script's job; this is the in-process regression that
/// runs on every suite pass.
#[test]
fn a_fully_live_social_world_is_deterministic_over_200_ticks() {
    let mut config = social_config(SEED ^ 0x1357);
    config.initial_organisms = 6;
    config.social.signal_corruption_q16 = 8_192;
    config.social.scramble_delivery = true;
    config.validate().expect("valid");
    let channels = [CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_REST];
    let mut first = scripted_world(config.clone(), &channels);
    let mut second = scripted_world(config, &channels);
    for _ in 0..200 {
        first.step();
        second.step();
    }
    assert_eq!(first.state_checksum(), second.state_checksum());
    let counters = first.social_counters().unwrap();
    assert!(counters.signals_emitted_total > 0);
    assert!(counters.corruption_draws_total > 0);
    assert!(counters.scrambled_deliveries_total > 0);
}
