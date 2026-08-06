//! ALIF world snapshot format version 1.
//!
//! Layout (all little-endian, matching the kernel's canonical hashing):
//!
//! header (fixed 112 bytes):
//!   magic "ALIF" | format u16 | header_len u16 | flags u32 (bit0 zstd)
//!   world_id u64 | parent_world_id u64 | tick u64 | seed u64
//!   config_hash u64 | save_state_version u16 | genome_schema u16
//!   build_len u16 | reserved u16 | event_log_offset u64
//!   uncompressed_len u64 | stored_len u64 | payload_crc32 u32
//!   state_checksum u64 | terrain_checksum u64
//! then: build version string (build_len <= 64 bytes)
//! then: payload (zstd-compressed when flagged), which is a sequence of
//! sections: tag u16 | flags u16 | length u64 | body | crc32 u32.
//!
//! Every decoder treats input as hostile: lengths are capped before any
//! allocation or decompression, checksums verify before parsing, and
//! unknown versions/sections fail closed with typed errors. Loaders never
//! repair data.

use sim_core::{
    GENOME_SCHEMA_VERSION, Ledger, Phase2SaveState, SAVE_STATE_VERSION, SaveState, TRAIT_COUNT,
};
use std::fmt;

pub const SNAPSHOT_MAGIC: &[u8; 4] = b"ALIF";
/// Format 3 makes the Phase 2 section describe its own two counts.
///
/// Format 2 wrote one count and drove the per-organism loop from
/// `traits.len()`. That is the organism count in a schema-1 world and zero
/// in a schema-2 world, which carries no flat genome - so a schema-2
/// snapshot encoded no per-organism records at all and dropped heading,
/// speed, turn, parents, depth, child count, birth tick, and memory. It
/// failed closed on restore rather than corrupting silently, but it failed:
/// **a schema-2 world could not be checkpointed.** Format 3 writes the
/// organism count and the flat-genome count separately, so the two are never
/// again assumed equal. No migration from 2, on the same grounds as 1 to 2:
/// a format-2 schema-2 file does not contain the dropped state, and a
/// format-2 schema-1 file would have to be re-framed on a guess.
///
/// Format 2 added the config sections that Phase 6, 7, and 8 introduced.
///
/// Format 1 encoded the config only as far as Phase 2, so every section
/// added afterwards was silently dropped on save and restored at its
/// default on load. For climate and contest the presence check in
/// `World::from_state` turned that into a confusing restore failure; for
/// the origin section, which has no presence check, it would have restored
/// a *different config* with no error at all. There is no migration from 1:
/// a format-1 file cannot say what its climate settings were, so inventing
/// them is exactly the "never alter meaning during load" rule this crate
/// exists to keep.
pub const FORMAT_VERSION: u16 = 3;
pub const FLAG_ZSTD: u32 = 1;
const HEADER_LEN: usize = 112;
const MAX_BUILD_LEN: usize = 64;
/// Absolute caps applied before allocation/decompression.
pub const MAX_STORED_LEN: u64 = 256 * 1024 * 1024;
pub const MAX_UNCOMPRESSED_LEN: u64 = 1024 * 1024 * 1024;
const MAX_SECTION_LEN: u64 = MAX_UNCOMPRESSED_LEN;

