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
pub const FORMAT_VERSION: u16 = 1;
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
        section.u64(phase2.traits.len() as u64);
        for index in 0..phase2.traits.len() {
            for &gene in &phase2.traits[index] {
                section.f32(gene);
            }
            for &gene in &phase2.neural[index] {
                section.f32(gene);
            }
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
        section.u64(phase2.counters.controller_faults_total);
        section.u64(phase2.counters.mutated_trait_genes_total);
        section.u64(phase2.counters.mutated_neural_genes_total);
        write_section(&mut payload, SECTION_PHASE2, section.0);
    }
    payload
}

fn decode_payload(bytes: &[u8], state_checksum: u64) -> Result<SaveState, CodecError> {
    let mut offset = 0_usize;
    let mut config = None;
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
                let count = reader.u64()?;
                // Exact per-organism record size plus the trailing counters.
                let record_len =
                    ((TRAIT_COUNT + sim_core::NEURAL_COUNT + 4) * 4 + 2 + 8 + 4 + 16 + 4 + 4 + 8)
                        as u64;
                if count
                    .checked_mul(record_len)
                    .and_then(|body_len| body_len.checked_add(8 + 7 * 8))
                    != Some(body.len() as u64)
                {
                    return Err(CodecError::ValueOutOfRange("phase2 count"));
                }
                let count = count as usize;
                let mut section = Phase2SaveState {
                    traits: Vec::with_capacity(count),
                    neural: Vec::with_capacity(count),
                    memory: Vec::with_capacity(count),
                    heading_bam: Vec::with_capacity(count),
                    speed_milli: Vec::with_capacity(count),
                    last_turn: Vec::with_capacity(count),
                    parents: Vec::with_capacity(count),
                    depth: Vec::with_capacity(count),
                    child_count: Vec::with_capacity(count),
                    birth_tick: Vec::with_capacity(count),
                    counters: Default::default(),
                };
                for _ in 0..count {
                    let mut traits = [0.0_f32; TRAIT_COUNT];
                    for gene in traits.iter_mut() {
                        *gene = reader.f32()?;
                    }
                    let mut neural = Vec::with_capacity(sim_core::NEURAL_COUNT);
                    for _ in 0..sim_core::NEURAL_COUNT {
                        neural.push(reader.f32()?);
                    }
                    let mut memory = [0.0_f32; 4];
                    for value in memory.iter_mut() {
                        *value = reader.f32()?;
                    }
                    section.traits.push(traits);
                    section.neural.push(neural);
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
                    controller_faults_total: reader.u64()?,
                    mutated_trait_genes_total: reader.u64()?,
                    mutated_neural_genes_total: reader.u64()?,
                };
                phase2 = Some(section);
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
