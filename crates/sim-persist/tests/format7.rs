//! ALIF format 7: the Phase 12 artifact config block (ADR-0028 section 13),
//! `genome2.mutation.binding_q16`, the schema-2 `counters.binding_applied`
//! word, `SECTION_OBJECTS`, the retained format-6 reader and writer, and the
//! registered 6-to-7 migration.
//!
//! # The chain test moved here, and "one byte" became "one row's declared
//! # delta"
//!
//! Format 5 and format 6 each reserved exactly one config byte, and
//! `format6.rs` used to carry a chain test asserting every adjacent pair
//! differed by exactly one byte. D-112 said, in advance, that a config-block
//! change which is **not** a single appended byte "would need its own
//! reasoning" - this is that format, and this is that reasoning: format 7
//! appends the whole artifact block in one piece (D-115/ADR-0028 section 13),
//! so the format-6 body stays a byte *prefix* of the format-7 body exactly as
//! every earlier pair did, but the appended amount is `FORMAT7_CONFIG_BYTES`
//! (thirty-one `ArtifactConfig` fields, then `binding_q16`), not one.
//! `each_adjacent_format_extends_its_predecessor_by_exactly_the_declared_bytes`
//! generalises the old chain test to a per-row byte delta instead of special-
//! casing this format inside a file named for format 6.
//!
//! # The trap this file exists not to repeat a fifth time
//!
//! D-108 (twice, in `phase12_format4.rs`), D-112 (`format5.rs`'s prefix
//! test), and `format6.rs`'s own history (see its module doc, fixed
//! alongside this file) all record the same defect: a test named for a
//! format that builds its subject with the *current*-format writer, which is
//! the same thing only until the next bump. Every test below that means
//! "format 6" or "format 5" or "format 4" specifically calls
//! `encode_snapshot_format6` / `decode_snapshot_format6` or the equivalent
//! format-5/format-4 functions; the bare `encode_snapshot` / `decode_snapshot`
//! / `FORMAT_VERSION` appear only where the test genuinely means "whatever is
//! current", and say so.

use sim_core::{
    ArtifactConfig, CAUSE_COMBINED, CAUSE_EXTRACTED, MATERIAL_STONE, ObjectRecord, ObjectTable,
    SaveState, SimConfig, World, material,
};
use sim_persist::{
    CodecError, FORMAT7_CONFIG_BYTES, FORMAT_VERSION, FORMAT_VERSION_4, FORMAT_VERSION_5,
    FORMAT_VERSION_6, FORMAT_VERSION_7, StoreError, decode_snapshot, decode_snapshot_format6,
    encode_snapshot, encode_snapshot_format4, encode_snapshot_format5, encode_snapshot_format6,
    migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// A second seed, for fixtures built alongside a `SEED` one in the same test
/// so the two worlds are not accidental duplicates of each other.
const CORRUPTION_SEED: u64 = 0x0b5e_e7c0_de5e_ed00;
/// Header offsets, from the fixed 112-byte layout in `codec.rs`.
const OFFSET_FORMAT: usize = 4;
const OFFSET_UNCOMPRESSED_LEN: usize = 68;
const OFFSET_STORED_LEN: usize = 76;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;
const SECTION_CONFIG: u16 = 1;
/// Phase 12 object table (ADR-0028 section 13). Matches `codec.rs`'s private
/// `SECTION_OBJECTS`; declared again here because a test file asserts against
/// the wire format from the outside; see `phase12_format4.rs`'s `13` for the
/// established pattern of a test file naming a section tag by number.
const SECTION_OBJECTS: u16 = 15;

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

/// A config with `genome2` enabled and `artifact` disabled: the base every
/// format-6-writer-refuses test in this file mutates away from, so the
/// `schema2` section exists to carry `counters.binding_applied` without the
/// artifact section itself being on.
fn genome2_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.genome2.enabled = true;
    config
}

/// A config with `genome2`, `worldmod`, and `artifact` all enabled -
/// `artifact.enabled` requires the first two (`config.rs`'s
/// `validate_subsystems`) - on a small map so the tests that step it and the
/// corruption sweep that encodes it 20,000 times stay fast.
fn artifact_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 32;
    config.cells_y = 32;
    config.initial_organisms = 20;
    config.max_entities = 200;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.artifact.enabled = true;
    config
}

/// Offsets of every section, walked from the payload start rather than
/// searched for, so this cannot match a tag-shaped value inside a genome.
/// Returns `(tag, flags, body_start, body_len)`.
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

/// One section's whole wire chunk - tag, flags, length, body, and CRC -
/// cut out verbatim so it can be spliced into a different file's payload.
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
/// `phase12_format4.rs`'s helper of the same name.
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
/// fixing the two payload-length header words and the payload CRC - the
/// splice `a_format_6_file_carrying_the_objects_section_is_refused` uses to
/// build a forgery no ordinary writer could produce.
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

type Writer = fn(&SaveState, u64, u64, u64, &str, u64, Option<i32>) -> Result<Vec<u8>, CodecError>;

// --- the chain property, generalised to a per-row byte delta -----------------

