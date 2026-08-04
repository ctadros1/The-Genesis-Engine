//! Disposable Phase 0 determinism and snapshot spike.
//!
//! This crate deliberately uses fixed-point integer state and no dependencies so
//! the benchmark measures a small, inspectable kernel rather than a framework.

use std::fmt;
use std::time::{Duration, Instant};

const WORLD_EXTENT: i32 = 256 * 1024;
const BUCKET_AXIS: usize = 16;
const BUCKET_COUNT: usize = BUCKET_AXIS * BUCKET_AXIS;
const SYSTEM_INITIALIZE: u64 = 1;
const SYSTEM_MOVEMENT: u64 = 2;

const SNAPSHOT_MAGIC: &[u8; 4] = b"ALIF";
const SNAPSHOT_VERSION: u16 = 1;
const SNAPSHOT_HEADER_LEN: usize = 60;
const SNAPSHOT_ENTITY_SECTION: u16 = 1;
const SNAPSHOT_SECTION_HEADER_LEN: usize = 12;
const SNAPSHOT_ENTITY_BYTES: usize = 16;
const SNAPSHOT_MAX_ENTITIES: usize = 100_000;
const SNAPSHOT_MAX_PAYLOAD: usize = 64 * 1024 * 1024;
const SNAPSHOT_FLAG_PAUSED: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickConfig {
    pub organisms: usize,
    pub seed: u64,
    pub tick_millis: u32,
}

impl TickConfig {
    pub fn new(organisms: usize, seed: u64) -> Result<Self, WorldError> {
        if organisms == 0 || organisms > SNAPSHOT_MAX_ENTITIES {
            return Err(WorldError::InvalidOrganismCount(organisms));
        }
        Ok(Self {
            organisms,
            seed,
            tick_millis: 100,
        })
    }

