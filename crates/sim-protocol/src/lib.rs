//! Versioned binary observer protocol (`specifications/websocket-protocol.md`).
//!
//! Pure codec: no I/O or transport. All integers are network byte order
//! (big-endian). Every decode validates magic, version, header length,
//! frame type, payload length, counts, and optional checksum BEFORE any
//! allocation sized by untrusted input. Malformed frames produce typed
//! errors and are never partially applied.
//!
//! One WebSocket binary message carries exactly one frame.

use std::fmt;

/// Protocol semantic version. Breaking layout changes bump MAJOR and
/// require a migration plan; additive fields negotiate via capabilities.
pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 0;

/// "ALSP" in ASCII.
pub const MAGIC: u32 = 0x414C_5350;

/// Flags bit 0: a CRC32 of the payload follows the fixed header.
pub const FLAG_PAYLOAD_CRC32: u8 = 0x01;

const HEADER_LEN_BASE: usize = 32;
const HEADER_LEN_WITH_CRC: usize = 36;

/// Hard bounds enforced before allocation.
pub const MAX_PAYLOAD_LEN: usize = 4 * 1024 * 1024;
pub const MAX_TOKEN_LEN: usize = 128;
pub const MAX_ERROR_MESSAGE_LEN: usize = 256;
pub const MAX_ENTITIES_PER_FRAME: usize = 65_536;
pub const MAX_REMOVALS_PER_FRAME: usize = 65_536;
pub const MAX_TILE_CELLS: usize = 512 * 512;

/// Layer bitmask values for subscriptions.
pub const LAYER_TERRAIN: u32 = 1;
pub const LAYER_ORGANISMS: u32 = 2;
pub const LAYER_METRICS: u32 = 4;

/// Entity record flags.
pub const ENTITY_FLAG_MATURE: u8 = 0x01;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameType {
    Hello = 1,
    Welcome = 2,
    Subscribe = 3,
    Subscribed = 4,
    Unsubscribe = 5,
    Keyframe = 6,
    Delta = 7,
    MetricsSample = 8,
    Ack = 9,
    Error = 10,
}

impl FrameType {
    fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Hello,
            2 => Self::Welcome,
            3 => Self::Subscribe,
            4 => Self::Subscribed,
            5 => Self::Unsubscribe,
            6 => Self::Keyframe,
            7 => Self::Delta,
            8 => Self::MetricsSample,
            9 => Self::Ack,
            10 => Self::Error,
            _ => return None,
        })
    }
}

/// Viewport in world fixed-point units (1/1024 m), plus level of detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    pub x0_fp: i32,
    pub y0_fp: i32,
    pub x1_fp: i32,
    pub y1_fp: i32,
    pub lod: u8,
}

impl Viewport {
    pub fn is_ordered(&self) -> bool {
        self.x0_fp <= self.x1_fp && self.y0_fp <= self.y1_fp
    }
}

/// Compact per-entity render record. Deep details use HTTP lookup; genome
/// and controller matrices never travel in movement frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityRecord {
    pub id: u64,
    pub x_fp: i32,
    pub y_fp: i32,
    pub heading_bam: u16,
    pub flags: u8,
    pub pigment_hue: u8,
    pub pigment_pattern: u8,
    pub body_scale: u8,
    pub energy_frac: u8,
}

const ENTITY_RECORD_LEN: usize = 8 + 4 + 4 + 2 + 1 + 1 + 1 + 1 + 1;

