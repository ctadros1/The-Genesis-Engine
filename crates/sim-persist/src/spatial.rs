//! ALSS spatial sample log, format version 1 (Phase 7).
//!
//! Phase 7's primary endpoint C7.1 needs a world-level index of spatial
//! aggregation and encounter avoidance. Positions are not in the event log
//! and never should be: ALEV records *what happened*, and a per-tick dump of
//! every coordinate would multiply its size by three orders of magnitude
//! while making the "one event, one thing that happened" reading false.
//!
//! So spatial structure gets its own artifact, written by the experiment
//! harness at a fixed tick cadence. The kernel is untouched — the harness
//! reads positions through the existing read-only observer view — which is
//! what keeps both fixtures unmovable by this work.
//!
//! Layout (little-endian, matching ALIF and ALEV):
//!
//! header (fixed 72 bytes):
//!   magic "ALSS" | format u16 | header_len u16 | flags u32
//!   world_id u64 | seed u64 | config_hash u64 | terrain_checksum u64
//!   cells_x u32 | cells_y u32 | cell_size_m u32 | sample_interval u32
//!   max_organisms u32 | build_len u16 | reserved u16 | header_crc32 u32
//! then: build version string (build_len <= 64 bytes)
//! then: zero or more segments in strictly ascending tick order:
//!   magic "SPL1" | tick u64 | count u32 | body_len u32 | body | crc32 u32
//!
//! The body is `count` records of `x_fp i32 | y_fp i32`.
//!
//! Two header fields exist purely so the analysis cannot silently measure
//! the wrong world. `terrain_checksum` lets the analysis regenerate terrain
//! from the manifest's embedded campaign and prove it got the same map
//! before it uses a land mask; `config_hash` catches a sample file paired
//! with the wrong condition. Both are provenance rather than framing, so —
//! exactly as in ALEV — the header checks itself, because nothing further
//! down the file would notice a corrupted one.
//!
//! Appending is the only supported mutation. There is no rewrite path and
//! no repair path.

use crate::codec::crc32;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

pub const SPATIAL_LOG_MAGIC: &[u8; 4] = b"ALSS";
pub const SPATIAL_LOG_FORMAT_VERSION: u16 = 1;
const SEGMENT_MAGIC: &[u8; 4] = b"SPL1";
const SPATIAL_HEADER_LEN: usize = 72;
/// Byte offset of the header CRC inside the fixed header.
const HEADER_CRC_OFFSET: usize = 68;
const SEGMENT_HEADER_LEN: usize = 20;
const MAX_BUILD_LEN: usize = 64;
const RECORD_LEN: usize = 8;

/// Absolute cap on a declared organism count, independent of the header's
/// own `max_organisms`. A file claiming more than this cannot have come
/// from any kernel this project supports, so it is refused before the
/// header's value is trusted enough to use as a bound.
pub const MAX_SAMPLE_ORGANISMS: u32 = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpatialLogError {
    TooShort,
    BadMagic,
    UnsupportedFormat(u16),
    BadHeaderLength(usize),
    UnknownFlags(u32),
    BuildStringTooLong(usize),
    BadBuildString,
    /// The header's own CRC does not match, so its provenance fields cannot
    /// be trusted even though the segments after it might decode.
    HeaderChecksumMismatch,
    /// Declared world geometry that no valid config could produce.
    BadGeometry {
        cells_x: u32,
        cells_y: u32,
        cell_size_m: u32,
    },
    /// A segment declared more organisms than the header's own cap, or than
    /// the absolute cap allows.
    SampleCountTooLarge(u32),
    /// `body_len` disagreed with `count * 8`.
    SegmentBodyLengthMismatch {
        tick: u64,
    },
    SegmentChecksumMismatch {
        tick: u64,
    },
    /// The trailing bytes are shorter than the segment they declare. A
    /// crash between `write` and `sync` produces exactly this.
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

impl fmt::Display for SpatialLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Provenance a spatial sample file carries about the world that wrote it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialLogInfo {
    pub format_version: u16,
    pub world_id: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub terrain_checksum: u64,
    pub cells_x: u32,
    pub cells_y: u32,
    pub cell_size_m: u32,
    /// Ticks between samples, as configured. Recorded so an analysis can
    /// state its own temporal resolution rather than infer it.
    pub sample_interval_ticks: u32,
    /// The world's `max_entities`; the decode bound for a segment count.
    pub max_organisms: u32,
    pub build_version: String,
}

/// One sampled configuration: every living organism's position at `tick`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialSample {
    pub tick: u64,
    /// `(x_fp, y_fp)` in stable entity-ID order, exactly as the kernel
    /// stores them.
    pub positions: Vec<(i32, i32)>,
}

