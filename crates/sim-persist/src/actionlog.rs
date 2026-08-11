//! ALAC per-individual action sample log, format version 1 (Phase 11).
//!
//! # Why a binary artifact rather than a text series
//!
//! Phase 10's `.almo` morphology series is text, and that was right for what
//! it carries: one sample is six world-level scalars, so a readable line per
//! sample is cheaper to audit than a versioned codec. This is the other case.
//! One sample here is `population x ACTION_CLASS_COUNT` counters plus an
//! identity and an age per organism, so a 2,000-organism world sampled every
//! 500 ticks over 10^6 ticks writes 2,000 rows x 2,000 samples = 4 million
//! records. As text at ~60 characters a record that is roughly a quarter of a
//! gigabyte per run, before any parsing cost; framed here it is 44 bytes a
//! record, about 176 MB, and every count is read with no float round trip.
//!
//! So the choice follows Phase 7's `.alss` precedent instead: a dense
//! per-individual dump gets a versioned binary format with per-segment CRCs
//! and a self-checking header.
//!
//! # Why identity is in every record
//!
//! C11.1's comparison is *within one organism* across two samples. An
//! artifact that recorded rows in array order and nothing else would let an
//! analysis line up two samples by index - and index order changes whenever
//! anything dies, so the "before" and "after" of an individual would silently
//! become two different individuals. That is the exact failure the criterion
//! exists to avoid, arriving through the file format instead of through the
//! kernel. The entity id is therefore a per-record field and not an
//! optimization to be removed later; `age_ticks` rides with it so an analysis
//! can also confirm the organism was alive across the whole window rather
//! than born inside it.
//!
//! # Counts are cumulative
//!
//! Each record is the organism's histogram since birth, never a per-window
//! delta. A window is the difference of two records with the same id, which
//! is strictly more information, and - decisively - it means the sampler only
//! reads. A per-window artifact would have to reset the kernel's counters at
//! every sample, and those counters are checksummed world state, so sampling
//! would change what the world computes. See
//! `sim-experiment/tests/action_sampling.rs`.
//!
//! Layout (little-endian, matching ALIF, ALEV and ALSS):
//!
//! header (fixed 72 bytes):
//!   magic "ALAC" | format u16 | header_len u16 | flags u32
//!   world_id u64 | seed u64 | config_hash u64 | terrain_checksum u64
//!   class_count u32 | sample_interval u32 | max_organisms u32
//!   policy_hash u64 | build_len u16 | reserved u16 | header_crc32 u32
//! then: build version string (build_len <= 64 bytes)
//! then: zero or more segments in strictly ascending tick order:
//!   magic "ACL1" | tick u64 | count u32 | body_len u32 | body | crc32 u32
//!
//! The body is `count` records of
//! `id u64 | age_ticks u64 | counts[class_count] u32`.
//!
//! `class_count` and `policy_hash` are provenance rather than framing, and
//! they are the two fields that stop an analysis silently reading a histogram
//! under the wrong convention: the class set and the precedence rule are what
//! a column *means*, so a file written under a different
//! `ACTION_CENSUS_POLICY_VERSION` is refused rather than reinterpreted.
//!
//! Appending is the only supported mutation. There is no rewrite path and no
//! repair path.

use crate::codec::crc32;
use sim_core::ACTION_CLASS_COUNT;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

pub const ACTION_LOG_MAGIC: &[u8; 4] = b"ALAC";
pub const ACTION_LOG_FORMAT_VERSION: u16 = 1;
const SEGMENT_MAGIC: &[u8; 4] = b"ACL1";
const ACTION_HEADER_LEN: usize = 72;
/// Byte offset of the header CRC inside the fixed header.
const HEADER_CRC_OFFSET: usize = 68;
const SEGMENT_HEADER_LEN: usize = 20;
const MAX_BUILD_LEN: usize = 64;
/// `id u64 | age_ticks u64 | counts[ACTION_CLASS_COUNT] u32`.
const RECORD_LEN: usize = 8 + 8 + 4 * ACTION_CLASS_COUNT;