/// One rectangular block of terrain cells in cell coordinates. `cells` is
/// row-major, exactly `width * height` entries: (flags, food) where flags
/// bit 0 is land and food is biomass/capacity quantized to 0..=255.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TileBlock {
    pub x0: u16,
    pub y0: u16,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<(u8, u8)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    Hello {
        major: u16,
        minor: u16,
        capabilities: u32,
        token: Vec<u8>,
    },
    Welcome {
        major: u16,
        minor: u16,
        capabilities: u32,
        world_id: u64,
        config_hash: u64,
        cells_x: u32,
        cells_y: u32,
        cell_size_m: u32,
        dt_ms: u32,
        phase2: bool,
        max_rate_hz: u8,
    },
    Subscribe {
        viewport: Viewport,
        layers: u32,
        max_rate_hz: u8,
    },
    Subscribed {
        viewport: Viewport,
        layers: u32,
        max_rate_hz: u8,
    },
    Unsubscribe,
    Keyframe {
        tick: u64,
        tiles: Option<TileBlock>,
        entities: Vec<EntityRecord>,
    },
    Delta {
        tick: u64,
        base_sequence: u64,
        removed: Vec<u64>,
        upserts: Vec<EntityRecord>,
        tiles: Option<TileBlock>,
    },
    MetricsSample {
        tick: u64,
        population: u32,
        births_total: u64,
        deaths_starvation_total: u64,
        deaths_old_age_total: u64,
        paired_births_total: u64,
        total_biomass_milli: i64,
        total_energy_milli: i64,
        max_ancestry_depth: u32,
    },
    Ack {
        applied_sequence: u64,
    },
    Error {
        code: u16,
        message: String,
    },
}

impl Frame {
    pub fn frame_type(&self) -> FrameType {
        match self {
            Frame::Hello { .. } => FrameType::Hello,
            Frame::Welcome { .. } => FrameType::Welcome,
            Frame::Subscribe { .. } => FrameType::Subscribe,
            Frame::Subscribed { .. } => FrameType::Subscribed,
            Frame::Unsubscribe => FrameType::Unsubscribe,
            Frame::Keyframe { .. } => FrameType::Keyframe,
            Frame::Delta { .. } => FrameType::Delta,
            Frame::MetricsSample { .. } => FrameType::MetricsSample,
            Frame::Ack { .. } => FrameType::Ack,
            Frame::Error { .. } => FrameType::Error,
        }
    }
}

/// Envelope metadata carried by every frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameMeta {
    pub world_epoch: u64,
    pub sequence: u64,
    pub checksummed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    TooShort { actual: usize },
    BadMagic(u32),
    UnsupportedVersion { major: u16, minor: u16 },
    BadHeaderLength { declared: usize },
    UnknownFrameType(u8),
    UnknownFlags(u8),
    PayloadTooLarge { declared: usize },
    LengthMismatch { expected: usize, actual: usize },
    ChecksumMismatch,
    TruncatedPayload,
    TrailingBytes { extra: usize },
    CountTooLarge { count: usize, limit: usize },
    TokenTooLong { length: usize },
    MessageTooLong { length: usize },
    InvalidViewport,
    InvalidUtf8,
    InvalidTileBlock,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProtocolError {}

// --- Encoding ---------------------------------------------------------------

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
}

fn write_viewport(writer: &mut Writer, viewport: &Viewport) {
    writer.i32(viewport.x0_fp);
    writer.i32(viewport.y0_fp);
    writer.i32(viewport.x1_fp);
    writer.i32(viewport.y1_fp);
    writer.u8(viewport.lod);
}

fn write_entity(writer: &mut Writer, entity: &EntityRecord) {
    writer.u64(entity.id);
    writer.i32(entity.x_fp);
    writer.i32(entity.y_fp);
    writer.u16(entity.heading_bam);
    writer.u8(entity.flags);
    writer.u8(entity.pigment_hue);
    writer.u8(entity.pigment_pattern);
    writer.u8(entity.body_scale);
    writer.u8(entity.energy_frac);
}

fn write_tiles(writer: &mut Writer, tiles: &Option<TileBlock>) {
    match tiles {
        None => writer.u8(0),
        Some(block) => {
            writer.u8(1);
            writer.u16(block.x0);
            writer.u16(block.y0);
            writer.u16(block.width);
            writer.u16(block.height);
            for &(flags, food) in &block.cells {
                writer.u8(flags);
                writer.u8(food);
            }
        }
    }
}

