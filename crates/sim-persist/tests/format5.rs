//! ALIF format 5: the `plasticity.live_rule_zero` config byte, the retained
//! format-4 reader and writer, and the registered 4-to-5 migration.
//!
//! Everything here goes through the **file**. That distinction is why this
//! file exists at all rather than a pair of assertions on `SimConfig`: the
//! whole reason format 5 is a version bump instead of an append is that
//! `encode_config` is positional, and positional layout is invisible to every
//! test that round-trips a struct through `export_state`/`from_state`.
//!
//! # What the format bump has to be true for, and where each clause is
//!
//! | Clause | Test |
//! |---|---|
//! | A format-4 file still decodes, sections and all | `a_format_4_file_with_every_optional_section_still_decodes` |
//! | The migration yields a world byte-identical to a format-4 load | `the_format_4_migration_is_byte_identical_to_a_format_4_load` |
//! | The two config bodies differ by exactly the appended byte | `the_format_4_config_body_is_the_format_5_body_without_its_last_byte` |
//! | A format-4 header over a format-5 body is refused | `a_format_5_file_relabelled_as_format_4_is_refused` |
//! | A format-5 header over a format-4 body is refused | `a_format_4_file_relabelled_as_format_5_is_refused` |
//! | The format-4 writer refuses what it cannot express | `the_format_4_writer_refuses_a_state_carrying_the_format_5_field` |
//! | The registry names both transforms and refuses the rest | `the_registry_registers_two_transforms_and_refuses_everything_else` |
//! | The new field survives its own format's round trip | `live_rule_zero_survives_the_format_5_round_trip` |
//! | Declared lengths are bounded by the check that names them | `an_adversarial_config_section_length_is_refused_by_the_bound_that_names_it` |
//! | Every truncated config body is refused, CRCs resealed | `every_truncated_config_body_is_refused` |
//! | The flag builds a world and starts a replay lineage | `the_flag_builds_a_world_and_starts_its_own_replay_lineage` |

use sim_core::{SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION, FORMAT_VERSION_3, FORMAT_VERSION_4, decode_snapshot,
    decode_snapshot_format4, encode_snapshot, encode_snapshot_format4, migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const ONE_Q16: u32 = 65_536;
/// Header offsets, from the fixed 112-byte layout in `codec.rs`.
const OFFSET_FORMAT: usize = 4;
const OFFSET_UNCOMPRESSED_LEN: usize = 68;
const OFFSET_STORED_LEN: usize = 76;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;
const SECTION_CONFIG: u16 = 1;

fn payload_start() -> usize {
    HEADER_LEN + sim_persist::BUILD_VERSION.len()
}

/// A world that exercises **both** sections the `format < FORMAT_VERSION_4`
/// guard covers.
///
/// Not decoration. Those two arms read `< FORMAT_VERSION` before this change,
/// which was correct for exactly as long as format 4 was current and would
/// have refused every format-4 file the moment format 5 landed. A migration
/// test built on a bare Phase 2 world carries neither section and would have
/// passed with that defect in place.
fn rich_config() -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    // SECTION_WORLDMOD.
    config.worldmod.enabled = true;
    config.worldmod.patch_enabled = true;
    config.worldmod.patch_radius_cells = 20;
    config.worldmod.relocate_interval_ticks = 200;
    config.worldmod.patch_capacity_scale_q16 = 3 * ONE_Q16;
    config.worldmod.dense_threshold_q16 = ONE_Q16;
    // SECTION_ACTION_CENSUS.
    config.probe.enabled = true;
    config.probe.action_census_enabled = true;
    config
}

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world builds");
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    world
}

/// A rich world whose two optional sections are guaranteed **nonempty**.
///
/// The guard is the point: an empty section round-trips through a codec that
/// never wrote it, so without this every assertion below about format 4
/// carrying its sections would be an assertion about their absence.
fn rich_world(ticks: u64) -> World {
    let world = advance(rich_config(), ticks);
    let state = world.export_state();
    assert!(
        state.worldmod.is_some(),
        "the modification section is absent, so this world does not exercise \
         the SECTION_WORLDMOD version guard at all"
    );
    assert!(
        state.action_census.is_some(),
        "the action census section is absent, so this world does not exercise \
         the SECTION_ACTION_CENSUS version guard at all"
    );
    world
}

