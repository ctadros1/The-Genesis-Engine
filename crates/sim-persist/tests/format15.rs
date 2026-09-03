//! ALIF format 15: the Phase 19 consumption config fields (ADR-0034), the
//! extended `SECTION_CHEMISTRY` body, the retained format-14 reader/writer,
//! and the registered 14-to-15 migration.
//!
//! Helpers below are copied rather than shared, on the pattern every earlier
//! format's test file (`format7.rs`, `format14.rs`) already follows in this
//! crate: each format's test file is self-contained.

use sim_core::{ChemistryConfig, SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION_14, decode_snapshot, decode_snapshot_format14,
    decode_snapshot_migrating, encode_snapshot, encode_snapshot_format14, migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// Header offsets, from the fixed 112-byte layout in `codec.rs`.
const OFFSET_UNCOMPRESSED_LEN: usize = 68;
const OFFSET_STORED_LEN: usize = 76;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;
/// Phase 15 chemistry field state (ADR-0031/ADR-0034). Matches `codec.rs`'s
/// private `SECTION_CHEMISTRY`; declared again here because a test file
/// asserts against the wire format from the outside, on the pattern
/// `format7.rs`'s `SECTION_OBJECTS` establishes.
const SECTION_CHEMISTRY: u16 = 19;

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
/// Returns `(tag, flags, body_start, body_len)`. Copied from `format14.rs`.
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
/// from `format14.rs`.
fn raw_section(bytes: &[u8], tag: u16) -> Vec<u8> {
    let (start, len) = section(bytes, tag);
    bytes[start - 12..start + len + 4].to_vec()
}

/// Re-seal a snapshot whose section body was patched in place: the section
/// CRC that covers it, then the payload CRC in the header. Copied from
/// `format14.rs`'s helper of the same name.
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
/// as the original - that mismatch is exactly what (e) below exploits: a
/// format-14-shaped `SECTION_CHEMISTRY` body spliced into an otherwise
/// genuine format-15 file, and the reverse, so each reader meets the
/// other's section shape with nothing else about the file disturbed.
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

/// A chemistry-and-microbial world with the field coupled to organism
/// feeding: `phase2_default` at 16x16 with 40 initial organisms (small
/// enough that the round trip and the corruption sweep below stay fast).
fn consumption_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 16;
    config.cells_y = 16;
    config.initial_organisms = 40;
    config.max_entities = 200;
    config.chemistry.enabled = true;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config.chemistry.consumption_fraction_q16 = 65_536;
    config
        .validate()
        .expect("a chemistry+microbial+consumption world validates");
    config
}

fn consumption_world(seed: u64) -> World {
    World::new(consumption_config(seed)).expect("consumption world builds")
}

/// The Phase 16 world this format's fraction-zero default reproduces:
/// chemistry and microbial content exercised (so formats 11-13's own
/// additions are not vacuous), consumption left off.
fn plain_chemistry_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.chemistry.enabled = true;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config
        .validate()
        .expect("a plain chemistry+microbial world validates");
    config
}

// --- (a) byte-for-byte round trip through the current format ----------------

/// `consumed_milli` is made non-vacuous by hand, on the pattern
/// `format14.rs`'s transition test sets its counters: the term is real state
/// a restore cannot rederive from a fresh export, so the round trip has to
/// prove it carries whatever is there. Compared as `SaveState`s rather than
/// restored through `World::from_state`, because the edited value need not
/// satisfy the field identity a live world enforces.
#[test]
fn a_consuming_world_round_trips_byte_for_byte_through_the_current_format() {
    let mut world = consumption_world(SEED);
    for _ in 0..50 {
        world.step();
    }
    world.check_invariants().expect("invariants hold");

    let mut state = world.export_state();
    {
        let chemistry = state
            .chemistry
            .as_mut()
            .expect("a chemistry-enabled world carries the section");
        chemistry.consumed_milli = 12_345;
    }
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 15");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 15");

    assert_eq!(decoded, state, "the full state must round-trip exactly");

    let decoded_chemistry = decoded
        .chemistry
        .as_ref()
        .expect("the chemistry section must survive the round trip");
    assert_eq!(decoded_chemistry.consumed_milli, 12_345);
    assert_eq!(
        decoded.config.chemistry.consumption_fraction_q16,
        state.config.chemistry.consumption_fraction_q16
    );
    assert_eq!(decoded.config.chemistry.consumption_fraction_q16, 65_536);
    assert_eq!(
        decoded.config.chemistry.consumption_yield_q16,
        state.config.chemistry.consumption_yield_q16
    );
    assert_eq!(
        decoded.config.chemistry.consumption_yield_q16,
        ChemistryConfig::chemistry_default().consumption_yield_q16
    );
}

