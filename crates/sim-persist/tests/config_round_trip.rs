//! The snapshot config section must round-trip **every** field.
//!
//! This test exists because it did not, and nothing noticed. Format 1
//! encoded the config only as far as Phase 2, so the climate, origin,
//! contest, and physiology sections added by Phases 6, 7, and 8 were
//! silently dropped on save and restored at their defaults. For climate and
//! contest that surfaced as a confusing restore failure; for the origin
//! section, which has no presence check, a `seeded` world would have
//! restored as `random` with **no error at all**.
//!
//! The structural defence is below: perturb every field away from its
//! default, round-trip, and compare the whole `SimConfig` with one
//! `assert_eq!`. A field added later and not encoded fails here
//! immediately, which a field-by-field test would not.

use sim_core::{OriginMode, SimConfig, WorldgenVersion};
use sim_persist::{decode_snapshot, encode_snapshot};

/// A config in which no field holds its default value, so a dropped field
/// cannot coincidentally compare equal.
fn perturbed() -> SimConfig {
    let mut config = SimConfig::phase2_default(0xabcd_ef01_2345_6789);
    config.cells_x = 96;
    config.cells_y = 112;
    config.cell_size_m = 5;
    config.initial_organisms = 77;
    config.max_entities = 4_321;
    config.dt_ms = 120;
    config.growth_rate_q16_per_s = 4_100;
    config.cell_capacity_milli = 31_500;
    config.maturity_age_ticks = 611;
    config.max_age_ticks = 35_111;
    config.phase2.pairing_range_m = 7;
    config.phase2.cluster_sample_max = 999;

    config.climate.enabled = true;
    config.climate.worldgen_version = WorldgenVersion::V2;
    config.climate.base_temperature_milli = 21_111;
    config.climate.lapse_milli_per_full_elevation = 1_234;
    config.climate.latitude_amplitude_milli = 27_777;
    config.climate.season_period_ticks = 9_999;
    config.climate.season_amplitude_milli = 5_555;
    // Drift periods must exceed the season period; validation says so.
    config.climate.drift_period_ticks = [90_001, 180_002, 360_003];
    config.climate.drift_amplitude_milli = [441, 551, 661];
    config.climate.temperature_min_milli = -19_000;
    config.climate.temperature_max_milli = 61_000;
    config.climate.initial_moisture_milli = 119_000;
    config.climate.coastal_moisture_bonus_milli = 777;
    // The ceiling must not sit below the maximum.
    config.climate.moisture_max_milli = 188_000;
    config.climate.moisture_ceiling_milli = 199_000;
    config.climate.sea_proximity_weight_q16 = 39_000;
    config.climate.moisture_diffusion_q16 = 3_111;
    config.climate.moisture_drain_weight = 3;
    config.climate.highland_elevation_q16 = 30_111;
    config.climate.wetland_moisture_milli = 104_000;
    config.climate.arid_moisture_milli = 54_000;
    config.climate.forest_moisture_milli = 89_000;
    config.climate.forest_min_temperature_milli = 2_222;
    for (index, value) in config.climate.biome_capacity_q16.iter_mut().enumerate() {
        // Water must stay at exactly zero: nothing grows in the sea, and
        // validation enforces it.
        *value = if index == 0 { 0 } else { 60_000 + index as u32 };
    }
    config.climate.reclassify_interval_ticks = 111;

    config.origin.mode = OriginMode::Seeded;
    config.origin.trait_low_q16 = 16_001;
    config.origin.trait_span_q16 = 32_002;
    config.origin.neural_span_q16 = 32_003;
    config.origin.deme_count = 3;
    config.origin.deme_radius_m = 127;
    config.origin.deme_min_separation_m = 191;
    config.origin.deme_trait_spread_q16 = 6_553;
    config.origin.archetype_count = 2;
    for (index, archetype) in config.origin.archetypes.iter_mut().enumerate() {
        archetype.id = index as u16 + 1;
        for (gene, mean) in archetype.trait_mean_q16.iter_mut().enumerate() {
            *mean = (1_000 + index * 100 + gene) as u16;
        }
        archetype.trait_spread_q16 = 4_000 + index as u16;
        archetype.neural_spread_q16 = 5_000 + index as u16;
        archetype.biome_affinity = 0b0101_0101;
    }

    config.contest.enabled = true;
    config.contest.base_health_milli = 10_101;
    config.contest.damage_base_milli = 1_201;
    config.contest.damage_variance_q16 = 16_385;
    config.contest.attack_cost_milli = 121;
    config.contest.attack_range_m = 4;
    config.contest.attack_threshold_q16 = -1_234;
    config.contest.attack_cooldown_ticks = 11;
    config.contest.heal_milli_per_s = 61;
    config.contest.heal_energy_cost_q16 = 131_073;
    config.contest.heal_energy_floor_q16 = 32_769;
    config.contest.damage_decay_q16_per_s = 6_555;
    config.contest.carcass_energy_q16 = 45_876;
    config.contest.carcass_decay_q16_per_s = 3_278;
    config.contest.carcass_reach_m = 3;
    config.contest.max_carcasses = 4_097;
    config.contest.local_depletion_milli = 7;

    config.physiology.enabled = true;
    config.physiology.allometry_enabled = false;
    config.physiology.basal_exponent_quarters = 5;
    config.physiology.thermoregulation_enabled = false;
    config.physiology.thermal_pref_low_milli = 1_111;
    config.physiology.thermal_pref_high_milli = 41_111;
    config.physiology.thermal_neutral_band_milli = 6_001;
    config.physiology.thermal_cost_milli_per_s_per_degree = 5;
    config.physiology.senescence_enabled = false;
    config.physiology.senescence_onset_ticks = 6_001;
    config.physiology.senescence_scale_ticks = 12_001;
    config.physiology.senescence_power = 3;
    config.physiology.senescence_hazard_q16_per_s = 656;
    config.physiology.extrinsic_hazard_q16_per_s = 14;
    config.physiology.juvenile_hazard_multiplier_q16 = 3 * 65_536;
    // Phase 9 genome 2. **The whole section was missing from this list**,
    // which is the same defect as the morphology one below wearing different
    // clothes: every genome2 field sat at its default, so a field dropped by
    // the codec compared default-to-default and round-tripped "successfully".
    // `regulatory_enabled` - C10.3's entire control - was undefended for two
    // phases, and `plasticity_enabled` was silently dropped on save when it
    // was added, which would have turned a plasticity treatment run into a
    // control across any checkpoint. Perturb every field, including both
    // gates, and set each away from its default so absence is detectable.
    config.genome2.enabled = true;
    config.genome2.caps.max_chromosomes = 3;
    config.genome2.caps.max_loci_per_chromosome = 151;
    config.genome2.caps.max_nodes = 149;
    config.genome2.caps.max_edges = 147;
    config.genome2.caps.max_edges_per_node = 29;
    config.genome2.caps.max_genome_bytes = 15_360;
    config.genome2.caps.min_nodes = 3;
    config.genome2.meiosis.max_extra_crossovers = 2;
    config.genome2.mutation.point_q16 = 5_555;
    config.genome2.mutation.duplication_q16 = 777;
    config.genome2.mutation.deletion_q16 = 555;
    config.genome2.mutation.insertion_q16 = 333;
    config.genome2.mutation.transposition_q16 = 222;
    config.genome2.mutation.max_run = 4;
    config.genome2.mutation.point_delta_q16 = 4_444;
    config.genome2.mutation.regulatory_enabled = false;
    config.genome2.mutation.plasticity_enabled = true;
    // Phase 10 morphology. Added because the first version of this test
    // predated the section and therefore passed while the morphology config
    // was silently dropped on save - a restored world came back with
    // morphology disabled and no bodies at all. A round-trip test only
    // defends the fields it actually perturbs.
    config.morphology.enabled = true;
    config.morphology.lattice = sim_core::LatticeKind::Hex;
    config.morphology.base_node_budget = 11;
    config.morphology.caps.max_modules = 37;
    config.morphology.caps.lattice_radius = 5;
    config.morphology.caps.max_growth_steps = 9;
    config.morphology.caps.required_types_mask = 1 << 3;
    // Phase 11 plasticity. **All four fields, including the two nobody would
    // think to check.** This is the fourth section added to `SimConfig` and
    // the third time the codec has been extended without this list being
    // extended with it, most recently for `genome2.mutation.plasticity_enabled`
    // in this same phase. The consequence here is the same one that block
    // records: a plasticity treatment run that is checkpointed and resumed
    // comes back as its own control, and the analysis reports "plasticity was
    // not selected for".
    //
    // `lamarckian_fraction_q16` is perturbed to a value `SimConfig::validate`
    // refuses, and that is deliberate: this test never validates - it
    // substitutes the config into a carrier world's state and exercises the
    // codec - and a field defended only at its legal value is a field whose
    // encoding is defended by one bit pattern that happens to be zero. The
    // day a Lamarckian condition is implemented, this list already covers it.
    config.plasticity.enabled = true;
    config.plasticity.plastic_edge_cost_milli_per_s = 37;
    config.plasticity.max_plastic_edges = 19;
    config.plasticity.lamarckian_fraction_q16 = 12_345;
    config
}