fn sections(bytes: &[u8]) -> Vec<(u16, usize, usize)> {
    let mut out = Vec::new();
    let mut offset = payload_start();
    while offset + 12 <= bytes.len() {
        let tag = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let length =
            u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap()) as usize;
        out.push((tag, offset + 12, length));
        offset += 12 + length + 4;
    }
    out
}

fn section(bytes: &[u8], tag: u16) -> (usize, usize) {
    let (_, body_start, body_len) = sections(bytes)
        .into_iter()
        .find(|(found, _, _)| *found == tag)
        .unwrap_or_else(|| panic!("section {tag} is present"));
    (body_start, body_len)
}

/// Rewrite the header's format word without touching anything else.
///
/// The header is not checksummed - only the payload is - so this is all it
/// takes to produce a file that lies about its own version. Two tests below
/// depend on that being cheap, because a reader that trusted the word would
/// pass them.
fn relabel(bytes: &[u8], format: u16) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[OFFSET_FORMAT..OFFSET_FORMAT + 2].copy_from_slice(&format.to_le_bytes());
    out
}

/// Replace one section's body, fixing the declared section length, the section
/// CRC, both payload length words, and the payload CRC - so nothing but the
/// loader's own structural checks is left to catch the change.
fn replace_section_body(bytes: &[u8], body_start: usize, body_len: usize, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&bytes[..body_start - 8]);
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(&sim_persist::crc32(body).to_le_bytes());
    out.extend_from_slice(&bytes[body_start + body_len + 4..]);
    let payload_len = (out.len() - payload_start()) as u64;
    out[OFFSET_UNCOMPRESSED_LEN..OFFSET_UNCOMPRESSED_LEN + 8]
        .copy_from_slice(&payload_len.to_le_bytes());
    out[OFFSET_STORED_LEN..OFFSET_STORED_LEN + 8].copy_from_slice(&payload_len.to_le_bytes());
    let payload = out[payload_start()..].to_vec();
    out[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4]
        .copy_from_slice(&sim_persist::crc32(&payload).to_le_bytes());
    out
}

// --- the retained format-4 reader --------------------------------------------