const SECTION_CONFIG: u16 = 1;
const SECTION_WORLD_META: u16 = 2;
const SECTION_ORGANISMS: u16 = 3;
const SECTION_BIOMASS: u16 = 4;
const SECTION_LEDGER: u16 = 5;
const SECTION_PHASE2: u16 = 6;
/// Phase 6 climate. Optional exactly as the Phase 2 section is: present
/// only when the subsystem exists, absent otherwise, so a world without
/// climate encodes byte-identically to the way it always did. Section tags
/// are permanent and never reused.
const SECTION_CLIMATE: u16 = 7;
/// Phase 7 contest. Optional on the same terms as Phase 2 and climate.
const SECTION_CONTEST: u16 = 8;
/// Phase 8 demography. Optional on the same terms. Tags are permanent and
/// never reused, so a Phase 7 snapshot decodes unchanged.
const SECTION_PHYSIOLOGY: u16 = 9;
/// Phase 9 genome schema 2. Optional on the same terms.
const SECTION_SCHEMA2: u16 = 10;
/// Phase 10 morphology. Optional on the same terms, and deliberately tiny:
/// bodies are derived and never stored, so this section carries only the
/// developmental counters.
const SECTION_MORPHOLOGY: u16 = 11;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodecError {
    TooShort,
    BadMagic,
    UnsupportedFormat(u16),
    UnsupportedSaveState(u16),
    UnsupportedGenomeSchema(u16),
    BadHeaderLength(usize),
    UnknownFlags(u32),
    BuildStringTooLong(usize),
    StoredTooLarge(u64),
    UncompressedTooLarge(u64),
    LengthMismatch { expected: usize, actual: usize },
    PayloadChecksumMismatch,
    SectionChecksumMismatch(u16),
    DecompressionFailed,
    DecompressedLengthMismatch { declared: u64, actual: usize },
    TruncatedSection,
    UnknownSection(u16),
    MissingSection(u16),
    DuplicateSection(u16),
    ValueOutOfRange(&'static str),
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CodecError {}

/// Decoded header metadata (available without touching the payload).
#[derive(Clone, Debug)]
pub struct SnapshotInfo {
    pub format_version: u16,
    pub compressed: bool,
    pub world_id: u64,
    pub parent_world_id: u64,
    pub tick: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub save_state_version: u16,
    pub genome_schema_version: u16,
    pub build_version: String,
    pub event_log_offset: u64,
    pub uncompressed_len: u64,
    pub stored_len: u64,
    pub state_checksum: u64,
    pub terrain_checksum: u64,
}

// --- primitive writers/readers ---------------------------------------------

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i128(&mut self, value: i128) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], CodecError> {
        let slice = self
            .bytes
            .get(self.offset..self.offset + count)
            .ok_or(CodecError::TruncatedSection)?;
        self.offset += count;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, CodecError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i128(&mut self) -> Result<i128, CodecError> {
        Ok(i128::from_le_bytes(self.take(16)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, CodecError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

// --- sections ----------------------------------------------------------------

fn write_section(out: &mut Vec<u8>, tag: u16, body: Vec<u8>) {
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    let checksum = crc32(&body);
    out.extend_from_slice(&body);
    out.extend_from_slice(&checksum.to_le_bytes());
}

fn encode_config(config: &sim_core::SimConfig) -> Vec<u8> {
    let mut writer = Writer(Vec::new());
    writer.u64(config.world_seed);
    writer.u32(config.cells_x);
    writer.u32(config.cells_y);
    writer.u32(config.cell_size_m);
    writer.u32(config.initial_organisms);
    writer.u32(config.max_entities);
    writer.u32(config.dt_ms);
    writer.u32(config.growth_rate_q16_per_s);
    writer.i64(config.cell_capacity_milli);
    writer.u32(config.initial_biomass_q16);
    writer.i64(config.energy_max_milli);
    writer.i64(config.initial_energy_milli);
    writer.i64(config.basal_cost_milli_per_s);
    writer.i64(config.move_cost_milli_per_s);
    writer.i64(config.intake_rate_milli_per_s);
    writer.u32(config.assimilation_q16);
    writer.u32(config.speed_mps_q16);
    writer.u32(config.crowding_radius_m);
    writer.u32(config.crowding_threshold);
    writer.i64(config.crowding_cost_milli_per_s);
    writer.u64(config.maturity_age_ticks);
    writer.u64(config.max_age_ticks);
    writer.u8(u8::from(config.reproduction_enabled));
    writer.i64(config.repro_threshold_milli);
    writer.i64(config.offspring_energy_milli);
    writer.i64(config.repro_overhead_milli);
    writer.u64(config.repro_cooldown_ticks);
    writer.u32(config.land_threshold_q16);
    writer.u32(config.min_land_fraction_q16);
    writer.u32(config.max_land_fraction_q16);
    let phase2 = &config.phase2;
    writer.u8(u8::from(phase2.enabled));
    writer.u32(phase2.variation_probability_q16);
    writer.u32(phase2.variation_trait_sigma_q16);
    writer.u32(phase2.variation_neural_sigma_q16);
    writer.u32(phase2.pairing_range_m);
    writer.u32(phase2.compatibility_threshold_q16);
    writer.i64(phase2.pairing_energy_threshold_milli);
    writer.i64(phase2.pairing_overhead_milli);
    writer.i32(phase2.eat_threshold_q16);
    writer.i32(phase2.mate_threshold_q16);
    writer.i32(phase2.rest_threshold_q16);
    writer.u32(phase2.max_turn_per_tick_bam);
    writer.u32(phase2.cluster_threshold_q16);
    writer.u32(phase2.cluster_sample_max);
    writer.u32(phase2.cluster_neural_weight_q16);

    // Phase 6 climate. Written unconditionally, unlike the *state*
    // sections: a config is not optional, and a disabled section still has
    // parameters that have to survive a round trip.
    let climate = &config.climate;
    writer.u8(u8::from(climate.enabled));
    writer.u8(match climate.worldgen_version {
        sim_core::WorldgenVersion::V1 => 1,
        sim_core::WorldgenVersion::V2 => 2,
    });
    writer.i32(climate.base_temperature_milli);
    writer.i32(climate.lapse_milli_per_full_elevation);
    writer.i32(climate.latitude_amplitude_milli);
    writer.u64(climate.season_period_ticks);
    writer.i32(climate.season_amplitude_milli);
    for value in climate.drift_period_ticks {
        writer.u64(value);
    }
    for value in climate.drift_amplitude_milli {
        writer.i32(value);
    }
    writer.i32(climate.temperature_min_milli);
    writer.i32(climate.temperature_max_milli);
    writer.i64(climate.initial_moisture_milli);
    writer.i64(climate.coastal_moisture_bonus_milli);
    writer.i64(climate.moisture_max_milli);
    writer.i64(climate.moisture_ceiling_milli);
    writer.u32(climate.sea_proximity_weight_q16);
    writer.u32(climate.moisture_diffusion_q16);
    writer.u32(climate.moisture_drain_weight);
    writer.u32(climate.highland_elevation_q16);
    writer.i64(climate.wetland_moisture_milli);
    writer.i64(climate.arid_moisture_milli);
    writer.i64(climate.forest_moisture_milli);
    writer.i32(climate.forest_min_temperature_milli);
    for value in climate.biome_capacity_q16 {
        writer.u32(value);
    }
    writer.u64(climate.reclassify_interval_ticks);

    // Phase 6 origin.
    let origin = &config.origin;
    writer.u8(match origin.mode {
        sim_core::OriginMode::Random => 1,
        sim_core::OriginMode::Seeded => 2,
    });
    writer.u32(origin.trait_low_q16);
    writer.u32(origin.trait_span_q16);
    writer.u32(origin.neural_span_q16);
    writer.u32(origin.deme_count);
    writer.u32(origin.deme_radius_m);
    writer.u32(origin.deme_min_separation_m);
    writer.u32(origin.deme_trait_spread_q16);
    writer.u32(origin.archetype_count);
    for archetype in &origin.archetypes {
        writer.u32(u32::from(archetype.id));
        for mean in archetype.trait_mean_q16 {
            writer.u32(u32::from(mean));
        }
        writer.u32(u32::from(archetype.trait_spread_q16));
        writer.u32(u32::from(archetype.neural_spread_q16));
        writer.u32(u32::from(archetype.biome_affinity));
    }

    // Phase 7 contest.
    let contest = &config.contest;
    writer.u8(u8::from(contest.enabled));
    writer.i64(contest.base_health_milli);
    writer.i64(contest.damage_base_milli);
    writer.u32(contest.damage_variance_q16);
    writer.i64(contest.attack_cost_milli);
    writer.u32(contest.attack_range_m);
    writer.i32(contest.attack_threshold_q16);
    writer.u64(contest.attack_cooldown_ticks);
    writer.i64(contest.heal_milli_per_s);
    writer.u32(contest.heal_energy_cost_q16);
    writer.u32(contest.heal_energy_floor_q16);
    writer.u32(contest.damage_decay_q16_per_s);
    writer.u32(contest.carcass_energy_q16);
    writer.u32(contest.carcass_decay_q16_per_s);
    writer.u32(contest.carcass_reach_m);
    writer.u32(contest.max_carcasses);
    writer.i64(contest.local_depletion_milli);

    // Phase 8 physiology.
    let physiology = &config.physiology;
    writer.u8(u8::from(physiology.enabled));
    writer.u8(u8::from(physiology.allometry_enabled));
    writer.u32(physiology.basal_exponent_quarters);
    writer.u8(u8::from(physiology.thermoregulation_enabled));
    writer.i32(physiology.thermal_pref_low_milli);
    writer.i32(physiology.thermal_pref_high_milli);
    writer.i32(physiology.thermal_neutral_band_milli);
    writer.i64(physiology.thermal_cost_milli_per_s_per_degree);
    writer.u8(u8::from(physiology.senescence_enabled));
    writer.u64(physiology.senescence_onset_ticks);
    writer.u64(physiology.senescence_scale_ticks);
    writer.u32(physiology.senescence_power);
    writer.u32(physiology.senescence_hazard_q16_per_s);
    writer.u32(physiology.extrinsic_hazard_q16_per_s);
    writer.u32(physiology.juvenile_hazard_multiplier_q16);

    // Phase 9 genome schema 2.
    let genome2 = &config.genome2;
    writer.u8(u8::from(genome2.enabled));
    writer.u8(genome2.caps.max_chromosomes);
    writer.u32(genome2.caps.max_loci_per_chromosome);
    writer.u32(genome2.caps.max_nodes);
    writer.u32(genome2.caps.max_edges);
    writer.u32(genome2.caps.max_edges_per_node);
    writer.u32(genome2.caps.max_genome_bytes);
    writer.u32(genome2.caps.min_nodes);
    writer.u8(genome2.meiosis.mode.id());
    writer.u32(genome2.meiosis.max_extra_crossovers);
    writer.u32(genome2.mutation.point_q16);
    writer.u32(genome2.mutation.duplication_q16);
    writer.u32(genome2.mutation.deletion_q16);
    writer.u32(genome2.mutation.insertion_q16);
    writer.u32(genome2.mutation.transposition_q16);
    writer.u32(genome2.mutation.max_run);
    writer.u32(genome2.mutation.point_delta_q16);
    writer.u8(u8::from(genome2.mutation.regulatory_enabled));
    writer.0
}

fn decode_config(reader: &mut Reader) -> Result<sim_core::SimConfig, CodecError> {
    let mut config = sim_core::SimConfig::phase1_default(reader.u64()?);
    config.cells_x = reader.u32()?;
    config.cells_y = reader.u32()?;
    config.cell_size_m = reader.u32()?;
    config.initial_organisms = reader.u32()?;
    config.max_entities = reader.u32()?;
    config.dt_ms = reader.u32()?;
    config.growth_rate_q16_per_s = reader.u32()?;
    config.cell_capacity_milli = reader.i64()?;
    config.initial_biomass_q16 = reader.u32()?;
    config.energy_max_milli = reader.i64()?;
    config.initial_energy_milli = reader.i64()?;
    config.basal_cost_milli_per_s = reader.i64()?;
    config.move_cost_milli_per_s = reader.i64()?;
    config.intake_rate_milli_per_s = reader.i64()?;
    config.assimilation_q16 = reader.u32()?;
    config.speed_mps_q16 = reader.u32()?;
    config.crowding_radius_m = reader.u32()?;
    config.crowding_threshold = reader.u32()?;
    config.crowding_cost_milli_per_s = reader.i64()?;
    config.maturity_age_ticks = reader.u64()?;
    config.max_age_ticks = reader.u64()?;
    config.reproduction_enabled = reader.u8()? != 0;
    config.repro_threshold_milli = reader.i64()?;
    config.offspring_energy_milli = reader.i64()?;
    config.repro_overhead_milli = reader.i64()?;
    config.repro_cooldown_ticks = reader.u64()?;
    config.land_threshold_q16 = reader.u32()?;
    config.min_land_fraction_q16 = reader.u32()?;
    config.max_land_fraction_q16 = reader.u32()?;
    config.phase2.enabled = reader.u8()? != 0;
    config.phase2.variation_probability_q16 = reader.u32()?;
    config.phase2.variation_trait_sigma_q16 = reader.u32()?;
    config.phase2.variation_neural_sigma_q16 = reader.u32()?;
    config.phase2.pairing_range_m = reader.u32()?;
    config.phase2.compatibility_threshold_q16 = reader.u32()?;
    config.phase2.pairing_energy_threshold_milli = reader.i64()?;
    config.phase2.pairing_overhead_milli = reader.i64()?;
    config.phase2.eat_threshold_q16 = reader.i32()?;
    config.phase2.mate_threshold_q16 = reader.i32()?;
    config.phase2.rest_threshold_q16 = reader.i32()?;
    config.phase2.max_turn_per_tick_bam = reader.u32()?;
    config.phase2.cluster_threshold_q16 = reader.u32()?;
    config.phase2.cluster_sample_max = reader.u32()?;
    config.phase2.cluster_neural_weight_q16 = reader.u32()?;

    config.climate.enabled = reader.u8()? != 0;
    config.climate.worldgen_version = match reader.u8()? {
        1 => sim_core::WorldgenVersion::V1,
        2 => sim_core::WorldgenVersion::V2,
        _ => return Err(CodecError::ValueOutOfRange("worldgen_version")),
    };
    config.climate.base_temperature_milli = reader.i32()?;
    config.climate.lapse_milli_per_full_elevation = reader.i32()?;
    config.climate.latitude_amplitude_milli = reader.i32()?;
    config.climate.season_period_ticks = reader.u64()?;
    config.climate.season_amplitude_milli = reader.i32()?;
    for index in 0..config.climate.drift_period_ticks.len() {
        config.climate.drift_period_ticks[index] = reader.u64()?;
    }
    for index in 0..config.climate.drift_amplitude_milli.len() {
        config.climate.drift_amplitude_milli[index] = reader.i32()?;
    }
    config.climate.temperature_min_milli = reader.i32()?;
    config.climate.temperature_max_milli = reader.i32()?;
    config.climate.initial_moisture_milli = reader.i64()?;
    config.climate.coastal_moisture_bonus_milli = reader.i64()?;
    config.climate.moisture_max_milli = reader.i64()?;
    config.climate.moisture_ceiling_milli = reader.i64()?;
    config.climate.sea_proximity_weight_q16 = reader.u32()?;
    config.climate.moisture_diffusion_q16 = reader.u32()?;
    config.climate.moisture_drain_weight = reader.u32()?;
    config.climate.highland_elevation_q16 = reader.u32()?;
    config.climate.wetland_moisture_milli = reader.i64()?;
    config.climate.arid_moisture_milli = reader.i64()?;
    config.climate.forest_moisture_milli = reader.i64()?;
    config.climate.forest_min_temperature_milli = reader.i32()?;
    for index in 0..config.climate.biome_capacity_q16.len() {
        config.climate.biome_capacity_q16[index] = reader.u32()?;
    }
    config.climate.reclassify_interval_ticks = reader.u64()?;

    config.origin.mode = match reader.u8()? {
        1 => sim_core::OriginMode::Random,
        2 => sim_core::OriginMode::Seeded,
        _ => return Err(CodecError::ValueOutOfRange("origin_mode")),
    };
    config.origin.trait_low_q16 = reader.u32()?;
    config.origin.trait_span_q16 = reader.u32()?;
    config.origin.neural_span_q16 = reader.u32()?;
    config.origin.deme_count = reader.u32()?;
    config.origin.deme_radius_m = reader.u32()?;
    config.origin.deme_min_separation_m = reader.u32()?;
    config.origin.deme_trait_spread_q16 = reader.u32()?;
    config.origin.archetype_count = reader.u32()?;
    for index in 0..config.origin.archetypes.len() {
        let archetype = &mut config.origin.archetypes[index];
        archetype.id = u16::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype id"))?;
        for gene in 0..archetype.trait_mean_q16.len() {
            archetype.trait_mean_q16[gene] = u16::try_from(reader.u32()?)
                .map_err(|_| CodecError::ValueOutOfRange("archetype trait mean"))?;
        }
        archetype.trait_spread_q16 = u16::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype trait spread"))?;
        archetype.neural_spread_q16 = u16::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype neural spread"))?;
        archetype.biome_affinity = u8::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype biome affinity"))?;
    }

    config.contest.enabled = reader.u8()? != 0;
    config.contest.base_health_milli = reader.i64()?;
    config.contest.damage_base_milli = reader.i64()?;
    config.contest.damage_variance_q16 = reader.u32()?;
    config.contest.attack_cost_milli = reader.i64()?;
    config.contest.attack_range_m = reader.u32()?;
    config.contest.attack_threshold_q16 = reader.i32()?;
    config.contest.attack_cooldown_ticks = reader.u64()?;
    config.contest.heal_milli_per_s = reader.i64()?;
    config.contest.heal_energy_cost_q16 = reader.u32()?;
    config.contest.heal_energy_floor_q16 = reader.u32()?;
    config.contest.damage_decay_q16_per_s = reader.u32()?;
    config.contest.carcass_energy_q16 = reader.u32()?;
    config.contest.carcass_decay_q16_per_s = reader.u32()?;
    config.contest.carcass_reach_m = reader.u32()?;
    config.contest.max_carcasses = reader.u32()?;
    config.contest.local_depletion_milli = reader.i64()?;

    config.physiology.enabled = reader.u8()? != 0;
    config.physiology.allometry_enabled = reader.u8()? != 0;
    config.physiology.basal_exponent_quarters = reader.u32()?;
    config.physiology.thermoregulation_enabled = reader.u8()? != 0;
    config.physiology.thermal_pref_low_milli = reader.i32()?;
    config.physiology.thermal_pref_high_milli = reader.i32()?;
    config.physiology.thermal_neutral_band_milli = reader.i32()?;
    config.physiology.thermal_cost_milli_per_s_per_degree = reader.i64()?;
    config.physiology.senescence_enabled = reader.u8()? != 0;
    config.physiology.senescence_onset_ticks = reader.u64()?;
    config.physiology.senescence_scale_ticks = reader.u64()?;
    config.physiology.senescence_power = reader.u32()?;
    config.physiology.senescence_hazard_q16_per_s = reader.u32()?;
    config.physiology.extrinsic_hazard_q16_per_s = reader.u32()?;
    config.physiology.juvenile_hazard_multiplier_q16 = reader.u32()?;

    config.genome2.enabled = reader.u8()? != 0;
    config.genome2.caps.max_chromosomes = reader.u8()?;
    config.genome2.caps.max_loci_per_chromosome = reader.u32()?;
    config.genome2.caps.max_nodes = reader.u32()?;
    config.genome2.caps.max_edges = reader.u32()?;
    config.genome2.caps.max_edges_per_node = reader.u32()?;
    config.genome2.caps.max_genome_bytes = reader.u32()?;
    config.genome2.caps.min_nodes = reader.u32()?;
    config.genome2.meiosis.mode = sim_core::InheritanceMode::from_id(reader.u8()?)
        .ok_or(CodecError::ValueOutOfRange("inheritance_mode"))?;
    config.genome2.meiosis.max_extra_crossovers = reader.u32()?;
    config.genome2.mutation.point_q16 = reader.u32()?;
    config.genome2.mutation.duplication_q16 = reader.u32()?;
    config.genome2.mutation.deletion_q16 = reader.u32()?;
    config.genome2.mutation.insertion_q16 = reader.u32()?;
    config.genome2.mutation.transposition_q16 = reader.u32()?;
    config.genome2.mutation.max_run = reader.u32()?;
    config.genome2.mutation.point_delta_q16 = reader.u32()?;
    config.genome2.mutation.regulatory_enabled = reader.u8()? != 0;
    Ok(config)
}

fn encode_payload(state: &SaveState) -> Vec<u8> {
    let mut payload = Vec::new();
    write_section(&mut payload, SECTION_CONFIG, encode_config(&state.config));

    let mut meta = Writer(Vec::new());
    meta.u64(state.tick);
    meta.u8(u8::from(state.paused));
    meta.u8(u8::from(state.extinct));
    meta.u64(state.next_entity_id);
    meta.u64(state.terrain_checksum);
    write_section(&mut payload, SECTION_WORLD_META, meta.0);

    let mut organisms = Writer(Vec::new());
    organisms.u64(state.ids.len() as u64);
    for index in 0..state.ids.len() {
        organisms.u64(state.ids[index]);
        organisms.i32(state.x_fp[index]);
        organisms.i32(state.y_fp[index]);
        organisms.i64(state.energy_milli[index]);
        organisms.u64(state.age_ticks[index]);
        organisms.u64(state.cooldown_ticks[index]);
    }
    write_section(&mut payload, SECTION_ORGANISMS, organisms.0);

    let mut biomass = Writer(Vec::new());
    biomass.u64(state.biomass_milli.len() as u64);
    for &value in &state.biomass_milli {
        biomass.i64(value);
    }
    write_section(&mut payload, SECTION_BIOMASS, biomass.0);

    let mut ledger = Writer(Vec::new());
    ledger.i128(state.ledger.initial_energy_milli);
    ledger.i128(state.ledger.assimilated_milli);
    ledger.i128(state.ledger.spent_milli);
    ledger.i128(state.ledger.removed_at_death_milli);
    ledger.i128(state.ledger.initial_biomass_milli);
    ledger.i128(state.ledger.grown_milli);
    ledger.i128(state.ledger.consumed_biomass_milli);
    ledger.u64(state.counters.births_total);
    ledger.u64(state.counters.deaths_starvation_total);
    ledger.u64(state.counters.deaths_old_age_total);
    ledger.u64(state.counters.capacity_rejections_total);
    ledger.u64(state.counters.dropped_events_total);
    write_section(&mut payload, SECTION_LEDGER, ledger.0);

    if let Some(phase2) = &state.phase2 {
        let mut section = Writer(Vec::new());
        // **Two counts, because they are not the same number.** A schema-2
        // world carries no flat genome, so `traits` and `neural` are empty
        // by construction while every other per-organism array is full
        // length. This loop used to be driven by `traits.len()`, which meant
        // a schema-2 snapshot encoded zero per-organism records and silently
        // dropped heading, speed, turn, parents, depth, child count, birth
        // tick, and memory - all state a schema-2 world uses. It failed
        // closed on restore rather than corrupting, but it failed: a
        // schema-2 world could not be checkpointed at all.
        let organisms = phase2.heading_bam.len() as u64;
        let flat_genomes = phase2.traits.len() as u64;
        section.u64(organisms);
        section.u64(flat_genomes);
        for index in 0..phase2.traits.len() {
            for &gene in &phase2.traits[index] {
                section.f32(gene);
            }
            for &gene in &phase2.neural[index] {
                section.f32(gene);
            }
        }
        for index in 0..phase2.heading_bam.len() {
            for &value in &phase2.memory[index] {
                section.f32(value);
            }
            section.u16(phase2.heading_bam[index]);
            section.i64(phase2.speed_milli[index]);
            section.f32(phase2.last_turn[index]);
            section.u64(phase2.parents[index][0]);
            section.u64(phase2.parents[index][1]);
            section.u32(phase2.depth[index]);
            section.u32(phase2.child_count[index]);
            section.u64(phase2.birth_tick[index]);
        }
        section.u64(phase2.counters.paired_births_total);
        section.u64(phase2.counters.pair_rejected_capacity_total);
        section.u64(phase2.counters.pair_rejected_placement_total);
        section.u64(phase2.counters.pair_rejected_energy_total);
        section.u64(phase2.counters.pair_rejected_nonviable_total);
        section.u64(phase2.counters.controller_faults_total);
        section.u64(phase2.counters.mutated_trait_genes_total);
        section.u64(phase2.counters.mutated_neural_genes_total);
        write_section(&mut payload, SECTION_PHASE2, section.0);
    }
    if let Some(climate) = state.climate.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(climate.moisture_milli.len() as u64);
        for &value in &climate.moisture_milli {
            section.i64(value);
        }
        // The biome map is stored state, not a derived field: it is a
        // classification cached on a cadence, so recomputing it on load
        // gives a different map and the restored world diverges.
        section.u64(climate.biome.len() as u64);
        for biome in &climate.biome {
            section.u8(*biome as u8);
        }
        section.i128(climate.capacity_loss_milli);
        write_section(&mut payload, SECTION_CLIMATE, section.0);
    }
    if let Some(contest) = state.contest.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(contest.health_milli.len() as u64);
        for index in 0..contest.health_milli.len() {
            section.i64(contest.health_milli[index]);
            section.i64(contest.recent_damage_milli[index]);
        }
        section.u64(contest.carcasses.len() as u64);
        for carcass in &contest.carcasses {
            section.u64(carcass.id);
            section.i32(carcass.x_fp);
            section.i32(carcass.y_fp);
            section.i64(carcass.energy_milli);
            section.u64(carcass.created_tick);
        }
        section.i128(contest.carcass_created_milli);
        section.i128(contest.carcass_consumed_milli);
        section.i128(contest.carcass_decayed_milli);
        section.u64(contest.attacks_total);
        section.i128(contest.damage_dealt_milli);
        section.u64(contest.deaths_by_damage_total);
        section.i128(contest.healed_milli);
        write_section(&mut payload, SECTION_CONTEST, section.0);
    }
    if let Some(physiology) = state.physiology.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(physiology.cumulative_hazard_q16.len() as u64);
        for &hazard in &physiology.cumulative_hazard_q16 {
            section.i64(hazard);
        }
        section.u64(physiology.deaths_senescence_total);
        section.u64(physiology.deaths_extrinsic_total);
        section.u64(physiology.deaths_juvenile_total);
        section.i128(physiology.thermal_cost_milli);
        section.i128(physiology.allometric_cost_milli);
        write_section(&mut payload, SECTION_PHYSIOLOGY, section.0);
    }
    if let Some(schema2) = state.schema2.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(schema2.genomes.len() as u64);
        for index in 0..schema2.genomes.len() {
            let genome = &schema2.genomes[index];
            section.u32(genome.len() as u32);
            section.0.extend_from_slice(genome);
            let values = &schema2.activation_values[index];
            let prior = &schema2.activation_prior[index];
            section.u32(values.len() as u32);
            for value in values {
                section.u32(value.to_bits());
            }
            for value in prior {
                section.u32(value.to_bits());
            }
            section.u32(schema2.activation_faults[index]);
        }
        // **Destructured rather than field-accessed, so the compiler fails
        // this when a counter is added.** The previous form was a list of
        // eleven `counters.x` reads, and two counters were added without it:
        // they were dropped on save, and since the counters are hashed into
        // the state checksum, a restored world's checksum silently differed
        // from the one it was saved from. An exhaustive destructuring with
        // no `..` cannot be left behind that way.
        let sim_core::MutationCounters {
            point_applied,
            duplication_applied,
            deletion_applied,
            insertion_applied,
            transposition_applied,
            rejected_homology_collision,
            rejected_orphaned,
            rejected_min_nodes,
            rejected_no_bindings,
            rejected_cap,
            rejected_inapplicable,
            rejected_cycle,
            rejected_invalid,
        } = schema2.counters;
        for value in [
            point_applied,
            duplication_applied,
            deletion_applied,
            insertion_applied,
            transposition_applied,
            rejected_homology_collision,
            rejected_orphaned,
            rejected_min_nodes,
            rejected_no_bindings,
            rejected_cap,
            rejected_inapplicable,
            rejected_cycle,
            rejected_invalid,
        ] {
            section.u64(value);
        }
        write_section(&mut payload, SECTION_SCHEMA2, section.0);
    }
    if let Some(morphology) = state.morphology.as_ref() {
        let mut section = Writer(Vec::new());
        // Exhaustive destructuring with no `..`, so adding a counter fails
        // this line rather than silently dropping it on save (D-077).
        let sim_core::DevelopCounters {
            bodies_grown,
            modules_placed,
            differentiations,
            scale_changes,
            refused_occupied,
            refused_out_of_bounds,
            refused_max_modules,
            refused_node_budget,
            nonviable_empty,
            nonviable_disconnected,
            nonviable_missing_type,
            nonviable_other,
        } = morphology.counters;
        for value in [
            bodies_grown,
            modules_placed,
            differentiations,
            scale_changes,
            refused_occupied,
            refused_out_of_bounds,
            refused_max_modules,
            refused_node_budget,
            nonviable_empty,
            nonviable_disconnected,
            nonviable_missing_type,
            nonviable_other,
        ] {
            section.u64(value);
        }
        write_section(&mut payload, SECTION_MORPHOLOGY, section.0);
    }
    payload
}

fn decode_payload(bytes: &[u8], state_checksum: u64) -> Result<SaveState, CodecError> {
    let mut offset = 0_usize;
    let mut config = None;
    let mut climate: Option<sim_core::ClimateSaveState> = None;
    let mut physiology: Option<sim_core::PhysiologySaveState> = None;
    let mut schema2: Option<sim_core::Schema2SaveState> = None;
    let mut morphology: Option<sim_core::MorphologySaveState> = None;
    let mut contest: Option<sim_core::ContestSaveState> = None;
    let mut meta: Option<(u64, bool, bool, u64, u64)> = None;
    type OrganismColumns = (Vec<u64>, Vec<i32>, Vec<i32>, Vec<i64>, Vec<u64>, Vec<u64>);
    let mut organisms: Option<OrganismColumns> = None;
    let mut biomass = None;
    let mut ledger_counters: Option<(Ledger, sim_core::Counters)> = None;
    let mut phase2 = None;

    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err(CodecError::TruncatedSection);
        }
        let tag = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let _flags = u16::from_le_bytes(bytes[offset + 2..offset + 4].try_into().unwrap());
        let length = u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap());
        if length > MAX_SECTION_LEN {
            return Err(CodecError::ValueOutOfRange("section length"));
        }
        let length = length as usize;
        let body_start = offset + 12;
        let body_end = body_start
            .checked_add(length)
            .ok_or(CodecError::TruncatedSection)?;
        if bytes.len() < body_end + 4 {
            return Err(CodecError::TruncatedSection);
        }
        let body = &bytes[body_start..body_end];
        let declared = u32::from_le_bytes(bytes[body_end..body_end + 4].try_into().unwrap());
        if declared != crc32(body) {
            return Err(CodecError::SectionChecksumMismatch(tag));
        }
        offset = body_end + 4;

        let mut reader = Reader {
            bytes: body,
            offset: 0,
        };
        match tag {
            SECTION_CONFIG => {
                if config.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                config = Some(decode_config(&mut reader)?);
            }
            SECTION_WORLD_META => {
                if meta.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                meta = Some((
                    reader.u64()?,
                    reader.u8()? != 0,
                    reader.u8()? != 0,
                    reader.u64()?,
                    reader.u64()?,
                ));
            }
            SECTION_ORGANISMS => {
                if organisms.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let count = reader.u64()?;
                const RECORD: u64 = 8 + 4 + 4 + 8 + 8 + 8;
                // Exact-size check before allocation.
                if count
                    .checked_mul(RECORD)
                    .and_then(|body_len| body_len.checked_add(8))
                    != Some(body.len() as u64)
                {
                    return Err(CodecError::ValueOutOfRange("organism count"));
                }
                let count = count as usize;
                let mut ids = Vec::with_capacity(count);
                let mut x_fp = Vec::with_capacity(count);
                let mut y_fp = Vec::with_capacity(count);
                let mut energy = Vec::with_capacity(count);
                let mut ages = Vec::with_capacity(count);
                let mut cooldowns = Vec::with_capacity(count);
                for _ in 0..count {
                    ids.push(reader.u64()?);
                    x_fp.push(reader.i32()?);
                    y_fp.push(reader.i32()?);
                    energy.push(reader.i64()?);
                    ages.push(reader.u64()?);
                    cooldowns.push(reader.u64()?);
                }
                organisms = Some((ids, x_fp, y_fp, energy, ages, cooldowns));
            }
            SECTION_BIOMASS => {
                if biomass.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let count = reader.u64()?;
                if count
                    .checked_mul(8)
                    .and_then(|body_len| body_len.checked_add(8))
                    != Some(body.len() as u64)
                {
                    return Err(CodecError::ValueOutOfRange("biomass count"));
                }
                let count = count as usize;
                let mut cells = Vec::with_capacity(count);
                for _ in 0..count {
                    cells.push(reader.i64()?);
                }
                biomass = Some(cells);
            }
            SECTION_LEDGER => {
                if ledger_counters.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let ledger = Ledger {
                    initial_energy_milli: reader.i128()?,
                    assimilated_milli: reader.i128()?,
                    spent_milli: reader.i128()?,
                    removed_at_death_milli: reader.i128()?,
                    initial_biomass_milli: reader.i128()?,
                    grown_milli: reader.i128()?,
                    consumed_biomass_milli: reader.i128()?,
                };
                let counters = sim_core::Counters {
                    births_total: reader.u64()?,
                    deaths_starvation_total: reader.u64()?,
                    deaths_old_age_total: reader.u64()?,
                    capacity_rejections_total: reader.u64()?,
                    dropped_events_total: reader.u64()?,
                };
                ledger_counters = Some((ledger, counters));
            }
            SECTION_PHASE2 => {
                if phase2.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                let flat_genomes = reader.u64()?;
                // Cap the declared counts against the body before allocating,
                // exactly as the climate section does. This was an equality
                // against `8 + 7 * 8` - the count word plus one word per
                // Phase 2 counter - and adding an eighth counter broke every
                // snapshot in the build, which is the same way the climate
                // check broke when its section grew. A bound is what the
                // fail-closed rule actually needs; exactness is still
                // enforced, by the trailing-bytes check every section runs at
                // the end, and that check needs no editing when a field is
                // added.
                let organism_len = (4 * 4 + 2 + 8 + 4 + 16 + 4 + 4 + 8) as u64;
                let flat_len = ((TRAIT_COUNT + sim_core::NEURAL_COUNT) * 4) as u64;
                let declared = organisms
                    .checked_mul(organism_len)
                    .and_then(|bytes| bytes.checked_add(flat_genomes.checked_mul(flat_len)?));
                if declared > Some(body.len() as u64) || declared.is_none() {
                    return Err(CodecError::ValueOutOfRange("phase2 count"));
                }
                // A flat-genome count that is neither zero nor the organism
                // count cannot describe any world: schema 1 carries one per
                // organism and schema 2 carries none.
                if flat_genomes != 0 && flat_genomes != organisms {
                    return Err(CodecError::ValueOutOfRange("phase2 flat genome count"));
                }
                let organisms = organisms as usize;
                let flat_genomes = flat_genomes as usize;
                let mut section = Phase2SaveState {
                    traits: Vec::with_capacity(flat_genomes),
                    neural: Vec::with_capacity(flat_genomes),
                    memory: Vec::with_capacity(organisms),
                    heading_bam: Vec::with_capacity(organisms),
                    speed_milli: Vec::with_capacity(organisms),
                    last_turn: Vec::with_capacity(organisms),
                    parents: Vec::with_capacity(organisms),
                    depth: Vec::with_capacity(organisms),
                    child_count: Vec::with_capacity(organisms),
                    birth_tick: Vec::with_capacity(organisms),
                    counters: Default::default(),
                };
                for _ in 0..flat_genomes {
                    let mut traits = [0.0_f32; TRAIT_COUNT];
                    for gene in traits.iter_mut() {
                        *gene = reader.f32()?;
                    }
                    let mut neural = Vec::with_capacity(sim_core::NEURAL_COUNT);
                    for _ in 0..sim_core::NEURAL_COUNT {
                        neural.push(reader.f32()?);
                    }
                    section.traits.push(traits);
                    section.neural.push(neural);
                }
                for _ in 0..organisms {
                    let mut memory = [0.0_f32; 4];
                    for value in memory.iter_mut() {
                        *value = reader.f32()?;
                    }
                    section.memory.push(memory);
                    section.heading_bam.push(reader.u16()?);
                    section.speed_milli.push(reader.i64()?);
                    section.last_turn.push(reader.f32()?);
                    section.parents.push([reader.u64()?, reader.u64()?]);
                    section.depth.push(reader.u32()?);
                    section.child_count.push(reader.u32()?);
                    section.birth_tick.push(reader.u64()?);
                }
                section.counters = sim_core::Phase2Counters {
                    paired_births_total: reader.u64()?,
                    pair_rejected_capacity_total: reader.u64()?,
                    pair_rejected_placement_total: reader.u64()?,
                    pair_rejected_energy_total: reader.u64()?,
                    pair_rejected_nonviable_total: reader.u64()?,
                    controller_faults_total: reader.u64()?,
                    mutated_trait_genes_total: reader.u64()?,
                    mutated_neural_genes_total: reader.u64()?,
                };
                phase2 = Some(section);
            }
            SECTION_CLIMATE => {
                if climate.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let count = reader.u64()?;
                // Cap the declared length against the section body before
                // any allocation. A *cap* rather than an exact-length
                // equality: the biome map that follows also contributes
                // bytes, and an exact check here would have to be edited
                // every time the section gains a field -- which is how it
                // broke when it did. Exactness is still enforced, by the
                // trailing-bytes check every section runs at the end.
                if count.checked_mul(8) > Some(body.len() as u64) {
                    return Err(CodecError::ValueOutOfRange("climate count"));
                }
                let mut moisture_milli = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    moisture_milli.push(reader.i64()?);
                }
                let biome_count = reader.u64()?;
                if biome_count > body.len() as u64 {
                    return Err(CodecError::ValueOutOfRange("climate biome cells"));
                }
                let mut biome = Vec::with_capacity(biome_count as usize);
                for _ in 0..biome_count {
                    biome.push(
                        sim_core::Biome::from_id(reader.u8()?)
                            .ok_or(CodecError::ValueOutOfRange("climate biome id"))?,
                    );
                }
                climate = Some(sim_core::ClimateSaveState {
                    moisture_milli,
                    biome,
                    capacity_loss_milli: reader.i128()?,
                });
            }
            SECTION_CONTEST => {
                if contest.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Cap before allocating: 16 bytes per organism plus the
                // carcass count that follows.
                if organisms.checked_mul(16).and_then(|len| len.checked_add(8))
                    > Some(body.len() as u64)
                {
                    return Err(CodecError::ValueOutOfRange("contest organisms"));
                }
                let mut health_milli = Vec::with_capacity(organisms as usize);
                let mut recent_damage_milli = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    health_milli.push(reader.i64()?);
                    recent_damage_milli.push(reader.i64()?);
                }
                let carcass_count = reader.u64()?;
                if carcass_count.checked_mul(32) > Some(body.len() as u64) {
                    return Err(CodecError::ValueOutOfRange("contest carcasses"));
                }
                let mut carcasses = Vec::with_capacity(carcass_count as usize);
                for _ in 0..carcass_count {
                    carcasses.push(sim_core::Carcass {
                        id: reader.u64()?,
                        x_fp: reader.i32()?,
                        y_fp: reader.i32()?,
                        energy_milli: reader.i64()?,
                        created_tick: reader.u64()?,
                    });
                }
                contest = Some(sim_core::ContestSaveState {
                    health_milli,
                    recent_damage_milli,
                    carcasses,
                    carcass_created_milli: reader.i128()?,
                    carcass_consumed_milli: reader.i128()?,
                    carcass_decayed_milli: reader.i128()?,
                    attacks_total: reader.u64()?,
                    damage_dealt_milli: reader.i128()?,
                    deaths_by_damage_total: reader.u64()?,
                    healed_milli: reader.i128()?,
                });
            }
            SECTION_PHYSIOLOGY => {
                if physiology.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Cap before allocating: 8 bytes per organism.
                if organisms.checked_mul(8) > Some(body.len() as u64) {
                    return Err(CodecError::ValueOutOfRange("physiology organisms"));
                }
                let mut cumulative_hazard_q16 = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    cumulative_hazard_q16.push(reader.i64()?);
                }
                physiology = Some(sim_core::PhysiologySaveState {
                    cumulative_hazard_q16,
                    deaths_senescence_total: reader.u64()?,
                    deaths_extrinsic_total: reader.u64()?,
                    deaths_juvenile_total: reader.u64()?,
                    thermal_cost_milli: reader.i128()?,
                    allometric_cost_milli: reader.i128()?,
                });
            }
            SECTION_SCHEMA2 => {
                if schema2.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Each organism contributes at least a length word, so a
                // count beyond the section body is refused before anything
                // is allocated.
                if organisms.checked_mul(4) > Some(body.len() as u64) {
                    return Err(CodecError::ValueOutOfRange("schema2 organisms"));
                }
                let mut genomes = Vec::with_capacity(organisms as usize);
                let mut activation_values = Vec::with_capacity(organisms as usize);
                let mut activation_prior = Vec::with_capacity(organisms as usize);
                let mut activation_faults = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    let length = reader.u32()?;
                    if length as u64 > body.len() as u64 {
                        return Err(CodecError::ValueOutOfRange("schema2 genome length"));
                    }
                    let mut genome = Vec::with_capacity(length as usize);
                    for _ in 0..length {
                        genome.push(reader.u8()?);
                    }
                    genomes.push(genome);
                    let nodes = reader.u32()?;
                    if (nodes as u64).checked_mul(8) > Some(body.len() as u64) {
                        return Err(CodecError::ValueOutOfRange("schema2 activation length"));
                    }
                    let mut values = Vec::with_capacity(nodes as usize);
                    for _ in 0..nodes {
                        values.push(f32::from_bits(reader.u32()?));
                    }
                    let mut prior = Vec::with_capacity(nodes as usize);
                    for _ in 0..nodes {
                        prior.push(f32::from_bits(reader.u32()?));
                    }
                    activation_values.push(values);
                    activation_prior.push(prior);
                    activation_faults.push(reader.u32()?);
                }
                let mut counters = sim_core::MutationCounters::default();
                for slot in [
                    &mut counters.point_applied,
                    &mut counters.duplication_applied,
                    &mut counters.deletion_applied,
                    &mut counters.insertion_applied,
                    &mut counters.transposition_applied,
                    &mut counters.rejected_homology_collision,
                    &mut counters.rejected_orphaned,
                    &mut counters.rejected_min_nodes,
                    &mut counters.rejected_no_bindings,
                    &mut counters.rejected_cap,
                    &mut counters.rejected_inapplicable,
                    &mut counters.rejected_cycle,
                    &mut counters.rejected_invalid,
                ] {
                    *slot = reader.u64()?;
                }
                schema2 = Some(sim_core::Schema2SaveState {
                    genomes,
                    activation_values,
                    activation_prior,
                    activation_faults,
                    counters,
                });
            }
            SECTION_MORPHOLOGY => {
                if morphology.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let mut counters = sim_core::DevelopCounters::default();
                for slot in [
                    &mut counters.bodies_grown,
                    &mut counters.modules_placed,
                    &mut counters.differentiations,
                    &mut counters.scale_changes,
                    &mut counters.refused_occupied,
                    &mut counters.refused_out_of_bounds,
                    &mut counters.refused_max_modules,
                    &mut counters.refused_node_budget,
                    &mut counters.nonviable_empty,
                    &mut counters.nonviable_disconnected,
                    &mut counters.nonviable_missing_type,
                    &mut counters.nonviable_other,
                ] {
                    *slot = reader.u64()?;
                }
                morphology = Some(sim_core::MorphologySaveState { counters });
            }
            unknown => return Err(CodecError::UnknownSection(unknown)),
        }
        if !reader.done() {
            return Err(CodecError::ValueOutOfRange("section trailing bytes"));
        }
    }

    let config = config.ok_or(CodecError::MissingSection(SECTION_CONFIG))?;
    let (tick, paused, extinct, next_entity_id, terrain_checksum) =
        meta.ok_or(CodecError::MissingSection(SECTION_WORLD_META))?;
    let (ids, x_fp, y_fp, energy_milli, age_ticks, cooldown_ticks) =
        organisms.ok_or(CodecError::MissingSection(SECTION_ORGANISMS))?;
    let biomass_milli = biomass.ok_or(CodecError::MissingSection(SECTION_BIOMASS))?;
    let (ledger, counters) = ledger_counters.ok_or(CodecError::MissingSection(SECTION_LEDGER))?;

    let _ = state_checksum; // verified by the restore path via World::from_state
    Ok(SaveState {
        config,
        tick,
        paused,
        extinct,
        next_entity_id,
        terrain_checksum,
        ids,
        x_fp,
        y_fp,
        energy_milli,
        age_ticks,
        cooldown_ticks,
        biomass_milli,
        ledger,
        counters,
        phase2,
        climate,
        contest,
        physiology,
        schema2,
        morphology,
    })
}