/// Absolute cap on a declared organism count, independent of the header's
/// own `max_organisms`. A file claiming more than this cannot have come from
/// any kernel this project supports, so it is refused before the header's
/// value is trusted enough to use as a bound.
pub const MAX_SAMPLE_ORGANISMS: u32 = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionLogError {
    TooShort,
    BadMagic,
    UnsupportedFormat(u16),
    BadHeaderLength(usize),
    UnknownFlags(u32),
    BuildStringTooLong(usize),
    BadBuildString,
    HeaderChecksumMismatch,
    /// The file was written under a different action-class set. Refused
    /// rather than truncated or padded: a column index only means something
    /// against the class list that produced it.
    ClassCountMismatch {
        declared: u32,
        expected: u32,
    },
    /// The file was written under a different `ACTION_CENSUS_POLICY_VERSION`.
    /// The class *count* can match while the precedence rule differs, and a
    /// histogram under one precedence is not comparable with a histogram
    /// under another, so both are checked.
    PolicyMismatch {
        declared: u64,
        expected: u64,
    },
    SampleCountTooLarge(u32),
    SegmentBodyLengthMismatch {
        tick: u64,
    },
    SegmentChecksumMismatch {
        tick: u64,
    },
    /// The trailing bytes are shorter than the segment they declare. A crash
    /// between `write` and `sync` produces exactly this.
    TruncatedSegment {
        offset: usize,
    },
    BadSegmentMagic {
        offset: usize,
    },
    /// Segments must be in strictly ascending tick order.
    TickOutOfOrder {
        previous: u64,
        found: u64,
    },
    Io(String),
}

impl fmt::Display for ActionLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Provenance an action sample file carries about the world that wrote it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionLogInfo {
    pub format_version: u16,
    pub world_id: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub terrain_checksum: u64,
    /// Columns per record. Always `ACTION_CLASS_COUNT` for a file this build
    /// wrote; checked rather than assumed on read.
    pub class_count: u32,
    pub sample_interval_ticks: u32,
    /// The world's `max_entities`; the decode bound for a segment count.
    pub max_organisms: u32,
    /// FNV-1a of `ACTION_CENSUS_POLICY_VERSION`.
    pub policy_hash: u64,
    pub build_version: String,
}

/// One organism's cumulative histogram at one sample tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionRecord {
    pub id: u64,
    pub age_ticks: u64,
    pub counts: [u32; ACTION_CLASS_COUNT],
}

/// One sampled population.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionSampleSet {
    pub tick: u64,
    /// In stable entity-ID order, exactly as the kernel stores them.
    pub records: Vec<ActionRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionLogScan {
    pub info: ActionLogInfo,
    pub samples: Vec<ActionSampleSet>,
    pub bytes_consumed: usize,
    /// Set only by `decode_action_prefix`: the file has a torn tail that was
    /// reported rather than repaired.
    pub truncated_at: Option<usize>,
}

/// The policy hash this build writes and requires.
pub fn policy_hash() -> u64 {
    sim_core::fnv1a64(sim_core::ACTION_CENSUS_POLICY_VERSION.as_bytes())
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

fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    let mut buffer = [0_u8; 8];
    buffer.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(buffer)
}