/// Encode one frame with its envelope. `with_checksum` appends a CRC32 of
/// the payload to the header and sets the flag bit.
pub fn encode(frame: &Frame, meta: FrameMeta) -> Vec<u8> {
    let mut payload = Writer::new();
    match frame {
        Frame::Hello {
            major,
            minor,
            capabilities,
            token,
        } => {
            payload.u16(*major);
            payload.u16(*minor);
            payload.u32(*capabilities);
            payload.u16(token.len() as u16);
            payload.bytes.extend_from_slice(token);
        }
        Frame::Welcome {
            major,
            minor,
            capabilities,
            world_id,
            config_hash,
            cells_x,
            cells_y,
            cell_size_m,
            dt_ms,
            phase2,
            max_rate_hz,
        } => {
            payload.u16(*major);
            payload.u16(*minor);
            payload.u32(*capabilities);
            payload.u64(*world_id);
            payload.u64(*config_hash);
            payload.u32(*cells_x);
            payload.u32(*cells_y);
            payload.u32(*cell_size_m);
            payload.u32(*dt_ms);
            payload.u8(u8::from(*phase2));
            payload.u8(*max_rate_hz);
        }
        Frame::Subscribe {
            viewport,
            layers,
            max_rate_hz,
        }
        | Frame::Subscribed {
            viewport,
            layers,
            max_rate_hz,
        } => {
            write_viewport(&mut payload, viewport);
            payload.u32(*layers);
            payload.u8(*max_rate_hz);
        }
        Frame::Unsubscribe => {}
        Frame::Keyframe {
            tick,
            tiles,
            entities,
        } => {
            payload.u64(*tick);
            write_tiles(&mut payload, tiles);
            payload.u32(entities.len() as u32);
            for entity in entities {
                write_entity(&mut payload, entity);
            }
        }
        Frame::Delta {
            tick,
            base_sequence,
            removed,
            upserts,
            tiles,
        } => {
            payload.u64(*tick);
            payload.u64(*base_sequence);
            payload.u32(removed.len() as u32);
            for id in removed {
                payload.u64(*id);
            }
            payload.u32(upserts.len() as u32);
            for entity in upserts {
                write_entity(&mut payload, entity);
            }
            write_tiles(&mut payload, tiles);
        }
        Frame::MetricsSample {
            tick,
            population,
            births_total,
            deaths_starvation_total,
            deaths_old_age_total,
            paired_births_total,
            total_biomass_milli,
            total_energy_milli,
            max_ancestry_depth,
        } => {
            payload.u64(*tick);
            payload.u32(*population);
            payload.u64(*births_total);
            payload.u64(*deaths_starvation_total);
            payload.u64(*deaths_old_age_total);
            payload.u64(*paired_births_total);
            payload.i64(*total_biomass_milli);
            payload.i64(*total_energy_milli);
            payload.u32(*max_ancestry_depth);
        }
        Frame::Ack { applied_sequence } => {
            payload.u64(*applied_sequence);
        }
        Frame::Error { code, message } => {
            payload.u16(*code);
            let bytes = message.as_bytes();
            payload.u16(bytes.len() as u16);
            payload.bytes.extend_from_slice(bytes);
        }
    }

    let payload = payload.bytes;
    let header_len = if meta.checksummed {
        HEADER_LEN_WITH_CRC
    } else {
        HEADER_LEN_BASE
    };
    let mut out = Writer::new();
    out.u32(MAGIC);
    out.u16(PROTOCOL_MAJOR);
    out.u16(PROTOCOL_MINOR);
    out.u8(frame.frame_type() as u8);
    out.u8(if meta.checksummed {
        FLAG_PAYLOAD_CRC32
    } else {
        0
    });
    out.u16(header_len as u16);
    out.u64(meta.world_epoch);
    out.u64(meta.sequence);
    out.u32(payload.len() as u32);
    if meta.checksummed {
        out.u32(crc32(&payload));
    }
    out.bytes.extend_from_slice(&payload);
    out.bytes
}

