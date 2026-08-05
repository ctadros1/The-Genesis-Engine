//! ALFP founder-population file, format version 1 (Phase 6).
//!
//! "Any mode may instead load founders from a file: a versioned, checksummed,
//! bounded-decode founder set carrying genomes, positions, and provenance.
//! This is how a campaign starts from a previously evolved population without
//! branching a full save" (`specifications/world-origin-modes.md`).
//!
//! Layout (little-endian, matching the ALIF and ALEV codecs):
//!
//! header (fixed 64 bytes):
//!   magic "ALFP" | format u16 | header_len u16 | flags u32
//!   source_world_id u64 | source_seed u64 | source_config_hash u64
//!   source_tick u64 | genome_schema u16 | trait_count u16 | neural_count u32
//!   founder_count u32 | build_len u16 | reserved u16 | header_crc32 u32
//! then: build version string (build_len <= 64)
//! then: `founder_count` records, each
//!   cell_index u32 | trait f32 x trait_count | neural f32 x neural_count
//! then: trailing crc32 over the whole record block
//!
//! **Provenance is the point.** A founder file records the run that produced
//! it — world, seed, config hash, and tick — so a pre-adapted starting
//! condition can never be mistaken for a naive one. That is why the header
//! checksums itself: a corrupted provenance field would mislabel an
//! experiment rather than fail it, and nothing downstream would notice.
//!
//! Decode is fail-closed on the same terms as every other codec here: magic,
//! version, counts, and checksums verified before allocation; every genome
//! validated; positions checked against the terrain by the caller. There is
//! no repair path.

use sim_core::{GENOME_SCHEMA_VERSION, Genome, NEURAL_COUNT, TRAIT_COUNT};
use std::fmt;

pub const FOUNDER_MAGIC: &[u8; 4] = b"ALFP";
pub const FOUNDER_FORMAT_VERSION: u16 = 1;
const HEADER_LEN: usize = 64;
const HEADER_CRC_OFFSET: usize = 60;
const MAX_BUILD_LEN: usize = 64;
/// Bounded before any allocation: a file claiming more founders than a world
/// could hold did not come from this project.
pub const MAX_FOUNDERS: u32 = 200_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FounderError {
    TooShort,
    BadMagic,
    UnsupportedFormat(u16),
    BadHeaderLength(usize),
    UnknownFlags(u32),
    UnsupportedGenomeSchema(u16),
    /// The file was written for a different genome shape entirely.
    ShapeMismatch {
        traits: u16,
        neural: u32,
    },
    TooManyFounders(u32),
    BuildStringTooLong(usize),
    BadBuildString,
    HeaderChecksumMismatch,
    RecordChecksumMismatch,
    LengthMismatch {
        expected: usize,
        actual: usize,
    },
    /// A genome in the file is not a valid genome. Never repaired.
    InvalidGenome {
        index: u32,
    },
    /// A founder references a cell outside the world it is being loaded into.
    CellOutOfRange {
        index: u32,
        cell: u32,
        cells: u32,
    },
    Io(String),
}

impl fmt::Display for FounderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FounderError {}

/// Where a founder population came from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FounderProvenance {
    pub source_world_id: u64,
    pub source_seed: u64,
    pub source_config_hash: u64,
    pub source_tick: u64,
    pub build_version: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FounderSet {
    pub provenance: FounderProvenance,
    pub cells: Vec<u32>,
    pub genomes: Vec<Genome>,
}

impl FounderSet {
    pub fn len(&self) -> usize {
        self.genomes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.genomes.is_empty()
    }