/// Result of decoding a whole file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialLogScan {
    pub info: SpatialLogInfo,
    pub samples: Vec<SpatialSample>,
    /// Bytes of valid file consumed, including the header.
    pub bytes_consumed: usize,
    /// Set only by `decode_spatial_prefix`: the file has a torn tail that
    /// was reported rather than repaired.
    pub truncated_at: Option<usize>,
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

fn read_i32(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    let mut buffer = [0_u8; 8];
    buffer.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(buffer)
}

pub fn encode_header(info: &SpatialLogInfo) -> Result<Vec<u8>, SpatialLogError> {
    if info.build_version.len() > MAX_BUILD_LEN {
        return Err(SpatialLogError::BuildStringTooLong(
            info.build_version.len(),
        ));
    }
    if info.cells_x == 0 || info.cells_y == 0 || info.cell_size_m == 0 {
        return Err(SpatialLogError::BadGeometry {
            cells_x: info.cells_x,
            cells_y: info.cells_y,
            cell_size_m: info.cell_size_m,
        });
    }
    if info.max_organisms > MAX_SAMPLE_ORGANISMS {
        return Err(SpatialLogError::SampleCountTooLarge(info.max_organisms));
    }
    let mut out = Vec::with_capacity(SPATIAL_HEADER_LEN + info.build_version.len());
    out.extend_from_slice(SPATIAL_LOG_MAGIC);
    put_u16(&mut out, SPATIAL_LOG_FORMAT_VERSION);
    put_u16(&mut out, SPATIAL_HEADER_LEN as u16);
    put_u32(&mut out, 0); // flags
    put_u64(&mut out, info.world_id);
    put_u64(&mut out, info.seed);
    put_u64(&mut out, info.config_hash);
    put_u64(&mut out, info.terrain_checksum);
    put_u32(&mut out, info.cells_x);
    put_u32(&mut out, info.cells_y);
    put_u32(&mut out, info.cell_size_m);
    put_u32(&mut out, info.sample_interval_ticks);
    put_u32(&mut out, info.max_organisms);
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
pub fn read_spatial_info(bytes: &[u8]) -> Result<(SpatialLogInfo, usize), SpatialLogError> {
    if bytes.len() < SPATIAL_HEADER_LEN {
        return Err(SpatialLogError::TooShort);
    }
    if &bytes[0..4] != SPATIAL_LOG_MAGIC {
        return Err(SpatialLogError::BadMagic);
    }
    let format_version = read_u16(bytes, 4);
    if format_version != SPATIAL_LOG_FORMAT_VERSION {
        return Err(SpatialLogError::UnsupportedFormat(format_version));
    }
    let header_len = read_u16(bytes, 6) as usize;
    if header_len != SPATIAL_HEADER_LEN {
        return Err(SpatialLogError::BadHeaderLength(header_len));
    }
    let flags = read_u32(bytes, 8);
    if flags != 0 {
        return Err(SpatialLogError::UnknownFlags(flags));
    }
    let build_len = read_u16(bytes, 64) as usize;
    if build_len > MAX_BUILD_LEN {
        return Err(SpatialLogError::BuildStringTooLong(build_len));
    }
    let total = SPATIAL_HEADER_LEN + build_len;
    if bytes.len() < total {
        return Err(SpatialLogError::TooShort);
    }
    let build_bytes = &bytes[SPATIAL_HEADER_LEN..total];
    let build_version =
        std::str::from_utf8(build_bytes).map_err(|_| SpatialLogError::BadBuildString)?;

    let declared = read_u32(bytes, HEADER_CRC_OFFSET);
    let mut checked = bytes[..HEADER_CRC_OFFSET].to_vec();
    checked.extend_from_slice(build_bytes);
    if crc32(&checked) != declared {
        return Err(SpatialLogError::HeaderChecksumMismatch);
    }

    let cells_x = read_u32(bytes, 44);
    let cells_y = read_u32(bytes, 48);
    let cell_size_m = read_u32(bytes, 52);
    if cells_x == 0 || cells_y == 0 || cell_size_m == 0 {
        return Err(SpatialLogError::BadGeometry {
            cells_x,
            cells_y,
            cell_size_m,
        });
    }
    let max_organisms = read_u32(bytes, 60);
    if max_organisms > MAX_SAMPLE_ORGANISMS {
        return Err(SpatialLogError::SampleCountTooLarge(max_organisms));
    }

    Ok((
        SpatialLogInfo {
            format_version,
            world_id: read_u64(bytes, 12),
            seed: read_u64(bytes, 20),
            config_hash: read_u64(bytes, 28),
            terrain_checksum: read_u64(bytes, 36),
            cells_x,
            cells_y,
            cell_size_m,
            sample_interval_ticks: read_u32(bytes, 56),
            max_organisms,
            build_version: build_version.to_owned(),
        },
        total,
    ))
}

pub fn encode_segment(
    tick: u64,
    positions: &[(i32, i32)],
    max_organisms: u32,
) -> Result<Vec<u8>, SpatialLogError> {
    let count = positions.len();
    if count > max_organisms as usize || count > MAX_SAMPLE_ORGANISMS as usize {
        return Err(SpatialLogError::SampleCountTooLarge(count as u32));
    }
    let body_len = count * RECORD_LEN;
    let mut out = Vec::with_capacity(SEGMENT_HEADER_LEN + body_len + 4);
    out.extend_from_slice(SEGMENT_MAGIC);
    put_u64(&mut out, tick);
    put_u32(&mut out, count as u32);
    put_u32(&mut out, body_len as u32);
    debug_assert_eq!(out.len(), SEGMENT_HEADER_LEN);
    for &(x_fp, y_fp) in positions {
        out.extend_from_slice(&x_fp.to_le_bytes());
        out.extend_from_slice(&y_fp.to_le_bytes());
    }
    // The CRC covers the segment header as well as the body, matching ALEV.
    // Covering the body alone would leave `tick` unchecked, and a flipped
    // bit there moves a sample in time instead of failing the decode --
    // the same "provenance is not framing" hole the file header closes,
    // one level down.
    let checksum = crc32(&out);
    put_u32(&mut out, checksum);
    Ok(out)
}

/// Strict decode. Any torn or corrupted byte anywhere is an error.
pub fn decode_spatial(bytes: &[u8]) -> Result<SpatialLogScan, SpatialLogError> {
    decode_inner(bytes, false)
}

/// Decode the longest intact prefix, reporting where it stops instead of
/// repairing it. A crash between `write` and `sync` leaves exactly one torn
/// trailing segment; a campaign that lost a world this way must say so
/// rather than analyse a silently shortened run.
pub fn decode_spatial_prefix(bytes: &[u8]) -> Result<SpatialLogScan, SpatialLogError> {
    decode_inner(bytes, true)
}

fn decode_inner(bytes: &[u8], tolerate_torn_tail: bool) -> Result<SpatialLogScan, SpatialLogError> {
    let (info, header_len) = read_spatial_info(bytes)?;
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
            Err(SpatialLogError::TruncatedSegment { offset: at }) if tolerate_torn_tail => {
                truncated_at = Some(at);
                break;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(SpatialLogScan {
        info,
        samples,
        bytes_consumed: offset,
        truncated_at,
    })
}

fn decode_one(
    bytes: &[u8],
    offset: usize,
    info: &SpatialLogInfo,
    previous_tick: Option<u64>,
) -> Result<(SpatialSample, usize), SpatialLogError> {
    if offset + SEGMENT_HEADER_LEN > bytes.len() {
        return Err(SpatialLogError::TruncatedSegment { offset });
    }
    if &bytes[offset..offset + 4] != SEGMENT_MAGIC {
        return Err(SpatialLogError::BadSegmentMagic { offset });
    }
    let tick = read_u64(bytes, offset + 4);
    let count = read_u32(bytes, offset + 12);
    let body_len = read_u32(bytes, offset + 16);

    // Every length is capped before it is used to size anything.
    if count > info.max_organisms || count > MAX_SAMPLE_ORGANISMS {
        return Err(SpatialLogError::SampleCountTooLarge(count));
    }
    if body_len as u64 != u64::from(count) * RECORD_LEN as u64 {
        return Err(SpatialLogError::SegmentBodyLengthMismatch { tick });
    }
    if let Some(previous) = previous_tick
        && tick <= previous
    {
        return Err(SpatialLogError::TickOutOfOrder {
            previous,
            found: tick,
        });
    }

    let body_start = offset + SEGMENT_HEADER_LEN;
    let body_end = body_start + body_len as usize;
    let segment_end = body_end + 4;
    if segment_end > bytes.len() {
        return Err(SpatialLogError::TruncatedSegment { offset });
    }
    // The CRC is verified before any payload byte is interpreted, and it
    // covers the segment header too, so `tick` and `count` are checked.
    let declared = read_u32(bytes, body_end);
    if crc32(&bytes[offset..body_end]) != declared {
        return Err(SpatialLogError::SegmentChecksumMismatch { tick });
    }

    let mut positions = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let at = body_start + index * RECORD_LEN;
        positions.push((read_i32(bytes, at), read_i32(bytes, at + 4)));
    }
    Ok((SpatialSample { tick, positions }, segment_end))
}

/// Streaming writer, same durability ordering as the event log.
pub struct SpatialLogWriter {
    file: BufWriter<File>,
    offset: u64,
    samples: u64,
    max_organisms: u32,
}

impl SpatialLogWriter {
    /// Create a new file, failing if one already exists. A run gets a new
    /// file, so a stale sample log can never be silently extended.
    pub fn create(path: &Path, info: &SpatialLogInfo) -> Result<Self, SpatialLogError> {
        let header = encode_header(info)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| SpatialLogError::Io(error.to_string()))?;
        let mut writer = BufWriter::new(file);
        writer
            .write_all(&header)
            .map_err(|error| SpatialLogError::Io(error.to_string()))?;
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

    /// Append one sample. An empty world still writes a segment: "nobody
    /// was alive at tick T" is a measurement, and a reader that cannot tell
    /// it from "no sample was taken" would silently shorten the series.
    pub fn append(&mut self, tick: u64, positions: &[(i32, i32)]) -> Result<(), SpatialLogError> {
        let segment = encode_segment(tick, positions, self.max_organisms)?;
        self.file
            .write_all(&segment)
            .map_err(|error| SpatialLogError::Io(error.to_string()))?;
        self.offset += segment.len() as u64;
        self.samples += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), SpatialLogError> {
        self.file
            .flush()
            .map_err(|error| SpatialLogError::Io(error.to_string()))
    }

    pub fn sync(&mut self) -> Result<(), SpatialLogError> {
        self.flush()?;
        self.file
            .get_ref()
            .sync_all()
            .map_err(|error| SpatialLogError::Io(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> SpatialLogInfo {
        SpatialLogInfo {
            format_version: SPATIAL_LOG_FORMAT_VERSION,
            world_id: 7,
            seed: 0x5eed_cafe_f00d_beef,
            config_hash: 0x0123_4567_89ab_cdef,
            terrain_checksum: 0xfeed_face_dead_beef,
            cells_x: 256,
            cells_y: 256,
            cell_size_m: 4,
            sample_interval_ticks: 50,
            max_organisms: 5_000,
            build_version: "test-build".to_owned(),
        }
    }

    fn file_with(samples: &[(u64, Vec<(i32, i32)>)]) -> Vec<u8> {
        let mut bytes = encode_header(&info()).expect("header");
        for (tick, positions) in samples {
            bytes.extend_from_slice(&encode_segment(*tick, positions, 5_000).expect("segment"));
        }
        bytes
    }

    #[test]
    fn a_round_trip_preserves_provenance_and_every_position() {
        let samples = vec![
            (0_u64, vec![(1_i32, 2_i32), (-3, 4)]),
            (50, vec![(5, 6)]),
            (100, Vec::new()),
        ];
        let bytes = file_with(&samples);
        let scan = decode_spatial(&bytes).expect("decodes");
        assert_eq!(scan.info, info());
        assert_eq!(scan.samples.len(), 3);
        assert_eq!(scan.samples[0].positions, vec![(1, 2), (-3, 4)]);
        assert_eq!(scan.samples[2].positions, Vec::new());
        assert_eq!(scan.bytes_consumed, bytes.len());
        assert_eq!(scan.truncated_at, None);
    }

    #[test]
    fn an_empty_sample_is_recorded_rather_than_skipped() {
        // An extinct world must produce a segment saying so. If empty
        // samples were skipped, an extinct run and an unsampled run would
        // decode identically, and every rate computed per sample would be
        // divided by the wrong denominator.
        let bytes = file_with(&[(0, Vec::new()), (50, Vec::new())]);
        let scan = decode_spatial(&bytes).expect("decodes");
        assert_eq!(scan.samples.len(), 2);
        assert!(scan.samples.iter().all(|s| s.positions.is_empty()));
    }

    #[test]
    fn every_single_byte_corruption_is_caught() {
        // The whole file is checked, not just the payload: a flipped bit in
        // the seed or the terrain checksum would mislabel an experiment
        // rather than fail it, which is worse than a decode error.
        let bytes = file_with(&[(0, vec![(11, 22), (33, 44)]), (50, vec![(55, 66)])]);
        let mut caught = 0;
        for index in 0..bytes.len() {
            for bit in [0x01_u8, 0x80] {
                let mut damaged = bytes.clone();
                damaged[index] ^= bit;
                if damaged == bytes {
                    continue;
                }
                match decode_spatial(&damaged) {
                    Err(_) => caught += 1,
                    Ok(scan) => {
                        panic!(
                            "corruption at byte {index} bit {bit:#x} decoded cleanly: {:?}",
                            scan.info
                        );
                    }
                }
            }
        }
        assert_eq!(caught, bytes.len() * 2);
    }

    #[test]
    fn a_torn_tail_is_reported_not_repaired() {
        let bytes = file_with(&[(0, vec![(1, 1)]), (50, vec![(2, 2), (3, 3)])]);
        let torn = &bytes[..bytes.len() - 3];
        // Strict decode refuses.
        match decode_spatial(torn) {
            Err(SpatialLogError::TruncatedSegment { .. }) => {}
            other => panic!("expected a truncation error, got {other:?}"),
        }
        // The prefix reader reports the tear rather than pretending the
        // file ended cleanly.
        let scan = decode_spatial_prefix(torn).expect("prefix decodes");
        assert_eq!(scan.samples.len(), 1);
        assert!(scan.truncated_at.is_some());
    }

    #[test]
    fn a_count_beyond_the_declared_cap_is_refused_before_allocation() {
        let mut bytes = file_with(&[(0, vec![(1, 1)])]);
        let segment_start = bytes.len() - (SEGMENT_HEADER_LEN + RECORD_LEN + 4);
        // Claim four billion organisms with the body length to match.
        bytes[segment_start + 12..segment_start + 16].copy_from_slice(&u32::MAX.to_le_bytes());
        bytes[segment_start + 16..segment_start + 20].copy_from_slice(&(u32::MAX).to_le_bytes());
        match decode_spatial(&bytes) {
            Err(SpatialLogError::SampleCountTooLarge(_)) => {}
            other => panic!("expected a count cap error, got {other:?}"),
        }
    }

    #[test]
    fn ticks_must_strictly_ascend() {
        let bytes = file_with(&[(50, vec![(1, 1)]), (50, vec![(2, 2)])]);
        match decode_spatial(&bytes) {
            Err(SpatialLogError::TickOutOfOrder { previous, found }) => {
                assert_eq!((previous, found), (50, 50));
            }
            other => panic!("expected an ordering error, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_format_or_flag_fails_closed() {
        let mut bytes = file_with(&[(0, vec![(1, 1)])]);
        let good = bytes.clone();
        bytes[4..6].copy_from_slice(&2_u16.to_le_bytes());
        assert!(matches!(
            decode_spatial(&bytes),
            Err(SpatialLogError::UnsupportedFormat(2))
                | Err(SpatialLogError::HeaderChecksumMismatch)
        ));
        // A flag set with a matching CRC still fails: the reader refuses
        // what it does not understand rather than ignoring it.
        let mut info = super::tests::info();
        info.build_version = "test-build".to_owned();
        let mut header = encode_header(&info).expect("header");
        header[8..12].copy_from_slice(&1_u32.to_le_bytes());
        let mut checked = header[..HEADER_CRC_OFFSET].to_vec();
        checked.extend_from_slice(info.build_version.as_bytes());
        let fixed = crc32(&checked);
        header[HEADER_CRC_OFFSET..HEADER_CRC_OFFSET + 4].copy_from_slice(&fixed.to_le_bytes());
        match read_spatial_info(&header) {
            Err(SpatialLogError::UnknownFlags(1)) => {}
            other => panic!("expected an unknown-flag error, got {other:?}"),
        }
        assert!(decode_spatial(&good).is_ok());
    }

    #[test]
    fn the_writer_and_the_encoder_agree_byte_for_byte() {
        let directory = std::env::temp_dir().join(format!(
            "alss-writer-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&directory).expect("temp dir");
        let path = directory.join("sample.alss");
        let _ = std::fs::remove_file(&path);
        let mut writer = SpatialLogWriter::create(&path, &info()).expect("writer");
        writer.append(0, &[(1, 2), (-3, 4)]).expect("append");
        writer.append(50, &[(5, 6)]).expect("append");
        writer.sync().expect("sync");
        let offset = writer.offset();
        drop(writer);

        let bytes = std::fs::read(&path).expect("read back");
        assert_eq!(bytes.len() as u64, offset);
        assert_eq!(
            bytes,
            file_with(&[(0, vec![(1, 2), (-3, 4)]), (50, vec![(5, 6)])])
        );
        std::fs::remove_file(&path).expect("cleanup");
        let _ = std::fs::remove_dir(&directory);
    }
}