// --- Decoding ---------------------------------------------------------------

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], ProtocolError> {
        let slice = self
            .bytes
            .get(self.offset..self.offset + count)
            .ok_or(ProtocolError::TruncatedPayload)?;
        self.offset += count;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, ProtocolError> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, ProtocolError> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn finish(&self) -> Result<(), ProtocolError> {
        if self.offset != self.bytes.len() {
            return Err(ProtocolError::TrailingBytes {
                extra: self.bytes.len() - self.offset,
            });
        }
        Ok(())
    }
}

fn read_viewport(reader: &mut Reader) -> Result<Viewport, ProtocolError> {
    let viewport = Viewport {
        x0_fp: reader.i32()?,
        y0_fp: reader.i32()?,
        x1_fp: reader.i32()?,
        y1_fp: reader.i32()?,
        lod: reader.u8()?,
    };
    if !viewport.is_ordered() {
        return Err(ProtocolError::InvalidViewport);
    }
    Ok(viewport)
}

fn read_entity(reader: &mut Reader) -> Result<EntityRecord, ProtocolError> {
    Ok(EntityRecord {
        id: reader.u64()?,
        x_fp: reader.i32()?,
        y_fp: reader.i32()?,
        heading_bam: reader.u16()?,
        flags: reader.u8()?,
        pigment_hue: reader.u8()?,
        pigment_pattern: reader.u8()?,
        body_scale: reader.u8()?,
        energy_frac: reader.u8()?,
    })
}

fn read_entities(reader: &mut Reader) -> Result<Vec<EntityRecord>, ProtocolError> {
    let count = reader.u32()? as usize;
    if count > MAX_ENTITIES_PER_FRAME {
        return Err(ProtocolError::CountTooLarge {
            count,
            limit: MAX_ENTITIES_PER_FRAME,
        });
    }
    // Verify the remaining bytes can hold the declared count before
    // allocating.
    if reader.bytes.len() - reader.offset < count * ENTITY_RECORD_LEN {
        return Err(ProtocolError::TruncatedPayload);
    }
    let mut entities = Vec::with_capacity(count);
    for _ in 0..count {
        entities.push(read_entity(reader)?);
    }
    Ok(entities)
}

fn read_tiles(reader: &mut Reader) -> Result<Option<TileBlock>, ProtocolError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => {
            let x0 = reader.u16()?;
            let y0 = reader.u16()?;
            let width = reader.u16()?;
            let height = reader.u16()?;
            let cell_count = usize::from(width) * usize::from(height);
            if cell_count == 0 || cell_count > MAX_TILE_CELLS {
                return Err(ProtocolError::InvalidTileBlock);
            }
            if reader.bytes.len() - reader.offset < cell_count * 2 {
                return Err(ProtocolError::TruncatedPayload);
            }
            let mut cells = Vec::with_capacity(cell_count);
            for _ in 0..cell_count {
                let flags = reader.u8()?;
                let food = reader.u8()?;
                cells.push((flags, food));
            }
            Ok(Some(TileBlock {
                x0,
                y0,
                width,
                height,
                cells,
            }))
        }
        _ => Err(ProtocolError::InvalidTileBlock),
    }
}