// --- (b) a format-14 file migrates with consumption at its defaults --------

#[test]
fn a_format_14_file_migrates_to_current_with_consumption_at_its_defaults() {
    let world = advance(plain_chemistry_config(SEED), 60);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert_eq!(state.config.chemistry.consumption_fraction_q16, 0);
    assert_eq!(
        state.config.chemistry.consumption_yield_q16,
        ChemistryConfig::chemistry_default().consumption_yield_q16
    );
    assert_eq!(
        state.chemistry.as_ref().expect("chemistry section present").consumed_milli,
        0
    );

    let legacy = encode_snapshot_format14(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 14");

    let migration = migration_for(FORMAT_VERSION_14)
        .expect("format 14 is registered")
        .expect("format 14 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_14);
    assert_eq!(migration.to_format, sim_persist::FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 14 to 15 transform invents nothing and must not claim to"
    );

    let (_, migrated_state) = decode_snapshot_migrating(&legacy).expect("migrates to current");
    assert_eq!(migrated_state.config.chemistry.consumption_fraction_q16, 0);
    assert_eq!(
        migrated_state.config.chemistry.consumption_yield_q16,
        ChemistryConfig::chemistry_default().consumption_yield_q16
    );
    assert_eq!(
        migrated_state
            .chemistry
            .as_ref()
            .expect("chemistry section present")
            .consumed_milli,
        0,
        "a format-14 file has no consumed term to migrate"
    );
    assert_eq!(migrated_state, state, "the migration must invent nothing");

    let (_, legacy_state) = decode_snapshot_format14(&legacy).expect("legacy decode");
    assert_eq!(
        migrated_state, legacy_state,
        "the migrated state must agree with a native format-14 load"
    );
}

// --- (c) the retained format-14 writer refuses what it cannot express -------

#[test]
fn the_retained_format_14_writer_refuses_what_it_cannot_express() {
    // A nonzero consumption fraction is refused by name, even though the
    // chemistry section itself is otherwise expressible at format 14.
    let fraction_state = consumption_world(SEED).export_state();
    assert_eq!(
        fraction_state.config.chemistry.consumption_fraction_q16,
        65_536
    );
    assert!(
        matches!(
            encode_snapshot_format14(
                &fraction_state,
                1,
                0,
                0,
                sim_persist::BUILD_VERSION,
                0,
                None,
            ),
            Err(CodecError::FieldNotInFormat {
                field: "chemistry consumption",
                format: FORMAT_VERSION_14,
            })
        ),
        "a nonzero consumption fraction must be refused by name"
    );

    // A nonzero `consumed_milli` is refused by name even while the config
    // fraction is zero, because it is real state a pre-15 file cannot
    // express - the same class of refusal `refuse_format14_state` makes for
    // a real transition section under a disabled gate.
    let mut consumed_state = World::new(plain_chemistry_config(SEED))
        .expect("plain chemistry world builds")
        .export_state();
    assert_eq!(consumed_state.config.chemistry.consumption_fraction_q16, 0);
    consumed_state
        .chemistry
        .as_mut()
        .expect("chemistry section present")
        .consumed_milli = 7;
    assert!(
        matches!(
            encode_snapshot_format14(
                &consumed_state,
                1,
                0,
                0,
                sim_persist::BUILD_VERSION,
                0,
                None,
            ),
            Err(CodecError::FieldNotInFormat {
                field: "chemistry consumed",
                format: FORMAT_VERSION_14,
            })
        ),
        "a nonzero consumed_milli must be refused by name"
    );
}

// --- (d) the chemistry section bounds its allocation at format 15 ----------

/// Three hostile counts, patched into the same genuine format-15 file's
/// `SECTION_CHEMISTRY` body: a count that overflows the multiplication, a
/// count that overflows only the addition, and a count equal to the raw
/// body length in bytes. None may panic; each fails closed with the variant
/// this codec actually produces for it, on the pattern `format14.rs`'s
/// transition-section analogue establishes.
#[test]
fn the_chemistry_section_bounds_its_allocation_at_format_15_before_trusting_the_count() {
    let world = consumption_world(SEED);
    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 15");

    let (body_start, body_len) = section(&bytes, SECTION_CHEMISTRY);

    let patch_count = |value: u64| -> Vec<u8> {
        let mut patched = bytes.clone();
        patched[body_start..body_start + 8].copy_from_slice(&value.to_le_bytes());
        reseal(&mut patched, body_start, body_len);
        patched
    };

    // A count that overflows the `count * 8` multiplication outright.
    let overflow_mul = patch_count(u64::MAX);
    assert_eq!(
        decode_snapshot(&overflow_mul).err(),
        Some(CodecError::ValueOutOfRange("chemistry values")),
        "a count overflowing the multiplication must be refused before allocating"
    );

    // A count whose product fits u64 but whose sum with the trailing bytes
    // overflows.
    let overflow_add = patch_count(u64::MAX / 8);
    assert_eq!(
        decode_snapshot(&overflow_add).err(),
        Some(CodecError::ValueOutOfRange("chemistry values")),
        "a count overflowing only the trailing-bytes addition must be refused too"
    );

    // A count equal to the section's whole body length in bytes - nowhere
    // near overflow, but far more elements than the body could ever hold.
    let body_length_count = patch_count(body_len as u64);
    assert_eq!(
        decode_snapshot(&body_length_count).err(),
        Some(CodecError::ValueOutOfRange("chemistry values")),
        "a count equal to the body length must still be refused by the bound, \
         never by exhausting the reader"
    );
}

// --- (e) a section shaped for the other format fails closed under each -----

/// The same `plain_chemistry_config` world encoded twice - once at format
/// 14, once at the current format 15 - donates its `SECTION_CHEMISTRY`
/// chunk to the other file, with nothing else about either file disturbed.
/// A format-15 body has sixteen more trailing bytes than a format-14 one, so
/// each direction fails closed on the section alone. The format-15 reader
/// fails on `allocation_fits` itself: its 72-byte trailing requirement does
/// not fit inside a body only 56 bytes longer than the concentrations, so
/// the count is refused before a single byte is read - the same bound
/// `the_chemistry_section_bounds_its_allocation...` exercises with a hostile
/// count, reached here by a hostile body instead. The format-14 reader gets
/// as far as decoding every field it knows and is left with the extra
/// `consumed_milli` bytes over.
#[test]
fn a_section_shaped_for_the_other_format_fails_closed_under_each_reader() {
    let world = advance(plain_chemistry_config(SEED), 40);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let fourteen = encode_snapshot_format14(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 14");
    let fifteen = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 15");

    // A genuine format-14 (56-trailing) chemistry body spliced into an
    // otherwise genuine format-15 file: the format-15 reader expects one
    // more trailing i128 than this body has.
    let forged_fifteen = replace_section_bytes(
        &fifteen,
        SECTION_CHEMISTRY,
        &raw_section(&fourteen, SECTION_CHEMISTRY),
    );
    assert_eq!(
        decode_snapshot(&forged_fifteen).err(),
        Some(CodecError::ValueOutOfRange("chemistry values")),
        "a format-15 file carrying a format-14-shaped chemistry body must be \
         refused by the allocation bound before any byte of it is read"
    );

    // The reverse: a genuine format-15 (72-trailing) chemistry body spliced
    // into an otherwise genuine format-14 file. The format-14 reader never
    // reads the extra i128, so it is left with sixteen bytes over.
    let forged_fourteen = replace_section_bytes(
        &fourteen,
        SECTION_CHEMISTRY,
        &raw_section(&fifteen, SECTION_CHEMISTRY),
    );
    assert_eq!(
        decode_snapshot_format14(&forged_fourteen).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format-14 file carrying a format-15-shaped chemistry body must be \
         refused on the section's trailing bytes"
    );
}

// --- (f) a consuming world survives a save round trip with the same future --

#[test]
fn a_consuming_world_survives_a_save_round_trip_with_the_same_future() {
    let mut world = consumption_world(SEED);
    for _ in 0..50 {
        world.step();
    }
    world.check_invariants().expect("invariants hold");

    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 15");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 15");

    let mut restored = World::from_state(decoded).expect("restore the consuming world");
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