/// Encode a snapshot. `compression_level`: None = uncompressed, Some(level)
/// = zstd at that level.
pub fn encode_snapshot(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    let build = build_version.as_bytes();
    if build.len() > MAX_BUILD_LEN {
        return Err(CodecError::BuildStringTooLong(build.len()));
    }
    let payload = encode_payload(state);
    let uncompressed_len = payload.len() as u64;
    let (stored, flags) = match compression_level {
        Some(level) => (
            zstd::bulk::compress(&payload, level).map_err(|_| CodecError::DecompressionFailed)?,
            FLAG_ZSTD,
        ),
        None => (payload, 0),
    };
    let payload_crc = crc32(&stored);

    let mut out = Writer(Vec::with_capacity(HEADER_LEN + build.len() + stored.len()));
    out.0.extend_from_slice(SNAPSHOT_MAGIC);
    out.u16(FORMAT_VERSION);
    out.u16(HEADER_LEN as u16);
    out.u32(flags);
    out.u64(world_id);
    out.u64(parent_world_id);
    out.u64(state.tick);
    out.u64(state.config.world_seed);
    out.u64(state.config.stable_hash());
    out.u16(SAVE_STATE_VERSION);
    out.u16(GENOME_SCHEMA_VERSION);
    out.u16(build.len() as u16);
    out.u16(0);
    out.u64(event_log_offset);
    out.u64(uncompressed_len);
    out.u64(stored.len() as u64);
    out.u32(payload_crc);
    out.u64(state_checksum);
    out.u64(state.terrain_checksum);
    // Pad the fixed header to HEADER_LEN.
    while out.0.len() < HEADER_LEN {
        out.u8(0);
    }
    debug_assert_eq!(out.0.len(), HEADER_LEN);
    out.0.extend_from_slice(build);
    out.0.extend_from_slice(&stored);
    Ok(out.0)
}

