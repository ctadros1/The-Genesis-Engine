//! ALIF section 14: the Phase 11 action census, at the file level.
//!
//! The section's contents are covered by `phase11_probe.rs` at the kernel
//! level and by `actionlog.rs` for the separate `.alac` artifact. What is
//! left, and what this file is for, is the framing: that a probe snapshot
//! round-trips, that a world without the probe writes a byte-identical
//! snapshot to the one it wrote before this section existed, and that the
//! declared count inside the section is bounded **before** it sizes anything.
//!
//! Standing rule 2 governs the last of those. Every patched count is followed
//! by a reseal of the section CRC and the payload CRC, so the value is
//! reached by the decoder rather than rejected as corruption: a bit-flip
//! sweep cannot produce a count near 2^61 and would never exercise the bound.

use sim_core::{SaveState, SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION_3, decode_snapshot, encode_snapshot, encode_snapshot_format3,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;
/// Section tag of the action census, from `codec.rs`.
const SECTION_ACTION_CENSUS: u16 = 14;

fn payload_start() -> usize {
    HEADER_LEN + sim_persist::BUILD_VERSION.len()
}

fn probe_config() -> SimConfig {
    let mut config = SimConfig::phase11_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
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

/// A probe world whose census is guaranteed nonempty.
///
/// The guard matters for the reason `modified_world`'s does in
/// `phase12_format4.rs`: every assertion below would otherwise be an
/// assertion about an empty section, and an empty section round-trips through
/// a codec that never wrote it.
fn probe_world(ticks: u64) -> World {
    let world = advance(probe_config(), ticks);
    let census = world.action_census();
    assert!(
        census.len() > 20
            && census
                .iter()
                .any(|sample| sample.counts.iter().filter(|value| **value > 0).count() > 1),
        "the census is empty or degenerate, so every assertion below is vacuous"
    );
    world
}

fn encode(state: &SaveState, checksum: u64) -> Vec<u8> {
    encode_snapshot(state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None).expect("encode")
}

/// `(tag, flags, body_start, body_len)` for every section, walked from the
/// payload start rather than searched for, so this cannot match a tag-shaped
/// value inside a genome.
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
    let (_, _, body_start, body_len) = sections(bytes)
        .into_iter()
        .find(|(found, _, _, _)| *found == tag)
        .unwrap_or_else(|| panic!("section {tag} is present"));
    (body_start, body_len)
}

/// Re-seal a snapshot whose section body was patched in place: the section
/// CRC that covers it, then the payload CRC in the header.
fn reseal(bytes: &mut [u8], body_start: usize, body_len: usize) {
    let body = bytes[body_start..body_start + body_len].to_vec();
    let section_crc = sim_persist::crc32(&body);
    bytes[body_start + body_len..body_start + body_len + 4]
        .copy_from_slice(&section_crc.to_le_bytes());
    let payload = bytes[payload_start()..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    bytes[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
}

#[test]
fn a_probe_snapshot_round_trips_every_column_and_both_counters() {
    let mut world = probe_world(700);
    world.reset_action_census();
    for _ in 0..300 {
        world.step();
    }
    let before = world.action_census();
    let counters = world.action_census_counters().expect("counters");
    assert_eq!(
        counters.resets_total, 1,
        "the reset counter must be nonzero"
    );

    let state = world.export_state();
    let bytes = encode(&state, world.state_checksum());
    let (_, decoded) = decode_snapshot(&bytes).expect("decodes");
    assert_eq!(decoded.action_census, state.action_census);

    let restored = World::from_state(decoded).expect("restores");
    assert_eq!(restored.action_census(), before);
    assert_eq!(
        restored.action_census_counters().expect("counters"),
        counters,
        "a counter was dropped on the way through the file"
    );
    assert_eq!(restored.state_checksum(), world.state_checksum());
}

#[test]
fn a_world_without_the_probe_writes_no_section() {
    let mut config = probe_config();
    config.probe.enabled = false;
    config.probe.action_census_enabled = false;
    let world = advance(config, 200);
    let bytes = encode(&world.export_state(), world.state_checksum());
    assert!(
        !sections(&bytes)
            .iter()
            .any(|(tag, _, _, _)| *tag == SECTION_ACTION_CENSUS),
        "a world with no census wrote a census section"
    );
    // ...and the probe world does write one, so the absence above is a fact
    // about the gate rather than about this test's ability to find a section.
    let probed = probe_world(200);
    let probed_bytes = encode(&probed.export_state(), probed.state_checksum());
    assert!(
        sections(&probed_bytes)
            .iter()
            .any(|(tag, _, _, _)| *tag == SECTION_ACTION_CENSUS)
    );
}

#[test]
fn the_section_costs_exactly_four_bytes_per_column_per_organism_plus_forty() {
    // **Framing stated exactly, in C11.7's style.** A budget claim written as
    // "about 28 bytes" is a claim nothing can fail, and the number is what a
    // later revision has to restate from: 4 bytes x `ACTION_CLASS_COUNT`
    // columns per organism, plus 40 bytes of section framing (12 header, 8
    // count word, 16 counters, 4 CRC), and nothing else.
    //
    // Measured against the same world with the census gate off, which is the
    // only comparison that isolates the section: two different worlds would
    // differ in genome bytes as well.
    let mut off = probe_config();
    off.probe.enabled = false;
    off.probe.action_census_enabled = false;
    let plain = advance(off, 2_000);
    let probed = advance(probe_config(), 2_000);
    assert_eq!(
        plain.population(),
        probed.population(),
        "the census changed the population, so the byte difference is not the section"
    );
    let population = probed.population() as u64;
    assert!(population > 20);

    let plain_bytes = encode(&plain.export_state(), plain.state_checksum()).len() as u64;
    let probed_bytes = encode(&probed.export_state(), probed.state_checksum()).len() as u64;
    let expected = population * 4 * sim_core::ACTION_CLASS_COUNT as u64 + 40;
    assert_eq!(
        probed_bytes - plain_bytes,
        expected,
        "the section costs {} bytes for {population} organisms, not {expected}",
        probed_bytes - plain_bytes
    );
}

#[test]
fn the_declared_organism_count_is_bounded_before_it_sizes_anything() {
    // Standing rule 2. Each value is patched **and resealed**, so it reaches
    // the decoder's bound instead of failing the CRC first - which is the
    // difference between testing the bound and testing CRC32 (D-091).
    let world = probe_world(500);
    let state = world.export_state();
    let bytes = encode(&state, world.state_checksum());
    let (body_start, body_len) = section(&bytes, SECTION_ACTION_CENSUS);
    assert!(
        decode_snapshot(&bytes).is_ok(),
        "the unpatched snapshot must decode, or every refusal below is vacuous"
    );

    let per_organism = 4 * sim_core::ACTION_CLASS_COUNT as u64;
    for count in [
        u64::MAX,
        u64::MAX / per_organism,
        body_len as u64,
        u64::from(u32::MAX),
    ] {
        let mut patched = bytes.clone();
        patched[body_start..body_start + 8].copy_from_slice(&count.to_le_bytes());
        reseal(&mut patched, body_start, body_len);
        let error = decode_snapshot(&patched).expect_err("must be refused");
        // Pinned to the diagnostic of the near guard. Two things can refuse
        // this - the allocation bound here and the trailing-bytes check at
        // the end of the section - and `matches!(err, ValueOutOfRange(_))`
        // would pass with the bound deleted, because the trailing-bytes check
        // would fire instead *after* the allocation had already been made.
        assert_eq!(
            error,
            CodecError::ValueOutOfRange("action census organisms"),
            "count {count} was not refused by the allocation bound"
        );
    }

    // A count that is merely *wrong* rather than enormous is caught by the
    // trailing-bytes check, which is the other half of D-075's discipline:
    // the bound caps, and exactness comes from the section consuming its body
    // exactly.
    let mut patched = bytes.clone();
    patched[body_start..body_start + 8].copy_from_slice(&1_u64.to_le_bytes());
    reseal(&mut patched, body_start, body_len);
    assert_eq!(
        decode_snapshot(&patched).expect_err("must be refused"),
        CodecError::ValueOutOfRange("section trailing bytes")
    );
}

#[test]
fn a_duplicate_census_section_is_refused() {
    // Sections are optional and identified by tag, so "two of them" is a
    // reachable shape for a hand-built file and the last one would silently
    // win without this.
    let world = probe_world(200);
    let state = world.export_state();
    let bytes = encode(&state, world.state_checksum());
    let (body_start, body_len) = section(&bytes, SECTION_ACTION_CENSUS);
    let whole = body_start - 12..body_start + body_len + 4;

    let mut doubled = bytes[..whole.end].to_vec();
    doubled.extend_from_slice(&bytes[whole.clone()]);
    doubled.extend_from_slice(&bytes[whole.end..]);
    // Fix both payload length words and the payload CRC so the duplicate is
    // reached rather than rejected as a length mismatch.
    let added = whole.len() as u64;
    for offset in [68_usize, 76] {
        let value = u64::from_le_bytes(doubled[offset..offset + 8].try_into().unwrap());
        doubled[offset..offset + 8].copy_from_slice(&(value + added).to_le_bytes());
    }
    let payload = doubled[payload_start()..].to_vec();
    let crc = sim_persist::crc32(&payload);
    doubled[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&crc.to_le_bytes());

    assert_eq!(
        decode_snapshot(&doubled).expect_err("must be refused"),
        CodecError::DuplicateSection(SECTION_ACTION_CENSUS)
    );
}

#[test]
fn a_format_3_writer_refuses_a_state_carrying_a_census() {
    // Silently dropping it would be the "never alter meaning during load"
    // rule broken on the write side, and the drop would be invisible: the
    // restored world's rows would all be zero and nothing in the tick reads
    // them, so no later check would notice.
    let world = probe_world(200);
    let state = world.export_state();
    assert_eq!(
        encode_snapshot_format3(&state, 1, 0, 0, sim_persist::BUILD_VERSION, 0, None)
            .expect_err("must be refused"),
        CodecError::SectionNotInFormat {
            tag: SECTION_ACTION_CENSUS,
            format: FORMAT_VERSION_3,
        }
    );
}