/// Decode exactly one frame from one message buffer.
pub fn decode(bytes: &[u8]) -> Result<(FrameMeta, Frame), ProtocolError> {
    if bytes.len() < HEADER_LEN_BASE {
        return Err(ProtocolError::TooShort {
            actual: bytes.len(),
        });
    }
    let mut header = Reader::new(&bytes[..HEADER_LEN_BASE]);
    let magic = header.u32()?;
    if magic != MAGIC {
        return Err(ProtocolError::BadMagic(magic));
    }
    let major = header.u16()?;
    let minor = header.u16()?;
    if major != PROTOCOL_MAJOR {
        return Err(ProtocolError::UnsupportedVersion { major, minor });
    }
    let frame_type_raw = header.u8()?;
    let frame_type = FrameType::from_u8(frame_type_raw)
        .ok_or(ProtocolError::UnknownFrameType(frame_type_raw))?;
    let flags = header.u8()?;
    if flags & !FLAG_PAYLOAD_CRC32 != 0 {
        return Err(ProtocolError::UnknownFlags(flags));
    }
    let checksummed = flags & FLAG_PAYLOAD_CRC32 != 0;
    let declared_header_len = usize::from(header.u16()?);
    let expected_header_len = if checksummed {
        HEADER_LEN_WITH_CRC
    } else {
        HEADER_LEN_BASE
    };
    if declared_header_len != expected_header_len {
        return Err(ProtocolError::BadHeaderLength {
            declared: declared_header_len,
        });
    }
    let world_epoch = header.u64()?;
    let sequence = header.u64()?;
    let payload_len = header.u32()? as usize;
    if payload_len > MAX_PAYLOAD_LEN {
        return Err(ProtocolError::PayloadTooLarge {
            declared: payload_len,
        });
    }
    let expected_total = expected_header_len + payload_len;
    if bytes.len() != expected_total {
        return Err(ProtocolError::LengthMismatch {
            expected: expected_total,
            actual: bytes.len(),
        });
    }
    let payload = &bytes[expected_header_len..];
    if checksummed {
        let declared_crc = u32::from_be_bytes(
            bytes[HEADER_LEN_BASE..HEADER_LEN_WITH_CRC]
                .try_into()
                .unwrap(),
        );
        if declared_crc != crc32(payload) {
            return Err(ProtocolError::ChecksumMismatch);
        }
    }

    let mut reader = Reader::new(payload);
    let frame = match frame_type {
        FrameType::Hello => {
            let major = reader.u16()?;
            let minor = reader.u16()?;
            let capabilities = reader.u32()?;
            let token_len = usize::from(reader.u16()?);
            if token_len > MAX_TOKEN_LEN {
                return Err(ProtocolError::TokenTooLong { length: token_len });
            }
            let token = reader.take(token_len)?.to_vec();
            Frame::Hello {
                major,
                minor,
                capabilities,
                token,
            }
        }
        FrameType::Welcome => Frame::Welcome {
            major: reader.u16()?,
            minor: reader.u16()?,
            capabilities: reader.u32()?,
            world_id: reader.u64()?,
            config_hash: reader.u64()?,
            cells_x: reader.u32()?,
            cells_y: reader.u32()?,
            cell_size_m: reader.u32()?,
            dt_ms: reader.u32()?,
            phase2: reader.u8()? != 0,
            max_rate_hz: reader.u8()?,
        },
        FrameType::Subscribe => Frame::Subscribe {
            viewport: read_viewport(&mut reader)?,
            layers: reader.u32()?,
            max_rate_hz: reader.u8()?,
        },
        FrameType::Subscribed => Frame::Subscribed {
            viewport: read_viewport(&mut reader)?,
            layers: reader.u32()?,
            max_rate_hz: reader.u8()?,
        },
        FrameType::Unsubscribe => Frame::Unsubscribe,
        FrameType::Keyframe => {
            let tick = reader.u64()?;
            let tiles = read_tiles(&mut reader)?;
            let entities = read_entities(&mut reader)?;
            Frame::Keyframe {
                tick,
                tiles,
                entities,
            }
        }
        FrameType::Delta => {
            let tick = reader.u64()?;
            let base_sequence = reader.u64()?;
            let removed_count = reader.u32()? as usize;
            if removed_count > MAX_REMOVALS_PER_FRAME {
                return Err(ProtocolError::CountTooLarge {
                    count: removed_count,
                    limit: MAX_REMOVALS_PER_FRAME,
                });
            }
            if reader.bytes.len() - reader.offset < removed_count * 8 {
                return Err(ProtocolError::TruncatedPayload);
            }
            let mut removed = Vec::with_capacity(removed_count);
            for _ in 0..removed_count {
                removed.push(reader.u64()?);
            }
            let upserts = read_entities(&mut reader)?;
            let tiles = read_tiles(&mut reader)?;
            Frame::Delta {
                tick,
                base_sequence,
                removed,
                upserts,
                tiles,
            }
        }
        FrameType::MetricsSample => Frame::MetricsSample {
            tick: reader.u64()?,
            population: reader.u32()?,
            births_total: reader.u64()?,
            deaths_starvation_total: reader.u64()?,
            deaths_old_age_total: reader.u64()?,
            paired_births_total: reader.u64()?,
            total_biomass_milli: reader.i64()?,
            total_energy_milli: reader.i64()?,
            max_ancestry_depth: reader.u32()?,
        },
        FrameType::Ack => Frame::Ack {
            applied_sequence: reader.u64()?,
        },
        FrameType::Error => {
            let code = reader.u16()?;
            let message_len = usize::from(reader.u16()?);
            if message_len > MAX_ERROR_MESSAGE_LEN {
                return Err(ProtocolError::MessageTooLong {
                    length: message_len,
                });
            }
            let message = std::str::from_utf8(reader.take(message_len)?)
                .map_err(|_| ProtocolError::InvalidUtf8)?
                .to_owned();
            Frame::Error { code, message }
        }
    };
    reader.finish()?;
    Ok((
        FrameMeta {
            world_epoch,
            sequence,
            checksummed,
        },
        frame,
    ))
}

