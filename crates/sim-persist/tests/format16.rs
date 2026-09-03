//! ALIF format 16: the Phase 21 intake-order config field (ADR-0036), the
//! retained format-15 reader/writer, and the registered 15-to-16 migration.
//!
//! Helpers below are copied rather than shared, on the pattern every earlier
//! format's test file (`format7.rs`, `format15.rs`) already follows in this
//! crate: each format's test file is self-contained.

use sim_core::{IntakeOrder, SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION_15, decode_snapshot, decode_snapshot_format15,
    decode_snapshot_migrating, encode_snapshot, encode_snapshot_format15, migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// Header offsets, from the fixed 112-byte layout in `codec.rs`.
const OFFSET_UNCOMPRESSED_LEN: usize = 68;
const OFFSET_STORED_LEN: usize = 76;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;
/// The config section tag. Matches `codec.rs`'s private `SECTION_CONFIG`;
/// declared again here because a test file asserts against the wire format
/// from the outside, on the pattern `format15.rs`'s `SECTION_CHEMISTRY`
/// establishes.
const SECTION_CONFIG: u16 = 1;

fn payload_start() -> usize {
    HEADER_LEN + sim_persist::BUILD_VERSION.len()
}

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world builds");
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    world
}

/// Offsets of every section, walked from the payload start rather than
/// searched for, so this cannot match a tag-shaped value inside a genome.
/// Returns `(tag, flags, body_start, body_len)`. Copied from `format15.rs`.
fn sections(bytes: &[u8]) -> Vec<(u16, u16, usize, usize)> {
    let mut out = Vec::new();
    let mut offset = payload_start();
    while offset + 12 <= bytes.len() {
        let tag = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let flags = u16::from_le_bytes(bytes[offset + 2..offset + 4].try_into().unwrap());
        let length =
            u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap()) as usize;
        out.push((tag, flags, offset + 12, length));
        offset += 12 + length + 4;
    }
    out
}

fn section(bytes: &[u8], tag: u16) -> (usize, usize) {
    sections(bytes)
        .into_iter()
        .find_map(|(found, _, start, len)| (found == tag).then_some((start, len)))
        .unwrap_or_else(|| panic!("section {tag} is present"))
}

/// One section's whole wire chunk - tag, flags, length, body, and CRC - cut
/// out verbatim so it can be spliced into a different file's payload. Copied
/// from `format15.rs`.
fn raw_section(bytes: &[u8], tag: u16) -> Vec<u8> {
    let (start, len) = section(bytes, tag);
    bytes[start - 12..start + len + 4].to_vec()
}