pub fn encode_header(info: &ActionLogInfo) -> Result<Vec<u8>, ActionLogError> {
    if info.build_version.len() > MAX_BUILD_LEN {
        return Err(ActionLogError::BuildStringTooLong(info.build_version.len()));
    }
    if info.class_count != ACTION_CLASS_COUNT as u32 {
        return Err(ActionLogError::ClassCountMismatch {
            declared: info.class_count,
            expected: ACTION_CLASS_COUNT as u32,
        });
    }
    if info.max_organisms > MAX_SAMPLE_ORGANISMS {
        return Err(ActionLogError::SampleCountTooLarge(info.max_organisms));
    }
    let mut out = Vec::with_capacity(ACTION_HEADER_LEN + info.build_version.len());
    out.extend_from_slice(ACTION_LOG_MAGIC);
    put_u16(&mut out, ACTION_LOG_FORMAT_VERSION);
    put_u16(&mut out, ACTION_HEADER_LEN as u16);
    put_u32(&mut out, 0); // flags
    put_u64(&mut out, info.world_id);
    put_u64(&mut out, info.seed);
    put_u64(&mut out, info.config_hash);
    put_u64(&mut out, info.terrain_checksum);
    put_u32(&mut out, info.class_count);
    put_u32(&mut out, info.sample_interval_ticks);
    put_u32(&mut out, info.max_organisms);
    put_u64(&mut out, info.policy_hash);
    put_u16(&mut out, info.build_version.len() as u16);
    put_u16(&mut out, 0); // reserved
    debug_assert_eq!(out.len(), HEADER_CRC_OFFSET);
    put_u32(&mut out, 0); // CRC placeholder
    out.extend_from_slice(info.build_version.as_bytes());

    let mut checked = out[..HEADER_CRC_OFFSET].to_vec();
    checked.extend_from_slice(info.build_version.as_bytes());
    let checksum = crc32(&checked);
    out[HEADER_CRC_OFFSET..HEADER_CRC_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

/// Decode and verify the header, returning it and its byte length.
pub fn read_action_info(bytes: &[u8]) -> Result<(ActionLogInfo, usize), ActionLogError> {
    if bytes.len() < ACTION_HEADER_LEN {
        return Err(ActionLogError::TooShort);
    }
    if &bytes[0..4] != ACTION_LOG_MAGIC {
        return Err(ActionLogError::BadMagic);
    }
    let format_version = read_u16(bytes, 4);
    if format_version != ACTION_LOG_FORMAT_VERSION {
        return Err(ActionLogError::UnsupportedFormat(format_version));
    }
    let header_len = read_u16(bytes, 6) as usize;
    if header_len != ACTION_HEADER_LEN {
        return Err(ActionLogError::BadHeaderLength(header_len));
    }
    let flags = read_u32(bytes, 8);
    if flags != 0 {
        return Err(ActionLogError::UnknownFlags(flags));
    }
    let build_len = read_u16(bytes, 64) as usize;
    if build_len > MAX_BUILD_LEN {
        return Err(ActionLogError::BuildStringTooLong(build_len));
    }
    let total = ACTION_HEADER_LEN + build_len;
    if bytes.len() < total {
        return Err(ActionLogError::TooShort);
    }
    let build_bytes = &bytes[ACTION_HEADER_LEN..total];
    let build_version =
        std::str::from_utf8(build_bytes).map_err(|_| ActionLogError::BadBuildString)?;

    // The header checks itself before any of its own fields are trusted,
    // exactly as ALSS's does: nothing further down the file would notice a
    // corrupted provenance field, and `class_count` below is used as an
    // allocation bound.
    let declared = read_u32(bytes, HEADER_CRC_OFFSET);
    let mut checked = bytes[..HEADER_CRC_OFFSET].to_vec();
    checked.extend_from_slice(build_bytes);
    if crc32(&checked) != declared {
        return Err(ActionLogError::HeaderChecksumMismatch);
    }

    let class_count = read_u32(bytes, 44);
    if class_count != ACTION_CLASS_COUNT as u32 {
        return Err(ActionLogError::ClassCountMismatch {
            declared: class_count,
            expected: ACTION_CLASS_COUNT as u32,
        });
    }
    let declared_policy = read_u64(bytes, 56);
    if declared_policy != policy_hash() {
        return Err(ActionLogError::PolicyMismatch {
            declared: declared_policy,
            expected: policy_hash(),
        });
    }
    let max_organisms = read_u32(bytes, 52);
    if max_organisms > MAX_SAMPLE_ORGANISMS {
        return Err(ActionLogError::SampleCountTooLarge(max_organisms));
    }

    Ok((
        ActionLogInfo {
            format_version,
            world_id: read_u64(bytes, 12),
            seed: read_u64(bytes, 20),
            config_hash: read_u64(bytes, 28),
            terrain_checksum: read_u64(bytes, 36),
            class_count,
            sample_interval_ticks: read_u32(bytes, 48),
            max_organisms,
            policy_hash: declared_policy,
            build_version: build_version.to_owned(),
        },
        total,
    ))
}

pub fn encode_segment(
    tick: u64,
    records: &[ActionRecord],
    max_organisms: u32,
) -> Result<Vec<u8>, ActionLogError> {
    let count = records.len();
    if count > max_organisms as usize || count > MAX_SAMPLE_ORGANISMS as usize {
        return Err(ActionLogError::SampleCountTooLarge(count as u32));
    }
    let body_len = count * RECORD_LEN;
    let mut out = Vec::with_capacity(SEGMENT_HEADER_LEN + body_len + 4);
    out.extend_from_slice(SEGMENT_MAGIC);
    put_u64(&mut out, tick);
    put_u32(&mut out, count as u32);
    put_u32(&mut out, body_len as u32);
    debug_assert_eq!(out.len(), SEGMENT_HEADER_LEN);
    for record in records {
        put_u64(&mut out, record.id);
        put_u64(&mut out, record.age_ticks);
        for value in record.counts {
            put_u32(&mut out, value);
        }
    }
    // The CRC covers the segment header as well as the body, matching ALEV
    // and ALSS. Covering the body alone would leave `tick` unchecked, and a
    // flipped bit there moves a whole sample in time instead of failing the
    // decode - which for this file would silently move C11.1's before/after
    // boundary.
    let checksum = crc32(&out);
    put_u32(&mut out, checksum);
    Ok(out)
}

/// Strict decode. Any torn or corrupted byte anywhere is an error.
pub fn decode_action(bytes: &[u8]) -> Result<ActionLogScan, ActionLogError> {
    decode_inner(bytes, false)
}

/// Decode the longest intact prefix, reporting where it stops instead of
/// repairing it.
pub fn decode_action_prefix(bytes: &[u8]) -> Result<ActionLogScan, ActionLogError> {
    decode_inner(bytes, true)
}

fn decode_inner(bytes: &[u8], tolerate_torn_tail: bool) -> Result<ActionLogScan, ActionLogError> {
    let (info, header_len) = read_action_info(bytes)?;
    let mut samples = Vec::new();
    let mut offset = header_len;
    let mut previous_tick: Option<u64> = None;
    let mut truncated_at = None;

    while offset < bytes.len() {
        match decode_one(bytes, offset, &info, previous_tick) {
            Ok((sample, next)) => {
                previous_tick = Some(sample.tick);
                samples.push(sample);
                offset = next;
            }
            Err(ActionLogError::TruncatedSegment { offset: at }) if tolerate_torn_tail => {
                truncated_at = Some(at);
                break;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(ActionLogScan {
        info,
        samples,
        bytes_consumed: offset,
        truncated_at,
    })
}

fn decode_one(
    bytes: &[u8],
    offset: usize,
    info: &ActionLogInfo,
    previous_tick: Option<u64>,
) -> Result<(ActionSampleSet, usize), ActionLogError> {
    if offset + SEGMENT_HEADER_LEN > bytes.len() {
        return Err(ActionLogError::TruncatedSegment { offset });
    }
    if &bytes[offset..offset + 4] != SEGMENT_MAGIC {
        return Err(ActionLogError::BadSegmentMagic { offset });
    }
    let tick = read_u64(bytes, offset + 4);
    let count = read_u32(bytes, offset + 12);
    let body_len = read_u32(bytes, offset + 16);

    // Every declared length is capped **before** it is used to size
    // anything. A count near 2^32 with a matching `body_len` would otherwise
    // reserve a multi-gigabyte vector on a file a few hundred bytes long,
    // which is the fail-open D-091 records and which no bit-flipping sweep
    // can reach (standing rule 2).
    if count > info.max_organisms || count > MAX_SAMPLE_ORGANISMS {
        return Err(ActionLogError::SampleCountTooLarge(count));
    }
    if u64::from(body_len) != u64::from(count) * RECORD_LEN as u64 {
        return Err(ActionLogError::SegmentBodyLengthMismatch { tick });
    }
    if let Some(previous) = previous_tick
        && tick <= previous
    {
        return Err(ActionLogError::TickOutOfOrder {
            previous,
            found: tick,
        });
    }

    let body_start = offset + SEGMENT_HEADER_LEN;
    let body_end = body_start + body_len as usize;
    let segment_end = body_end + 4;
    if segment_end > bytes.len() {
        return Err(ActionLogError::TruncatedSegment { offset });
    }
    let declared = read_u32(bytes, body_end);
    if crc32(&bytes[offset..body_end]) != declared {
        return Err(ActionLogError::SegmentChecksumMismatch { tick });
    }

    let mut records = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let at = body_start + index * RECORD_LEN;
        let mut counts = [0_u32; ACTION_CLASS_COUNT];
        for (slot, value) in counts.iter_mut().enumerate() {
            *value = read_u32(bytes, at + 16 + slot * 4);
        }
        records.push(ActionRecord {
            id: read_u64(bytes, at),
            age_ticks: read_u64(bytes, at + 8),
            counts,
        });
    }
    Ok((ActionSampleSet { tick, records }, segment_end))
}

/// Streaming writer, same durability ordering as the event and spatial logs.
pub struct ActionLogWriter {
    file: BufWriter<File>,
    offset: u64,
    samples: u64,
    max_organisms: u32,
}

impl ActionLogWriter {
    /// Create a new file, failing if one already exists. A run gets a new
    /// file, so a stale sample log can never be silently extended.
    pub fn create(path: &Path, info: &ActionLogInfo) -> Result<Self, ActionLogError> {
        let header = encode_header(info)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| ActionLogError::Io(error.to_string()))?;
        let mut writer = BufWriter::new(file);
        writer
            .write_all(&header)
            .map_err(|error| ActionLogError::Io(error.to_string()))?;
        Ok(Self {
            file: writer,
            offset: header.len() as u64,
            samples: 0,
            max_organisms: info.max_organisms,
        })
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// Append one sample. An empty world still writes a segment: "nobody was
    /// alive at tick T" is a measurement, and a reader that cannot tell it
    /// from "no sample was taken" would silently shorten the series.
    pub fn append(&mut self, tick: u64, records: &[ActionRecord]) -> Result<(), ActionLogError> {
        let segment = encode_segment(tick, records, self.max_organisms)?;
        self.file
            .write_all(&segment)
            .map_err(|error| ActionLogError::Io(error.to_string()))?;
        self.offset += segment.len() as u64;
        self.samples += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), ActionLogError> {
        self.file
            .flush()
            .map_err(|error| ActionLogError::Io(error.to_string()))
    }

    pub fn sync(&mut self) -> Result<(), ActionLogError> {
        self.flush()?;
        self.file
            .get_ref()
            .sync_all()
            .map_err(|error| ActionLogError::Io(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> ActionLogInfo {
        ActionLogInfo {
            format_version: ACTION_LOG_FORMAT_VERSION,
            world_id: 7,
            seed: 0x5eed_cafe_f00d_beef,
            config_hash: 0x0123_4567_89ab_cdef,
            terrain_checksum: 0xfeed_face_dead_beef,
            class_count: ACTION_CLASS_COUNT as u32,
            sample_interval_ticks: 50,
            max_organisms: 5_000,
            policy_hash: policy_hash(),
            build_version: "test-build".to_owned(),
        }
    }

    fn record(id: u64, age: u64, first: u32) -> ActionRecord {
        let mut counts = [0_u32; ACTION_CLASS_COUNT];
        for (slot, value) in counts.iter_mut().enumerate() {
            *value = first + slot as u32;
        }
        ActionRecord {
            id,
            age_ticks: age,
            counts,
        }
    }

    fn file_with(samples: &[(u64, Vec<ActionRecord>)]) -> Vec<u8> {
        let mut bytes = encode_header(&info()).expect("header");
        for (tick, records) in samples {
            bytes.extend_from_slice(&encode_segment(*tick, records, 5_000).expect("segment"));
        }
        bytes
    }

    /// Reseal the header CRC after patching a header field, so the patched
    /// value is *reached* rather than rejected as corruption (standing rule
    /// 2).
    fn reseal_header(bytes: &mut [u8]) {
        let build_len = read_u16(bytes, 64) as usize;
        let mut checked = bytes[..HEADER_CRC_OFFSET].to_vec();
        checked.extend_from_slice(&bytes[ACTION_HEADER_LEN..ACTION_HEADER_LEN + build_len]);
        let checksum = crc32(&checked);
        bytes[HEADER_CRC_OFFSET..HEADER_CRC_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());
    }

    /// Reseal a segment CRC after patching its declared count or length.
    fn reseal_segment(bytes: &mut [u8], offset: usize, body_end: usize) {
        let checksum = crc32(&bytes[offset..body_end]);
        bytes[body_end..body_end + 4].copy_from_slice(&checksum.to_le_bytes());
    }

    #[test]
    fn a_round_trip_preserves_provenance_and_every_count() {
        let samples = vec![
            (10_u64, vec![record(1, 10, 0), record(4, 7, 100)]),
            (20, vec![record(4, 17, 200)]),
            (30, Vec::new()),
        ];
        let bytes = file_with(&samples);
        let scan = decode_action(&bytes).expect("decodes");
        assert_eq!(scan.info, info());
        assert_eq!(scan.samples.len(), 3);
        assert_eq!(scan.samples[0].records, samples[0].1);
        assert_eq!(scan.samples[2].records, Vec::new());
        assert_eq!(scan.bytes_consumed, bytes.len());
        assert_eq!(scan.truncated_at, None);
        // The last column round trips: a record loop that wrote
        // `class_count - 1` values would still balance `body_len` if
        // `RECORD_LEN` were computed the same wrong way, and only reading the
        // final column back catches it.
        assert_eq!(
            scan.samples[0].records[1].counts[ACTION_CLASS_COUNT - 1],
            100 + ACTION_CLASS_COUNT as u32 - 1
        );
    }

    #[test]
    fn a_declared_count_is_bounded_before_it_sizes_anything() {
        // Standing rule 2: the declared count is patched to adversarial
        // values and **the CRCs are resealed**, so each value is reached by
        // the decoder rather than rejected as corruption. A bit-flip sweep
        // cannot produce a count near 2^32 and would never exercise this.
        let base = file_with(&[(10_u64, vec![record(1, 10, 0), record(2, 10, 5)])]);
        let header_len = ACTION_HEADER_LEN + info().build_version.len();
        let body_end = header_len + SEGMENT_HEADER_LEN + 2 * RECORD_LEN;

        for count in [
            u32::MAX,
            u32::MAX / RECORD_LEN as u32,
            MAX_SAMPLE_ORGANISMS + 1,
            info().max_organisms + 1,
        ] {
            let mut bytes = base.clone();
            bytes[header_len + 12..header_len + 16].copy_from_slice(&count.to_le_bytes());
            reseal_segment(&mut bytes, header_len, body_end);
            assert_eq!(
                decode_action(&bytes),
                Err(ActionLogError::SampleCountTooLarge(count)),
                "count {count} was not refused before allocation"
            );
        }

        // A count whose implied body length is the *actual* body length but
        // whose record count disagrees: caught by the length cross-check
        // rather than by the cap.
        let mut bytes = base.clone();
        bytes[header_len + 12..header_len + 16].copy_from_slice(&1_u32.to_le_bytes());
        reseal_segment(&mut bytes, header_len, body_end);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::SegmentBodyLengthMismatch { tick: 10 })
        );

        // ...and the mirror image: `body_len` patched to the buffer length
        // with the count left alone.
        let mut bytes = base.clone();
        let declared = u32::try_from(base.len()).expect("small");
        bytes[header_len + 16..header_len + 20].copy_from_slice(&declared.to_le_bytes());
        reseal_segment(&mut bytes, header_len, body_end);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::SegmentBodyLengthMismatch { tick: 10 })
        );
    }

    #[test]
    fn a_header_from_a_different_class_set_or_policy_is_refused() {
        // The two fields that stop an analysis reading a histogram under the
        // wrong convention. Both are patched **and resealed**, so each is
        // reached rather than failing the header CRC first - the trap that
        // would make this test pass for the wrong reason.
        let mut bytes = file_with(&[(10_u64, vec![record(1, 10, 0)])]);
        bytes[44..48].copy_from_slice(&(ACTION_CLASS_COUNT as u32 + 1).to_le_bytes());
        reseal_header(&mut bytes);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::ClassCountMismatch {
                declared: ACTION_CLASS_COUNT as u32 + 1,
                expected: ACTION_CLASS_COUNT as u32,
            })
        );

        let mut bytes = file_with(&[(10_u64, vec![record(1, 10, 0)])]);
        bytes[56..64].copy_from_slice(&0xdead_beef_u64.to_le_bytes());
        reseal_header(&mut bytes);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::PolicyMismatch {
                declared: 0xdead_beef,
                expected: policy_hash(),
            })
        );

        // And the header CRC itself still guards: an unresealed patch fails
        // with the checksum error, which is what makes the two assertions
        // above evidence that the *field* checks fired rather than the CRC.
        let mut bytes = file_with(&[(10_u64, vec![record(1, 10, 0)])]);
        bytes[44..48].copy_from_slice(&(ACTION_CLASS_COUNT as u32 + 1).to_le_bytes());
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::HeaderChecksumMismatch)
        );
    }

    #[test]
    fn every_other_declared_header_field_is_refused_after_the_crc_is_resealed() {
        // Standing rule 2 applied to the header fields the class-set and
        // policy test does not reach. Each is patched **and resealed**, so the
        // field check is what fires rather than the CRC - the trap that makes
        // a decode test pass for the wrong reason.
        //
        // Every one of these three guards survived deletion against the suite
        // as it stood: nothing patched `max_organisms`, the `flags` word or
        // the declared `header_len` at all, so all three were unpinned
        // fail-closed checks.
        let base = file_with(&[(10_u64, vec![record(1, 10, 0)])]);

        // `max_organisms` is the decode bound a segment count is checked
        // against, so a file may not declare one no kernel could produce.
        for declared in [u32::MAX, MAX_SAMPLE_ORGANISMS + 1] {
            let mut bytes = base.clone();
            bytes[52..56].copy_from_slice(&declared.to_le_bytes());
            reseal_header(&mut bytes);
            assert_eq!(
                decode_action(&bytes),
                Err(ActionLogError::SampleCountTooLarge(declared)),
                "a header declaring {declared} organisms was accepted"
            );
        }

        // The flags word is this format's only forward-compatibility escape
        // hatch. Accepting an unknown bit silently would mean a future
        // writer's flag is ignored rather than refused, which is the one
        // outcome a versioned format may not have.
        for declared in [1_u32, 0x8000_0000, u32::MAX] {
            let mut bytes = base.clone();
            bytes[8..12].copy_from_slice(&declared.to_le_bytes());
            reseal_header(&mut bytes);
            assert_eq!(
                decode_action(&bytes),
                Err(ActionLogError::UnknownFlags(declared)),
                "unknown flags {declared:#x} were ignored"
            );
        }

        // The declared header length is checked against the constant the
        // reader actually uses. It is never used as a bound itself, so this
        // guard's whole job is to fail closed on a header that disagrees with
        // its own format version.
        for declared in [0_u16, ACTION_HEADER_LEN as u16 - 1, u16::MAX] {
            let mut bytes = base.clone();
            bytes[6..8].copy_from_slice(&declared.to_le_bytes());
            reseal_header(&mut bytes);
            assert_eq!(
                decode_action(&bytes),
                Err(ActionLogError::BadHeaderLength(declared as usize)),
                "a header declaring length {declared} was accepted"
            );
        }

        // ...and the CRC still guards each of them unresealed, which is what
        // makes the assertions above evidence about the fields.
        let mut bytes = base.clone();
        bytes[52..56].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::HeaderChecksumMismatch)
        );
    }

    #[test]
    fn a_declared_build_length_is_capped_even_when_the_bytes_are_really_there() {
        // **The load-bearing case, and the one a short file cannot make.**
        // With `build_len` set past the cap on a *short* file, `TooShort`
        // fires whether or not the cap exists - two layers refusing the same
        // input, so that assertion alone would pass with the cap deleted.
        // Here the 100 declared bytes are present and the CRC is resealed, so
        // without the cap the file decodes cleanly. Deleting the cap was
        // measured: this file was accepted.
        let mut bytes = encode_header(&ActionLogInfo {
            build_version: "b".repeat(MAX_BUILD_LEN),
            ..info()
        })
        .expect("header");
        assert_eq!(bytes.len(), ACTION_HEADER_LEN + MAX_BUILD_LEN);
        bytes.extend(std::iter::repeat_n(b'b', 36));
        let declared = MAX_BUILD_LEN + 36;
        bytes[64..66].copy_from_slice(&(declared as u16).to_le_bytes());
        reseal_header(&mut bytes);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::BuildStringTooLong(declared)),
            "an over-long build string was accepted because its bytes were present"
        );

        // The exact cap still decodes, so the assertion above is about the
        // boundary and not about build strings in general.
        let mut at_cap = encode_header(&ActionLogInfo {
            build_version: "b".repeat(MAX_BUILD_LEN),
            ..info()
        })
        .expect("header");
        at_cap.extend_from_slice(&encode_segment(10, &[record(1, 10, 0)], 5_000).expect("segment"));
        let scan = decode_action(&at_cap).expect("a build string at the cap decodes");
        assert_eq!(scan.info.build_version.len(), MAX_BUILD_LEN);

        // ...and the short-file case, pinned to the cap rather than to
        // `TooShort`, so a future reordering of the two cannot go unnoticed.
        // Not resealed, and it does not need to be: the cap is checked
        // *before* the header CRC, which is the ordering that lets a 65,535
        // byte claim be refused without first reading 65,535 bytes to check
        // them.
        let mut bytes = file_with(&[(10_u64, vec![record(1, 10, 0)])]);
        bytes[64..66].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::BuildStringTooLong(u16::MAX as usize))
        );
    }

    #[test]
    fn the_declared_count_cap_is_what_keeps_the_body_length_multiply_exact() {
        // `decode_one` cross-checks `body_len` against `count * RECORD_LEN`
        // **in u64**. Rewriting that multiply in u32 survives every test in
        // this file, and it is unobservable rather than untested: the cap
        // above holds `count` at 10^6, and 10^6 x 44 is far inside u32, so no
        // admissible input can make a u32 multiply wrap.
        //
        // That safety is a property of the cap's *value*, not of the code, and
        // nothing said so. Raising `MAX_SAMPLE_ORGANISMS` past u32::MAX /
        // RECORD_LEN would make a u32 multiply wrap and let a declared count
        // near 10^8 pass the cross-check with a tiny `body_len`. This is the
        // assertion that fires if that day comes.
        assert!(
            MAX_SAMPLE_ORGANISMS as u64 * RECORD_LEN as u64 <= u32::MAX as u64,
            "MAX_SAMPLE_ORGANISMS x RECORD_LEN no longer fits in u32; the body \
             length cross-check must stay in u64 and this coupling must be restated"
        );
        // The largest count the cap admits, with the body length it implies,
        // still fails closed on the buffer bound rather than allocating.
        let base = file_with(&[(10_u64, vec![record(1, 10, 0)])]);
        let header_len = ACTION_HEADER_LEN + info().build_version.len();
        let body_end = header_len + SEGMENT_HEADER_LEN + RECORD_LEN;
        let mut bytes = base.clone();
        bytes[header_len + 12..header_len + 16]
            .copy_from_slice(&info().max_organisms.to_le_bytes());
        bytes[header_len + 16..header_len + 20]
            .copy_from_slice(&(info().max_organisms * RECORD_LEN as u32).to_le_bytes());
        reseal_segment(&mut bytes, header_len, body_end);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::TruncatedSegment {
                offset: header_len
            })
        );
    }

    #[test]
    fn the_writer_refuses_what_no_kernel_could_have_produced() {
        // The encode side's own bounds. Unreachable through `execute_unit` -
        // `max_organisms` is `max_entities`, which config validation caps at
        // 200,000, well under `MAX_SAMPLE_ORGANISMS` - but both functions are
        // public, and a guard reachable only through the public API is still
        // a guard. Both survived deletion against the suite as it stood.
        assert_eq!(
            encode_header(&ActionLogInfo {
                max_organisms: MAX_SAMPLE_ORGANISMS + 1,
                ..info()
            }),
            Err(ActionLogError::SampleCountTooLarge(
                MAX_SAMPLE_ORGANISMS + 1
            ))
        );
        assert_eq!(
            encode_header(&ActionLogInfo {
                class_count: ACTION_CLASS_COUNT as u32 + 1,
                ..info()
            }),
            Err(ActionLogError::ClassCountMismatch {
                declared: ACTION_CLASS_COUNT as u32 + 1,
                expected: ACTION_CLASS_COUNT as u32,
            })
        );
        assert_eq!(
            encode_header(&ActionLogInfo {
                build_version: "b".repeat(MAX_BUILD_LEN + 1),
                ..info()
            }),
            Err(ActionLogError::BuildStringTooLong(MAX_BUILD_LEN + 1))
        );
        // A segment with more records than the header's own bound admits.
        let records: Vec<ActionRecord> = (0..4_u64).map(|id| record(id, 10, 0)).collect();
        assert_eq!(
            encode_segment(10, &records, 3),
            Err(ActionLogError::SampleCountTooLarge(4))
        );
        assert!(encode_segment(10, &records, 4).is_ok(), "the bound is >=");
    }

    #[test]
    fn ticks_must_strictly_ascend_and_a_torn_tail_is_reported_not_repaired() {
        let bytes = file_with(&[
            (20_u64, vec![record(1, 20, 0)]),
            (10, vec![record(1, 10, 0)]),
        ]);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::TickOutOfOrder {
                previous: 20,
                found: 10
            })
        );

        // **Strictly, and the descending pair above does not test that.**
        // Relaxing `tick <= previous` to `tick < previous` survives the
        // assertion above, because 10 is still less than 20. Only a repeated
        // tick separates the two, and a repeat is the failure that matters
        // downstream: C11.1 keys its before/after windows by tick, so two
        // samples at the same tick would give one organism two different
        // "before" rows and the window would be a difference of a sample
        // with itself.
        let bytes = file_with(&[
            (10_u64, vec![record(1, 10, 0)]),
            (10, vec![record(1, 10, 5)]),
        ]);
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::TickOutOfOrder {
                previous: 10,
                found: 10
            }),
            "a repeated sample tick was accepted, so the order is not strict"
        );

        let full = file_with(&[
            (10_u64, vec![record(1, 10, 0)]),
            (20, vec![record(1, 20, 5)]),
        ]);
        let torn = &full[..full.len() - 3];
        assert!(matches!(
            decode_action(torn),
            Err(ActionLogError::TruncatedSegment { .. })
        ));
        let scan = decode_action_prefix(torn).expect("prefix decodes");
        assert_eq!(scan.samples.len(), 1);
        assert!(scan.truncated_at.is_some());
    }

    #[test]
    fn the_segment_crc_covers_the_segment_header_and_not_only_the_body() {
        // **Pinned to the exact error, and the first version of this test was
        // not.** It accepted `SegmentChecksumMismatch | TickOutOfOrder |
        // SegmentBodyLengthMismatch`, and a mutation that narrowed the CRC to
        // the body alone still produced one of those - so the alternation
        // passed while the property it names was gone. Three error kinds that
        // can all fire on the same input is exactly the "two layers can
        // refuse the same thing" trap; the near guard has to be named.
        //
        // A flipped bit in `tick` changes 10 to 11, which is a legal tick and
        // the first segment in the file, so nothing about ordering or length
        // can object. Only a CRC that covers the segment header rejects it,
        // and it does so reporting the *flipped* tick.
        let base = file_with(&[(10_u64, vec![record(1, 10, 0)])]);
        let header_len = ACTION_HEADER_LEN + info().build_version.len();

        let mut bytes = base.clone();
        bytes[header_len + 4] ^= 0x01;
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::SegmentChecksumMismatch { tick: 11 }),
            "a flipped tick was accepted, so the CRC does not cover the segment header"
        );

        // ...and a flipped body byte is caught with the tick unchanged, which
        // is what says the assertion above is about the header rather than
        // about the CRC existing at all.
        let mut bytes = base.clone();
        bytes[header_len + SEGMENT_HEADER_LEN + 2] ^= 0x01;
        assert_eq!(
            decode_action(&bytes),
            Err(ActionLogError::SegmentChecksumMismatch { tick: 10 })
        );
    }
}