    /// Reject any founder whose cell falls outside the target world.
    /// Position validity against terrain is the caller's check; this is the
    /// bounds check the codec can make on its own.
    pub fn validate_against(&self, cell_count: u32) -> Result<(), FounderError> {
        for (index, &cell) in self.cells.iter().enumerate() {
            if cell >= cell_count {
                return Err(FounderError::CellOutOfRange {
                    index: index as u32,
                    cell,
                    cells: cell_count,
                });
            }
        }
        Ok(())
    }
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn encode_founders(set: &FounderSet) -> Result<Vec<u8>, FounderError> {
    let build = set.provenance.build_version.as_bytes();
    if build.len() > MAX_BUILD_LEN {
        return Err(FounderError::BuildStringTooLong(build.len()));
    }
    if set.genomes.len() as u64 > u64::from(MAX_FOUNDERS) {
        return Err(FounderError::TooManyFounders(set.genomes.len() as u32));
    }
    if set.cells.len() != set.genomes.len() {
        return Err(FounderError::LengthMismatch {
            expected: set.genomes.len(),
            actual: set.cells.len(),
        });
    }

    let mut out = Vec::with_capacity(HEADER_LEN + build.len());
    out.extend_from_slice(FOUNDER_MAGIC);
    put_u16(&mut out, FOUNDER_FORMAT_VERSION);
    put_u16(&mut out, HEADER_LEN as u16);
    put_u32(&mut out, 0); // flags
    put_u64(&mut out, set.provenance.source_world_id);
    put_u64(&mut out, set.provenance.source_seed);
    put_u64(&mut out, set.provenance.source_config_hash);
    put_u64(&mut out, set.provenance.source_tick);
    put_u16(&mut out, GENOME_SCHEMA_VERSION);
    put_u16(&mut out, TRAIT_COUNT as u16);
    put_u32(&mut out, NEURAL_COUNT as u32);
    put_u32(&mut out, set.genomes.len() as u32);
    put_u16(&mut out, build.len() as u16);
    put_u16(&mut out, 0); // reserved
    assert_eq!(out.len(), HEADER_CRC_OFFSET, "founder header layout");
    let mut covered = out.clone();
    covered.extend_from_slice(build);
    put_u32(&mut out, crate::codec::crc32(&covered));
    assert_eq!(out.len(), HEADER_LEN, "founder header layout");
    out.extend_from_slice(build);

    let mut records =
        Vec::with_capacity(set.genomes.len() * (4 + (TRAIT_COUNT + NEURAL_COUNT) * 4));
    for (index, genome) in set.genomes.iter().enumerate() {
        put_u32(&mut records, set.cells[index]);
        for gene in genome.traits() {
            records.extend_from_slice(&gene.to_le_bytes());
        }
        for weight in genome.neural() {
            records.extend_from_slice(&weight.to_le_bytes());
        }
    }
    let record_crc = crate::codec::crc32(&records);
    out.extend_from_slice(&records);
    put_u32(&mut out, record_crc);
    Ok(out)
}

pub fn decode_founders(bytes: &[u8]) -> Result<FounderSet, FounderError> {
    if bytes.len() < HEADER_LEN {
        return Err(FounderError::TooShort);
    }
    if &bytes[0..4] != FOUNDER_MAGIC {
        return Err(FounderError::BadMagic);
    }
    let u16_at = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    let u32_at = |offset: usize| {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    };
    let u64_at = |offset: usize| {
        let mut value = [0_u8; 8];
        value.copy_from_slice(&bytes[offset..offset + 8]);
        u64::from_le_bytes(value)
    };

    let format_version = u16_at(4);
    if format_version != FOUNDER_FORMAT_VERSION {
        return Err(FounderError::UnsupportedFormat(format_version));
    }
    let header_len = usize::from(u16_at(6));
    if header_len != HEADER_LEN {
        return Err(FounderError::BadHeaderLength(header_len));
    }
    let flags = u32_at(8);
    if flags != 0 {
        return Err(FounderError::UnknownFlags(flags));
    }
    let genome_schema = u16_at(44);
    if genome_schema != GENOME_SCHEMA_VERSION {
        return Err(FounderError::UnsupportedGenomeSchema(genome_schema));
    }
    let traits = u16_at(46);
    let neural = u32_at(48);
    if usize::from(traits) != TRAIT_COUNT || neural as usize != NEURAL_COUNT {
        return Err(FounderError::ShapeMismatch { traits, neural });
    }
    let founder_count = u32_at(52);
    if founder_count > MAX_FOUNDERS {
        return Err(FounderError::TooManyFounders(founder_count));
    }
    let build_len = usize::from(u16_at(56));
    if build_len > MAX_BUILD_LEN {
        return Err(FounderError::BuildStringTooLong(build_len));
    }
    let recorded_header_crc = u32_at(HEADER_CRC_OFFSET);

    let build_end = HEADER_LEN + build_len;
    let build_bytes = bytes
        .get(HEADER_LEN..build_end)
        .ok_or(FounderError::TooShort)?;
    let mut covered = bytes[..HEADER_CRC_OFFSET].to_vec();
    covered.extend_from_slice(build_bytes);
    if crate::codec::crc32(&covered) != recorded_header_crc {
        return Err(FounderError::HeaderChecksumMismatch);
    }
    let build_version = std::str::from_utf8(build_bytes)
        .map_err(|_| FounderError::BadBuildString)?
        .to_owned();

    // Every length is now known and capped, so the exact total is checked
    // before a single byte is parsed or allocated.
    let record_len = 4 + (TRAIT_COUNT + NEURAL_COUNT) * 4;
    let expected = build_end
        .checked_add(
            (founder_count as usize)
                .checked_mul(record_len)
                .ok_or(FounderError::TooManyFounders(founder_count))?,
        )
        .and_then(|value| value.checked_add(4))
        .ok_or(FounderError::TooManyFounders(founder_count))?;
    if bytes.len() != expected {
        return Err(FounderError::LengthMismatch {
            expected,
            actual: bytes.len(),
        });
    }
    let records = &bytes[build_end..expected - 4];
    let recorded_record_crc = u32::from_le_bytes(
        bytes[expected - 4..expected]
            .try_into()
            .map_err(|_| FounderError::TooShort)?,
    );
    if crate::codec::crc32(records) != recorded_record_crc {
        return Err(FounderError::RecordChecksumMismatch);
    }

    let mut cells = Vec::with_capacity(founder_count as usize);
    let mut genomes = Vec::with_capacity(founder_count as usize);
    let mut offset = 0_usize;
    let read_f32 = |records: &[u8], offset: usize| -> f32 {
        f32::from_le_bytes([
            records[offset],
            records[offset + 1],
            records[offset + 2],
            records[offset + 3],
        ])
    };
    for index in 0..founder_count {
        let cell = u32::from_le_bytes([
            records[offset],
            records[offset + 1],
            records[offset + 2],
            records[offset + 3],
        ]);
        offset += 4;
        let mut trait_genes = [0.0_f32; TRAIT_COUNT];
        for gene in trait_genes.iter_mut() {
            *gene = read_f32(records, offset);
            offset += 4;
        }
        let mut neural_genes = Vec::with_capacity(NEURAL_COUNT);
        for _ in 0..NEURAL_COUNT {
            neural_genes.push(read_f32(records, offset));
            offset += 4;
        }
        // Validated, never repaired: a non-finite or out-of-range gene is a
        // rejection, not something to clamp into acceptability.
        let genome = Genome::validated(trait_genes, neural_genes)
            .map_err(|_| FounderError::InvalidGenome { index })?;
        cells.push(cell);
        genomes.push(genome);
    }

    Ok(FounderSet {
        provenance: FounderProvenance {
            source_world_id: u64_at(12),
            source_seed: u64_at(20),
            source_config_hash: u64_at(28),
            source_tick: u64_at(36),
            build_version,
        },
        cells,
        genomes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{SimConfig, World};

    fn sample_set(count: usize) -> FounderSet {
        let mut config = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
        config.cells_x = 64;
        config.cells_y = 64;
        config.initial_organisms = count as u32;
        config.max_entities = 600;
        let world = World::new(config).unwrap();
        let ids = world.organism_ids_view().to_vec();
        let mut cells = Vec::new();
        let mut genomes = Vec::new();
        for (index, &id) in ids.iter().enumerate() {
            let detail = world.organism_detail(id).unwrap();
            let phase2 = detail.phase2.unwrap();
            cells.push(index as u32);
            genomes
                .push(Genome::validated(phase2.trait_genes, vec![0.25_f32; NEURAL_COUNT]).unwrap());
        }
        FounderSet {
            provenance: FounderProvenance {
                source_world_id: 1,
                source_seed: config.world_seed,
                source_config_hash: config.stable_hash(),
                source_tick: 4_242,
                build_version: "lifesim-test".to_owned(),
            },
            cells,
            genomes,
        }
    }

    #[test]
    fn the_encoded_header_is_exactly_the_declared_length() {
        // Checked in every profile. A `debug_assert` here was compiled out
        // of release and let a wrong offset constant produce a header the
        // decoder could not read.
        let set = sample_set(1);
        let bytes = encode_founders(&set).unwrap();
        assert!(bytes.len() > HEADER_LEN);
        assert_eq!(
            u16::from_le_bytes([bytes[6], bytes[7]]) as usize,
            HEADER_LEN
        );
    }

    #[test]
    fn round_trip_preserves_every_founder_and_its_provenance() {
        let set = sample_set(40);
        let bytes = encode_founders(&set).unwrap();
        let decoded = decode_founders(&bytes).unwrap();
        assert_eq!(decoded, set);
        assert_eq!(decoded.provenance.source_tick, 4_242);
        assert_eq!(decoded.len(), 40);
        // Encoding is deterministic.
        assert_eq!(encode_founders(&set).unwrap(), bytes);
    }

    #[test]
    fn an_empty_set_is_valid() {
        let mut set = sample_set(1);
        set.cells.clear();
        set.genomes.clear();
        let bytes = encode_founders(&set).unwrap();
        let decoded = decode_founders(&bytes).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn provenance_corruption_is_caught_by_the_header_checksum() {
        let set = sample_set(8);
        let valid = encode_founders(&set).unwrap();
        // Source seed, config hash, and tick are provenance: nothing else
        // would notice a flipped bit in them.
        for offset in [12, 20, 28, 36] {
            let mut bad = valid.clone();
            bad[offset] ^= 0x01;
            assert_eq!(
                decode_founders(&bad).unwrap_err(),
                FounderError::HeaderChecksumMismatch,
                "corruption at byte {offset} was accepted"
            );
        }
    }

    #[test]
    fn structural_rejections_are_typed() {
        let set = sample_set(4);
        let valid = encode_founders(&set).unwrap();

        assert_eq!(
            decode_founders(&valid[..10]).unwrap_err(),
            FounderError::TooShort
        );
        let mut bad = valid.clone();
        bad[0] = b'X';
        assert_eq!(decode_founders(&bad).unwrap_err(), FounderError::BadMagic);

        let mut bad = valid.clone();
        bad[4..6].copy_from_slice(&99_u16.to_le_bytes());
        assert_eq!(
            decode_founders(&bad).unwrap_err(),
            FounderError::UnsupportedFormat(99)
        );

        // A declared count far beyond the cap is refused before allocation.
        let mut bad = valid.clone();
        bad[52..56].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode_founders(&bad),
            Err(FounderError::TooManyFounders(_))
        ));

        // A record bit flip is caught by the record checksum.
        let mut bad = valid.clone();
        let last = bad.len() - 8;
        bad[last] ^= 0x01;
        assert_eq!(
            decode_founders(&bad).unwrap_err(),
            FounderError::RecordChecksumMismatch
        );
    }

    #[test]
    fn an_invalid_genome_is_rejected_never_repaired() {
        let set = sample_set(4);
        let mut bytes = encode_founders(&set).unwrap();
        // Stomp the first trait gene of founder 0 with a NaN, then repair
        // the record checksum so the genome validator is what has to catch
        // it rather than the framing.
        let build_end = HEADER_LEN + "lifesim-test".len();
        let gene = build_end + 4;
        bytes[gene..gene + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        let records_end = bytes.len() - 4;
        let crc = crate::codec::crc32(&bytes[build_end..records_end]);
        bytes[records_end..].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(
            decode_founders(&bytes).unwrap_err(),
            FounderError::InvalidGenome { index: 0 }
        );
    }

    #[test]
    fn cells_are_bounds_checked_against_the_target_world() {
        let mut set = sample_set(4);
        set.cells[2] = 999_999;
        assert!(matches!(
            set.validate_against(4_096),
            Err(FounderError::CellOutOfRange { index: 2, .. })
        ));
        set.cells[2] = 10;
        assert!(set.validate_against(4_096).is_ok());
    }
}