/// Rewrite every founder's genome so both of its edges are plastic, and
/// rewrite the learn section to match.
///
/// Duplicated in spirit from `sim-core`'s `phase11_learning.rs` because
/// `sim-persist` cannot reach that crate's test helpers, and duplicated
/// deliberately rather than moved into `sim-core` proper: authoring a plastic
/// genome is a test fixture, not a production path. Nothing in the engine
/// writes `EDGE_FLAG_PLASTIC` - only point mutation can, over generations -
/// which is exactly why a test that waited for evolution to produce one would
/// be slow and seed-dependent.
///
/// The learn rows have to be rewritten too. The exported world had no plastic
/// edge anywhere, so every row is empty; `World::from_state` recompiles the
/// plans from the rewritten genomes and refuses a section that does not name
/// the edges those plans mark plastic. That refusal is the point of the edge
/// ids, so the fixture satisfies it rather than working around it.
fn plastic_world(mut config: SimConfig, eta: f32) -> sim_core::World {
    config.genome2.enabled = true;
    config.genome2.mutation.plasticity_enabled = true;
    config.plasticity.enabled = true;
    let world = sim_core::World::new(config).expect("world builds");
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let budget = state.config.plasticity_budget();
    let mut rows = Vec::new();
    let schema2 = state.schema2.as_mut().expect("a schema-2 world");
    for encoded in schema2.genomes.iter_mut() {
        let mut genome = sim_core::Genome2::decode(encoded, &caps).expect("a live genome decodes");
        for haplotype in &mut genome.haplotypes {
            for chromosome in &mut haplotype.chromosomes {
                for locus in chromosome.iter_mut() {
                    if let sim_core::LocusKind::Edge {
                        flags, plasticity, ..
                    } = &mut locus.kind
                    {
                        *flags |= sim_core::EDGE_FLAG_PLASTIC;
                        *plasticity = sim_core::PlasticityGenes {
                            rule_id: sim_core::RULE_HEBBIAN,
                            eta,
                            // a = 1: the delta is exactly x*y, so "did
                            // anything learn" is not a question about
                            // coefficient cancellation.
                            coefficients: [1.0, 0.0, 0.0, 0.0],
                            decay: 0.0,
                            modulator_node: 0,
                        };
                    }
                }
            }
        }
        rows.push(
            sim_core::compile_network_with_budget(&genome.express_network(), budget)
                .expect("a rewritten genome compiles")
                .plastic_edges
                .iter()
                .map(|edge| sim_core::LearnedEdgeSave {
                    edge_homology_id: edge.homology_id,
                    learned_q16: 0,
                    trace_q16: 0,
                })
                .collect::<Vec<_>>(),
        );
        *encoded = genome.encode();
    }
    state
        .learn
        .as_mut()
        .expect("a plasticity save section")
        .edges = rows;
    let world = sim_core::World::from_state(state).expect("the rewritten genomes restore");
    assert!(
        world
            .learned_census()
            .iter()
            .all(|sample| sample.plastic_edges == 2),
        "the rewrite did not reach the compiled plans"
    );
    world
}

