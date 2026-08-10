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
    config
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
