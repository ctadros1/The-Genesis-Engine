//! ALIF format 6: the `plasticity.price_moved_edges_only` config byte, the
//! retained format-5 reader and writer, and the registered 5-to-6 migration.
//!
//! # The chain test is the point of this file
//!
//! Format 5 was the first config-block bump; format 6 is the second, and
//! adding it broke a test in `format5.rs` for the **third** instance of one
//! trap - a test named for a format that built its subject with the
//! current-format writer, which is the same thing only until the next bump.
//! The two earlier instances were in `phase12_format4.rs` and are recorded in
//! D-108.
//!
//! `each_adjacent_format_adds_exactly_one_config_byte` replaces that whole
//! class with one parameterised assertion over the chain of retained writers.
//! A seventh format adds a row to its table and nothing else, and if the new
//! format does not extend its predecessor by exactly one byte the table says
//! so by name.

use sim_core::{SaveState, SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION, FORMAT_VERSION_4, FORMAT_VERSION_5, decode_snapshot,
    decode_snapshot_format5, encode_snapshot, encode_snapshot_format4, encode_snapshot_format5,
    migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const OFFSET_FORMAT: usize = 4;
const HEADER_LEN: usize = 112;
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

fn section(bytes: &[u8], tag: u16) -> (usize, usize) {
    let mut offset = payload_start();
    while offset + 12 <= bytes.len() {
        let found = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let length =
            u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap()) as usize;
        if found == tag {
            return (offset + 12, length);
        }
        offset += 12 + length + 4;
    }
    panic!("section {tag} is present");
}

fn relabel(bytes: &[u8], format: u16) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[OFFSET_FORMAT..OFFSET_FORMAT + 2].copy_from_slice(&format.to_le_bytes());
    out
}

type Writer = fn(&SaveState, u64, u64, u64, &str, u64, Option<i32>) -> Result<Vec<u8>, CodecError>;

/// Every config-block format, oldest first, with the writer that produces it.
///
/// Format 3 is deliberately absent: it differs from format 4 by a *section*
/// and a logical-state version, not by a config byte, so it is not part of
/// this chain and `phase12_format4.rs` owns that comparison.
fn config_block_chain() -> Vec<(u16, Writer)> {
    vec![
        (FORMAT_VERSION_4, encode_snapshot_format4 as Writer),
        (FORMAT_VERSION_5, encode_snapshot_format5 as Writer),
        (FORMAT_VERSION, encode_snapshot as Writer),
    ]
}

// --- the chain property ------------------------------------------------------