/// A plasticity world run until its learned deltas are nonzero.
///
/// The guard is not decoration. A round trip over a world whose learned array
/// is all zeros is a round trip over zeros: it would pass for a codec that
/// wrote the section and never read it back, for one that read it back and
/// zeroed it, and for one that never wrote it at all.
fn learned_world(seed: u64, ticks: u64) -> sim_core::World {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    config.cell_capacity_milli = 240_000;
    // Off the shipped default of 2, which at a 100 ms tick truncates to a
    // **zero** per-tick charge - so `LearnSaveState::cost_milli` would sit at
    // zero and a codec that dropped it would round-trip "successfully". 20
    // milli/s is 2 milli per tick per edge, which is visible in the record.
    config.plasticity.plastic_edge_cost_milli_per_s = 20;
    let mut world = plastic_world(config, 0.01);
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    let learned: i64 = world
        .learned_census()
        .iter()
        .map(|sample| sample.sum_abs_learned_q16)
        .sum();
    assert!(
        learned > 0,
        "nothing was learned, so a save round trip over this world would be a \
         check on an all-zero array"
    );
    assert!(world.population() > 20, "too few organisms survived");
    world
}

#[test]
fn every_config_field_survives_a_snapshot_round_trip() {
    let config = perturbed();
    // Guard the guard: if this config were somehow the default, dropping a
    // field would compare equal and the test would pass while proving
    // nothing.
    assert_ne!(config, SimConfig::phase2_default(config.world_seed));

    // The state comes from a world that generates cleanly; the *config*
    // under test is then substituted in. This is deliberately a codec test
    // and not a world-generation test: some perturbations above make an
    // ecologically degenerate map, which world generation rightly refuses,
    // and refusing to encode them would leave those fields unchecked.
    let carrier = sim_core::World::new(SimConfig::phase2_default(config.world_seed))
        .expect("carrier world builds");
    let mut state = carrier.export_state();
    state.config = config;
    let bytes = encode_snapshot(
        &state,
        1,
        0,
        carrier.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode");

    assert_eq!(
        decoded.config, config,
        "a config field did not survive the round trip"
    );
    // The hash is derived from the config, so this is redundant if the
    // struct compares equal -- and it is the check a reader will actually
    // trust, because it is what a replay depends on.
    assert_eq!(decoded.config.stable_hash(), config.stable_hash());
}

#[test]
fn a_restored_world_of_every_section_continues_identically() {
    // The end-to-end property the round trip exists for: encode, decode,
    // rebuild, and keep ticking without diverging.
    // A config that is both non-default in every section and actually
    // generates a world: the climate thresholds stay at their calibrated
    // values, because perturbing those makes a map with no `Arid` cells
    // that world generation correctly refuses.
    // Seed 3 rather than an arbitrary one: the Phase 6 record measured
    // that roughly a quarter of seeds produce a map with no `Arid` cells at
    // 256x256, which world generation refuses.
    let mut config = SimConfig::phase2_default(3);
    config.initial_organisms = 200;
    config.climate.enabled = true;
    config.climate.worldgen_version = WorldgenVersion::V2;
    config.climate.reclassify_interval_ticks = 111;
    config.origin.mode = OriginMode::Random;
    config.origin.deme_count = 3;
    config.origin.deme_radius_m = 127;
    config.contest.enabled = true;
    config.contest.attack_range_m = 4;
    config.contest.attack_cooldown_ticks = 11;
    config.physiology.enabled = true;
    config.physiology.allometry_enabled = true;
    config.physiology.basal_exponent_quarters = 3;
    config.physiology.thermoregulation_enabled = true;
    config.physiology.senescence_enabled = true;
    config.physiology.senescence_onset_ticks = 6_001;
    config.physiology.extrinsic_hazard_q16_per_s = 14;

    let mut original = sim_core::World::new(config).expect("world builds");
    for _ in 0..600 {
        original.step();
    }
    let bytes = encode_snapshot(
        &original.export_state(),
        1,
        0,
        original.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        Some(3),
    )
    .expect("encode");
    let (_, state) = decode_snapshot(&bytes).expect("decode");
    let mut restored = sim_core::World::from_state(state).expect("restore");
    assert_eq!(restored.state_checksum(), original.state_checksum());

    for _ in 0..400 {
        original.step();
        restored.step();
    }
    assert_eq!(
        original.state_checksum(),
        restored.state_checksum(),
        "a restored world diverged from the one it was captured from"
    );
    assert!(original.population() > 0, "the world went extinct");
}

/// A schema-2 world must survive the **codec**, not merely `export_state`
/// followed by `from_state`.
///
/// The Phase 9 world test checked the logical path and passed while the
/// encoded path was broken the whole time: the Phase 2 section drove its
/// per-organism loop from `traits.len()`, which is zero in a schema-2 world,
/// so heading, speed, turn, parents, depth, child count, birth tick and
/// memory were all dropped on write. Restore failed closed on a length
/// mismatch rather than corrupting, but the effect was that a schema-2 world
/// could not be checkpointed at all.
///
/// The organism count and the flat-genome count are different numbers and
/// this is the test that says so.
#[test]
fn a_schema_2_world_round_trips_through_the_codec() {
    let mut config = sim_core::SimConfig::phase2_default(0x9107);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 4_000;
    config.cell_capacity_milli = 240_000;
    config.genome2.enabled = true;
    config.genome2.mutation.duplication_q16 = 6_554;

    let mut world = sim_core::World::new(config).expect("world");
    // Long enough that structure has diversified and organisms are moving,
    // so the dropped fields would actually differ from their defaults.
    for _ in 0..3_000 {
        world.step();
    }
    assert!(world.population() > 0);
    assert!(
        world.metrics().distinct_structures > 1,
        "structure never diversified, so this would not exercise the section"
    );
    let checksum = world.state_checksum();
    let state = world.export_state();
    let phase2 = state.phase2.as_ref().expect("phase 2 state");
    assert!(
        phase2.traits.is_empty() && !phase2.heading_bam.is_empty(),
        "the premise of this test is that the two counts differ: traits={} heading={}",
        phase2.traits.len(),
        phase2.heading_bam.len()
    );

    let encoded =
        sim_persist::encode_snapshot(&state, 1, 0, checksum, "test", 0, None).expect("encode");
    let (_, decoded) = sim_persist::decode_snapshot(&encoded).expect("decode");
    let restored = sim_core::World::from_state(decoded).expect("restore");
    assert_eq!(restored.state_checksum(), checksum);

    // ...and it must keep stepping identically, because a checksum match at
    // rest would not catch a field that only matters once the world moves.
    let mut original = world;
    let mut restored = restored;
    for _ in 0..300 {
        original.step();
        restored.step();
    }
    assert_eq!(original.state_checksum(), restored.state_checksum());
}

/// A plasticity world must survive the **codec**, with learned deltas that
/// are actually nonzero.
///
/// The whole-world round trips above run schema-1 and schema-2 worlds with no
/// plasticity section, so before this test the learn section could have been
/// dropped on write and nothing would have noticed - the same shape of hole
/// that let a schema-2 world be uncheckpointable for two phases while a
/// logical-path test passed.
///
/// This is C11.3's positive half at the encoded level: save, restore, and
/// continue is bit-identical with plastic edges carrying nonzero learned
/// state. The negative half - that the state is load-bearing rather than
/// reconstructible - is the corruption test below.
#[test]
fn a_plasticity_world_round_trips_through_the_codec_with_nonzero_learned_deltas() {
    let mut original = learned_world(0x11a5, 600);
    let checksum = original.state_checksum();
    let state = original.export_state();

    // Asserted on the record, not inferred: a section that is present but
    // empty, or present and all zero, would make everything below vacuous.
    let learn = state
        .learn
        .as_ref()
        .expect("a plasticity world saves learning");
    assert_eq!(learn.edges.len(), original.population());
    let nonzero = learn
        .edges
        .iter()
        .flatten()
        .filter(|edge| edge.learned_q16 != 0)
        .count();
    assert!(nonzero > 20, "only {nonzero} stored deltas are nonzero");

    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode");
    // The decoded *record* equals the encoded one, edge ids and all. A
    // checksum comparison after restore would also pass for a codec that
    // dropped the ids and let `from_state` re-derive them from the plan,
    // which is exactly the misalignment the ids exist to refuse.
    assert_eq!(
        decoded.learn.as_ref(),
        state.learn.as_ref(),
        "the learn section did not survive the encoded round trip"
    );

    let mut restored = sim_core::World::from_state(decoded).expect("restore");
    assert_eq!(restored.state_checksum(), checksum);
    assert_eq!(restored.learned_census(), original.learned_census());

    // ...and it keeps stepping identically, because a checksum match at rest
    // would not catch state that only matters once the world moves.
    for _ in 0..300 {
        original.step();
        restored.step();
    }
    assert_eq!(
        original.state_checksum(),
        restored.state_checksum(),
        "a restored plasticity world diverged from the one it was captured from"
    );
    assert!(original.population() > 0, "the world went extinct");
}

/// C11.3's demand in full: corrupt the saved learned state and watch the
/// trajectory diverge.
///
/// Corruption is injected at the **logical `SaveState` level and re-encoded**,
/// not by flipping bytes in the file: every section carries a CRC32 and the
/// payload carries another, so a byte flip is caught by the framing and
/// proves only that the CRC works. Substituting into a carrier state and
/// re-encoding is the pattern
/// `every_config_field_survives_a_snapshot_round_trip` already uses, for the
/// same reason.
///
/// The corruption is deliberately **legal**: a value well inside the clamp,
/// on an edge the rebuilt plan really does have plastic. A restore that
/// failed closed here would prove nothing about whether learned state is
/// read - only that validation works, which the refusal tests in `sim-core`
/// already cover. It has to be accepted and then matter.
#[test]
fn corrupting_the_saved_learned_state_diverges_the_restored_trajectory() {
    let original = learned_world(0x11a5, 600);
    let checksum = original.state_checksum();
    let state = original.export_state();

    let encode = |state: &sim_core::SaveState| {
        encode_snapshot(state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None).expect("encode")
    };
    let restore = |bytes: &[u8]| {
        let (_, decoded) = decode_snapshot(bytes).expect("decode");
        sim_core::World::from_state(decoded).expect("restore")
    };

    let mut clean = restore(&encode(&state));
    assert_eq!(
        clean.state_checksum(),
        checksum,
        "the control restores exactly"
    );

    let mut corrupt = state.clone();
    {
        let learn = corrupt.learn.as_mut().expect("section");
        // Every organism, so the divergence cannot be attributed to one
        // organism dying for an unrelated reason, and a fixed offset rather
        // than a random one so the corruption is reproducible. Half the clamp
        // is far outside anything 600 ticks at eta 0.01 produced, and well
        // inside what `from_state` accepts.
        for row in learn.edges.iter_mut() {
            for edge in row.iter_mut() {
                edge.learned_q16 = sim_core::LEARN_LIMIT_Q16 / 2;
            }
        }
    }
    assert_ne!(
        corrupt.learn, state.learn,
        "the corruption is a no-op, so this test compares a value with itself"
    );
    let mut corrupted = restore(&encode(&corrupt));
    assert_ne!(
        corrupted.state_checksum(),
        checksum,
        "corrupted learned state did not reach the state checksum"
    );

    // The claim C11.3 actually makes: the corrupted world *behaves*
    // differently, so learned state is world state rather than a hashed
    // decoration that a restore could have recomputed from the genome. Both
    // worlds have identical genomes, identical positions at restore, and
    // identical everything else - the learned array is the only difference.
    let before = clean.export_state();
    assert_eq!(
        before.x_fp,
        corrupted.export_state().x_fp,
        "the two worlds start together"
    );
    for _ in 0..300 {
        clean.step();
        corrupted.step();
    }
    assert!(
        clean.population() > 10,
        "the control died before it could diverge"
    );
    assert_ne!(
        clean.export_state().x_fp,
        corrupted.export_state().x_fp,
        "corrupting every organism's learned state changed no trajectory, so \
         the learned state in the snapshot is not read by the tick"
    );
}

/// A world where **no organism is plastic** must still frame every organism.
///
/// This is D-076's trap and the reason the learn section's per-organism loop
/// is driven by the organism count rather than by anything edge-shaped. The
/// Phase 2 section carries the scar: its loop ran over `traits.len()`, which
/// is the organism count in a schema-1 world and zero in a schema-2 world, so
/// a schema-2 snapshot encoded no per-organism records at all and a schema-2
/// world could not be checkpointed for two phases.
///
/// The mirror image here is not an edge case, it is the phase's **predicted**
/// outcome: under `E-stationary`, C11.2 predicts plasticity is selected down,
/// and a world where every organism has zero plastic edges is what that looks
/// like. A section framed by edges would encode "no organisms" in that world,
/// lose every per-organism fault count with it, and fail closed on restore
/// with a length mismatch that named the wrong thing.
#[test]
fn a_world_where_no_organism_is_plastic_still_frames_every_organism() {
    let mut config = SimConfig::phase11_default(0x11a5_0000);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    // No genome rewrite: founders are `minimal_founder`, whose edges are not
    // flagged, and 200 ticks is far too short for point mutation to find the
    // flag. So every learned row is legitimately empty.
    let mut world = sim_core::World::new(config).expect("world");
    for _ in 0..200 {
        world.step();
    }
    let population = world.population();
    assert!(population > 0);

    let state = world.export_state();
    let learn = state.learn.as_ref().expect("the section is present");
    assert!(
        learn.edges.iter().all(|row| row.is_empty()),
        "an organism became plastic, so this world is not the empty case"
    );

    let bytes = encode_snapshot(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode");
    let decoded_learn = decoded.learn.as_ref().expect("the section survived");
    assert_eq!(
        decoded_learn.edges.len(),
        population,
        "the per-organism framing was driven by the plastic-edge count"
    );
    assert_eq!(decoded_learn.faults.len(), population);
    assert!(decoded_learn.edges.iter().all(|row| row.is_empty()));

    let restored = sim_core::World::from_state(decoded).expect("restore");
    assert_eq!(restored.state_checksum(), world.state_checksum());
}

/// Offsets of every section body in an encoded snapshot, walked from the
/// payload start rather than searched for, so this cannot match a tag-shaped
/// value inside a genome.
///
/// Returns `(tag, body_start, body_len)`. Uncompressed snapshots only, which
/// is why every caller passes `None` for the compression level.
fn sections(bytes: &[u8], payload_start: usize) -> Vec<(u16, usize, usize)> {
    let mut out = Vec::new();
    let mut offset = payload_start;
    while offset + 12 <= bytes.len() {
        let tag = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let length =
            u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap()) as usize;
        out.push((tag, offset + 12, length));
        offset += 12 + length + 4;
    }
    out
}

/// Re-seal a patched snapshot: recompute the section CRC that covers
/// `(body_start, body_len)` and the payload CRC in the header.
///
/// Without this every "rejection" below would be a CRC failure and the test
/// would prove that CRC32 works - which `persistence.rs`'s corruption sweep
/// already does. What is under test here is the **parser's** bound.
fn reseal(bytes: &mut [u8], payload_start: usize, body_start: usize, body_len: usize) {
    let body = bytes[body_start..body_start + body_len].to_vec();
    let section_crc = sim_persist::crc32(&body);
    bytes[body_start + body_len..body_start + body_len + 4]
        .copy_from_slice(&section_crc.to_le_bytes());
    let payload = bytes[payload_start..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    // payload_crc32 sits at header offset 84; see the module header in
    // `codec.rs` for the fixed 112-byte layout.
    bytes[84..88].copy_from_slice(&payload_crc.to_le_bytes());
}

/// Every declared count in a snapshot is bounded against the section body
/// **before** anything is allocated, and an unrepresentable count is refused
/// rather than panicking.
///
/// D-075 says a decode-time length check bounds an allocation and never
/// encodes a field count. The bound was written as
/// `count.checked_mul(size) > Some(body.len() as u64)` in five sections, and
/// that spelling admits exactly the counts it exists to refuse: `checked_mul`
/// returns `None` on overflow and `None > Some(_)` is **false**, so a count
/// of `u64::MAX` passed the guard and reached `Vec::with_capacity`, which
/// aborts the process with a capacity overflow. A loader that panics on
/// hostile input has failed open into a crash - the caller never sees the
/// typed error it is supposed to handle.
///
/// This test found that, in the Phase 11 section it was written for, and the
/// climate, contest, physiology and schema-2 sections had it too. It now
/// sweeps **every** section whose body begins with a count, on two worlds
/// that between them carry all of them, so the next section to be added is
/// covered by an existing test rather than by a new one nobody writes.
///
/// `persistence.rs`'s 2,000-flip corruption sweep did not catch this: a
/// handful of flipped bits does not produce a count near 2^61, and a panic
/// aborts a sweep rather than counting as a rejection.
#[test]
fn every_declared_count_is_bounded_before_allocation() {
    // Sections whose body starts with a `u64` count. The world-meta, ledger,
    // config and morphology sections start with data, not a count, and are
    // skipped by tag rather than by luck.
    const COUNT_LED: [u16; 8] = [3, 4, 6, 7, 8, 9, 10, 12];

    let mut worlds = Vec::new();
    // A plasticity world: organisms, biomass, phase 2, schema 2, learn.
    worlds.push(learned_world(0x11a5, 200));
    // ...and one carrying climate, contest and physiology, which the
    // plasticity world does not. Seed 3 and the calibrated climate
    // thresholds, for the reason
    // `a_restored_world_of_every_section_continues_identically` gives: about
    // a quarter of seeds produce a map with no `Arid` cells, which world
    // generation correctly refuses.
    let mut config = SimConfig::phase2_default(3);
    config.initial_organisms = 120;
    config.climate.enabled = true;
    // Climate requires the v2 generator; v1 refuses to build a climate world
    // at all, which is the same pairing the round-trip test above sets.
    config.climate.worldgen_version = WorldgenVersion::V2;
    config.contest.enabled = true;
    config.physiology.enabled = true;
    let mut ecology = sim_core::World::new(config).expect("world builds");
    for _ in 0..200 {
        ecology.step();
    }
    worlds.push(ecology);

    let mut seen: Vec<u16> = Vec::new();
    for world in &worlds {
        let state = world.export_state();
        let bytes = encode_snapshot(
            &state,
            1,
            0,
            world.state_checksum(),
            sim_persist::BUILD_VERSION,
            0,
            None,
        )
        .expect("encode");
        let payload_start = 112 + sim_persist::BUILD_VERSION.len();
        assert!(
            decode_snapshot(&bytes).is_ok(),
            "the unpatched snapshot must decode, or every refusal below is vacuous"
        );

        for (tag, body_start, body_len) in sections(&bytes, payload_start) {
            if !COUNT_LED.contains(&tag) || body_len < 8 {
                continue;
            }
            if !seen.contains(&tag) {
                seen.push(tag);
            }
            // `u64::MAX` is the overflow case; `u64::MAX / 8` overflows only
            // for the wider record sizes, so it catches a bound that special-
            // cases `u64::MAX`; `body_len` is a count that multiplies without
            // overflowing and still cannot fit.
            for count in [u64::MAX, u64::MAX / 8, body_len as u64] {
                let mut patched = bytes.clone();
                patched[body_start..body_start + 8].copy_from_slice(&count.to_le_bytes());
                reseal(&mut patched, payload_start, body_start, body_len);
                assert!(
                    decode_snapshot(&patched).is_err(),
                    "section {tag} admitted a declared count of {count} against a \
                     {body_len}-byte body"
                );
            }
        }
    }
    // Guard the sweep: if the two worlds stopped carrying these sections this
    // test would pass by iterating over nothing.
    seen.sort_unstable();
    assert_eq!(
        seen, COUNT_LED,
        "a count-led section was not present in either world, so it was never swept"
    );
}

/// The learn section's **inner** count - one organism's plastic-edge count -
/// is bounded independently of the organism count, and a body of the wrong
/// length is caught by the trailing-bytes check rather than by an equality on
/// a field count.
///
/// The outer sweep above patches the first word of each section. This one
/// reaches the second, which is the count that a nested loop makes easy to
/// leave unguarded, and then checks the property that lets both bounds be
/// caps rather than equalities: exactness comes from the parser insisting it
/// consumed the whole body.
#[test]
fn the_learn_sections_inner_count_and_body_length_are_checked_too() {
    let world = learned_world(0x11a5, 200);
    let state = world.export_state();
    let bytes = encode_snapshot(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode");
    let payload_start = 112 + sim_persist::BUILD_VERSION.len();
    let (_, body_start, body_len) = sections(&bytes, payload_start)
        .into_iter()
        .find(|(tag, _, _)| *tag == 12)
        .expect("the learn section is present");

    // The first organism's plastic-edge count, four bytes past the organism
    // count word.
    for count in [u32::MAX, body_len as u32] {
        let mut patched = bytes.clone();
        patched[body_start + 8..body_start + 12].copy_from_slice(&count.to_le_bytes());
        reseal(&mut patched, payload_start, body_start, body_len);
        assert!(
            decode_snapshot(&patched).is_err(),
            "a plastic-edge count of {count} was admitted against a {body_len}-byte body"
        );
    }

    // A body four bytes short, with every declared length and both checksums
    // made consistent, so the only thing left to catch it is the parser
    // noticing it did not consume the body.
    let mut short = bytes.clone();
    short.drain(body_start + body_len - 4..body_start + body_len);
    let new_len = (body_len - 4) as u64;
    short[body_start - 8..body_start].copy_from_slice(&new_len.to_le_bytes());
    reseal(&mut short, payload_start, body_start, body_len - 4);
    let payload_len = (short.len() - payload_start) as u64;
    short[68..76].copy_from_slice(&payload_len.to_le_bytes());
    short[76..84].copy_from_slice(&payload_len.to_le_bytes());
    reseal(&mut short, payload_start, body_start, body_len - 4);
    assert!(
        decode_snapshot(&short).is_err(),
        "a learn section with a truncated body was accepted"
    );
}

/// **Every field of the learn record survives the codec, and every one is
/// perturbed away from what a legal world produces before it is checked.**
///
/// This is the structural defence `every_config_field_survives_a_snapshot_round_trip`
/// is for the config section, and it exists because the round-trip tests
/// above cannot reach three of the fields:
///
/// - `trace_q16` is written only by rule 4, and every other test here uses
///   rule 1, so a codec that dropped the trace would compare zero to zero.
///   (`sim-core`'s
///   `an_eligibility_trace_survives_the_round_trip_as_well_as_the_learned_delta`
///   covers the mechanism; this covers the encoding.)
/// - `faults` is **unreachable through validated genes** - coefficients are
///   bounded to [-1, 1] and activations to [-8, 8], so no legal world
///   produces a non-finite delta. Only a hand-built record can put a nonzero
///   value there, and a field that no test can populate is a field the codec
///   can drop for free.
/// - `cost_milli` and the six counters are aggregates that would survive a
///   codec that wrote them in the wrong order, since several are equal in an
///   ordinary run.
///
/// So each is set to a distinct value, round-tripped, and then checked twice:
/// the decoded record must equal the encoded one, and the restored world's
/// checksum must **differ from the unperturbed restore**, which is what says
/// the values reached the checksum rather than merely surviving the file.
#[test]
fn every_learn_record_field_survives_the_codec_and_reaches_the_checksum() {
    let world = learned_world(0x11a5, 400);
    let baseline = world.export_state();
    let encode = |state: &sim_core::SaveState| {
        encode_snapshot(
            state,
            1,
            0,
            world.state_checksum(),
            sim_persist::BUILD_VERSION,
            0,
            None,
        )
        .expect("encode")
    };
    let (_, decoded_baseline) = decode_snapshot(&encode(&baseline)).expect("decode");
    let baseline_checksum = sim_core::World::from_state(decoded_baseline)
        .expect("restore")
        .state_checksum();
    assert_eq!(baseline_checksum, world.state_checksum());

    let mut state = baseline.clone();
    {
        let learn = state.learn.as_mut().expect("section");
        // Distinct per organism and per slot, so a record written with its
        // fields swapped or its rows mis-paired cannot compare equal. All
        // well inside `LEARN_LIMIT_Q16`, because `from_state` correctly
        // refuses anything outside it and a refusal is not a round trip.
        for (index, row) in learn.edges.iter_mut().enumerate() {
            for (slot, edge) in row.iter_mut().enumerate() {
                edge.learned_q16 = 1_000 + index as i32 * 7 + slot as i32;
                edge.trace_q16 = -(2_000 + index as i32 * 11 + slot as i32);
            }
            learn.faults[index] = index as u32 + 1;
        }
        // Distinct values, in declaration order, so a counter written into
        // the wrong slot is visible.
        learn.counters.updates_applied = 11;
        learn.counters.updates_static = 22;
        learn.counters.updates_refused = 33;
        learn.counters.faults = 44;
        learn.counters.clamped = 55;
        learn.counters.trace_clamped = 66;
        learn.cost_milli = 777_777;
    }
    assert!(
        baseline.learn.as_ref().expect("section").cost_milli != 777_777,
        "the perturbation is not a perturbation"
    );

    let (_, decoded) = decode_snapshot(&encode(&state)).expect("decode");
    assert_eq!(
        decoded.learn.as_ref(),
        state.learn.as_ref(),
        "a learn record field did not survive the round trip"
    );

    let restored = sim_core::World::from_state(decoded).expect("restore");
    assert_ne!(
        restored.state_checksum(),
        baseline_checksum,
        "the perturbed learn record restored to the same checksum, so the \
         fields are carried in the save and never hashed"
    );
    // ...and the per-organism values arrived where they belong, not merely
    // somewhere: the census is read back and compared against what was
    // written, organism by organism.
    for (index, sample) in restored.learned_census().iter().enumerate() {
        let row = &state.learn.as_ref().expect("section").edges[index];
        let learned: i64 = row
            .iter()
            .map(|edge| i64::from(edge.learned_q16.unsigned_abs()))
            .sum();
        let trace: i64 = row
            .iter()
            .map(|edge| i64::from(edge.trace_q16.unsigned_abs()))
            .sum();
        assert_eq!(sample.sum_abs_learned_q16, learned, "organism {index}");
        assert_eq!(sample.sum_abs_trace_q16, trace, "organism {index}");
        assert_eq!(sample.faults, index as u32 + 1, "organism {index}");
    }
}
