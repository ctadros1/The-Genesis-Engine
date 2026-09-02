//! ALIF format 14: the Phase 16 transition config block (ADR-0032),
//! `SECTION_TRANSITION`, the retained format-13 reader/writer, and the
//! registered 13-to-14 migration.
//!
//! Helpers below are copied rather than shared, on the pattern every earlier
//! format's test file (`format7.rs`, `phase12_format4.rs`) already follows in
//! this crate: each format's test file is self-contained.

use sim_core::{OriginMode, SimConfig, TransitionConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION_13, decode_snapshot, decode_snapshot_format13,
    decode_snapshot_migrating, encode_snapshot, encode_snapshot_format13, migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// Header offsets, from the fixed 112-byte layout in `codec.rs`.
const OFFSET_FORMAT: usize = 4;
const OFFSET_UNCOMPRESSED_LEN: usize = 68;
const OFFSET_STORED_LEN: usize = 76;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;
const SECTION_CONFIG: u16 = 1;
/// Phase 16 transition state (ADR-0032). Matches `codec.rs`'s private
/// `SECTION_TRANSITION`; declared again here because a test file asserts
/// against the wire format from the outside, on the pattern `format7.rs`'s
/// `SECTION_OBJECTS` establishes.
const SECTION_TRANSITION: u16 = 21;

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
/// Returns `(tag, flags, body_start, body_len)`. Copied from `format7.rs`.
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
/// from `format7.rs`.
fn raw_section(bytes: &[u8], tag: u16) -> Vec<u8> {
    let (start, len) = section(bytes, tag);
    bytes[start - 12..start + len + 4].to_vec()
}

fn relabel(bytes: &[u8], format: u16) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[OFFSET_FORMAT..OFFSET_FORMAT + 2].copy_from_slice(&format.to_le_bytes());
    out
}

