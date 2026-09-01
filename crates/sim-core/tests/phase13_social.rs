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

// --- rule 5: offered under P, verified absent by counter under S ------------

/// The observational rule's gate, end to end (ADR-0029 section 5): the same
/// rule-5 allele learns under condition P and is inert under condition S,
/// and the S arm's proof is the mechanism's own counter, not the config
/// flag. The edge feeds a hidden node nothing reads (the Phase 11 neutral-
/// edge pattern), so the two arms' ecologies are identical and the only
/// difference is the gate.
#[test]
fn a_rule_5_allele_learns_under_p_and_is_inert_under_s_verified_by_counter() {
    use sim_core::{EDGE_FLAG_PLASTIC, LearnedEdgeSave, PlasticityGenes, RULE_OBSERVATIONAL};

    fn add_rule5_edge(genome: &mut Genome2) {
        const BASE: u32 = STRUCTURAL_HOMOLOGY_BASE;
        const FOUNDER_INPUT: u32 = BASE + 1_000;
        const NEUTRAL_NODE: u32 = BASE + 8_000;
        const NEUTRAL_EDGE: u32 = BASE + 9_000;
        for haplotype in &mut genome.haplotypes {
            for chromosome in &mut haplotype.chromosomes {
                chromosome.push(Locus {
                    homology_id: NEUTRAL_NODE,
                    gene_lineage_id: u64::from(NEUTRAL_NODE),
                    mutation_event_id: 0,
                    kind: LocusKind::Node {
                        role: NodeRole::Hidden,
                        activation_id: Activation::TanhApprox.id(),
                        bias: 0.0,
                        time_constant: 0,
                    },
                });
                chromosome.push(Locus {
                    homology_id: NEUTRAL_EDGE,
                    gene_lineage_id: u64::from(NEUTRAL_EDGE),
                    mutation_event_id: 0,
                    kind: LocusKind::Edge {
                        source: FOUNDER_INPUT,
                        target: NEUTRAL_NODE,
                        weight: 1.0,
                        flags: EDGE_FLAG_PLASTIC,
                        plasticity: PlasticityGenes {
                            rule_id: RULE_OBSERVATIONAL,
                            eta: 0.5,
                            // c*y + d: applies whenever the rule runs, so
                            // the arms discriminate on the gate alone. The
                            // social-term-not-presynaptic discrimination is
                            // unit-tested beside the registry.
                            coefficients: [0.0, 0.0, 1.0, 1.0],
                            decay: 0.0,
                            modulator_node: 0,
                        },
                    },
                });
            }
        }
    }

    fn rule5_world(observational: bool) -> World {
        let mut config = social_config(SEED);
        config.initial_organisms = 4;
        config.plasticity.enabled = true;
        config.social.observational_enabled = observational;
        config.validate().expect("valid");
        let world = World::new(config).expect("world");
        let mut state = world.export_state();
        let caps: GenomeCaps = state.config.genome2.caps;
        let schema2 = state.schema2.as_mut().expect("a schema-2 world");
        let mut rows = Vec::new();
        for index in 0..schema2.genomes.len() {
            let mut genome = Genome2::decode(&schema2.genomes[index], &caps).expect("decodes");
            add_rule5_edge(&mut genome);
            genome
                .validate_structure(&caps)
                .expect("the rewritten genome validates");
            rows.push(vec![LearnedEdgeSave {
                edge_homology_id: STRUCTURAL_HOMOLOGY_BASE + 9_000,
                learned_q16: 0,
                trace_q16: 0,
            }]);
            schema2.genomes[index] = genome.encode();
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
        if let Some(learn) = state.learn.as_mut() {
            learn.edges = rows;
        }
        World::from_state(state).expect("restores")
    }

    let mut p_arm = rule5_world(true);
    let mut s_arm = rule5_world(false);
    assert_ne!(
        p_arm.export_state().config.stable_hash(),
        s_arm.export_state().config.stable_hash(),
        "P and S are distinct replay lineages"
    );
    run(&mut p_arm, 30);
    run(&mut s_arm, 30);

    let p = p_arm.social_counters().unwrap();
    assert!(
        p.rule5_updates_total > 0,
        "the offered rule never ran: {p:?}"
    );
    let s = s_arm.social_counters().unwrap();
    assert_eq!(
        s.rule5_updates_total, 0,
        "condition S's ablation is verified by the counter: {s:?}"
    );
}

// --- gap closures from the independent mutation pass (D-123) ----------------

/// The commit arithmetic, pinned analytically: the first-tick stamp equals
/// `amplitude * (range^2 - d^2) / range^2` computed from first principles
/// (the amplitude recovered exactly from the emission bill), and the second
/// tick equals `clamp(first * retain >> 16 + stamp)`. An empirical
/// two-world calibration cannot pin this - add-then-decay is
/// observationally identical to a stamp rescaling, `(c + S) * k = c * k +
/// S * k`, clamp included, so only an absolute expectation catches it
/// (the independent pass's M06, which the fixture checksum caught and no
/// cargo test could).
#[test]
fn the_commit_is_decay_then_add_against_the_analytic_stamp() {
    let mut config = social_config(SEED);
    config.initial_organisms = 1;
    // Cost 1 milli per whole amplitude: the bill IS the amplitude, so the
    // analytic stamp needs no access to the private emission scratch.
    config.social.signal_cost_milli = 1;
    config.validate().expect("valid");
    let world = World::new(config.clone()).expect("world");
    let mut state = world.export_state();
    {
        let caps: GenomeCaps = state.config.genome2.caps;
        let schema2 = state.schema2.as_mut().expect("schema 2");
        let mut genome = Genome2::decode(&schema2.genomes[0], &caps).expect("decodes");
        bind_always_on(&mut genome, CHANNEL_SIGNAL_EMIT_BASE, 0.25, 0);
        bind_always_on(&mut genome, CHANNEL_REST, 1.0, 1);
        genome.validate_structure(&caps).expect("validates");
        schema2.genomes[0] = genome.encode();
        for _ in 0..2 {
            schema2.activation_values[0].push(0.0);
            schema2.activation_prior[0].push(0.0);
        }
    }
    let mut world = World::from_state(state).expect("restores");
    world.step();

    let table = world.social_table().expect("section on");
    let counters = world.social_counters().unwrap();
    let amplitude =
        ((counters.signal_cost_milli_total as i64) << 16) + table.emission_remainder_milli[0];
    assert!(amplitude > 0 && amplitude < 65_536, "a sub-whole amplitude");

    let after = world.export_state();
    let cell_fp = i64::from(config.cell_size_m as i32 * 1024);
    let cell_x = i64::from(after.x_fp[0]) / cell_fp;
    let cell_y = i64::from(after.y_fp[0]) / cell_fp;
    let cell = (cell_y * i64::from(config.cells_x) + cell_x) as usize;
    let channels = config.social.signal_channels as usize;
    let slot = cell * channels;

    // The analytic stamp at the emitter's own cell, the kernel's formula
    // recomputed from first principles: range scales with amplitude,
    // attenuation is 1 - (d/range)^2 over the cell-centre offset.
    let base_range_fp = i64::from(config.social.signal_base_range_m) * 1024;
    let range_fp = base_range_fp * amplitude >> 16;
    let range_squared = range_fp * range_fp;
    let centre_x = cell_x * cell_fp + cell_fp / 2;
    let centre_y = cell_y * cell_fp + cell_fp / 2;
    let dx = centre_x - i64::from(after.x_fp[0]);
    let dy = centre_y - i64::from(after.y_fp[0]);
    let d_squared = dx * dx + dy * dy;
    assert!(
        d_squared <= range_squared,
        "the emitter covers its own cell"
    );
    let stamp = amplitude * (range_squared - d_squared) / range_squared;

    let first = i64::from(table.committed_field_q16[slot]);
    assert_eq!(
        first, stamp,
        "the first commit must equal the analytic stamp exactly"
    );

    // Second tick: the organism rests in place, so the same stamp lands on
    // a decayed field.
    world.step();
    let second = i64::from(world.social_table().unwrap().committed_field_q16[slot]);
    let retain = i64::from(config.social.signal_retain_q16);
    let expected = ((first * retain) >> 16) + stamp;
    assert_eq!(
        second,
        expected.min(65_536),
        "the second commit must be clamp(decay(first) + stamp)"
    );
}

/// Two co-located emitters sum before the clamp: the committed value is the
/// clamp of the sum of the stamps each produces alone, never an overwrite
/// and never order-dependent.
#[test]
fn simultaneous_emitters_sum_before_the_clamp() {
    let mut config = social_config(SEED);
    config.initial_organisms = 2;
    let world = scripted_world(config.clone(), &[CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_REST]);
    let mut state = world.export_state();
    // Both on one exact spot, so their stamps land on identical cells.
    state.x_fp[1] = state.x_fp[0];
    state.y_fp[1] = state.y_fp[0];
    let cell = {
        let cell_fp = i64::from(config.cell_size_m as i32 * 1024);
        let x = i64::from(state.x_fp[0]) / cell_fp;
        let y = i64::from(state.y_fp[0]) / cell_fp;
        (y * i64::from(config.cells_x) + x) as usize
    };
    let channels = config.social.signal_channels as usize;
    let slot = cell * channels;

    // Solo worlds measure each founder's stamp alone (kill the other by
    // removing it from the pair world is not expressible; instead run the
    // one-founder config, whose founder is the same genome the pair's
    // founder 0 carries).
    let mut solo_config = social_config(SEED);
    solo_config.initial_organisms = 1;
    let mut solo = scripted_world(solo_config, &[CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_REST]);
    solo.step();
    let solo_stamp = i64::from(solo.social_table().unwrap().committed_field_q16[slot]);

    let mut pair = World::from_state(state).expect("restores");
    pair.step();
    let committed = i64::from(pair.social_table().unwrap().committed_field_q16[slot]);
    // The two founders' amplitudes may differ by a few Q16 steps (their
    // evolved networks differ), so the assertion is bounded rather than
    // exact: the pair's commit exceeds either alone and never exceeds the
    // clamp, and when the sum is over one whole it IS the clamp.
    assert!(
        committed > solo_stamp,
        "an emitter was overwritten: {committed} vs {solo_stamp}"
    );
    assert!(committed <= 65_536);
    if 2 * solo_stamp >= 65_536 + 4_096 {
        assert_eq!(committed, 65_536, "a saturating sum must commit the clamp");
    }
}

/// The emission bill is exact to the rational charge (D-094): over N ticks
/// at a constant amplitude, whole-milli charges plus the retained remainder
/// equal `cost * amplitude * N` in Q16 milli, to the bit.
#[test]
fn the_emission_bill_is_exact_to_the_rational_charge() {
    let mut config = social_config(SEED);
    config.initial_organisms = 1;
    // A cost whose product with a sub-whole amplitude is never a whole
    // number of milli, so the carry has to fire or the total drifts. The
    // emitter binds at gain 0.9 because a saturated node at gain 1.0
    // clamps to exactly one whole and the remainder never moves - which is
    // itself the condition the independent pass measured (its M08/M09
    // survived because every scripted amplitude was exactly 1.0).
    config.social.signal_cost_milli = 7;
    config.validate().expect("valid");
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    {
        let schema2 = state.schema2.as_mut().expect("schema 2");
        let mut genome = Genome2::decode(&schema2.genomes[0], &caps).expect("decodes");
        bind_always_on(&mut genome, CHANNEL_SIGNAL_EMIT_BASE, 0.9, 0);
        bind_always_on(&mut genome, CHANNEL_REST, 1.0, 1);
        genome.validate_structure(&caps).expect("validates");
        schema2.genomes[0] = genome.encode();
        for _ in 0..2 {
            schema2.activation_values[0].push(0.0);
            schema2.activation_prior[0].push(0.0);
        }
    }
    let mut world = World::from_state(state).expect("restores");
    let ticks = 400_u64;
    run(&mut world, ticks);
    let table = world.social_table().expect("section on");
    let counters = world.social_counters().unwrap();
    assert_eq!(
        counters.signals_emitted_total, ticks,
        "one emission per tick"
    );
    // The amplitude is constant (an always-on saturated node), so the exact
    // charge is N * cost * amplitude_q16, and what was billed plus what is
    // still carried must equal it exactly.
    let remainder = table.emission_remainder_milli[0];
    assert!(
        remainder > 0,
        "a 7-milli cost at a sub-whole amplitude must carry a live remainder"
    );
    let billed_q16 = (counters.signal_cost_milli_total as i64) << 16;
    let total_q16 = billed_q16 + remainder;
    // Reconstruct the per-tick scaled charge: total must divide evenly into
    // N equal per-tick contributions of cost * amplitude.
    assert_eq!(
        total_q16 % ticks as i64,
        0,
        "billed + carried must be exactly N identical per-tick charges: \
         billed {billed_q16}, carried {remainder}"
    );
    let per_tick = total_q16 / ticks as i64;
    assert_eq!(per_tick % 7, 0, "each tick's charge is cost * amplitude");
    let amplitude = per_tick / 7;
    assert!(
        (50_000..=60_000).contains(&amplitude),
        "the recovered amplitude is the gain-0.9 request: {amplitude}"
    );
}

/// Deaths and births keep every social per-organism array in lockstep: the
/// scenario the whole suite otherwise avoids (no test bred or starved
/// inside its window, so `retain` and `push_organism` never ran in cargo -
/// the pass's M25/M26 were caught only by the out-of-suite fixture).
#[test]
fn deaths_and_births_keep_the_social_arrays_in_lockstep() {
    let mut config = social_config(SEED);
    config.initial_organisms = 24;
    config.reproduction_enabled = true;
    config.physiology.enabled = true;
    // Heavy enough that deaths happen inside the window, light enough that
    // most founders outlive the 600-tick maturity age and breed.
    config.physiology.extrinsic_hazard_q16_per_s = 300;
    config.validate().expect("valid");
    let mut world = scripted_world(config, &[CHANNEL_SIGNAL_EMIT_BASE]);
    let mut deaths = 0_u64;
    let mut births = 0_u64;
    // Past the 600-tick maturity age, or nobody can breed at all.
    for _ in 0..2_000 {
        world.step();
        world.check_invariants().expect("lockstep holds every tick");
        for event in world.events() {
            match event.kind {
                sim_core::EventKind::Death { .. } | sim_core::EventKind::DeathByDamage { .. } => {
                    deaths += 1;
                }
                sim_core::EventKind::Birth { .. } | sim_core::EventKind::PairedBirth { .. } => {
                    births += 1;
                }
                _ => {}
            }
        }
    }
    assert!(deaths > 0, "the hazard never killed, so retain never ran");
    assert!(births > 0, "nobody bred, so push_organism never ran");
}

/// The four nearest conspecifics fill the slots in ascending distance, the
/// fifth is truncated after the sort, and an exact distance-squared tie
/// breaks by the lower organism id (Rule 5's form) - pinned through the
/// cue vector, which is the only surface the selection has.
#[test]
fn the_four_nearest_fill_the_slots_in_order_and_ties_break_by_id() {
    let mut config = social_config(SEED);
    config.initial_organisms = 6;
    let world = scripted_world(config, &[CHANNEL_REST]);
    let mut state = world.export_state();
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    // Five neighbours at distinct distances, nearest-to-farthest NOT in id
    // order, all inside the 8 m radius; the farthest still inside, so the
    // truncation has to happen after the sort to exclude exactly it.
    state.x_fp[1] = x + 4 * 1024;
    state.y_fp[1] = y;
    state.x_fp[2] = x + 1024;
    state.y_fp[2] = y;
    state.x_fp[3] = x + 6 * 1024;
    state.y_fp[3] = y;
    state.x_fp[4] = x + 2 * 1024;
    state.y_fp[4] = y;
    state.x_fp[5] = x + 3 * 1024;
    state.y_fp[5] = y;
    let mut world = World::from_state(state).expect("restores");
    world.step();
    let cues = world.social_perception_of(0).expect("row exists");
    // Slots hold organisms 2 (1 m), 4 (2 m), 5 (3 m), 1 (4 m); organism 3
    // (6 m) is truncated. Distance cue is 1 - d/8m.
    let expected = [1.0_f32, 2.0, 3.0, 4.0];
    for (slot, metres) in expected.iter().enumerate() {
        assert_eq!(cues[slot * 9], 1.0, "slot {slot} occupied");
        let distance_cue = cues[slot * 9 + 1];
        let want = 1.0 - metres / 8.0;
        assert!(
            (distance_cue - want).abs() < 0.02,
            "slot {slot}: distance cue {distance_cue} but the {metres} m \
             neighbour belongs here"
        );
    }

    // The tie: two organisms at exactly 2 m on opposite sides. The lower id
    // must take the nearer slot set; asserted by giving the higher id a
    // distinguishing contact record and checking slot order.
    let world = scripted_world(
        {
            let mut config = social_config(SEED);
            config.initial_organisms = 3;
            config
        },
        &[CHANNEL_REST],
    );
    let mut state = world.export_state();
    let (x, y) = (state.x_fp[0], state.y_fp[0]);
    state.x_fp[1] = x + 2 * 1024;
    state.y_fp[1] = y;
    state.x_fp[2] = x - 2 * 1024;
    state.y_fp[2] = y;
    // Mark the higher id's committed contact record so the slots are
    // tellable apart through the cue vector.
    state.social.as_mut().unwrap().prior_contact[2] = true;
    let mut world = World::from_state(state).expect("restores");
    world.step();
    let cues = world.social_perception_of(0).expect("row exists");
    assert_eq!(cues[0], 1.0);
    assert_eq!(cues[9], 1.0);
    assert_eq!(
        (cues[4], cues[13]),
        (0.0, 1.0),
        "an exact tie must put the lower id in slot 0: {cues:?}"
    );
}

/// Every `signal_in` channel reaches a bound node, including the last one:
/// the gather range that stops one channel short (the pass's M28) leaves
/// channel 62 reading zero forever, which only a genome bound to it can
/// see.
#[test]
fn every_signal_in_channel_reaches_a_bound_node() {
    use sim_core::CHANNEL_SIGNAL_IN_BASE;
    for channel in 0..4_u16 {
        let mut config = social_config(SEED);
        config.initial_organisms = 1;
        let world = World::new(config.clone()).expect("world");
        let mut state = world.export_state();
        let caps: GenomeCaps = state.config.genome2.caps;
        let schema2 = state.schema2.as_mut().expect("schema 2");
        let mut genome = Genome2::decode(&schema2.genomes[0], &caps).expect("decodes");
        // An Input-role node fed by the channel, linear so the activation
        // IS the gathered value.
        let node_id = STRUCTURAL_HOMOLOGY_BASE + 70_000;
        for haplotype in &mut genome.haplotypes {
            let chromosome = &mut haplotype.chromosomes[0];
            chromosome.push(Locus {
                homology_id: node_id,
                gene_lineage_id: u64::from(node_id),
                mutation_event_id: 0,
                kind: LocusKind::Node {
                    role: NodeRole::Input,
                    activation_id: Activation::Linear.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            });
            chromosome.push(Locus {
                homology_id: node_id + 1,
                gene_lineage_id: u64::from(node_id + 1),
                mutation_event_id: 0,
                kind: LocusKind::IoBinding {
                    node: node_id,
                    channel_id: CHANNEL_SIGNAL_IN_BASE + channel,
                    gain: 1.0,
                },
            });
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
        genome.validate_structure(&caps).expect("validates");
        schema2.genomes[0] = genome.encode();
        schema2.activation_values[0].push(0.0);
        schema2.activation_prior[0].push(0.0);
        // A committed field value on the organism's own cell, this channel.
        let cell = {
            let cell_fp = i64::from(config.cell_size_m as i32 * 1024);
            let x = i64::from(state.x_fp[0]) / cell_fp;
            let y = i64::from(state.y_fp[0]) / cell_fp;
            (y * i64::from(config.cells_x) + x) as usize
        };
        let channels = config.social.signal_channels as usize;
        state.social.as_mut().unwrap().committed_field_q16[cell * channels + channel as usize] =
            32_768;
        let mut world = World::from_state(state).expect("restores");
        world.step();
        let sensed = world.social_perception_of(0).expect("row")[36 + channel as usize];
        assert!(
            (sensed - 0.5).abs() < 0.02,
            "channel {channel}: the sense phase reads the committed field"
        );
        let after = world.export_state();
        let values = &after.schema2.as_ref().unwrap().activation_values[0];
        let bound = values[values.len() - 1];
        assert!(
            (bound - 0.5).abs() < 0.05,
            "channel {channel}: the gather must hand the bound node the \
             committed value, got {bound}"
        );
    }
}

/// Corruption noise is keyed per channel and per receiver: four channels
/// holding the same committed value read four different noisy values, and
/// two co-located receivers read different noise on the same channel. A
/// draw index pinned to one channel (the pass's M11) makes the four
/// channels' noise identical, which no state checksum can see - corrupted
/// cues are scratch unless a genome binds them - so the keying has to be
/// pinned at the sense surface.
#[test]
fn corruption_noise_is_keyed_per_channel_and_per_receiver() {
    let mut config = social_config(SEED);
    config.initial_organisms = 2;
    config.social.signal_corruption_q16 = 32_768;
    config.validate().expect("valid");
    let world = scripted_world(config.clone(), &[CHANNEL_REST]);
    let mut state = world.export_state();
    state.x_fp[1] = state.x_fp[0];
    state.y_fp[1] = state.y_fp[0];
    let cell = {
        let cell_fp = i64::from(config.cell_size_m as i32 * 1024);
        let x = i64::from(state.x_fp[0]) / cell_fp;
        let y = i64::from(state.y_fp[0]) / cell_fp;
        (y * i64::from(config.cells_x) + x) as usize
    };
    let channels = config.social.signal_channels as usize;
    {
        let social = state.social.as_mut().unwrap();
        for channel in 0..channels {
            social.committed_field_q16[cell * channels + channel] = 32_768;
        }
    }
    let mut world = World::from_state(state).expect("restores");
    world.step();
    let first = world.social_perception_of(0).expect("row");
    let second = world.social_perception_of(1).expect("row");
    let received: Vec<f32> = (0..channels).map(|c| first[36 + c]).collect();
    assert!(
        received.windows(2).any(|pair| pair[0] != pair[1]),
        "equal committed values must read differently across channels or \
         the draw index is not the channel: {received:?}"
    );
    assert!(
        (0..channels).any(|c| first[36 + c] != second[36 + c]),
        "two receivers on one cell must read different noise or the draw \
         subject is not the receiver"
    );
}

/// The metrics snapshot's social block is the table's counters, not a
/// parallel account: every field equals its counter, the
/// perceived-neighbours gauge equals a hand count of present cues from the
/// same perception rows the controllers read, and a social-disabled world
/// reports `social_enabled` false with every field zero - the server
/// renders the block only on that flag, so this is the gate the metrics
/// endpoint stands on.
#[test]
fn the_metrics_snapshot_mirrors_the_social_counters_and_only_when_enabled() {
    let mut config = social_config(SEED);
    config.initial_organisms = 6;
    let world = scripted_world(config, &[CHANNEL_SIGNAL_EMIT_BASE]);
    // Co-locate everyone so perception slots actually fill.
    let mut state = world.export_state();
    for index in 1..state.x_fp.len() {
        state.x_fp[index] = state.x_fp[0] + index as i32;
        state.y_fp[index] = state.y_fp[0];
    }
    let mut world = World::from_state(state).expect("restores");
    for _ in 0..3 {
        world.step();
    }
    let metrics = world.metrics();
    let counters = world.social_counters().expect("section on");
    assert!(metrics.social_enabled);
    assert_eq!(
        metrics.signals_emitted_total,
        counters.signals_emitted_total
    );
    assert_eq!(
        metrics.signal_cost_milli_total,
        counters.signal_cost_milli_total
    );
    assert_eq!(
        metrics.perception_faults_total,
        counters.perception_faults_total
    );
    assert_eq!(
        metrics.corruption_draws_total,
        counters.corruption_draws_total
    );
    assert_eq!(
        metrics.scrambled_deliveries_total,
        counters.scrambled_deliveries_total
    );
    assert_eq!(metrics.rule5_updates_total, counters.rule5_updates_total);
    assert!(
        metrics.signals_emitted_total > 0,
        "non-vacuity: the scripted trace emits"
    );
    let by_hand: u64 = (0..world.population())
        .map(|index| {
            let cues = world.social_perception_of(index).expect("row exists");
            (0..4).filter(|slot| cues[slot * 9] > 0.5).count() as u64
        })
        .sum();
    assert_eq!(metrics.perceived_neighbours, by_hand);
    assert!(by_hand > 0, "non-vacuity: co-located organisms perceive");

    let plain = World::new(SimConfig::phase1_default(SEED)).expect("plain world");
    let metrics = plain.metrics();
    assert!(!metrics.social_enabled);
    assert_eq!(metrics.signals_emitted_total, 0);
    assert_eq!(metrics.signal_cost_milli_total, 0);
    assert_eq!(metrics.perceived_neighbours, 0);
}