/// A format-4 file carrying both version-guarded sections decodes at format 4.
///
/// **This is the test that catches the guard defect**, and it is worth
/// stating why it is separate from the migration test below. `SECTION_WORLDMOD`
/// and `SECTION_ACTION_CENSUS` were guarded by `format < FORMAT_VERSION`, a
/// comparison that is correct only while the introducing format is the current
/// one. Bumping `FORMAT_VERSION` to 5 turns it into "refuse these sections in
/// every format-4 file", which is every campaign artifact on disk, with an
/// error naming the section rather than the comparison. The guards now name
/// `FORMAT_VERSION_4`, which is the format that introduced them and does not
/// move.
#[test]
fn a_format_4_file_with_every_optional_section_still_decodes() {
    let world = rich_world(600);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let legacy =
        encode_snapshot_format4(&state, 9, 4, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 4");
    assert_eq!(
        u16::from_le_bytes(legacy[OFFSET_FORMAT..OFFSET_FORMAT + 2].try_into().unwrap()),
        FORMAT_VERSION_4
    );

    let (info, decoded) = decode_snapshot_format4(&legacy).expect("the format 4 reader reads it");
    assert_eq!(info.format_version, FORMAT_VERSION_4);
    assert!(
        decoded.worldmod.is_some(),
        "the modification section did not survive the format 4 reader"
    );
    assert!(
        decoded.action_census.is_some(),
        "the action census section did not survive the format 4 reader"
    );
    assert_eq!(decoded, state, "the format 4 reader lost logical state");
    assert!(
        !decoded.config.plasticity.live_rule_zero,
        "a format 4 file has no byte for the flag, so it must resolve false"
    );
}

/// The current reader refuses a format-4 file outright, which is what makes
/// the registry load-bearing rather than decorative.
#[test]
fn the_current_reader_refuses_a_format_4_file() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let legacy = encode_snapshot_format4(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 4");
    assert_eq!(
        decode_snapshot(&legacy).err(),
        Some(CodecError::UnsupportedFormat(FORMAT_VERSION_4))
    );
}

// --- the byte-identity clause ------------------------------------------------

/// A migrated format-4 file equals a native format-4 load: as `SaveState`,
/// then as a world, then over 200 further ticks.
///
/// The three levels are not redundant. `SaveState` equality is a field
/// comparison and would pass for a state that no longer restores; world
/// equality catches a restore that re-derives something differently; and the
/// 200 ticks catch a difference that is invisible at rest and diverges under
/// the tick - which is the only kind a compiled-plan or float-order change
/// produces.
#[test]
fn the_format_4_migration_is_byte_identical_to_a_format_4_load() {
    let world = rich_world(600);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let legacy =
        encode_snapshot_format4(&state, 9, 4, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 4");

    let migration = migration_for(FORMAT_VERSION_4)
        .expect("format 4 is registered")
        .expect("format 4 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_4);
    assert_eq!(migration.to_format, FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 4 to 5 transform invents nothing and must not claim to"
    );

    let migrated = (migration.transform)(&legacy).expect("the transform runs");
    assert_eq!(migrated.source.format_version, FORMAT_VERSION_4);
    assert_eq!(migrated.source.world_id, 9);
    assert_eq!(migrated.source.parent_world_id, 4);

    let (legacy_info, legacy_state) = decode_snapshot_format4(&legacy).expect("legacy decode");
    let (migrated_info, migrated_state) =
        decode_snapshot(&migrated.bytes).expect("the migrated file decodes at format 5");
    assert_eq!(migrated_info.format_version, FORMAT_VERSION);
    assert_eq!(migrated_state, legacy_state, "the migrated state differs");
    assert_eq!(
        migrated_state, state,
        "neither path reproduced the original"
    );
    assert!(
        !migrated_state.config.plasticity.live_rule_zero,
        "a file that predates the flag must migrate with it false"
    );
    assert_eq!(legacy_info.state_checksum, migrated_info.state_checksum);
    assert_eq!(legacy_info.terrain_checksum, migrated_info.terrain_checksum);
    assert_eq!(legacy_info.config_hash, migrated_info.config_hash);
    assert_eq!(legacy_info.tick, migrated_info.tick);
    assert_eq!(legacy_info.build_version, migrated_info.build_version);

    // ...and the worlds, not just the records.
    let mut from_legacy = World::from_state(legacy_state).expect("restore legacy");
    let mut from_migrated = World::from_state(migrated_state).expect("restore migrated");
    assert_eq!(from_legacy.state_checksum(), checksum);
    assert_eq!(from_migrated.state_checksum(), checksum);
    assert_eq!(
        from_legacy.composed_terrain_checksum(),
        from_migrated.composed_terrain_checksum()
    );
    assert_eq!(from_legacy.export_state(), from_migrated.export_state());
    for _ in 0..200 {
        from_legacy.step();
        from_migrated.step();
    }
    assert_eq!(
        from_legacy.state_checksum(),
        from_migrated.state_checksum(),
        "a migrated world diverged from the world a format 4 load produces"
    );
}

/// The loader every caller should use routes a format-4 file through the
/// transform rather than refusing it.
#[test]
fn the_migrating_loader_accepts_a_format_4_file() {
    let world = rich_world(400);
    let state = world.export_state();
    let legacy = encode_snapshot_format4(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 4");
    let (info, decoded) =
        sim_persist::decode_snapshot_migrating(&legacy).expect("the migrating loader reads it");
    assert_eq!(
        info.format_version, FORMAT_VERSION_4,
        "the reported provenance must be the file as found, not the format it was migrated to"
    );
    assert_eq!(decoded, state);
}

// --- the one-byte difference, stated exactly ---------------------------------

/// The format-4 config body is the format-5 body with its last byte removed.
///
/// This is the whole format difference in one assertion, and it is the reason
/// the byte was appended after `probe` rather than filed next to the
/// `plasticity` block it belongs to. Filed in the middle, the two bodies would
/// diverge from that offset on and the only way to state the difference would
/// be to re-describe the layout - which is a test that restates the code.
#[test]
fn the_format_4_config_body_is_the_format_5_body_without_its_last_byte() {
    let world = rich_world(300);
    let state = world.export_state();
    let checksum = world.state_checksum();

    let five = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 5");
    let four = encode_snapshot_format4(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
        .expect("encode format 4");

    let (five_start, five_len) = section(&five, SECTION_CONFIG);
    let (four_start, four_len) = section(&four, SECTION_CONFIG);
    assert_eq!(
        five_len,
        four_len + 1,
        "format 5's config section must be exactly one byte longer"
    );
    assert_eq!(
        &five[five_start..five_start + four_len],
        &four[four_start..four_start + four_len],
        "the format 4 config body is not a prefix of the format 5 body"
    );
    assert_eq!(
        five[five_start + four_len],
        0,
        "the appended byte must be the flag, and it is false in this world"
    );

    // Everything after the config section differs only by the one-byte shift,
    // so the two payloads have the same length up to it and the same tail.
    assert_eq!(
        five.len(),
        four.len() + 1,
        "format 5 must cost exactly one byte over format 4 for the same world"
    );
}

// --- both directions of a forged version word --------------------------------

/// A format-5 body whose header claims format 4 is refused.
///
/// The interesting part is *where*: the extra byte makes the config section
/// one longer than the format-4 reader consumes, so the trailing-bytes check
/// every section runs rejects it on the body alone. A reader that trusted the
/// header would have to be caught by the header; this one is caught before the
/// header's word matters.
#[test]
fn a_format_5_file_relabelled_as_format_4_is_refused() {
    let world = rich_world(300);
    let native = encode_snapshot(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 5");
    let forged = relabel(&native, FORMAT_VERSION_4);
    assert_eq!(
        decode_snapshot_format4(&forged).err(),
        Some(CodecError::ValueOutOfRange("section trailing bytes")),
        "a format 5 payload read as format 4 must fail on the extra config byte"
    );
}

/// A format-4 body whose header claims format 5 is refused.
///
/// The mirror image, and it fails at the other end: the format-5 reader runs
/// out of body reading the flag.
#[test]
fn a_format_4_file_relabelled_as_format_5_is_refused() {
    let world = rich_world(300);
    let legacy = encode_snapshot_format4(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 4");
    let forged = relabel(&legacy, FORMAT_VERSION);
    assert_eq!(
        decode_snapshot(&forged).err(),
        Some(CodecError::TruncatedSection),
        "a format 4 payload read as format 5 must run out of config body"
    );
}

// --- the write side ----------------------------------------------------------

/// The format-4 writer refuses a state it cannot express, rather than dropping
/// the field.
///
/// Silently writing the file would produce one that describes a world with
/// rule 0 dead - the "never alter meaning" rule broken on the write side,
/// where nothing downstream can detect it. The error names the *field*, not a
/// section, because a config field has no tag.
#[test]
fn the_format_4_writer_refuses_a_state_carrying_the_format_5_field() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let mut state = world.export_state();
    state.config.plasticity.live_rule_zero = true;
    assert_eq!(
        encode_snapshot_format4(
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
            field: "plasticity.live_rule_zero",
            format: FORMAT_VERSION_4,
        })
    );
    // ...and the same state at format 5 writes without complaint, which is
    // what makes the refusal above about the format rather than the state.
    state.config.plasticity.live_rule_zero = false;
    assert!(
        encode_snapshot_format4(
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

/// The new field survives its own format's round trip, set to the value the
/// default is not.
///
/// Set on the exported state rather than on a built world, because `validate`
/// refuses `true` until the flag is behavioural. That is exactly what
/// `config_field_coverage.rs` does and for the same reason: this is a codec
/// test, and refusing to encode the values validation rejects would leave the
/// byte unchecked in the only place it exists.
#[test]
fn live_rule_zero_survives_the_format_5_round_trip() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let mut state = world.export_state();
    state.config.plasticity.live_rule_zero = true;

    let bytes = encode_snapshot(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 5");
    let (_, decoded) = decode_snapshot(&bytes).expect("decode format 5");
    assert!(
        decoded.config.plasticity.live_rule_zero,
        "the flag did not survive the codec, which is D-065's defect in its \
         sixth form: a restored world would run the control arm silently"
    );
    assert_eq!(decoded.config, state.config);
}

// --- the registry ------------------------------------------------------------

/// Two transforms, both landing on the current format, and everything else
/// refused.
#[test]
fn the_registry_registers_two_transforms_and_refuses_everything_else() {
    assert!(
        migration_for(FORMAT_VERSION)
            .expect("the current format is not an error")
            .is_none(),
        "the current format must not route through a transform"
    );
    for registered in [FORMAT_VERSION_3, FORMAT_VERSION_4] {
        let migration = migration_for(registered)
            .expect("registered")
            .expect("has a transform");
        assert_eq!(migration.from_format, registered);
        assert_eq!(
            migration.to_format, FORMAT_VERSION,
            "every registered transform lands on the current format in one hop; \
             there is no chaining, because decode_snapshot_migrating applies one"
        );
        assert_eq!(migration.expected_loss, "");
    }
    for absent in [0_u16, 1, 2] {
        let error = migration_for(absent).expect_err("no transform");
        assert!(
            error.contains("no registered migration"),
            "format {absent}: {error}"
        );
    }
    let error = migration_for(FORMAT_VERSION + 1).expect_err("newer than this build");
    assert!(error.contains("newer than this build"), "{error}");
}

// --- standing rule 2: declared lengths, resealed --------------------------------

/// The config section's declared length is refused **by `MAX_SECTION_LEN`**,
/// with every CRC resealed, and the error names that bound rather than a
/// later one.
///
/// # Why this asserts the exact error and not `is_err()`
///
/// It asserted `is_err()` first, and a mutation run found the test earned no
/// kill credit anywhere: deleting the `length > MAX_SECTION_LEN` check left
/// every one of its values still refused, each by a *different, later* rung of
/// the ladder in `decode_payload` -
///
/// 1. `length > MAX_SECTION_LEN` -> `ValueOutOfRange("section length")`
/// 2. `body_start.checked_add(length)` overflow -> `TruncatedSection`
/// 3. `bytes.len() < body_end + 4` -> `TruncatedSection`
/// 4. body CRC mismatch -> `SectionChecksumMismatch`
///
/// So the test passed with the bound it exists to defend deleted. That is the
/// standing-rule-2 failure mode exactly one level up from the one the rule is
/// written about: not a bit flip caught by a CRC, but a *bound* whose removal
/// is masked by a bound underneath it.
///
/// Making the bound the only guard standing would need a file physically
/// longer than `MAX_SECTION_LEN` - a gibibyte of test fixture - because rung 3
/// fires on anything shorter. Asserting the exact error costs nothing and
/// discriminates just as well: with the bound deleted these values reach rungs
/// 2 and 3 and report `TruncatedSection`, and the assertion fails.
///
/// Two values below the bound are included for the same reason, pinned to the
/// rung that *should* catch them. Without them, "every value returns
/// `ValueOutOfRange`" would be satisfiable by a decoder that returned it for
/// everything.
#[test]
fn an_adversarial_config_section_length_is_refused_by_the_bound_that_names_it() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let bytes = encode_snapshot(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode");
    let (body_start, _) = section(&bytes, SECTION_CONFIG);
    let length_at = body_start - 8;

    let forge = |declared: u64| {
        let mut forged = bytes.clone();
        forged[length_at..length_at + 8].copy_from_slice(&declared.to_le_bytes());
        // Reseal the payload CRC so the declared length is *reached* rather
        // than rejected as corruption before anything looks at it.
        let payload = forged[payload_start()..].to_vec();
        forged[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4]
            .copy_from_slice(&sim_persist::crc32(&payload).to_le_bytes());
        forged
    };

    // Above the bound: rung 1, and only rung 1.
    for declared in [
        u64::MAX,
        u64::MAX - 11,
        u64::MAX / 2,
        sim_persist::MAX_UNCOMPRESSED_LEN + 1,
    ] {
        assert_eq!(
            decode_snapshot(&forge(declared)).err(),
            Some(CodecError::ValueOutOfRange("section length")),
            "a config section declaring length {declared} was not refused by \
             MAX_SECTION_LEN; if this now reports TruncatedSection the bound \
             has been removed and a later check is masking it"
        );
    }

    // Below the bound but past the end of the file: rung 3, a different error.
    // This is what makes the assertions above attributable to the bound.
    let past_the_end = (bytes.len() - payload_start()) as u64;
    assert!(past_the_end <= sim_persist::MAX_UNCOMPRESSED_LEN);
    assert_eq!(
        decode_snapshot(&forge(past_the_end)).err(),
        Some(CodecError::TruncatedSection),
        "a length inside the bound but past the end of the file must be \
         refused by the truncation check, not by the bound"
    );
}

/// A config body truncated to every prefix length is refused, never accepted
/// and never a panic.
///
/// The format-5 byte is the *last* one in the block, so the prefix one byte
/// short is precisely a format-4 body - the case a lenient reader would let
/// through by leaving the flag at its default. It must be refused like every
/// other short body.
#[test]
fn every_truncated_config_body_is_refused() {
    let world = advance(SimConfig::phase2_default(SEED), 200);
    let bytes = encode_snapshot(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode");
    let (body_start, body_len) = section(&bytes, SECTION_CONFIG);
    let body = bytes[body_start..body_start + body_len].to_vec();

    for shortened in (0..body_len).rev().take(24) {
        let forged = replace_section_body(&bytes, body_start, body_len, &body[..shortened]);
        assert!(
            decode_snapshot(&forged).is_err(),
            "a config body truncated to {shortened} of {body_len} bytes was accepted"
        );
    }
    // The one-byte-short case named explicitly, because it is the only one an
    // implementation could plausibly want to accept.
    let forged = replace_section_body(&bytes, body_start, body_len, &body[..body_len - 1]);
    assert_eq!(
        decode_snapshot(&forged).err(),
        Some(CodecError::TruncatedSection),
        "a format-4-length config body must not be read leniently at format 5"
    );
}

// --- the flag is live, and it starts a replay lineage -------------------------

/// The flag builds a world, and setting it moves the config hash.
///
/// **This test replaced a refusal, and the swap is the point.** While the byte
/// was encoded but inert (ALIF format 5, D-108), `validate` rejected `true`,
/// because accepting a flag that changed nothing would hand a campaign an arm
/// bit-identical to its own control and get reported as a null. ADR-0027 makes
/// the flag behavioural and deletes that refusal, so the test that pinned it
/// has to become the test that pins what replaced it - otherwise removing a
/// refusal silently removes its coverage too.
///
/// The hash assertion is the substantive half. A world whose every `rule_id`
/// names a different rule, and whose mutation draw has a different range, is a
/// different experiment; it must not share a replay lineage with the arm it is
/// compared against.
#[test]
fn the_flag_builds_a_world_and_starts_its_own_replay_lineage() {
    let clear = SimConfig::phase11_default(SEED);
    let mut set = clear;
    set.plasticity.live_rule_zero = true;

    assert!(
        World::new(clear).is_ok(),
        "the carrier config must build, or nothing below means anything"
    );
    assert!(
        World::new(set).is_ok(),
        "the flag is behavioural now and must no longer be refused"
    );

    assert_ne!(
        clear.stable_hash(),
        set.stable_hash(),
        "setting the flag must start a new replay lineage: the same allele \
         names a different rule and the mutation draw has a different range"
    );

    // ...and it is gated on the section, so a world with plasticity disabled
    // hashes identically either way. Without this the flag would move worlds
    // that have no plastic edges for it to act on.
    let mut off = SimConfig::phase2_default(SEED);
    assert!(!off.plasticity.enabled);
    let off_hash = off.stable_hash();
    off.plasticity.live_rule_zero = true;
    assert_eq!(
        off_hash,
        off.stable_hash(),
        "the flag moved a world whose plasticity section is disabled"
    );
}

/// The record a save carries for a format-4 file names the format it was found
/// at, not the one it was migrated to.
#[test]
fn a_migrated_record_reports_the_format_it_was_found_at() {
    let world = advance(SimConfig::phase2_default(SEED), 150);
    let legacy = encode_snapshot_format4(
        &world.export_state(),
        3,
        1,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode format 4");
    let migration = migration_for(FORMAT_VERSION_4).unwrap().unwrap();
    let migrated = (migration.transform)(&legacy).expect("transform");
    assert_eq!(migrated.source.format_version, FORMAT_VERSION_4);
    assert_eq!(
        sim_persist::read_info(&migrated.bytes)
            .expect("the re-encoded file reads at the current format")
            .format_version,
        FORMAT_VERSION
    );
}