/// Parse and validate only the header (cheap integrity/provenance check).
pub fn read_info(bytes: &[u8]) -> Result<SnapshotInfo, CodecError> {
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::TooShort);
    }
    if &bytes[0..4] != SNAPSHOT_MAGIC {
        return Err(CodecError::BadMagic);
    }
    let mut reader = Reader {
        bytes: &bytes[4..HEADER_LEN],
        offset: 0,
    };
    let format_version = reader.u16()?;
    if format_version != FORMAT_VERSION {
        return Err(CodecError::UnsupportedFormat(format_version));
    }
    let header_len = usize::from(reader.u16()?);
    if header_len != HEADER_LEN {
        return Err(CodecError::BadHeaderLength(header_len));
    }
    let flags = reader.u32()?;
    if flags & !FLAG_ZSTD != 0 {
        return Err(CodecError::UnknownFlags(flags));
    }
    let world_id = reader.u64()?;
    let parent_world_id = reader.u64()?;
    let tick = reader.u64()?;
    let seed = reader.u64()?;
    let config_hash = reader.u64()?;
    let save_state_version = reader.u16()?;
    if save_state_version != SAVE_STATE_VERSION {
        return Err(CodecError::UnsupportedSaveState(save_state_version));
    }
    let genome_schema_version = reader.u16()?;
    if genome_schema_version != GENOME_SCHEMA_VERSION {
        return Err(CodecError::UnsupportedGenomeSchema(genome_schema_version));
    }
    let build_len = usize::from(reader.u16()?);
    let _reserved = reader.u16()?;
    if build_len > MAX_BUILD_LEN {
        return Err(CodecError::BuildStringTooLong(build_len));
    }
    let event_log_offset = reader.u64()?;
    let uncompressed_len = reader.u64()?;
    if uncompressed_len > MAX_UNCOMPRESSED_LEN {
        return Err(CodecError::UncompressedTooLarge(uncompressed_len));
    }
    let stored_len = reader.u64()?;
    if stored_len > MAX_STORED_LEN {
        return Err(CodecError::StoredTooLarge(stored_len));
    }
    let payload_crc = reader.u32()?;
    let state_checksum = reader.u64()?;
    let terrain_checksum = reader.u64()?;

    let expected_total = HEADER_LEN + build_len + stored_len as usize;
    if bytes.len() != expected_total {
        return Err(CodecError::LengthMismatch {
            expected: expected_total,
            actual: bytes.len(),
        });
    }
    let build_version = std::str::from_utf8(&bytes[HEADER_LEN..HEADER_LEN + build_len])
        .map_err(|_| CodecError::ValueOutOfRange("build string"))?
        .to_owned();
    let stored = &bytes[HEADER_LEN + build_len..];
    if crc32(stored) != payload_crc {
        return Err(CodecError::PayloadChecksumMismatch);
    }
    Ok(SnapshotInfo {
        format_version,
        compressed: flags & FLAG_ZSTD != 0,
        world_id,
        parent_world_id,
        tick,
        seed,
        config_hash,
        save_state_version,
        genome_schema_version,
        build_version,
        event_log_offset,
        uncompressed_len,
        stored_len,
        state_checksum,
        terrain_checksum,
    })
}

/// Full decode to logical state (header validation included).
pub fn decode_snapshot(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    let info = read_info(bytes)?;
    let stored = &bytes[HEADER_LEN + info.build_version.len()..];
    let payload = if info.compressed {
        let decompressed = zstd::bulk::decompress(stored, info.uncompressed_len as usize)
            .map_err(|_| CodecError::DecompressionFailed)?;
        if decompressed.len() as u64 != info.uncompressed_len {
            return Err(CodecError::DecompressedLengthMismatch {
                declared: info.uncompressed_len,
                actual: decompressed.len(),
            });
        }
        decompressed
    } else {
        if stored.len() as u64 != info.uncompressed_len {
            return Err(CodecError::DecompressedLengthMismatch {
                declared: info.uncompressed_len,
                actual: stored.len(),
            });
        }
        stored.to_vec()
    };
    let state = decode_payload(&payload, info.state_checksum)?;
    Ok((info, state))
}
