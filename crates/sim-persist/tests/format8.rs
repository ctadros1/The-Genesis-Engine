//! ALIF format 8: the Phase 13 social config block (ADR-0029 section 6).
//!
//! The adjacent-format chain property lives in `format7.rs`'s generalized
//! table, which gained format 8's row; what this file owns is the write-side
//! refusal (a pre-8 writer has no bytes for the social section) and the
//! registered 7-to-8 migration's byte-identity obligation.

use sim_core::{SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION, FORMAT_VERSION_4, FORMAT_VERSION_5, FORMAT_VERSION_6,
    FORMAT_VERSION_7, decode_snapshot, decode_snapshot_format7, decode_snapshot_migrating,
    encode_snapshot_format4, encode_snapshot_format5, encode_snapshot_format6,
    encode_snapshot_format7, migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world builds");
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    world
}

/// A config with `genome2`, `worldmod`, and `artifact` enabled and `social`
/// at its default: the base every refusal test mutates away from, chosen so
/// the format-7 block is genuinely exercised (objects exist) while the
/// format-8 block holds nothing but defaults.
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

/// Every retained pre-8 writer refuses a state carrying the social section,
/// each by the named field, never a bare `is_err()`. The whole struct is the
/// refusal predicate, so a knob moved off its default with the section
/// disabled is refused too - a value the format has no bytes for restored at
/// its default would be meaning altered on load.
#[test]
fn every_pre_8_writer_refuses_a_state_carrying_a_format_8_field() {
    let world = advance(artifact_config(SEED), 100);
    let state = world.export_state();
    let checksum = world.state_checksum();

    // The unmodified state must encode at format 7, or every refusal below
    // could be explained by something unrelated to the field it names.
    assert!(
        encode_snapshot_format7(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .is_ok(),
        "the unmodified state must still be expressible at format 7"
    );

    let mut enabled = state.clone();
    enabled.config.social.enabled = true;
    let mut knob_moved = state.clone();
    knob_moved.config.social.perception_k = 2;

    for (subject, name) in [(&enabled, "the gate"), (&knob_moved, "a disabled knob")] {
        assert_eq!(
            encode_snapshot_format7(subject, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
                .err(),
            Some(CodecError::FieldNotInFormat {
                field: "social",
                format: FORMAT_VERSION_7,
            }),
            "{name} must be refused by the format-7 writer"
        );
    }
    assert_eq!(
        encode_snapshot_format6(
            &enabled,
            1,
            0,
            checksum,
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "social",
            format: FORMAT_VERSION_6,
        })
    );
    // The format-5 and format-4 writers refuse the *social* field first,
    // before their own format-7 refusals see the artifact section: the
    // newest-format refusal runs first in every retained writer, so the
    // diagnostic names the newest thing the state carries that the format
    // cannot hold.
    assert_eq!(
        encode_snapshot_format5(
            &enabled,
            1,
            0,
            checksum,
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "social",
            format: FORMAT_VERSION_5,
        })
    );
    assert_eq!(
        encode_snapshot_format4(
            &enabled,
            1,
            0,
            checksum,
            sim_persist::BUILD_VERSION,
            0,
            None
        )
        .err(),
        Some(CodecError::FieldNotInFormat {
            field: "social",
            format: FORMAT_VERSION_4,
        })
    );
}

/// The 7-to-8 migration is registered, claims no loss, and lands on the
/// current format in one hop.
#[test]
fn the_format_7_migration_is_registered_and_claims_no_loss() {
    let migration = migration_for(FORMAT_VERSION_7)
        .expect("format 7 is registered")
        .expect("format 7 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_7);
    assert_eq!(migration.to_format, FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 7 to 8 transform invents nothing and must not claim to"
    );
}

/// A migrated format-7 file equals a native format-7 load: as `SaveState`,
/// then as a world, then over 200 further ticks. Built on an artifact world
/// so the whole format-7 block is exercised - a world without objects would
/// leave the block the migration must carry through mostly defaults.
#[test]
fn the_format_7_migration_is_byte_identical_to_a_format_7_load() {
    let world = advance(artifact_config(SEED), 300);
    let state = world.export_state();
    let checksum = world.state_checksum();
    assert!(
        state.objects.is_some(),
        "a world with no object table does not exercise the section format 8 \
         must carry through"
    );

    let legacy =
        encode_snapshot_format7(&state, 11, 5, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 7");
    assert_eq!(
        decode_snapshot(&legacy).err(),
        Some(CodecError::UnsupportedFormat(FORMAT_VERSION_7)),
        "the current-format reader refuses a format-7 file; migration is the loader's job"
    );

    let (legacy_info, legacy_state) =
        decode_snapshot_format7(&legacy).expect("the retained reader accepts its own file");
    let (migrated_info, migrated_state) =
        decode_snapshot_migrating(&legacy).expect("the migrating loader reads it");

    assert_eq!(migrated_state, legacy_state, "the transform altered state");
    assert_eq!(
        migrated_state.config.social,
        sim_core::SocialConfig::social_default(),
        "a file that predates the social section must migrate with it at defaults"
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
        "a migrated world diverged from the world a format 7 load produces"
    );
}

/// A social world with a live nonzero field, committed cue records, and a
/// nonzero remainder round-trips through the real format-8 codec: every
/// section byte, not the logical path `phase13_social.rs` already covers
/// (D-076: the logical path and the encoded path are different paths).
#[test]
fn a_social_world_with_a_live_field_round_trips_through_the_codec() {
    let mut config = artifact_config(SEED);
    config.social.enabled = true;
    config.validate().expect("valid");
    let mut world = World::new(config).expect("world");
    for _ in 0..50 {
        world.step();
    }
    let mut state = world.export_state();
    // The founders bind nothing social, so make the section carry every
    // field class by hand: a live field value, a contact flag, a delta, a
    // remainder, and counters - injected through the same save path the
    // sim-core scenarios use, then validated by the restore.
    {
        let social = state.social.as_mut().expect("section on");
        social.committed_field_q16[7] = 12_345;
        if let Some(flag) = social.prior_contact.get_mut(0) {
            *flag = true;
        }
        if let Some(delta) = social.prior_object_delta_q16.get_mut(0) {
            *delta = 4_096;
        }
        if let Some(remainder) = social.emission_remainder_milli.get_mut(0) {
            *remainder = 65_535;
        }
        social.counters.signals_emitted_total = 3;
        social.counters.corruption_draws_total = 9;
    }
    let checksum = {
        let world = World::from_state(state.clone()).expect("the doctored state restores");
        world.state_checksum()
    };
    let bytes =
        sim_persist::encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 8");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 8");
    assert_eq!(decoded, state, "the social section did not round-trip");
    let restored = World::from_state(decoded).expect("restores");
    assert_eq!(restored.state_checksum(), checksum);
}