/// CRC32 (IEEE 802.3, reflected), same polynomial as the Phase 0 spike.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(sequence: u64) -> FrameMeta {
        FrameMeta {
            world_epoch: 1,
            sequence,
            checksummed: false,
        }
    }

    fn sample_entity(id: u64) -> EntityRecord {
        EntityRecord {
            id,
            x_fp: 123_456,
            y_fp: -1,
            heading_bam: 40_000,
            flags: ENTITY_FLAG_MATURE,
            pigment_hue: 200,
            pigment_pattern: 17,
            body_scale: 128,
            energy_frac: 255,
        }
    }

    fn all_frames() -> Vec<Frame> {
        vec![
            Frame::Hello {
                major: 1,
                minor: 0,
                capabilities: 0b101,
                token: b"secret-token".to_vec(),
            },
            Frame::Welcome {
                major: 1,
                minor: 0,
                capabilities: 0b111,
                world_id: 1,
                config_hash: 0xf83d_3981_bf7d_d189,
                cells_x: 256,
                cells_y: 256,
                cell_size_m: 4,
                dt_ms: 100,
                phase2: true,
                max_rate_hz: 30,
            },
            Frame::Subscribe {
                viewport: Viewport {
                    x0_fp: 0,
                    y0_fp: 0,
                    x1_fp: 1_048_575,
                    y1_fp: 524_287,
                    lod: 1,
                },
                layers: LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS,
                max_rate_hz: 20,
            },
            Frame::Subscribed {
                viewport: Viewport {
                    x0_fp: 0,
                    y0_fp: 0,
                    x1_fp: 1_048_575,
                    y1_fp: 524_287,
                    lod: 1,
                },
                layers: LAYER_TERRAIN | LAYER_ORGANISMS,
                max_rate_hz: 10,
            },
            Frame::Unsubscribe,
            Frame::Keyframe {
                tick: 42,
                tiles: Some(TileBlock {
                    x0: 3,
                    y0: 4,
                    width: 2,
                    height: 2,
                    cells: vec![(1, 200), (0, 0), (1, 17), (1, 255)],
                }),
                entities: vec![sample_entity(9), sample_entity(11)],
            },
            Frame::Delta {
                tick: 43,
                base_sequence: 7,
                removed: vec![5, 6],
                upserts: vec![sample_entity(12)],
                tiles: None,
            },
            Frame::MetricsSample {
                tick: 44,
                population: 4_706,
                births_total: 204_257,
                deaths_starvation_total: 199_871,
                deaths_old_age_total: 180,
                paired_births_total: 204_257,
                total_biomass_milli: 388_539_095,
                total_energy_milli: 33_185_204,
                max_ancestry_depth: 127,
            },
            Frame::Ack {
                applied_sequence: 99,
            },
            Frame::Error {
                code: 401,
                message: "unauthorized".to_owned(),
            },
        ]
    }

    #[test]
    fn every_frame_round_trips_with_and_without_checksum() {
        for (index, frame) in all_frames().into_iter().enumerate() {
            for checksummed in [false, true] {
                let sent = FrameMeta {
                    world_epoch: 5,
                    sequence: index as u64,
                    checksummed,
                };
                let bytes = encode(&frame, sent);
                let (received, decoded) = decode(&bytes).unwrap();
                assert_eq!(received, sent);
                assert_eq!(decoded, frame, "frame {index} mismatched");
            }
        }
    }

    #[test]
    fn golden_ack_frame_bytes_are_stable_and_big_endian() {
        let bytes = encode(
            &Frame::Ack {
                applied_sequence: 0x0102_0304_0506_0708,
            },
            meta(0x1122_3344_5566_7788),
        );
        // Envelope: magic, 1.0, type 9, flags 0, header 32, epoch 1,
        // sequence, payload length 8, then the big-endian payload.
        let expected: Vec<u8> = vec![
            0x41, 0x4C, 0x53, 0x50, // "ALSP"
            0x00, 0x01, 0x00, 0x00, // version 1.0
            0x09, 0x00, // type Ack, flags 0
            0x00, 0x20, // header length 32
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // epoch 1
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // sequence
            0x00, 0x00, 0x00, 0x08, // payload length
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // payload
        ];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn malformed_envelopes_fail_closed() {
        let valid = encode(&Frame::Unsubscribe, meta(1));

        assert!(matches!(
            decode(&valid[..10]),
            Err(ProtocolError::TooShort { .. })
        ));

        let mut bad_magic = valid.clone();
        bad_magic[0] = 0;
        assert!(matches!(
            decode(&bad_magic),
            Err(ProtocolError::BadMagic(_))
        ));

        let mut bad_version = valid.clone();
        bad_version[4] = 0x7f;
        assert!(matches!(
            decode(&bad_version),
            Err(ProtocolError::UnsupportedVersion { .. })
        ));

        let mut bad_type = valid.clone();
        bad_type[8] = 99;
        assert_eq!(decode(&bad_type), Err(ProtocolError::UnknownFrameType(99)));

        let mut bad_flags = valid.clone();
        bad_flags[9] = 0xfe;
        assert!(matches!(
            decode(&bad_flags),
            Err(ProtocolError::UnknownFlags(_))
        ));

        let mut bad_header = valid.clone();
        bad_header[11] = 0x21;
        assert!(matches!(
            decode(&bad_header),
            Err(ProtocolError::BadHeaderLength { .. })
        ));

        // Declared payload larger than the buffer.
        let mut oversold = valid.clone();
        oversold[28..32].copy_from_slice(&100_u32.to_be_bytes());
        assert!(matches!(
            decode(&oversold),
            Err(ProtocolError::LengthMismatch { .. })
        ));

        // Declared payload over the absolute cap.
        let mut huge = valid.clone();
        huge[28..32].copy_from_slice(&(MAX_PAYLOAD_LEN as u32 + 1).to_be_bytes());
        assert!(matches!(
            decode(&huge),
            Err(ProtocolError::PayloadTooLarge { .. })
        ));

        // Trailing garbage.
        let mut trailing = valid.clone();
        trailing.push(0);
        assert!(matches!(
            decode(&trailing),
            Err(ProtocolError::LengthMismatch { .. })
        ));
    }

    #[test]
    fn corrupted_checksum_is_rejected() {
        let bytes = encode(
            &Frame::Ack {
                applied_sequence: 3,
            },
            FrameMeta {
                world_epoch: 1,
                sequence: 1,
                checksummed: true,
            },
        );
        let mut corrupted = bytes.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x01;
        assert_eq!(decode(&corrupted), Err(ProtocolError::ChecksumMismatch));
        assert!(decode(&bytes).is_ok());
    }

    #[test]
    fn hostile_counts_are_rejected_before_allocation() {
        // Keyframe claiming u32::MAX entities with a tiny buffer.
        let mut writer = Writer::new();
        writer.u64(1); // tick
        writer.u8(0); // no tiles
        writer.u32(u32::MAX); // entity count
        let payload = writer.bytes;
        let mut framed = Writer::new();
        framed.u32(MAGIC);
        framed.u16(PROTOCOL_MAJOR);
        framed.u16(PROTOCOL_MINOR);
        framed.u8(FrameType::Keyframe as u8);
        framed.u8(0);
        framed.u16(HEADER_LEN_BASE as u16);
        framed.u64(1);
        framed.u64(1);
        framed.u32(payload.len() as u32);
        framed.bytes.extend_from_slice(&payload);
        assert!(matches!(
            decode(&framed.bytes),
            Err(ProtocolError::CountTooLarge { .. })
        ));

        // Tile block claiming an enormous area.
        let mut writer = Writer::new();
        writer.u64(1);
        writer.u8(1);
        writer.u16(0);
        writer.u16(0);
        writer.u16(u16::MAX);
        writer.u16(u16::MAX);
        let payload = writer.bytes;
        let mut framed = Writer::new();
        framed.u32(MAGIC);
        framed.u16(PROTOCOL_MAJOR);
        framed.u16(PROTOCOL_MINOR);
        framed.u8(FrameType::Keyframe as u8);
        framed.u8(0);
        framed.u16(HEADER_LEN_BASE as u16);
        framed.u64(1);
        framed.u64(1);
        framed.u32(payload.len() as u32);
        framed.bytes.extend_from_slice(&payload);
        assert!(matches!(
            decode(&framed.bytes),
            Err(ProtocolError::InvalidTileBlock)
        ));
    }

    #[test]
    fn unordered_viewport_is_rejected() {
        let frame = Frame::Subscribe {
            viewport: Viewport {
                x0_fp: 10,
                y0_fp: 0,
                x1_fp: 0,
                y1_fp: 5,
                lod: 0,
            },
            layers: LAYER_ORGANISMS,
            max_rate_hz: 10,
        };
        let bytes = encode(&frame, meta(1));
        assert_eq!(decode(&bytes), Err(ProtocolError::InvalidViewport));
    }

    #[test]
    fn seeded_corruption_sweep_never_panics() {
        // Deterministic malformed-input sweep across all frame kinds.
        let mut state = 0x5eed_cafe_f00d_beef_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let frames = all_frames();
        let mut rejected = 0_u64;
        for case in 0..20_000_u64 {
            let base = encode(&frames[(case % frames.len() as u64) as usize], meta(case));
            let mut bytes = base.clone();
            match next() % 4 {
                0 => {
                    let cut = (next() % (bytes.len() as u64 + 1)) as usize;
                    bytes.truncate(cut);
                }
                1 => {
                    let flips = 1 + (next() % 6) as usize;
                    for _ in 0..flips {
                        let position = (next() % bytes.len() as u64) as usize;
                        bytes[position] ^= 1 << (next() % 8);
                    }
                }
                2 => {
                    let extra = 1 + (next() % 32) as usize;
                    for _ in 0..extra {
                        bytes.push((next() & 0xff) as u8);
                    }
                }
                _ => {
                    let length = (next() % 96) as usize;
                    bytes = (0..length).map(|_| (next() & 0xff) as u8).collect();
                }
            }
            if decode(&bytes).is_err() {
                rejected += 1;
            }
        }
        assert!(rejected > 15_000, "rejected only {rejected}");
    }
}