/// Each format's config body is its predecessor's plus exactly its declared
/// delta, and the predecessor's body is a byte prefix of it. Format 7's row
/// declares `FORMAT7_CONFIG_BYTES` rather than `1`; every earlier row still
/// declares `1`, so this is a strict generalisation of the property
/// `format6.rs` used to assert, not a different one.
#[test]
fn each_adjacent_format_extends_its_predecessor_by_exactly_the_declared_bytes() {
    let world = advance(SimConfig::phase2_default(SEED), 300);
    let state = world.export_state();
    let checksum = world.state_checksum();

    // Format 3 is deliberately absent, on the same grounds `format6.rs`'s
    // chain gave: it differs from format 4 by a section and a logical-state
    // version, not by a config byte, and `phase12_format4.rs` owns that
    // comparison.
    let chain: Vec<(u16, Writer, usize)> = vec![
        (FORMAT_VERSION_4, encode_snapshot_format4 as Writer, 0),
        (FORMAT_VERSION_5, encode_snapshot_format5 as Writer, 1),
        (FORMAT_VERSION_6, encode_snapshot_format6 as Writer, 1),
        (FORMAT_VERSION_7, encode_snapshot as Writer, FORMAT7_CONFIG_BYTES),
    ];

    let encoded: Vec<(u16, Vec<u8>, usize)> = chain
        .iter()
        .map(|(format, write, delta)| {
            let bytes = write(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
                .unwrap_or_else(|error| panic!("format {format} writer: {error:?}"));
            assert_eq!(
                u16::from_le_bytes(bytes[OFFSET_FORMAT..OFFSET_FORMAT + 2].try_into().unwrap()),
                *format,
                "the format {format} writer stamped a different version"
            );
            (*format, bytes, *delta)
        })
        .collect();

    for pair in encoded.windows(2) {
        let (older_version, older, _) = &pair[0];
        let (newer_version, newer, delta) = &pair[1];
        let (older_start, older_len) = section(older, SECTION_CONFIG);
        let (newer_start, newer_len) = section(newer, SECTION_CONFIG);

        assert_eq!(
            newer_len,
            older_len + delta,
            "format {newer_version}'s config body must be exactly {delta} bytes \
             longer than format {older_version}'s"
        );
        assert_eq!(
            &newer[newer_start..newer_start + older_len],
            &older[older_start..older_start + older_len],
            "format {older_version}'s config body is not a prefix of format \
             {newer_version}'s, so the two formats differ somewhere other than \
             the appended bytes"
        );
        assert_eq!(
            newer.len(),
            older.len() + delta,
            "format {newer_version} must cost exactly {delta} bytes over format \
             {older_version} for the same world"
        );
    }

    // The 6-to-7 pair, additionally: the appended block round-trips to the
    // defaults. A default phase2 world has no artifact section and
    // `binding_q16 == 0`, so decoding the format-7 file must reproduce
    // exactly the config the format-6 reader already produces for the shared
    // prefix - i.e. every appended byte decodes back to its field's default.
    let (six_version, six_bytes, _) = &encoded[2];
    let (seven_version, seven_bytes, _) = &encoded[3];
    assert_eq!(*six_version, FORMAT_VERSION_6);
    assert_eq!(*seven_version, FORMAT_VERSION_7);
    let (_, six_state) = decode_snapshot_format6(six_bytes).expect("decode format 6");
    let (_, seven_state) = decode_snapshot(seven_bytes).expect("decode format 7");
    assert_eq!(
        seven_state.config, six_state.config,
        "the appended artifact block did not round-trip to its defaults"
    );
}

// --- the retained format-6 reader and the migration ---------------------------

/// **Genuinely a "current" test**, unlike the trap this module doc describes:
/// format 6 is not current today, so naming the reader that refuses it
/// `decode_snapshot` / `FORMAT_VERSION` is truthful now and will say so again
/// the day format 8 makes format 7 the retained one instead.
#[test]
fn the_current_reader_refuses_a_format_6_file() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let legacy = encode_snapshot_format6(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 6");
    assert_eq!(
        decode_snapshot(&legacy).err(),
        Some(CodecError::UnsupportedFormat(FORMAT_VERSION_6))
    );
    assert!(
        decode_snapshot_format6(&legacy).is_ok(),
        "the retained format-6 reader must still accept its own file"
    );
    let migration = migration_for(FORMAT_VERSION_6)
        .expect("format 6 is registered")
        .expect("format 6 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_6);
    assert_eq!(migration.to_format, FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 6 to 7 transform invents nothing and must not claim to"
    );
}

/// A migrated format-6 file equals a native format-6 load: as `SaveState`,
/// then as a world, then over 200 further ticks. Built on a genome2-enabled
/// world so the schema-2 section is exercised - format 7 appends
/// `counters.binding_applied` to it, and a world with no schema-2 section
/// would leave that append entirely untested.
#[test]
fn the_format_6_migration_is_byte_identical_to_a_format_6_load() {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    config.genome2.enabled = true;
    let world = advance(config, 400);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert!(
        state.schema2.is_some(),
        "a world with no schema2 section does not exercise the counter this \
         format appends"
    );

    let legacy =
        encode_snapshot_format6(&state, 11, 5, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 6");

    let migration = migration_for(FORMAT_VERSION_6)
        .expect("format 6 is registered")
        .expect("format 6 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_6);
    assert_eq!(migration.to_format, FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 6 to 7 transform invents nothing and must not claim to"
    );

    let migrated = (migration.transform)(&legacy).expect("the transform runs");
    assert_eq!(migrated.source.format_version, FORMAT_VERSION_6);
    assert_eq!(migrated.source.world_id, 11);

    let (legacy_info, legacy_state) = decode_snapshot_format6(&legacy).expect("legacy decode");
    let (migrated_info, migrated_state) =
        decode_snapshot(&migrated.bytes).expect("the migrated file decodes at format 7");
    assert_eq!(migrated_info.format_version, FORMAT_VERSION);
    assert_eq!(migrated_state, legacy_state, "the migrated state differs");
    assert_eq!(
        migrated_state, state,
        "neither path reproduced the original"
    );
    assert!(
        !migrated_state.config.artifact.enabled,
        "a file that predates objects must migrate with the section off"
    );
    assert_eq!(migrated_state.config.genome2.mutation.binding_q16, 0);
    assert_eq!(
        migrated_state
            .schema2
            .as_ref()
            .expect("schema2 survives the migration")
            .counters
            .binding_applied,
        0
    );
    assert!(migrated_state.objects.is_none());
    assert_eq!(legacy_info.config_hash, migrated_info.config_hash);
    assert_eq!(legacy_info.state_checksum, migrated_info.state_checksum);

    let mut from_legacy = World::from_state(legacy_state).expect("restore legacy");
    let mut from_migrated = World::from_state(migrated_state).expect("restore migrated");
    assert_eq!(from_legacy.export_state(), from_migrated.export_state());
    for _ in 0..200 {
        from_legacy.step();
        from_migrated.step();
    }
    assert_eq!(
        from_legacy.state_checksum(),
        from_migrated.state_checksum(),
        "a migrated world diverged from the world a format 6 load produces"
    );
}

/// Both directions of a forged version word, on the body rather than the
/// header. Built with `encode_snapshot_format6` / `encode_snapshot`
/// explicitly, on the same grounds `format6.rs`'s fixed version of this test
/// gives: reading either as "current" would silently drift the day format 8
/// lands.
#[test]
fn a_version_word_that_disagrees_with_the_body_is_refused_both_ways() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let seven = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 7");
    let six = encode_snapshot_format6(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 6");

    // A format-7 body has `FORMAT7_CONFIG_BYTES` more than a format-6 reader
    // expects; `decode_config` at format 6 stops after `price_moved_edges_only`
    // and the trailing-bytes check every section runs catches the rest -
    // still `ValueOutOfRange("section trailing bytes")`, the same variant
    // format6.rs's one-byte version of this test observed, just with more
    // bytes left over.
    assert_eq!(
        decode_snapshot_format6(&relabel(&seven, FORMAT_VERSION_6)).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format 7 payload read as format 6 must fail on the appended artifact block"
    );
    // A format-6 body read as format 7 runs out of bytes inside
    // `decode_artifact_config`, which reads far more fields than a format-6
    // body has left - `TruncatedSection`.
    assert_eq!(
        decode_snapshot(&relabel(&six, FORMAT_VERSION_7)).err(),
        Some(CodecError::TruncatedSection),
        "a format 6 payload read as format 7 must run out of config body inside \
         the appended artifact block"
    );
}

/// The format-6 writer refuses a state carrying anything format 7 added -
/// each refused by its own named field or section, never merged into one
/// bare `is_err()` - and the `artifact.enabled` refusal is also checked on
/// the format-5 and format-4 writers with their own format numbers, since
/// `refuse_format7_state` in `codec.rs` is shared by all three retained
/// pre-7 writers.
#[test]
fn the_format_6_writer_refuses_a_state_carrying_a_format_7_field() {
    let world = advance(genome2_config(SEED), 100);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert!(
        state.schema2.is_some(),
        "a schema2 world is needed to set schema2.counters.binding_applied"
    );

    // The base state itself, untouched, must still encode at format 6 - or
    // every refusal below could be explained by something that has nothing
    // to do with the field it names.
    assert!(
        encode_snapshot_format6(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .is_ok(),
        "the unmodified state must still be expressible at format 6"
    );

    let mut artifact_enabled = state.clone();
    artifact_enabled.config.artifact.enabled = true;
    assert_eq!(
        encode_snapshot_format6(
            &artifact_enabled,
            1,
            0,
            checksum,
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "artifact.enabled",
            format: FORMAT_VERSION_6,
        })
    );
    assert_eq!(
        encode_snapshot_format5(
            &artifact_enabled,
            1,
            0,
            checksum,
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "artifact.enabled",
            format: FORMAT_VERSION_5,
        })
    );
    assert_eq!(
        encode_snapshot_format4(
            &artifact_enabled,
            1,
            0,
            checksum,
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "artifact.enabled",
            format: FORMAT_VERSION_4,
        })
    );

    let mut binding = state.clone();
    binding.config.genome2.mutation.binding_q16 = 1;
    assert_eq!(
        encode_snapshot_format6(&binding, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .err(),
        Some(CodecError::FieldNotInFormat {
            field: "genome2.mutation.binding_q16",
            format: FORMAT_VERSION_6,
        })
    );

    let mut applied = state.clone();
    applied.schema2.as_mut().expect("schema2 present").counters.binding_applied = 1;
    assert_eq!(
        encode_snapshot_format6(&applied, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .err(),
        Some(CodecError::FieldNotInFormat {
            field: "schema2.counters.binding_applied",
            format: FORMAT_VERSION_6,
        })
    );

    let mut objects = state.clone();
    objects.objects = Some(ObjectTable::default());
    assert_eq!(
        encode_snapshot_format6(&objects, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .err(),
        Some(CodecError::SectionNotInFormat {
            tag: SECTION_OBJECTS,
            format: FORMAT_VERSION_6,
        })
    );
}

// --- the artifact config block ------------------------------------------------

/// Every `ArtifactConfig` field taken away from its default, plus
/// `binding_q16`, survives the format-7 round trip.
///
/// Values are chosen to satisfy `validate_subsystems`'s artifact block in
/// `sim-core/src/config.rs`: `max_fragments` in `2..=16` and at most
/// `max_objects`; `joint_floor_q16 <= 65_536`; `wood_relative_q16 <=
/// stone_relative_q16 <= 65_536`; `action_threshold_q16` in `[-65_536,
/// 65_536]`; every listed cap nonzero; `max_composition_breadth >= 2`;
/// `max_composition_depth <= 16`.
#[test]
fn the_artifact_config_survives_the_format_7_round_trip() {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 32;
    config.cells_y = 32;
    config.initial_organisms = 20;
    config.max_entities = 200;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.artifact = ArtifactConfig {
        enabled: true,
        inert: true,
        ephemeral: true,
        max_objects: 500,
        max_objects_per_cell: 5,
        max_composition_depth: 6,
        max_composition_breadth: 5,
        max_held_objects: 3,
        max_candidates: 6,
        carry_capacity_milli: 5_000,
        carry_move_cost_q16: 20_000,
        hold_cost_milli_per_s: 50,
        action_cost_milli: 90,
        strike_cost_milli: 150,
        action_threshold_q16: -10_000,
        reach_m: 3,
        consume_reach_m: 4,
        perception_range_m: 10,
        strike_force_q16: 300_000,
        strike_mass_reference_milli: 2_500,
        fracture_margin_q16: 70_000,
        max_fragments: 7,
        min_fragment_mass_milli: 450,
        joint_floor_q16: 20_000,
        blocking_mass_milli: 3_500,
        terrain_yield_milli: 6_500,
        extraction_milli: 850,
        yield_regen_milli: 450,
        yield_regen_interval_ticks: 700,
        stone_relative_q16: 40_000,
        wood_relative_q16: 20_000,
    };
    config.genome2.mutation.binding_q16 = 655;
    config
        .validate()
        .expect("the chosen values satisfy validate_subsystems");

    let world = advance(config, 10);
    let state = world.export_state();
    let bytes = encode_snapshot(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 7");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 7");

    assert_eq!(
        decoded.config.artifact, state.config.artifact,
        "the artifact block did not survive the round trip field for field"
    );
    assert_eq!(decoded.config.genome2.mutation.binding_q16, 655);
    assert_eq!(decoded.config, state.config);
}

// --- the object table -----------------------------------------------------

/// Push a depth-one composite (two owned constituents) and a held simple
/// object into `state.objects`, keeping the allocation identity
/// (`initial + births + objects_allocated + 1 == next_entity_id`) and the
/// mass/energy ledger exact so `ObjectTable::violation` reports none.
///
/// All four objects are `MATERIAL_STONE`, whose `energy_content_milli` is
/// zero, so every ledgered energy term below is zero too - not omitted, just
/// trivially satisfied.
fn push_sample_objects(state: &mut SaveState, tick: u64) {
    assert!(
        !state.ids.is_empty(),
        "the population died out before objects were added"
    );
    let (x_fp, y_fp) = (state.x_fp[0], state.y_fp[0]);
    let holder = state.ids[0];
    let base = state.next_entity_id;
    let stone = material(MATERIAL_STONE).expect("stone is a registered material");

    let a = ObjectRecord::simple(base, stone, 800, x_fp, y_fp, tick, CAUSE_EXTRACTED, 0);
    let b = ObjectRecord::simple(base + 1, stone, 800, x_fp, y_fp, tick, CAUSE_EXTRACTED, 0);
    let held = ObjectRecord::simple(base + 3, stone, 800, x_fp, y_fp, tick, CAUSE_EXTRACTED, 0);

    let table = state.objects.as_mut().expect("artifact world carries a table");
    for extracted in [&a, &b, &held] {
        table.ledger.mass_extracted_milli += i128::from(extracted.mass_milli);
        table.ledger.energy_extracted_milli += i128::from(extracted.energy_milli);
    }

    let mass = a.mass_milli + b.mass_milli;
    let energy = a.energy_milli + b.energy_milli;
    let hardness = a.hardness_q16.max(b.hardness_q16);
    let durability = a.durability_q16.min(b.durability_q16);
    let decay = a.decay_q16.max(b.decay_q16);
    let material_id = a.material_id;

    table.push(a);
    table.push(b);
    let composite_id = base + 2;
    table.push(ObjectRecord {
        id: composite_id,
        material_id,
        x_fp,
        y_fp,
        integrity_q16: sim_core::INTEGRITY_WHOLE_Q16,
        mass_milli: mass,
        energy_milli: energy,
        hardness_q16: hardness,
        durability_q16: durability,
        decay_q16: decay,
        holder_id: 0,
        owner_id: 0,
        depth: 1,
        created_tick: tick,
        creator_id: 0,
        cause: CAUSE_COMBINED,
        parent_id: 0,
        composition: vec![base, base + 1],
    });
    let index_a = table.index_of(base).expect("a was just pushed");
    let index_b = table.index_of(base + 1).expect("b was just pushed");
    table.owner_id[index_a] = composite_id;
    table.owner_id[index_b] = composite_id;

    table.push(held);
    let held_index = table.index_of(base + 3).expect("held was just pushed");
    table.holder_id[held_index] = holder;

    table.objects_allocated_total += 4;
    state.next_entity_id += 4;

    let max_depth = state.config.artifact.max_composition_depth.min(255) as u8;
    assert_eq!(
        state.objects.as_ref().unwrap().violation(max_depth),
        None,
        "the constructed object table is not internally consistent"
    );
}

/// The object table round-trips through the codec and a restore, and a world
/// restored through the codec stays bit-identical to one restored directly
/// from the same state over 200 further ticks. The section is present when
/// artifact is enabled and absent when it is not.
#[test]
fn an_object_table_with_a_held_object_and_a_depth_one_composite_round_trips_and_steps() {
    let world = advance(artifact_config(SEED), 50);
    let mut state = world.export_state();
    assert!(
        state.objects.is_some(),
        "an artifact world must carry an object table"
    );
    let tick = state.tick;
    push_sample_objects(&mut state, tick);

    let direct = World::from_state(state.clone()).expect("the constructed state restores");
    let checksum = direct.state_checksum();

    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 7");
    let (_, body_len) = section(&bytes, SECTION_OBJECTS);
    assert!(body_len > 0, "the objects section must not be empty");

    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 7");
    assert_eq!(
        decoded, state,
        "encoding then decoding the object table changed the state"
    );

    let via_codec = World::from_state(decoded).expect("restore from the decoded state");
    assert_eq!(
        via_codec.export_state(),
        state,
        "restoring the decoded state did not reproduce the constructed state"
    );

    let mut direct = direct;
    let mut via_codec = via_codec;
    assert_eq!(direct.export_state(), via_codec.export_state());
    for _ in 0..200 {
        direct.step();
        via_codec.step();
    }
    assert_eq!(
        direct.state_checksum(),
        via_codec.state_checksum(),
        "a world restored through the codec diverged from one restored directly \
         from the same state"
    );

    // Absent when artifact is disabled.
    let plain = advance(SimConfig::phase2_default(SEED), 20);
    let plain_bytes = encode_snapshot(
        &plain.export_state(),
        1,
        0,
        plain.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 7");
    assert!(
        sections(&plain_bytes)
            .iter()
            .all(|(tag, _, _, _)| *tag != SECTION_OBJECTS),
        "a disabled world must not carry the objects section"
    );
}

/// A format-6 file forged to carry `SECTION_OBJECTS` is refused, and so is
/// its migration.
///
/// **Built by splicing, not by relabelling a format-7 file's header.** A
/// format-7 file's config body already has `FORMAT7_CONFIG_BYTES` more than a
/// format-6 reader expects, so relabelling one as format 6 fails on the
/// config section's trailing bytes - `a_version_word_that_disagrees_with_the_
/// body_is_refused_both_ways` above already covers that path and it would
/// never reach section 15 at all. The forgery here is a genuine, native
/// format-6 file (a world that has never had an artifact section) with a
/// real `SECTION_OBJECTS` chunk - cut whole out of a genuine format-7 file -
/// appended to its payload, on the pattern `phase12_format4.rs`'s
/// `a_format_3_file_carrying_a_format_4_section_is_refused` uses for the
/// analogous format-3/format-4 case.
#[test]
fn a_format_6_file_carrying_the_objects_section_is_refused() {
    let plain = advance(SimConfig::phase2_default(SEED), 60);
    let plain_state = plain.export_state();
    assert!(plain_state.objects.is_none());
    let six = encode_snapshot_format6(
        &plain_state,
        1,
        0,
        plain.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 6");

    let artifact_world = advance(artifact_config(SEED), 40);
    let mut artifact_state = artifact_world.export_state();
    let artifact_tick = artifact_state.tick;
    push_sample_objects(&mut artifact_state, artifact_tick);
    let artifact_checksum = World::from_state(artifact_state.clone())
        .expect("the constructed artifact state restores")
        .state_checksum();
    let seven = encode_snapshot(
        &artifact_state,
        1,
        0,
        artifact_checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 7");
    let objects_chunk = raw_section(&seven, SECTION_OBJECTS);

    let forged = append_section_bytes(&six, &objects_chunk);
    assert_eq!(
        decode_snapshot_format6(&forged).err(),
        Some(CodecError::SectionNotInFormat {
            tag: SECTION_OBJECTS,
            format: FORMAT_VERSION_6,
        })
    );

    let migration = migration_for(FORMAT_VERSION_6)
        .expect("registered")
        .expect("some");
    match (migration.transform)(&forged) {
        Err(StoreError::Codec(error)) => assert_eq!(
            error,
            CodecError::SectionNotInFormat {
                tag: SECTION_OBJECTS,
                format: FORMAT_VERSION_6,
            },
            "the transform's own decode must fail the same way ours did"
        ),
        other => panic!(
            "the transform accepted a file that lies about its own version: {other:?}"
        ),
    }
}

/// Bytes one object's fixed fields occupy, up to but not including its
/// `composition.len()` word: id, material_id, x_fp, y_fp, integrity_q16,
/// mass_milli, energy_milli, hardness_q16, durability_q16, decay_q16,
/// holder_id, owner_id, depth, created_tick, creator_id, cause, parent_id.
/// Mirrors `codec.rs`'s private `OBJECT_FIXED_BYTES` minus the
/// composition-length word, whose own offset `object_layout` returns
/// separately below.
const OBJECT_PREFIX_BYTES: usize = 8 + 2 + 4 + 4 + 4 + 8 + 8 + 4 + 4 + 4 + 8 + 8 + 1 + 8 + 8 + 1 + 8;

/// The object count word's offset, and every object's `composition.len()`
/// word offset, walked the way `decode_objects` reads them rather than
/// assumed - so the offsets patched below track the encoder's layout exactly
/// and the walk itself is a check that this constant agrees with it.
fn object_layout(bytes: &[u8], body_start: usize, body_len: usize) -> (usize, Vec<usize>, usize) {
    let count_offset = body_start;
    let count = u64::from_le_bytes(bytes[count_offset..count_offset + 8].try_into().unwrap());
    let mut offset = body_start + 8;
    let mut breadth_offsets = Vec::new();
    for _ in 0..count {
        offset += OBJECT_PREFIX_BYTES;
        breadth_offsets.push(offset);
        let breadth = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8 + breadth as usize * 8;
    }
    // After the table proper: objects_allocated_total (8), the ledger (10
    // i128 terms), the counters (30 u64 terms), then the per-organism
    // observation rows behind their own count word (8 + 8 + 1 each).
    let trailer = 8 + 10 * 16 + 30 * 8;
    let rows_offset = offset + trailer;
    let rows = u64::from_le_bytes(bytes[rows_offset..rows_offset + 8].try_into().unwrap());
    assert_eq!(
        rows_offset + 8 + rows as usize * 17,
        body_start + body_len,
        "the layout walk disagrees with the encoder, so the offsets found above \
         are not the counts (objects_allocated_total: 8, ledger: 10 i128 terms, \
         counters: 30 u64 terms, observation rows: 8 + 17 each)"
    );
    (count_offset, breadth_offsets, rows_offset)
}

/// Every declared count in the objects section is bounded before allocation:
/// the object count word and one object's composition-length word, each
/// patched to `u64::MAX`, `u64::MAX / 8`, and the section body length, with
/// every CRC resealed so the value is *reached* rather than rejected as
/// corruption (standing rule 2).
#[test]
fn every_declared_count_in_the_objects_section_is_bounded() {
    let world = advance(artifact_config(SEED), 30);
    let mut state = world.export_state();
    let tick = state.tick;
    push_sample_objects(&mut state, tick);

    let checksum = World::from_state(state.clone())
        .expect("the constructed state restores")
        .state_checksum();
    let bytes = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 7");
    let (body_start, body_len) = section(&bytes, SECTION_OBJECTS);
    assert!(
        decode_snapshot(&bytes).is_ok(),
        "the unpatched snapshot must decode, or every refusal below is vacuous"
    );

    let (count_offset, breadth_offsets, rows_offset) = object_layout(&bytes, body_start, body_len);
    assert_eq!(
        breadth_offsets.len(),
        4,
        "the layout walk did not find all four pushed objects (a, b, composite, held)"
    );
    // The composite is the third object pushed: a, b, composite, held.
    let composite_breadth_offset = breadth_offsets[2];

    let probe = |offset: usize, expected: CodecError| {
        // **All three probed values land on the same named error, unlike the
        // config section's declared length in `format5.rs`.** That bound is
        // checked by a ladder - `MAX_SECTION_LEN`, then an overflow check,
        // then a truncation check, then a CRC - so a value below the top
        // bound but past the file's end is caught by a *different*, weaker
        // rung, which is what makes asserting the exact error meaningful
        // there. `decode_objects` has exactly one guard per declared count -
        // `allocation_fits` - so there is no weaker rung underneath it to
        // mask its removal: a declared count equal to the body length still
        // multiplies out (every object costs far more than one byte) to more
        // bytes than the body holds, so `body_len` fails the identical check
        // `u64::MAX` and `u64::MAX / 8` do.
        for declared in [u64::MAX, u64::MAX / 8, body_len as u64] {
            let mut patched = bytes.clone();
            patched[offset..offset + 8].copy_from_slice(&declared.to_le_bytes());
            reseal(&mut patched, body_start, body_len);
            assert_eq!(
                decode_snapshot(&patched).err(),
                Some(expected.clone()),
                "declared count {declared} at offset {offset} was not refused by \
                 the bound that names it"
            );
        }
    };
    probe(count_offset, CodecError::ValueOutOfRange("object count"));
    probe(
        composite_breadth_offset,
        CodecError::ValueOutOfRange("object composition length"),
    );
    probe(rows_offset, CodecError::ValueOutOfRange("object observation rows"));
}

/// Push three simple `MATERIAL_STONE` objects with no composite and no held
/// object - deliberately simpler than `push_sample_objects`, so the 20,000-
/// iteration sweep below stays fast.
fn push_three_simple_objects(state: &mut SaveState, tick: u64) {
    assert!(
        !state.ids.is_empty(),
        "the population died out before objects were added"
    );
    let (x_fp, y_fp) = (state.x_fp[0], state.y_fp[0]);
    let base = state.next_entity_id;
    let stone = material(MATERIAL_STONE).expect("stone is a registered material");
    let table = state.objects.as_mut().expect("artifact world carries a table");
    for offset in 0..3_u64 {
        let record =
            ObjectRecord::simple(base + offset, stone, 800, x_fp, y_fp, tick, CAUSE_EXTRACTED, 0);
        table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
        table.ledger.energy_extracted_milli += i128::from(record.energy_milli);
        table.push(record);
    }
    table.objects_allocated_total += 3;
    state.next_entity_id += 3;

    let max_depth = state.config.artifact.max_composition_depth.min(255) as u8;
    assert_eq!(
        state.objects.as_ref().unwrap().violation(max_depth),
        None,
        "the constructed object table is not internally consistent"
    );
}

/// **Twenty thousand seeded corruptions of the objects section, one to three
/// bits flipped each time, every one of them resealed, and not one produces
/// a world that claims to be the saved one.**
///
/// The pattern is `phase12_format4.rs`'s
/// `twenty_thousand_corruptions_of_the_modification_section_never_pass_as_the_original`,
/// aimed at `SECTION_OBJECTS` instead of the modification section: the flips
/// land inside the section body and both checksums are recomputed afterward,
/// so the bytes reach the parser, the declared-count bounds, the ascending-id
/// check, `ObjectTable::violation`'s domain and ledger checks, and the state
/// checksum in turn. Zero panics is asserted by the test completing (D-091).
#[test]
fn twenty_thousand_corruptions_of_the_objects_section_never_pass_as_the_original() {
    let world = advance(artifact_config(SEED), 30);
    let mut state = world.export_state();
    let tick = state.tick;
    push_three_simple_objects(&mut state, tick);

    let checksum = World::from_state(state.clone())
        .expect("the constructed state restores")
        .state_checksum();
    let valid = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 7");
    let (body_start, body_len) = section(&valid, SECTION_OBJECTS);
    assert!(body_len > 0);

    let mut rng = CORRUPTION_SEED;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut decode_refused = 0_u32;
    let mut restore_refused = 0_u32;
    let mut checksum_caught = 0_u32;
    let mut identical = 0_u32;
    for _ in 0..20_000 {
        let mut bytes = valid.clone();
        for _ in 0..1 + next() % 3 {
            let position = body_start + (next() % body_len as u64) as usize;
            bytes[position] ^= 1 << (next() % 8);
        }
        if bytes == valid {
            // An even number of flips of the same bit. Not a corruption at
            // all, counted so the tallies below add up rather than being
            // quietly absorbed.
            identical += 1;
            continue;
        }
        reseal(&mut bytes, body_start, body_len);
        let Ok((info, decoded)) = decode_snapshot(&bytes) else {
            decode_refused += 1;
            continue;
        };
        let Ok(restored) = World::from_state(decoded) else {
            restore_refused += 1;
            continue;
        };
        assert_ne!(
            restored.state_checksum(),
            info.state_checksum,
            "a corrupted objects section restored to a world that passes as the \
             one that was saved"
        );
        checksum_caught += 1;
    }
    assert!(decode_refused > 0, "no corruption reached a decode bound");
    assert!(
        restore_refused > 0,
        "no corruption reached ObjectTable::violation or the allocation identity"
    );
    assert!(
        checksum_caught > 0,
        "no corruption survived to the state checksum, so that clause is untested"
    );
    println!(
        "objects corruption sweep: decode refused {decode_refused}, restore \
         refused {restore_refused}, caught by state checksum {checksum_caught}, \
         no-op {identical}"
    );
}

/// **A table of two hundred simple objects decodes.** The first draft of the
/// object-count bound double-counted the composition-length word - the
/// per-item minimum was 108 against a real 100 - so a table's own trailer
/// (408 bytes) covered the overcount only up to fifty objects and every
/// larger legitimate table was refused as `ValueOutOfRange("object count")`.
/// The corruption sweep and the three-object round trip could not see it.
/// This is the test that would have: it is exactly the D-075 rule stated as
/// a positive case - the bound is a floor on what the body must hold, and a
/// legitimate table is always above the floor.
#[test]
fn a_large_object_table_is_not_refused_by_its_own_count_bound() {
    let world = advance(artifact_config(SEED), 20);
    let mut state = world.export_state();
    assert!(!state.ids.is_empty(), "the population died out before objects were added");
    let (x_fp, y_fp) = (state.x_fp[0], state.y_fp[0]);
    let base = state.next_entity_id;
    let stone = material(MATERIAL_STONE).expect("stone is a registered material");
    let table = state.objects.as_mut().expect("artifact world carries a table");
    for offset in 0..200_u64 {
        let record = ObjectRecord::simple(base + offset, stone, 800, x_fp, y_fp, 20, CAUSE_EXTRACTED, 0);
        table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
        table.ledger.energy_extracted_milli += i128::from(record.energy_milli);
        table.push(record);
    }
    table.objects_allocated_total += 200;
    state.next_entity_id += 200;
    let max_depth = state.config.artifact.max_composition_depth.min(255) as u8;
    assert_eq!(state.objects.as_ref().unwrap().violation(max_depth), None);
    let bytes = encode_snapshot(&state, 1, 0, world.state_checksum(), sim_persist::BUILD_VERSION, 0, None)
        .expect("encodes");
    let (_, decoded) = decode_snapshot(&bytes).expect("a two-hundred-object table decodes");
    assert_eq!(decoded.objects.as_ref().map(|table| table.len()), Some(200));
    assert_eq!(decoded, state);
    let restored = World::from_state(decoded).expect("and restores");
    assert_eq!(restored.object_table().unwrap().len(), 200);
}

/// The plan's restore-from-backup clause for this phase: a Phase 12 world
/// carrying a **nonempty modification set** (a live relocating patch, so
/// the worldmod section is populated) **and composite objects** (a held
/// stone and a depth-one composite) is saved into a store, the whole
/// recovery set is copied to an isolated directory, opened there, verified,
/// and the restored world continues bit-identically for 200 ticks against
/// the original - the Phase 4 test's shape, on the format-7 payload with
/// sections 12 and 15 both present.
#[test]
fn a_backup_set_with_overrides_and_composites_restores_in_isolation_and_continues_identically() {
    use sim_persist::SnapshotStore;
    use std::fs;
    let mut config = artifact_config(SEED);
    config.worldmod.patch_enabled = true;
    config.worldmod.relocate_interval_ticks = 20;
    config.worldmod.patch_radius_cells = 3;
    config.worldmod.patch_capacity_scale_q16 = 2 * 65_536;
    let world = advance(config, 60);
    let mut state = world.export_state();
    assert!(
        state.worldmod.as_ref().is_some_and(|w| !w.is_empty()),
        "the patch wrote nothing, so the modification set is empty and this test proves less than it says"
    );
    push_sample_objects(&mut state, 60);
    let mut world = World::from_state(state).expect("the object world restores");
    for _ in 0..20 {
        world.step();
    }
    world.check_invariants().expect("invariants");
    let table = world.object_table().expect("section on");
    assert!(table.count_with_depth_at_least(1) >= 1, "the composite did not survive twenty ticks");
    assert!(table.holder_id.iter().any(|&h| h != 0), "nothing is held");

    let root = std::env::temp_dir().join(format!("lifesim-format7-backup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let source = root.join("source");
    let target = root.join("target");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&target).unwrap();
    let (store, _) = SnapshotStore::open(&source).unwrap();
    let record = store
        .save(&world.export_state(), world.state_checksum(), 1, 0, "named-backup", "manual", 0, Some(3))
        .unwrap();
    for entry in fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }
    let (restored_store, report) = SnapshotStore::open(&target).unwrap();
    assert_eq!(report.valid_saves, 1);
    let restored_record = restored_store.list().unwrap().remove(0);
    assert_eq!(restored_record.state_checksum, record.state_checksum);
    let verify = restored_store.verify(restored_record.save_id).unwrap();
    assert_eq!(verify.state_checksum, world.state_checksum());
    assert_eq!(verify.config_hash, world.config_hash());
    let (_, mut branched) = SnapshotStore::load_world(&target.join(&restored_record.path)).unwrap();
    assert_eq!(branched.state_checksum(), world.state_checksum());
    // Both halves of the phase are in the restored world, not defaulted.
    assert_eq!(
        branched.object_table().map(|t| t.len()),
        world.object_table().map(|t| t.len())
    );
    assert!(branched.worldmod_state().is_some_and(|w| !w.is_empty()));
    for _ in 0..200 {
        world.step();
        branched.step();
    }
    assert_eq!(branched.state_checksum(), world.state_checksum(), "the restored world diverged");
    branched.check_invariants().expect("invariants after the branch");
    let _ = fs::remove_dir_all(&root);
}