/// Re-seal a snapshot whose section body was patched in place: the section
/// CRC that covers it, then the payload CRC in the header. Copied from
/// `format15.rs`'s helper of the same name.
fn reseal(bytes: &mut [u8], body_start: usize, body_len: usize) {
    let body = bytes[body_start..body_start + body_len].to_vec();
    let section_crc = sim_persist::crc32(&body);
    bytes[body_start + body_len..body_start + body_len + 4]
        .copy_from_slice(&section_crc.to_le_bytes());
    let payload = bytes[payload_start()..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    bytes[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
}

/// Replace one named section's whole wire chunk with a different one cut
/// from a donor file (`raw_section`), fixing the two payload-length header
/// words and the payload CRC. The replacement need not be the same length
/// as the original - that mismatch is exactly what the "shaped for the
/// other format" test below exploits: a format-15-shaped `SECTION_CONFIG`
/// body spliced into an otherwise genuine format-16 file, and the reverse.
/// Copied from `format15.rs`.
fn replace_section_bytes(bytes: &[u8], tag: u16, replacement: &[u8]) -> Vec<u8> {
    let (start, len) = section(bytes, tag);
    let chunk_start = start - 12;
    let chunk_end = start + len + 4;
    let mut out = bytes[..chunk_start].to_vec();
    out.extend_from_slice(replacement);
    out.extend_from_slice(&bytes[chunk_end..]);
    let payload_len = (out.len() - payload_start()) as u64;
    out[OFFSET_UNCOMPRESSED_LEN..OFFSET_UNCOMPRESSED_LEN + 8]
        .copy_from_slice(&payload_len.to_le_bytes());
    out[OFFSET_STORED_LEN..OFFSET_STORED_LEN + 8].copy_from_slice(&payload_len.to_le_bytes());
    let payload = out[payload_start()..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    out[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
    out
}

/// A world whose intake order is the probe rather than the shipped default.
/// Nothing else about `phase2_default` needs to change: `validate` has no
/// extra precondition on `intake_order` (unlike, say, artifact's
/// dependency on `worldmod`), so the flag is behavioural on its own.
fn descending_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.physiology.intake_order = IntakeOrder::Descending;
    config
        .validate()
        .expect("a descending-intake world validates");
    config
}

fn descending_world(seed: u64) -> World {
    World::new(descending_config(seed)).expect("descending-intake world builds")
}

// --- (a) byte-for-byte round trip through the current format ----------------

#[test]
fn a_descending_world_round_trips_byte_for_byte_through_the_current_format() {
    let mut world = descending_world(SEED);
    for _ in 0..50 {
        world.step();
    }
    world.check_invariants().expect("invariants hold");

    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 16");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 16");

    assert_eq!(decoded, state, "the full state must round-trip exactly");
    assert_eq!(
        decoded.config.physiology.intake_order,
        IntakeOrder::Descending
    );
}

// --- (b) a format-15 file migrates with the order at its default -----------

#[test]
fn a_format_15_file_migrates_to_current_with_the_order_at_ascending() {
    let world = advance(SimConfig::phase2_default(SEED), 60);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert_eq!(state.config.physiology.intake_order, IntakeOrder::Ascending);

    let legacy =
        encode_snapshot_format15(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 15");

    let migration = migration_for(FORMAT_VERSION_15)
        .expect("format 15 is registered")
        .expect("format 15 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_15);
    assert_eq!(migration.to_format, sim_persist::FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 15 to 16 transform invents nothing and must not claim to"
    );

    let (_, migrated_state) = decode_snapshot_migrating(&legacy).expect("migrates to current");
    assert_eq!(
        migrated_state.config.physiology.intake_order,
        IntakeOrder::Ascending
    );
    assert_eq!(migrated_state, state, "the migration must invent nothing");

    let (_, legacy_state) = decode_snapshot_format15(&legacy).expect("legacy decode");
    assert_eq!(
        migrated_state, legacy_state,
        "the migrated state must agree with a native format-15 load"
    );
}

// --- (c) the retained format-15 writer refuses what it cannot express -------

#[test]
fn the_retained_format_15_writer_refuses_a_descending_world_and_accepts_an_ascending_one() {
    let descending_state = descending_world(SEED).export_state();
    assert_eq!(
        descending_state.config.physiology.intake_order,
        IntakeOrder::Descending
    );
    assert!(
        matches!(
            encode_snapshot_format15(
                &descending_state,
                1,
                0,
                0,
                sim_persist::BUILD_VERSION,
                0,
                None,
            ),
            Err(CodecError::FieldNotInFormat {
                field: "intake order",
                format: FORMAT_VERSION_15,
            })
        ),
        "a descending intake order must be refused by name"
    );

    // ...and the same shape of world at the shipped order writes without
    // complaint, which is what makes the refusal above about the format
    // rather than the state.
    let ascending_state = advance(SimConfig::phase2_default(SEED), 40).export_state();
    assert_eq!(
        ascending_state.config.physiology.intake_order,
        IntakeOrder::Ascending
    );
    assert!(
        encode_snapshot_format15(
            &ascending_state,
            1,
            0,
            0,
            sim_persist::BUILD_VERSION,
            0,
            None,
        )
        .is_ok(),
        "an ascending world must still be expressible at format 15"
    );
}

// --- (d) a section shaped for the other format fails closed under each -----

/// The same `phase2_default` world encoded twice - once at format 15, once
/// at the current format 16 - donates its `SECTION_CONFIG` chunk to the
/// other file, with nothing else about either file disturbed. A format-16
/// config body has exactly one more trailing byte than a format-15 one, so
/// each direction fails closed on the section alone.
#[test]
fn a_section_shaped_for_the_other_format_fails_closed_under_each_reader() {
    let world = advance(SimConfig::phase2_default(SEED), 40);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let fifteen =
        encode_snapshot_format15(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 15");
    let sixteen = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 16");

    // A genuine format-15 (one-byte-shorter) config body spliced into an
    // otherwise genuine format-16 file: the format-16 reader runs out of
    // body reading the intake-order byte.
    let forged_sixteen = replace_section_bytes(
        &sixteen,
        SECTION_CONFIG,
        &raw_section(&fifteen, SECTION_CONFIG),
    );
    assert_eq!(
        decode_snapshot(&forged_sixteen).err(),
        Some(CodecError::TruncatedSection),
        "a format-16 file carrying a format-15-shaped config body must run \
         out of bytes reading the appended intake-order byte"
    );

    // The reverse: a genuine format-16 (one-byte-longer) config body
    // spliced into an otherwise genuine format-15 file. The format-15
    // reader never reads the extra byte, so it is left with one byte over.
    let forged_fifteen = replace_section_bytes(
        &fifteen,
        SECTION_CONFIG,
        &raw_section(&sixteen, SECTION_CONFIG),
    );
    assert_eq!(
        decode_snapshot_format15(&forged_fifteen).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format-15 file carrying a format-16-shaped config body must be \
         refused on the section's trailing bytes"
    );
}

// --- (e) an unknown order id is refused on decode ---------------------------

#[test]
fn an_unknown_intake_order_id_is_refused_on_decode() {
    let world = advance(SimConfig::phase2_default(SEED), 40);
    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 16");
    let (body_start, body_len) = section(&bytes, SECTION_CONFIG);
    // The intake-order byte is the last byte `encode_config` appends for
    // format 16, so it is the config body's final byte.
    let order_offset = body_start + body_len - 1;

    for id in [2_u8, 3, 255] {
        let mut patched = bytes.clone();
        patched[order_offset] = id;
        reseal(&mut patched, body_start, body_len);
        assert_eq!(
            decode_snapshot(&patched).err(),
            Some(CodecError::ValueOutOfRange("intake order")),
            "id {id} is neither ascending (0) nor descending (1) and must be \
             refused by name"
        );
    }
}

// --- (f) a descending world survives a save round trip with the same future -

#[test]
fn a_descending_world_survives_a_save_round_trip_with_the_same_future() {
    let mut world = descending_world(SEED);
    for _ in 0..50 {
        world.step();
    }
    world.check_invariants().expect("invariants hold");

    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 16");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 16");

    let mut restored = World::from_state(decoded).expect("restore the descending world");
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored world must start where the saved one did"
    );

    for _ in 0..50 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored world must have the same future as the one it was saved from"
    );
}