    pub fn stable_hash(self) -> u64 {
        let mut bytes = Vec::with_capacity(28);
        bytes.extend_from_slice(b"phase0-tick-config-v1");
        bytes.extend_from_slice(&(self.organisms as u64).to_le_bytes());
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&self.tick_millis.to_le_bytes());
        fnv1a64(&bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntitySeed {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub energy: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TickTimings {
    pub clock: Duration,
    pub spatial_index: Duration,
    pub sense_and_controller: Duration,
    pub apply: Duration,
    pub checksum: Duration,
    pub total: Duration,
}

#[derive(Clone, Debug)]
pub struct World {
    seed: u64,
    tick: u64,
    config_hash: u64,
    paused: bool,
    ids: Vec<u32>,
    x: Vec<i32>,
    y: Vec<i32>,
    energy: Vec<i32>,
    intent_x: Vec<i8>,
    intent_y: Vec<i8>,
    buckets: Vec<Vec<usize>>,
}

impl World {
    pub fn synthetic(config: TickConfig) -> Self {
        let entities = (0..config.organisms)
            .map(|index| {
                let id = u32::try_from(index + 1).expect("bounded organism count");
                let x = (named_random(config.seed, 0, SYSTEM_INITIALIZE, id, 0)
                    % WORLD_EXTENT as u64) as i32;
                let y = (named_random(config.seed, 0, SYSTEM_INITIALIZE, id, 1)
                    % WORLD_EXTENT as u64) as i32;
                let energy =
                    8_000 + (named_random(config.seed, 0, SYSTEM_INITIALIZE, id, 2) % 2_001) as i32;
                EntitySeed { id, x, y, energy }
            })
            .collect();

        Self::from_entities(config.seed, config.stable_hash(), entities)
            .expect("synthetic entities are valid")
    }

    pub fn from_entities(
        seed: u64,
        config_hash: u64,
        mut entities: Vec<EntitySeed>,
    ) -> Result<Self, WorldError> {
        if entities.is_empty() || entities.len() > SNAPSHOT_MAX_ENTITIES {
            return Err(WorldError::InvalidOrganismCount(entities.len()));
        }
        entities.sort_unstable_by_key(|entity| entity.id);
        if entities.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(WorldError::DuplicateEntityId);
        }
        if entities.iter().any(|entity| {
            !(0..WORLD_EXTENT).contains(&entity.x) || !(0..WORLD_EXTENT).contains(&entity.y)
        }) {
            return Err(WorldError::PositionOutOfBounds);
        }

        let len = entities.len();
        Ok(Self {
            seed,
            tick: 0,
            config_hash,
            paused: false,
            ids: entities.iter().map(|entity| entity.id).collect(),
            x: entities.iter().map(|entity| entity.x).collect(),
            y: entities.iter().map(|entity| entity.y).collect(),
            energy: entities.iter().map(|entity| entity.energy).collect(),
            intent_x: vec![0; len],
            intent_y: vec![0; len],
            buckets: (0..BUCKET_COUNT).map(|_| Vec::new()).collect(),
        })
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn tick_number(&self) -> u64 {
        self.tick
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn config_hash(&self) -> u64 {
        self.config_hash
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn step(&mut self) -> u64 {
        if self.paused {
            return self.state_checksum();
        }
        let next_tick = self.tick.saturating_add(1);
        self.build_spatial_index();
        self.gather_intents(next_tick);
        self.apply_intents();
        self.tick = next_tick;
        self.state_checksum()
    }

    pub fn step_profiled(&mut self) -> TickTimings {
        if self.paused {
            return TickTimings::default();
        }

        let total_started = Instant::now();

        let started = Instant::now();
        let next_tick = self.tick.saturating_add(1);
        let clock = started.elapsed();

        let started = Instant::now();
        self.build_spatial_index();
        let spatial_index = started.elapsed();

        let started = Instant::now();
        self.gather_intents(next_tick);
        let sense_and_controller = started.elapsed();

        let started = Instant::now();
        self.apply_intents();
        self.tick = next_tick;
        let apply = started.elapsed();

        let started = Instant::now();
        std::hint::black_box(self.state_checksum());
        let checksum = started.elapsed();

        TickTimings {
            clock,
            spatial_index,
            sense_and_controller,
            apply,
            checksum,
            total: total_started.elapsed(),
        }
    }

    pub fn state_checksum(&self) -> u64 {
        let mut bytes = Vec::with_capacity(29 + self.len() * SNAPSHOT_ENTITY_BYTES);
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.config_hash.to_le_bytes());
        bytes.push(u8::from(self.paused));
        for index in 0..self.len() {
            bytes.extend_from_slice(&self.ids[index].to_le_bytes());
            bytes.extend_from_slice(&self.x[index].to_le_bytes());
            bytes.extend_from_slice(&self.y[index].to_le_bytes());
            bytes.extend_from_slice(&self.energy[index].to_le_bytes());
        }
        fnv1a64(&bytes)
    }

    fn build_spatial_index(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        for index in 0..self.len() {
            let bucket = bucket_index(self.x[index], self.y[index]);
            self.buckets[bucket].push(index);
        }
    }

    fn gather_intents(&mut self, next_tick: u64) {
        for index in 0..self.len() {
            let bucket = bucket_index(self.x[index], self.y[index]);
            let bucket_x = bucket % BUCKET_AXIS;
            let bucket_y = bucket / BUCKET_AXIS;
            let mut nearest: Option<(i64, u32, usize)> = None;

            for neighbor_y in bucket_y.saturating_sub(1)..=(bucket_y + 1).min(BUCKET_AXIS - 1) {
                for neighbor_x in bucket_x.saturating_sub(1)..=(bucket_x + 1).min(BUCKET_AXIS - 1) {
                    for &candidate in &self.buckets[neighbor_y * BUCKET_AXIS + neighbor_x] {
                        if candidate == index {
                            continue;
                        }
                        let dx = i64::from(self.x[candidate] - self.x[index]);
                        let dy = i64::from(self.y[candidate] - self.y[index]);
                        let distance_squared = dx * dx + dy * dy;
                        let key = (distance_squared, self.ids[candidate], candidate);
                        if nearest.is_none_or(|current| key < current) {
                            nearest = Some(key);
                        }
                    }
                }
            }

            let (toward_x, toward_y) = nearest
                .map(|(_, _, candidate)| {
                    (
                        (self.x[candidate] - self.x[index]).signum() as i8,
                        (self.y[candidate] - self.y[index]).signum() as i8,
                    )
                })
                .unwrap_or((0, 0));
            let jitter_x = random_step(named_random(
                self.seed,
                next_tick,
                SYSTEM_MOVEMENT,
                self.ids[index],
                0,
            ));
            let jitter_y = random_step(named_random(
                self.seed,
                next_tick,
                SYSTEM_MOVEMENT,
                self.ids[index],
                1,
            ));
            self.intent_x[index] = (toward_x + jitter_x).clamp(-2, 2);
            self.intent_y[index] = (toward_y + jitter_y).clamp(-2, 2);
        }
    }

    fn apply_intents(&mut self) {
        for index in 0..self.len() {
            self.x[index] =
                (self.x[index] + i32::from(self.intent_x[index]) * 64).clamp(0, WORLD_EXTENT - 1);
            self.y[index] =
                (self.y[index] + i32::from(self.intent_y[index]) * 64).clamp(0, WORLD_EXTENT - 1);
            let movement_cost = i32::from(self.intent_x[index].unsigned_abs())
                + i32::from(self.intent_y[index].unsigned_abs());
            self.energy[index] = self.energy[index].saturating_sub(1 + movement_cost);
        }
    }
}

fn bucket_index(x: i32, y: i32) -> usize {
    let x = usize::try_from(x).expect("validated non-negative position");
    let y = usize::try_from(y).expect("validated non-negative position");
    let extent = usize::try_from(WORLD_EXTENT).expect("positive extent");
    let bucket_x = (x * BUCKET_AXIS / extent).min(BUCKET_AXIS - 1);
    let bucket_y = (y * BUCKET_AXIS / extent).min(BUCKET_AXIS - 1);
    bucket_y * BUCKET_AXIS + bucket_x
}

fn random_step(value: u64) -> i8 {
    match value % 3 {
        0 => -1,
        1 => 0,
        _ => 1,
    }
}

pub fn named_random(
    world_seed: u64,
    tick: u64,
    system_id: u64,
    subject_id: u32,
    draw_index: u32,
) -> u64 {
    let mut value = world_seed ^ 0x9e37_79b9_7f4a_7c15;
    value = mix64(value ^ tick.rotate_left(17));
    value = mix64(value ^ system_id.rotate_left(31));
    value = mix64(value ^ u64::from(subject_id).rotate_left(7));
    mix64(value ^ u64::from(draw_index).rotate_left(43))
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn encode_snapshot(world: &World) -> Vec<u8> {
    let entity_body_len = world.len() * SNAPSHOT_ENTITY_BYTES;
    let mut payload = Vec::with_capacity(SNAPSHOT_SECTION_HEADER_LEN + entity_body_len);
    payload.extend_from_slice(&SNAPSHOT_ENTITY_SECTION.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&(entity_body_len as u64).to_le_bytes());
    for index in 0..world.len() {
        payload.extend_from_slice(&world.ids[index].to_le_bytes());
        payload.extend_from_slice(&world.x[index].to_le_bytes());
        payload.extend_from_slice(&world.y[index].to_le_bytes());
        payload.extend_from_slice(&world.energy[index].to_le_bytes());
    }

    let flags = if world.paused {
        SNAPSHOT_FLAG_PAUSED
    } else {
        0
    };
    let mut snapshot = Vec::with_capacity(SNAPSHOT_HEADER_LEN + payload.len());
    snapshot.extend_from_slice(SNAPSHOT_MAGIC);
    snapshot.extend_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
    snapshot.extend_from_slice(&(SNAPSHOT_HEADER_LEN as u16).to_le_bytes());
    snapshot.extend_from_slice(&flags.to_le_bytes());
    snapshot.extend_from_slice(&world.tick.to_le_bytes());
    snapshot.extend_from_slice(&world.seed.to_le_bytes());
    snapshot.extend_from_slice(&world.config_hash.to_le_bytes());
    snapshot.extend_from_slice(&(world.len() as u32).to_le_bytes());
    snapshot.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    snapshot.extend_from_slice(&crc32(&payload).to_le_bytes());
    snapshot.extend_from_slice(&world.state_checksum().to_le_bytes());
    debug_assert_eq!(snapshot.len(), SNAPSHOT_HEADER_LEN);
    snapshot.extend_from_slice(&payload);
    snapshot
}

pub fn decode_snapshot(bytes: &[u8]) -> Result<World, SnapshotError> {
    if bytes.len() < SNAPSHOT_HEADER_LEN {
        return Err(SnapshotError::Truncated);
    }
    if &bytes[0..4] != SNAPSHOT_MAGIC {
        return Err(SnapshotError::BadMagic);
    }

    let version = read_u16(bytes, 4)?;
    if version != SNAPSHOT_VERSION {
        return Err(SnapshotError::UnsupportedVersion(version));
    }
    let header_len = usize::from(read_u16(bytes, 6)?);
    if header_len != SNAPSHOT_HEADER_LEN {
        return Err(SnapshotError::InvalidHeaderLength(header_len));
    }
    let flags = read_u32(bytes, 8)?;
    if flags & !SNAPSHOT_FLAG_PAUSED != 0 {
        return Err(SnapshotError::UnknownFlags(flags));
    }
    let tick = read_u64(bytes, 12)?;
    let seed = read_u64(bytes, 20)?;
    let config_hash = read_u64(bytes, 28)?;
    let organism_count = usize::try_from(read_u32(bytes, 36)?).expect("u32 fits usize");
    if organism_count == 0 || organism_count > SNAPSHOT_MAX_ENTITIES {
        return Err(SnapshotError::EntityCountOutOfBounds(organism_count));
    }
    let payload_len = usize::try_from(read_u64(bytes, 40)?)
        .map_err(|_| SnapshotError::PayloadTooLarge(usize::MAX))?;
    if payload_len > SNAPSHOT_MAX_PAYLOAD {
        return Err(SnapshotError::PayloadTooLarge(payload_len));
    }
    let expected_total = header_len
        .checked_add(payload_len)
        .ok_or(SnapshotError::PayloadTooLarge(payload_len))?;
    if bytes.len() != expected_total {
        return Err(SnapshotError::LengthMismatch {
            declared: expected_total,
            actual: bytes.len(),
        });
    }
    let expected_payload_checksum = read_u32(bytes, 48)?;
    let expected_state_checksum = read_u64(bytes, 52)?;
    let payload = &bytes[header_len..];
    let actual_payload_checksum = crc32(payload);
    if expected_payload_checksum != actual_payload_checksum {
        return Err(SnapshotError::ChecksumMismatch);
    }
    if payload.len() < SNAPSHOT_SECTION_HEADER_LEN {
        return Err(SnapshotError::Truncated);
    }
    let section_tag = read_u16(payload, 0)?;
    if section_tag != SNAPSHOT_ENTITY_SECTION {
        return Err(SnapshotError::UnknownSection(section_tag));
    }
    let section_flags = read_u16(payload, 2)?;
    if section_flags != 0 {
        return Err(SnapshotError::UnknownSectionFlags(section_flags));
    }
    let section_len = usize::try_from(read_u64(payload, 4)?)
        .map_err(|_| SnapshotError::PayloadTooLarge(usize::MAX))?;
    let expected_section_len = organism_count
        .checked_mul(SNAPSHOT_ENTITY_BYTES)
        .ok_or(SnapshotError::PayloadTooLarge(usize::MAX))?;
    if section_len != expected_section_len
        || payload.len() != SNAPSHOT_SECTION_HEADER_LEN + section_len
    {
        return Err(SnapshotError::InvalidEntitySectionLength);
    }

    let mut entities = Vec::with_capacity(organism_count);
    let body = &payload[SNAPSHOT_SECTION_HEADER_LEN..];
    for index in 0..organism_count {
        let offset = index * SNAPSHOT_ENTITY_BYTES;
        entities.push(EntitySeed {
            id: read_u32(body, offset)?,
            x: read_i32(body, offset + 4)?,
            y: read_i32(body, offset + 8)?,
            energy: read_i32(body, offset + 12)?,
        });
    }

    let mut world =
        World::from_entities(seed, config_hash, entities).map_err(SnapshotError::InvalidWorld)?;
    world.tick = tick;
    world.paused = flags & SNAPSHOT_FLAG_PAUSED != 0;
    if world.state_checksum() != expected_state_checksum {
        return Err(SnapshotError::StateChecksumMismatch);
    }
    Ok(world)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, SnapshotError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(SnapshotError::Truncated)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, SnapshotError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(SnapshotError::Truncated)?;
    Ok(u32::from_le_bytes(slice.try_into().expect("four bytes")))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, SnapshotError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(SnapshotError::Truncated)?;
    Ok(i32::from_le_bytes(slice.try_into().expect("four bytes")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, SnapshotError> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or(SnapshotError::Truncated)?;
    Ok(u64::from_le_bytes(slice.try_into().expect("eight bytes")))
}

fn crc32(bytes: &[u8]) -> u32 {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldError {
    InvalidOrganismCount(usize),
    DuplicateEntityId,
    PositionOutOfBounds,
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrganismCount(count) => {
                write!(formatter, "invalid organism count: {count}")
            }
            Self::DuplicateEntityId => formatter.write_str("duplicate entity ID"),
            Self::PositionOutOfBounds => formatter.write_str("entity position is out of bounds"),
        }
    }
}

impl std::error::Error for WorldError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    Truncated,
    BadMagic,
    UnsupportedVersion(u16),
    InvalidHeaderLength(usize),
    UnknownFlags(u32),
    EntityCountOutOfBounds(usize),
    PayloadTooLarge(usize),
    LengthMismatch { declared: usize, actual: usize },
    ChecksumMismatch,
    UnknownSection(u16),
    UnknownSectionFlags(u16),
    InvalidEntitySectionLength,
    InvalidWorld(WorldError),
    StateChecksumMismatch,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SnapshotError {}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: u64 = 0x5eed_cafe_f00d_beef;

    #[test]
    fn same_seed_and_config_repeat_exactly() {
        let config = TickConfig::new(500, TEST_SEED).unwrap();
        let mut first = World::synthetic(config);
        let mut second = World::synthetic(config);
        for _ in 0..250 {
            assert_eq!(first.step(), second.step());
        }
        assert_eq!(first.state_checksum(), second.state_checksum());
    }

    #[test]
    fn insertion_order_is_normalized_by_stable_id() {
        let entities = vec![
            EntitySeed {
                id: 30,
                x: 1_000,
                y: 2_000,
                energy: 8_000,
            },
            EntitySeed {
                id: 10,
                x: 4_000,
                y: 5_000,
                energy: 8_500,
            },
            EntitySeed {
                id: 20,
                x: 7_000,
                y: 8_000,
                energy: 9_000,
            },
        ];
        let mut reversed = entities.clone();
        reversed.reverse();
        let mut first = World::from_entities(TEST_SEED, 42, entities).unwrap();
        let mut second = World::from_entities(TEST_SEED, 42, reversed).unwrap();
        for _ in 0..50 {
            assert_eq!(first.step(), second.step());
        }
    }

    #[test]
    fn named_stream_draws_do_not_shift_other_subjects() {
        let before = named_random(TEST_SEED, 7, SYSTEM_MOVEMENT, 99, 0);
        let _unrelated = named_random(TEST_SEED, 7, SYSTEM_MOVEMENT, 12, 900);
        let after = named_random(TEST_SEED, 7, SYSTEM_MOVEMENT, 99, 0);
        assert_eq!(before, after);
    }

    #[test]
    fn paused_world_does_not_advance() {
        let mut world = World::synthetic(TickConfig::new(10, TEST_SEED).unwrap());
        world.set_paused(true);
        let before = world.state_checksum();
        assert_eq!(world.step(), before);
        assert_eq!(world.tick_number(), 0);
    }

    #[test]
    fn snapshot_round_trip_preserves_state_and_provenance() {
        let mut world = World::synthetic(TickConfig::new(500, TEST_SEED).unwrap());
        for _ in 0..25 {
            world.step();
        }
        world.set_paused(true);
        let encoded = encode_snapshot(&world);
        let decoded = decode_snapshot(&encoded).unwrap();
        assert_eq!(decoded.tick_number(), world.tick_number());
        assert_eq!(decoded.seed(), world.seed());
        assert_eq!(decoded.config_hash(), world.config_hash());
        assert_eq!(decoded.is_paused(), world.is_paused());
        assert_eq!(decoded.state_checksum(), world.state_checksum());
    }

    #[test]
    fn malformed_snapshots_fail_closed() {
        let world = World::synthetic(TickConfig::new(10, TEST_SEED).unwrap());
        let valid = encode_snapshot(&world);

        assert_eq!(
            decode_snapshot(&valid[..20]).unwrap_err(),
            SnapshotError::Truncated
        );

        let mut bad_magic = valid.clone();
        bad_magic[0] = b'X';
        assert_eq!(
            decode_snapshot(&bad_magic).unwrap_err(),
            SnapshotError::BadMagic
        );

        let mut bad_version = valid.clone();
        bad_version[4..6].copy_from_slice(&99_u16.to_le_bytes());
        assert_eq!(
            decode_snapshot(&bad_version).unwrap_err(),
            SnapshotError::UnsupportedVersion(99)
        );

        let mut bad_checksum = valid.clone();
        let last = bad_checksum.len() - 1;
        bad_checksum[last] ^= 0xff;
        assert_eq!(
            decode_snapshot(&bad_checksum).unwrap_err(),
            SnapshotError::ChecksumMismatch
        );

        let mut oversized = valid;
        oversized[40..48].copy_from_slice(&((SNAPSHOT_MAX_PAYLOAD as u64) + 1).to_le_bytes());
        assert_eq!(
            decode_snapshot(&oversized).unwrap_err(),
            SnapshotError::PayloadTooLarge(SNAPSHOT_MAX_PAYLOAD + 1)
        );
    }
}