/// Each format's config body is its predecessor's plus exactly one byte, and
/// the predecessor's body is a byte prefix of it.
///
/// One assertion for a property that was previously restated per format and
/// broke, in a different file, every time a format landed. Adding format 7
/// means adding one row above.
#[test]
fn each_adjacent_format_adds_exactly_one_config_byte() {
    let world = advance(SimConfig::phase2_default(SEED), 300);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let chain = config_block_chain();
    let encoded: Vec<(u16, Vec<u8>)> = chain
        .iter()
        .map(|(format, write)| {
            let bytes = write(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
                .unwrap_or_else(|error| panic!("format {format} writer: {error:?}"));
            assert_eq!(
                u16::from_le_bytes(bytes[OFFSET_FORMAT..OFFSET_FORMAT + 2].try_into().unwrap()),
                *format,
                "the format {format} writer stamped a different version"
            );
            (*format, bytes)
        })
        .collect();

    for pair in encoded.windows(2) {
        let (older_version, older) = &pair[0];
        let (newer_version, newer) = &pair[1];
        let (older_start, older_len) = section(older, SECTION_CONFIG);
        let (newer_start, newer_len) = section(newer, SECTION_CONFIG);

        assert_eq!(
            newer_len,
            older_len + 1,
            "format {newer_version}'s config body must be exactly one byte longer \
             than format {older_version}'s"
        );
        assert_eq!(
            &newer[newer_start..newer_start + older_len],
            &older[older_start..older_start + older_len],
            "format {older_version}'s config body is not a prefix of format \
             {newer_version}'s, so the two formats differ somewhere other than \
             the appended byte"
        );
        assert_eq!(
            newer[newer_start + older_len],
            0,
            "format {newer_version}'s appended byte is not the flag's false value \
             in a default world"
        );
        assert_eq!(
            newer.len(),
            older.len() + 1,
            "format {newer_version} must cost exactly one byte over format \
             {older_version} for the same world"
        );
    }
}

// --- the retained format-5 reader and the migration --------------------------

#[test]
fn the_current_reader_refuses_a_format_5_file() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let legacy = encode_snapshot_format5(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 5");
    assert_eq!(
        decode_snapshot(&legacy).err(),
        Some(CodecError::UnsupportedFormat(FORMAT_VERSION_5))
    );
}

/// A migrated format-5 file equals a native format-5 load: as `SaveState`,
/// then as a world, then over 200 further ticks.
#[test]
fn the_format_5_migration_is_byte_identical_to_a_format_5_load() {
    let mut config = SimConfig::phase11_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    let world = advance(config, 400);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert!(
        state.learn.is_some(),
        "a world with no learn section does not exercise the plasticity config \
         block this format extends"
    );

    let legacy =
        encode_snapshot_format5(&state, 11, 5, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 5");

    let migration = migration_for(FORMAT_VERSION_5)
        .expect("format 5 is registered")
        .expect("format 5 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_5);
    assert_eq!(migration.to_format, FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 5 to 6 transform invents nothing and must not claim to"
    );

    let migrated = (migration.transform)(&legacy).expect("the transform runs");
    assert_eq!(migrated.source.format_version, FORMAT_VERSION_5);
    assert_eq!(migrated.source.world_id, 11);

    let (legacy_info, legacy_state) = decode_snapshot_format5(&legacy).expect("legacy decode");
    let (migrated_info, migrated_state) =
        decode_snapshot(&migrated.bytes).expect("the migrated file decodes at format 6");
    assert_eq!(migrated_info.format_version, FORMAT_VERSION);
    assert_eq!(migrated_state, legacy_state, "the migrated state differs");
    assert_eq!(
        migrated_state, state,
        "neither path reproduced the original"
    );
    assert!(
        !migrated_state.config.plasticity.price_moved_edges_only,
        "a file that predates the moat must migrate with it off"
    );
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
        "a migrated world diverged from the world a format 5 load produces"
    );
}

/// Both directions of a forged version word, on the body rather than the
/// header.
#[test]
fn a_version_word_that_disagrees_with_the_body_is_refused_both_ways() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let six = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 6");
    let five = encode_snapshot_format5(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 5");

    assert_eq!(
        decode_snapshot_format5(&relabel(&six, FORMAT_VERSION_5)).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format 6 payload read as format 5 must fail on the extra config byte"
    );
    assert_eq!(
        decode_snapshot(&relabel(&five, FORMAT_VERSION)).err(),
        Some(CodecError::TruncatedSection),
        "a format 5 payload read as format 6 must run out of config body"
    );
}

/// The format-5 writer refuses a state it cannot express.
#[test]
fn the_format_5_writer_refuses_a_state_carrying_the_format_6_field() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let mut state = world.export_state();
    state.config.plasticity.price_moved_edges_only = true;
    assert_eq!(
        encode_snapshot_format5(
            &state,
            1,
            0,
            world.state_checksum(),
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "plasticity.price_moved_edges_only",
            format: FORMAT_VERSION_5,
        })
    );
    state.config.plasticity.price_moved_edges_only = false;
    assert!(
        encode_snapshot_format5(
            &state,
            1,
            0,
            world.state_checksum(),
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .is_ok()
    );
}

/// The moat flag survives its own format's round trip, set to the value the
/// default is not.
#[test]
fn price_moved_edges_only_survives_the_format_6_round_trip() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let mut state = world.export_state();
    state.config.plasticity.price_moved_edges_only = true;

    let bytes = encode_snapshot(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 6");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 6");
    assert!(
        decoded.config.plasticity.price_moved_edges_only,
        "the moat flag did not survive the codec: a restored campaign would \
         resume on the other arm's pricing with no error anywhere"
    );
    assert_eq!(decoded.config, state.config);
}
