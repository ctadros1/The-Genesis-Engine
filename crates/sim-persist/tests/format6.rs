//! ALIF format 6: the `plasticity.price_moved_edges_only` config byte, the
//! retained format-5 reader and writer, and the registered 5-to-6 migration.
//!
//! # This file means format 6, not "current" - D-108 and D-112's trap, a
//! # fourth time
//!
//! Format 5 was the first config-block bump; format 6 was the second, and
//! landing it broke a test in `format5.rs` for the **third** instance of one
//! trap - a test named for a format that built its subject with the
//! current-format writer, which is the same thing only until the next bump.
//! The two earlier instances were in `phase12_format4.rs` (D-108); the third
//! is D-112.
//!
//! Format 7 landed the trap a **fourth** time, here, in this file itself:
//! three of the tests below built "the format 6 file" with `encode_snapshot`
//! / `decode_snapshot` - the *current*-format functions - which was the same
//! thing as "format 6" only for as long as format 6 was current. They now
//! call `encode_snapshot_format6` / `decode_snapshot_format6` explicitly, so
//! they stay true whatever format 8 turns out to be. The one test that
//! genuinely means "current" - the 5-to-6 migration, which lands on whatever
//! format is current by construction, not on format 6 specifically - keeps
//! `FORMAT_VERSION` and `decode_snapshot`, and says so.
//!
//! # The chain test moved to `format7.rs`
//!
//! `each_adjacent_format_adds_exactly_one_config_byte` used to live here. It
//! asserted a "+1 byte" property that stopped being true the moment format 7
//! appended a whole block instead of a byte - D-112's note that a non-one-
//! byte extension would need "its own reasoning" was written for exactly this
//! day. Rather than special-case format 7 in a file named for format 6, the
//! test moved to `format7.rs` and generalised to a per-row byte delta.
//! Format 6's own link in that chain - "format 6 is format 5 plus one byte" -
//! is still asserted there, as one row among the others.

use sim_core::{SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION, FORMAT_VERSION_5, FORMAT_VERSION_6, decode_snapshot,
    decode_snapshot_format5, decode_snapshot_format6, encode_snapshot_format5,
    encode_snapshot_format6, migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const OFFSET_FORMAT: usize = 4;

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world builds");
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    world
}

fn relabel(bytes: &[u8], format: u16) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[OFFSET_FORMAT..OFFSET_FORMAT + 2].copy_from_slice(&format.to_le_bytes());
    out
}

// --- the retained format-5 reader and the migration --------------------------

/// **Named and built for format 6 explicitly**, not for "current" - see the
/// module doc. Reading `decode_snapshot` here would still pass today, and
/// would keep passing the day format 8 lands, without ever again testing
/// that *format 6* is the reader refusing this file.
#[test]
fn the_format_6_reader_refuses_a_format_5_file() {
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
        decode_snapshot_format6(&legacy).err(),
        Some(CodecError::UnsupportedFormat(FORMAT_VERSION_5))
    );
}

/// A migrated format-5 file equals a native format-5 load: as `SaveState`,
/// then as a world, then over 200 further ticks.
///
/// **Genuinely a 5-to-current test, not a 5-to-6 one, and `FORMAT_VERSION` /
/// `decode_snapshot` stay because of it.** `migration_for` always lands on
/// whatever format is current by construction (`decode_snapshot_migrating`
/// applies exactly one hop), so this test asked "does format 5 survive
/// migration" before format 6 existed and still asks the same question now
/// that format 7 is current - only the destination moved, and the assertions
/// below already track it through `FORMAT_VERSION` rather than a literal.
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
///
/// **Built with `encode_snapshot_format6`, not `encode_snapshot`.** The
/// original read `encode_snapshot` for "six", which was the same file only
/// while format 6 was current; at format 7 it silently became a format-7
/// payload carrying `FORMAT7_CONFIG_BYTES` of extra config rather than one
/// byte, and the assertions below would have kept passing - `TruncatedSection`
/// and `ValueOutOfRange("section trailing bytes")` fire regardless of how much
/// extra sits past the format-5 boundary - while testing a comparison this
/// file was never named for.
#[test]
fn a_version_word_that_disagrees_with_the_body_is_refused_both_ways() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let six = encode_snapshot_format6(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 6");
    let five = encode_snapshot_format5(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 5");

    assert_eq!(
        decode_snapshot_format5(&relabel(&six, FORMAT_VERSION_5)).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format 6 payload read as format 5 must fail on the extra config byte"
    );
    assert_eq!(
        decode_snapshot_format6(&relabel(&five, FORMAT_VERSION_6)).err(),
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
///
/// Built with `encode_snapshot_format6` / `decode_snapshot_format6`, not the
/// current-format pair - the same fix as the version-mismatch test above, for
/// the same reason: this test's name is a claim about format 6 specifically,
/// and the current-format functions stopped meaning that at format 7.
#[test]
fn price_moved_edges_only_survives_the_format_6_round_trip() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let mut state = world.export_state();
    state.config.plasticity.price_moved_edges_only = true;

    let bytes = encode_snapshot_format6(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 6");
    let (_, decoded) = decode_snapshot_format6(&bytes).expect("decode format 6");
    assert!(
        decoded.config.plasticity.price_moved_edges_only,
        "the moat flag did not survive the codec: a restored campaign would \
         resume on the other arm's pricing with no error anywhere"
    );
    assert_eq!(decoded.config, state.config);
}