/// Re-seal a snapshot whose section body was patched in place: the section
/// CRC that covers it, then the payload CRC in the header. Copied from
/// `format7.rs`'s helper of the same name.
fn reseal(bytes: &mut [u8], body_start: usize, body_len: usize) {
    let body = bytes[body_start..body_start + body_len].to_vec();
    let section_crc = sim_persist::crc32(&body);
    bytes[body_start + body_len..body_start + body_len + 4]
        .copy_from_slice(&section_crc.to_le_bytes());
    let payload = bytes[payload_start()..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    bytes[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
}

/// Append one raw section chunk to the tail of an encoded file's payload,
/// fixing the two payload-length header words and the payload CRC. Copied
/// from `format7.rs`.
fn append_section_bytes(bytes: &[u8], extra: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out.extend_from_slice(extra);
    let payload_len = (out.len() - payload_start()) as u64;
    out[OFFSET_UNCOMPRESSED_LEN..OFFSET_UNCOMPRESSED_LEN + 8]
        .copy_from_slice(&payload_len.to_le_bytes());
    out[OFFSET_STORED_LEN..OFFSET_STORED_LEN + 8].copy_from_slice(&payload_len.to_le_bytes());
    let payload = out[payload_start()..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    out[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
    out
}

/// A transition-enabled config: scratch origin (zero founders), chemistry
/// and microbial density as the source the transition converts, phase2 /
/// genome2 / morphology on (`TransitionRequires` in `config.rs` demands all
/// three), and the transition gate itself on at its documented defaults.
fn transition_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 16;
    config.cells_y = 16;
    config.initial_organisms = 0;
    config.max_entities = 200;
    config.origin.mode = OriginMode::Scratch;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.chemistry.enabled = true;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config.chemistry.mutation_q16 = 4096;
    config.transition.enabled = true;
    config
        .validate()
        .expect("the transition config satisfies validate_subsystems");
    config
}

fn transition_world(seed: u64) -> World {
    World::new(transition_config(seed)).expect("transition world builds")
}

// --- (a) byte-for-byte round trip through the current format ----------------

/// The transition section is made non-vacuous by hand (every field a
/// distinct, checkable value) rather than by running the world further,
/// because the counters are real state a restore cannot rederive - the
/// round trip has to prove it carries whatever is there, not just whatever
/// a fresh world happens to produce.
///
/// The edited state is never restored through `World::from_state`: the
/// counters below are inconsistent with the ledger identity a live world
/// enforces, and restoring them would fail the invariant this test has
/// nothing to do with. Compared as `SaveState`s instead.
#[test]
fn a_transition_world_round_trips_byte_for_byte_through_the_current_format() {
    let world = transition_world(SEED);
    let mut state = world.export_state();
    {
        let transition = state
            .transition
            .as_mut()
            .expect("a transition-enabled world carries the section");
        transition.persistence[3] = 7;
        transition.materialized_total = 2;
        transition.events_total = 1;
        transition.materialized_milli = 8_000;
        transition.deferred_cap_total = 3;
        transition.deferred_capacity_total = 4;
        transition.refused_total = 0;
    }
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 14");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 14");

    assert_eq!(decoded, state, "the full state must round-trip exactly");

    let decoded_transition = decoded
        .transition
        .as_ref()
        .expect("the transition section must survive the round trip");
    let original_transition = state.transition.as_ref().unwrap();
    assert_eq!(decoded_transition.persistence, original_transition.persistence);
    assert_eq!(decoded_transition.persistence[3], 7);
    assert_eq!(
        decoded_transition.materialized_total,
        original_transition.materialized_total
    );
    assert_eq!(decoded_transition.materialized_total, 2);
    assert_eq!(
        decoded_transition.events_total,
        original_transition.events_total
    );
    assert_eq!(decoded_transition.events_total, 1);
    assert_eq!(
        decoded_transition.materialized_milli,
        original_transition.materialized_milli
    );
    assert_eq!(decoded_transition.materialized_milli, 8_000);
    assert_eq!(
        decoded_transition.deferred_cap_total,
        original_transition.deferred_cap_total
    );
    assert_eq!(decoded_transition.deferred_cap_total, 3);
    assert_eq!(
        decoded_transition.deferred_capacity_total,
        original_transition.deferred_capacity_total
    );
    assert_eq!(decoded_transition.deferred_capacity_total, 4);
    assert_eq!(
        decoded_transition.refused_total,
        original_transition.refused_total
    );
    assert_eq!(decoded_transition.refused_total, 0);
}

// --- (b) a format-13 file migrates with the transition absent ---------------

/// A transition-*disabled* world - `origin.mode` `Random`, at least one
/// founder, chemistry and microbial content exercised so formats 11-13's
/// own additions are not vacuous either - migrates through the 13-to-14
/// transform with the transition resolved to its default and the section
/// absent.
#[test]
fn a_format_13_file_migrates_to_current_with_the_transition_absent() {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 32;
    config.cells_y = 32;
    config.initial_organisms = 20;
    config.max_entities = 200;
    config.chemistry.enabled = true;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config.chemistry.excretion_fraction_q16 = 4_000;
    config.chemistry.remains_fraction_q16 = 6_000;
    config
        .validate()
        .expect("a plain chemistry+microbial world validates");

    let world = advance(config, 60);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert_eq!(state.config.transition, TransitionConfig::transition_default());
    assert!(state.transition.is_none());

    let legacy = encode_snapshot_format13(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 13");

    let migration = migration_for(FORMAT_VERSION_13)
        .expect("format 13 is registered")
        .expect("format 13 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_13);
    assert_eq!(migration.to_format, sim_persist::FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 13 to 14 transform invents nothing and must not claim to"
    );

    let (_, migrated_state) = decode_snapshot_migrating(&legacy).expect("migrates to current");
    assert!(
        migrated_state.transition.is_none(),
        "a format-13 file has no transition state to migrate"
    );
    assert_eq!(
        migrated_state.config.transition,
        TransitionConfig::transition_default(),
        "the migrated config's transition block must be exactly the default"
    );
    assert_eq!(migrated_state, state, "the migration must invent nothing");

    let (_, legacy_state) = decode_snapshot_format13(&legacy).expect("legacy decode");
    assert_eq!(
        migrated_state, legacy_state,
        "the migrated state must agree with a native format-13 load"
    );
}

// --- (c) the retained format-13 writer refuses what it cannot express -------

#[test]
fn the_retained_format_13_writer_refuses_what_it_cannot_express() {
    // State 1: the transition config is enabled and a real section is
    // present - refused on the config field, which is checked before the
    // section is ever considered.
    let transition_state = transition_world(SEED).export_state();
    assert!(transition_state.transition.is_some());
    assert!(
        matches!(
            encode_snapshot_format13(
                &transition_state,
                1,
                0,
                0,
                sim_persist::BUILD_VERSION,
                0,
                None,
            ),
            Err(CodecError::FieldNotInFormat {
                field: "transition",
                format: FORMAT_VERSION_13,
            })
        ),
        "an enabled transition config must be refused by name"
    );

    // State 2: the transition config is non-default while disabled - no
    // section exists, but the knob is still a value format 13 has no byte
    // for.
    let world = advance(SimConfig::phase2_default(SEED), 40);
    let mut disabled_but_nondefault = world.export_state();
    assert!(!disabled_but_nondefault.config.transition.enabled);
    assert!(disabled_but_nondefault.transition.is_none());
    disabled_but_nondefault.config.transition.check_interval_ticks = 50;
    assert!(
        matches!(
            encode_snapshot_format13(
                &disabled_but_nondefault,
                1,
                0,
                0,
                sim_persist::BUILD_VERSION,
                0,
                None,
            ),
            Err(CodecError::FieldNotInFormat {
                field: "transition",
                format: FORMAT_VERSION_13,
            })
        ),
        "a non-default transition config must be refused even while disabled"
    );

    // State 3: a valid scratch-origin world with the transition itself left
    // disabled - refused on the origin field by name.
    let mut scratch_config = SimConfig::phase2_default(SEED);
    scratch_config.cells_x = 16;
    scratch_config.cells_y = 16;
    scratch_config.initial_organisms = 0;
    scratch_config.max_entities = 200;
    scratch_config.origin.mode = OriginMode::Scratch;
    scratch_config.chemistry.enabled = true;
    scratch_config.chemistry.microbial_enabled = true;
    scratch_config.chemistry.abiogenesis_enabled = true;
    scratch_config
        .validate()
        .expect("a scratch origin with abiogenesis validates");
    let scratch_state = World::new(scratch_config)
        .expect("scratch world builds")
        .export_state();
    assert_eq!(scratch_state.config.origin.mode, OriginMode::Scratch);
    assert_eq!(
        scratch_state.config.transition,
        TransitionConfig::transition_default()
    );
    assert!(
        matches!(
            encode_snapshot_format13(
                &scratch_state,
                1,
                0,
                0,
                sim_persist::BUILD_VERSION,
                0,
                None,
            ),
            Err(CodecError::FieldNotInFormat {
                field: "origin scratch",
                format: FORMAT_VERSION_13,
            })
        ),
        "a scratch origin must be refused by name"
    );
}

// --- (d) the transition section bounds its allocation before trusting the count --

/// Three hostile counts, patched into the same genuine format-14 file's
/// `SECTION_TRANSITION` body: a count that overflows the multiplication, a
/// count that overflows only the addition, and a count equal to the raw
/// body length in bytes (far larger than the section could actually hold).
/// None may panic; each fails closed with the variant this codec actually
/// produces for it (D-091's discipline, D-075's "never assert exact - only
/// ever bound" for the corruption case).
#[test]
fn the_transition_section_bounds_its_allocation_before_trusting_the_count() {
    let world = transition_world(SEED);
    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 14");

    let (body_start, body_len) = section(&bytes, SECTION_TRANSITION);

    let patch_count = |value: u64| -> Vec<u8> {
        let mut patched = bytes.clone();
        patched[body_start..body_start + 8].copy_from_slice(&value.to_le_bytes());
        reseal(&mut patched, body_start, body_len);
        patched
    };

    // A count that overflows the `count * 4` multiplication outright.
    let overflow_mul = patch_count(u64::MAX);
    assert_eq!(
        decode_snapshot(&overflow_mul).err(),
        Some(CodecError::ValueOutOfRange("transition values")),
        "a count overflowing the multiplication must be refused before allocating"
    );

    // A count whose product fits u64 but whose sum with the trailing bytes
    // overflows.
    let overflow_add = patch_count(u64::MAX / 4);
    assert_eq!(
        decode_snapshot(&overflow_add).err(),
        Some(CodecError::ValueOutOfRange("transition values")),
        "a count overflowing only the trailing-bytes addition must be refused too"
    );

    // A count equal to the section's whole body length in bytes - nowhere
    // near overflow, but far more elements than the body could ever hold.
    let body_length_count = patch_count(body_len as u64);
    assert_eq!(
        decode_snapshot(&body_length_count).err(),
        Some(CodecError::ValueOutOfRange("transition values")),
        "a count equal to the body length must still be refused by the bound, \
         never by exhausting the reader"
    );
}

// --- (e) SECTION_TRANSITION in a format-13-stamped file is refused by name --

/// The direct construction uses a **Random**-origin world, deliberately not
/// the scratch-origin transition world: a scratch origin byte (3) is out of
/// range for a pre-14 body all on its own (`an_origin_byte_of_3_in_a_pre_14_
/// config_is_out_of_range` covers that failure), which would reach a
/// different error before the config body's length is ever checked. With
/// the origin held constant, a genuine format-14 file with its header
/// relabelled 13 fails on the config body's trailing bytes before
/// `decode_payload` ever reaches section 21, because format 14's config
/// block is longer than format 13's. The `SectionNotInFormat` arm is
/// reached only by a second, spliced construction: a genuine format-13
/// file with a real `SECTION_TRANSITION` chunk (cut from a genuine
/// transition world) appended, on the pattern `format7.rs`'s
/// `a_format_6_file_carrying_the_objects_section_is_refused` establishes
/// for the analogous format-6/format-7 case.
#[test]
fn a_section_21_in_a_file_stamped_format_13_is_refused_by_name() {
    let plain_world = advance(SimConfig::phase2_default(SEED), 40);
    let plain_state = plain_world.export_state();
    assert_eq!(plain_state.config.origin.mode, OriginMode::Random);
    assert!(plain_state.transition.is_none());
    let checksum = plain_world.state_checksum();

    let fourteen = encode_snapshot(
        &plain_state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 14");

    let relabelled = relabel(&fourteen, FORMAT_VERSION_13);
    assert_eq!(
        decode_snapshot_format13(&relabelled).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format-14 body read as format 13 must fail on the config section's \
         trailing bytes before it ever reaches section 21"
    );

    let thirteen = encode_snapshot_format13(
        &plain_state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 13");

    let transition_bytes = encode_snapshot(
        &transition_world(SEED).export_state(),
        1,
        0,
        0,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode a genuine transition world at format 14");
    let transition_chunk = raw_section(&transition_bytes, SECTION_TRANSITION);
    let forged = append_section_bytes(&thirteen, &transition_chunk);
    assert_eq!(
        decode_snapshot_format13(&forged).err(),
        Some(CodecError::SectionNotInFormat {
            tag: SECTION_TRANSITION,
            format: FORMAT_VERSION_13,
        }),
        "a genuine format-13 file forged to carry section 21 must be refused by name"
    );
}

// --- (f) an origin byte of 3 in a pre-14 config is out of range -------------

/// The origin-mode byte's offset inside the format-13 config body is found
/// by diffing two encodings that differ *only* in `origin.mode` - `Random`
/// (1) versus `Seeded` (2) - rather than hand-counted from `encode_config`,
/// so this test cannot silently drift the day a field is inserted upstream
/// of it. `Seeded`'s own preconditions (`archetype_count`, `climate.enabled`)
/// are never checked here: `encode_snapshot_format13` writes bytes, it does
/// not validate, and only the byte at the located offset is used.
#[test]
fn an_origin_byte_of_3_in_a_pre_14_config_is_out_of_range() {
    let world = advance(SimConfig::phase2_default(SEED), 40);
    let random_state = world.export_state();
    assert_eq!(random_state.config.origin.mode, OriginMode::Random);
    let mut seeded_state = random_state.clone();
    seeded_state.config.origin.mode = OriginMode::Seeded;

    let random_bytes = encode_snapshot_format13(
        &random_state,
        1,
        0,
        0,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 13 (random origin)");
    let seeded_bytes = encode_snapshot_format13(
        &seeded_state,
        1,
        0,
        0,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 13 (seeded origin byte, otherwise unvalidated)");

    let (random_start, random_len) = section(&random_bytes, SECTION_CONFIG);
    let (seeded_start, seeded_len) = section(&seeded_bytes, SECTION_CONFIG);
    assert_eq!(
        random_len, seeded_len,
        "origin.mode alone must not change the config body's length"
    );
    let differing: Vec<usize> = (0..random_len)
        .filter(|&index| random_bytes[random_start + index] != seeded_bytes[seeded_start + index])
        .collect();
    assert_eq!(
        differing.len(),
        1,
        "origin.mode must be the only byte the two config bodies differ in"
    );
    let origin_byte_offset = random_start + differing[0];
    assert_eq!(
        random_bytes[origin_byte_offset], 1,
        "the located byte must be the origin-mode discriminant"
    );

    let mut patched = random_bytes.clone();
    patched[origin_byte_offset] = 3;
    let (body_start, body_len) = section(&patched, SECTION_CONFIG);
    reseal(&mut patched, body_start, body_len);

    assert_eq!(
        decode_snapshot_format13(&patched).err(),
        Some(CodecError::ValueOutOfRange("origin_mode")),
        "byte value 3 must be out of range for a format that predates scratch origin"
    );
}

// --- (g) a transition world survives a save round trip with the same future --

#[test]
fn a_transition_world_survives_a_save_round_trip_with_the_same_future() {
    let mut world = transition_world(SEED);
    for _ in 0..50 {
        world.step();
    }
    world.check_invariants().expect("invariants hold");

    let state = world.export_state();
    let checksum = world.state_checksum();
    let bytes = encode_snapshot(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 14");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 14");

    let mut restored = World::from_state(decoded).expect("restore the transition world");
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
