//! ALIF format 4: the terrain-modification section, the composed terrain
//! checksum, and the registered format 3 migration. C12.5 end to end.
//!
//! Everything here goes through the **file**, not through `export_state`
//! followed by `from_state`. That distinction is not pedantry: the Phase 9
//! logical round trip passed for a whole phase while the encoded path was
//! broken, because the Phase 2 section drove its per-organism loop from a
//! count that is zero in a schema-2 world. A section can be written, read,
//! and validated perfectly at the logical level and still never reach a byte.
//!
//! # What C12.5 asks for, and where each clause is
//!
//! | Clause | Test |
//! |---|---|
//! | Baseline check still fails closed on a `(seed, config)` mismatch | `the_baseline_check_still_fails_closed_through_the_file` |
//! | Composed check fails closed on a tampered modification section | `a_tampered_modification_section_is_refused_by_the_composed_check` |
//! | Sparse and dense restore to identical worlds | `the_two_representations_restore_to_the_same_world` |
//! | A world crossing the density threshold mid-run continues bit-identically | `a_world_that_crosses_the_density_threshold_saves_and_continues_identically` |
//! | The migration yields a world byte-identical to a format 3 load | `the_format_3_migration_is_byte_identical_to_a_format_3_load` |

use sim_core::{LAYER_CAPACITY_SCALE, RestoreError, SaveState, SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION, FORMAT_VERSION_3, SAVE_STATE_VERSION_3, StoreError,
    decode_snapshot, decode_snapshot_format3, encode_snapshot, encode_snapshot_format3,
    migration_for,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const ONE_Q16: u32 = 65_536;
/// Header offsets, from the fixed 112-byte layout in `codec.rs`.
const OFFSET_FORMAT: usize = 4;
const OFFSET_SAVE_STATE_VERSION: usize = 52;
const OFFSET_UNCOMPRESSED_LEN: usize = 68;
const OFFSET_STORED_LEN: usize = 76;
const OFFSET_PAYLOAD_CRC: usize = 84;
const HEADER_LEN: usize = 112;

fn payload_start() -> usize {
    HEADER_LEN + sim_persist::BUILD_VERSION.len()
}

/// A world with the relocating patch live, so its modification set is
/// nonempty and moves.
///
/// `dense_threshold_q16` is a parameter because it is the only knob that
/// selects a representation, and two of the tests below need to pin one.
fn patch_config(radius: u32, interval: u64, threshold_q16: u32) -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    config.worldmod.enabled = true;
    config.worldmod.patch_enabled = true;
    config.worldmod.patch_radius_cells = radius;
    config.worldmod.relocate_interval_ticks = interval;
    config.worldmod.patch_capacity_scale_q16 = 3 * ONE_Q16;
    config.worldmod.dense_threshold_q16 = threshold_q16;
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

/// A patch world whose modification set is guaranteed nonempty.
///
/// The guard matters for the same reason `learned_world`'s does: every
/// assertion below about the modification section is an assertion about an
/// empty section otherwise, and an empty section round-trips through a codec
/// that never wrote it.
fn modified_world(radius: u32, interval: u64, threshold_q16: u32, ticks: u64) -> World {
    let world = advance(patch_config(radius, interval, threshold_q16), ticks);
    let overrides = world
        .worldmod_state()
        .expect("the section is enabled")
        .layer_len(LAYER_CAPACITY_SCALE);
    assert!(
        overrides > 16,
        "the patch wrote only {overrides} overrides, so every assertion about the \
         modification section below would be an assertion about an empty one"
    );
    world
}

fn encode(state: &SaveState, checksum: u64) -> Vec<u8> {
    encode_snapshot(state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None).expect("encode")
}

/// Offsets of every section in an uncompressed snapshot, walked from the
/// payload start rather than searched for, so this cannot match a tag-shaped
/// value inside a genome. Returns `(tag, flags, body_start, body_len)`.
fn sections(bytes: &[u8], payload_start: usize) -> Vec<(u16, u16, usize, usize)> {
    let mut out = Vec::new();
    let mut offset = payload_start;
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

fn section(bytes: &[u8], tag: u16) -> (u16, usize, usize) {
    let (_, flags, body_start, body_len) = sections(bytes, payload_start())
        .into_iter()
        .find(|(found, _, _, _)| *found == tag)
        .unwrap_or_else(|| panic!("section {tag} is present"));
    (flags, body_start, body_len)
}

/// Re-seal a snapshot whose section body was patched in place: the section
/// CRC that covers it, then the payload CRC in the header.
///
/// Without this every "rejection" below would be a CRC failure, and the test
/// would prove CRC32 works rather than that the loader's check does.
fn reseal(bytes: &mut [u8], body_start: usize, body_len: usize) {
    let body = bytes[body_start..body_start + body_len].to_vec();
    let section_crc = sim_persist::crc32(&body);
    bytes[body_start + body_len..body_start + body_len + 4]
        .copy_from_slice(&section_crc.to_le_bytes());
    let payload = bytes[payload_start()..].to_vec();
    let payload_crc = sim_persist::crc32(&payload);
    bytes[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
}

/// Replace one section's body with a different-length one, fixing the
/// declared section length, the section CRC, both payload length words in the
/// header, and the payload CRC - so that nothing but the loader's own
/// structural checks is left to catch the change.
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
    let payload_crc = sim_persist::crc32(&payload);
    out[OFFSET_PAYLOAD_CRC..OFFSET_PAYLOAD_CRC + 4].copy_from_slice(&payload_crc.to_le_bytes());
    out
}

// --- C12.5 clause 1: the baseline check is untouched -------------------------

/// **The property the whole design exists to preserve**: a save whose
/// `(seed, config)` regenerates a different baseline is still refused with
/// the same typed error, and the modification section changes nothing about
/// that.
///
/// Two clauses, because the two ways to break the identity fail at different
/// places. A different seed regenerates a different world; a doctored
/// checksum in the file claims a world that was never generated. Both must
/// land on `TerrainChecksumMismatch` and neither may be absorbed by the new
/// composed check, which runs later and would otherwise report a
/// seed mismatch as a tampered delta.
#[test]
fn the_baseline_check_still_fails_closed_through_the_file() {
    let world = modified_world(20, 200, ONE_Q16, 1_200);
    let state = world.export_state();
    let bytes = encode(&state, world.state_checksum());
    // Non-vacuity: the unpatched file must restore, or every refusal below
    // could be caused by something else entirely.
    let (_, clean) = decode_snapshot(&bytes).expect("decode");
    World::from_state(clean).expect("the unpatched save restores");

    // 1. A different seed. Everything else about the save is untouched, so
    //    the only thing that changed is the world worldgen produces.
    let mut reseeded = state.clone();
    reseeded.config.world_seed ^= 0xff;
    let bytes = encode(&reseeded, world.state_checksum());
    let (_, decoded) = decode_snapshot(&bytes).expect("decode");
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::TerrainChecksumMismatch { .. })
        ),
        "a save presented against a different seed was not refused by the baseline check"
    );

    // 2. The recorded baseline checksum, rewritten in the file itself. It
    //    lives in the world-metadata section body at offset 18: tick (8),
    //    paused (1), extinct (1), next_entity_id (8).
    let mut patched = encode(&state, world.state_checksum());
    let (_, body_start, body_len) = section(&patched, 2);
    let forged = (state.terrain_checksum ^ 0xdead_beef).to_le_bytes();
    patched[body_start + 18..body_start + 26].copy_from_slice(&forged);
    reseal(&mut patched, body_start, body_len);
    let (_, decoded) = decode_snapshot(&patched).expect("a resealed file decodes");
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::TerrainChecksumMismatch { .. })
        ),
        "a forged baseline checksum was accepted"
    );
}

// --- C12.5 clause 2: the composed check ------------------------------------

/// A modification section altered after the save was written is refused, and
/// the error names the delta rather than the baseline.
///
/// Every tamper below is chosen to be **individually legal**: an in-domain
/// capacity scale replaced by another in-domain capacity scale, in the same
/// slot, keeping the set sorted and unique. Nothing else in the file
/// disagrees, the CRCs are resealed, and the baseline still regenerates
/// exactly - so the composed checksum is the only thing standing between a
/// silently different world and a restore.
#[test]
fn a_tampered_modification_section_is_refused_by_the_composed_check() {
    // Threshold at Q16 one means dense is chosen only for a layer with more
    // overrides than there are cells, which is impossible - so this file is
    // sparse everywhere and the offsets below are stable.
    let world = modified_world(20, 200, ONE_Q16, 1_200);
    let state = world.export_state();
    let bytes = encode(&state, world.state_checksum());
    let (flags, body_start, body_len) = section(&bytes, 13);
    assert_eq!(flags, 0, "this test reads a sparse body");
    let (_, clean) = decode_snapshot(&bytes).expect("decode");
    World::from_state(clean).expect("the unpatched save restores");

    // The body is: layer 0 count (8), layer 1 count (8), then layer 1's
    // entries as (cell u32, value i64). The first entry's value therefore
    // begins 20 bytes in. Layer 0 is empty in a patch world, which the
    // assertion below states rather than assumes.
    assert_eq!(
        u64::from_le_bytes(bytes[body_start..body_start + 8].try_into().unwrap()),
        0,
        "layer 0 has no producer yet, so a patch world writes none of it"
    );
    let entries = u64::from_le_bytes(bytes[body_start + 8..body_start + 16].try_into().unwrap());
    assert!(entries > 16, "the section carries {entries} entries");

    // 1. One stored override's value, replaced by another legal one.
    let mut patched = bytes.clone();
    let original = i64::from_le_bytes(
        patched[body_start + 20..body_start + 28]
            .try_into()
            .unwrap(),
    );
    assert_eq!(original, 3 * i64::from(ONE_Q16), "the patch scale");
    patched[body_start + 20..body_start + 28].copy_from_slice(&(2 * 65_536_i64).to_le_bytes());
    reseal(&mut patched, body_start, body_len);
    let (_, decoded) = decode_snapshot(&patched).expect("a resealed file decodes");
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::ComposedTerrainChecksumMismatch { .. })
        ),
        "an altered override value was accepted"
    );

    // 2. One stored override's *cell*, moved to another cell in range. The
    //    set stays sorted only if the new cell keeps its position, so this
    //    moves the last entry upward.
    let last = body_start + 16 + (entries as usize - 1) * 12;
    let mut patched = bytes.clone();
    let cell = u32::from_le_bytes(patched[last..last + 4].try_into().unwrap());
    let moved = cell + 1;
    assert!((moved as usize) < state.biomass_milli.len());
    patched[last..last + 4].copy_from_slice(&moved.to_le_bytes());
    reseal(&mut patched, body_start, body_len);
    let (_, decoded) = decode_snapshot(&patched).expect("a resealed file decodes");
    // Either the cell now duplicates its predecessor (refused as disorder) or
    // it is a different cell (refused by the composed checksum). Both are
    // fail-closed; which one fires depends on the neighbouring entry, and
    // pinning that would be pinning the seed's patch geometry.
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::ComposedTerrainChecksumMismatch { .. })
                | Err(RestoreError::StateInvalid(_))
        ),
        "an override moved to another cell was accepted"
    );

    // 3. The recorded composed checksum itself, at the end of the metadata
    //    section. Tampering with it is the mirror image of tampering with the
    //    delta and must fail the same way.
    let mut patched = bytes.clone();
    let (_, meta_start, meta_len) = section(&patched, 2);
    assert_eq!(
        meta_len, 34,
        "a snapshot with a modification section records the composed checksum"
    );
    let forged = (state.composed_terrain_checksum ^ 1).to_le_bytes();
    patched[meta_start + 26..meta_start + 34].copy_from_slice(&forged);
    reseal(&mut patched, meta_start, meta_len);
    let (_, decoded) = decode_snapshot(&patched).expect("a resealed file decodes");
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::ComposedTerrainChecksumMismatch { .. })
        ),
        "a forged composed checksum was accepted"
    );

    // 4. The section removed entirely from a world whose config says it
    //    exists. Presence is matched against the configuration exactly as
    //    every other optional section's is.
    let mut absent = state.clone();
    absent.worldmod = None;
    let bytes = encode(&absent, world.state_checksum());
    let (_, decoded) = decode_snapshot(&bytes).expect("decode");
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::StateInvalid(_))
        ),
        "a mutable world restored without its modification section"
    );
}

/// A composed checksum smuggled into the metadata of a world that has **no**
/// modification section is refused rather than ignored.
///
/// This is the clause that makes the composed check unconditional worth
/// having. The word is written only alongside a modification section, so the
/// lazy reading is "a world without one has nothing to check". It has: the
/// baseline is its composed value, and a file claiming otherwise is claiming
/// a world that does not exist.
#[test]
fn a_composed_checksum_on_a_world_without_a_modification_section_is_refused() {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    let world = advance(config, 200);
    assert!(world.worldmod_state().is_none());
    let state = world.export_state();
    let bytes = encode(&state, world.state_checksum());
    let (_, meta_start, meta_len) = section(&bytes, 2);
    assert_eq!(
        meta_len, 26,
        "a world with no modification section records no composed checksum"
    );

    let mut body = bytes[meta_start..meta_start + meta_len].to_vec();
    body.extend_from_slice(&(state.terrain_checksum ^ 0x5555).to_le_bytes());
    let forged = replace_section_body(&bytes, meta_start, meta_len, &body);
    let (_, decoded) = decode_snapshot(&forged).expect("a rebuilt file decodes");
    assert!(
        matches!(
            World::from_state(decoded),
            Err(RestoreError::ComposedTerrainChecksumMismatch { .. })
        ),
        "a composed checksum was accepted for a world that cannot have one"
    );

    // ...and the *correct* value in the same place is accepted, which is what
    // says the rejection above is about the value and not about the length.
    let mut body = bytes[meta_start..meta_start + meta_len].to_vec();
    body.extend_from_slice(&state.terrain_checksum.to_le_bytes());
    let honest = replace_section_body(&bytes, meta_start, meta_len, &body);
    let (_, decoded) = decode_snapshot(&honest).expect("decode");
    World::from_state(decoded).expect("the baseline is the composed value of an empty set");
}

// --- C12.5 clause 3: representation equivalence -----------------------------

/// The same modification set encoded sparse and dense restores to the same
/// world with the same composed checksum.
///
/// The two files are produced from **one** `SaveState`, differing only in the
/// threshold that selects the representation - and that field is normalized
/// away before the comparison, because it is folded into the config hash and
/// would otherwise make the two arms different experiments rather than two
/// encodings of one.
#[test]
fn the_two_representations_restore_to_the_same_world() {
    let world = modified_world(20, 200, ONE_Q16, 1_200);
    let checksum = world.state_checksum();
    let mut state = world.export_state();

    // Threshold Q16 one: dense would need more overrides than there are
    // cells. Threshold zero: any nonempty layer is dense.
    state.config.worldmod.dense_threshold_q16 = ONE_Q16;
    let sparse_bytes = encode(&state, checksum);
    state.config.worldmod.dense_threshold_q16 = 0;
    let dense_bytes = encode(&state, checksum);

    let sparse_flags = section(&sparse_bytes, 13).0;
    let dense_flags = section(&dense_bytes, 13).0;
    assert_eq!(sparse_flags, 0, "no layer should have been stored densely");
    // Layer 1 is the only nonempty one, so its bit and no other.
    assert_eq!(
        dense_flags,
        1 << LAYER_CAPACITY_SCALE,
        "exactly the nonempty layer should have been stored densely"
    );
    assert_ne!(
        sparse_bytes, dense_bytes,
        "the two representations produced identical files, so nothing was tested"
    );

    let (_, mut sparse) = decode_snapshot(&sparse_bytes).expect("decode sparse");
    let (_, mut dense) = decode_snapshot(&dense_bytes).expect("decode dense");
    assert_eq!(
        sparse.worldmod, dense.worldmod,
        "the two representations decoded to different modification sets"
    );
    // Normalize the one field that differs by construction, and then the two
    // logical states must be equal in every other respect.
    sparse.config.worldmod.dense_threshold_q16 = ONE_Q16;
    dense.config.worldmod.dense_threshold_q16 = ONE_Q16;
    assert_eq!(
        sparse, dense,
        "the two representations decoded to different states"
    );

    let mut from_sparse = World::from_state(sparse).expect("restore sparse");
    let mut from_dense = World::from_state(dense).expect("restore dense");
    assert_eq!(from_sparse.state_checksum(), from_dense.state_checksum());
    assert_eq!(
        from_sparse.composed_terrain_checksum(),
        from_dense.composed_terrain_checksum()
    );
    assert_eq!(
        from_sparse.composed_terrain_checksum(),
        state.composed_terrain_checksum,
        "neither representation reproduced the composed checksum that was saved"
    );
    // Cell by cell, not only by checksum: a hash equality is evidence that
    // two fields agree, and this is the field.
    for cell in 0..from_sparse.terrain().cell_count() {
        assert_eq!(
            from_sparse.effective_capacity_milli(cell),
            from_dense.effective_capacity_milli(cell),
            "cell {cell}"
        );
        assert_eq!(
            from_sparse.effective_traversable(cell),
            from_dense.effective_traversable(cell)
        );
    }
    for _ in 0..300 {
        from_sparse.step();
        from_dense.step();
    }
    assert_eq!(
        from_sparse.state_checksum(),
        from_dense.state_checksum(),
        "two worlds restored from the two representations diverged"
    );
}

// --- C12.5 clause 4: crossing the threshold mid-run -------------------------

/// A world whose modification set crosses the density threshold during a run
/// saves, restores, and continues bit-identically at every point - including
/// the saves on either side of the crossing.
///
/// **The threshold is chosen from the run rather than guessed.** A hard-coded
/// one makes the test a hostage to the seed's patch geometry: if every save
/// landed on the same side of it the test would pass having encoded one
/// representation throughout, which is precisely the thing it exists to rule
/// out. Pass one measures the override counts the schedule actually produces,
/// pass two pins a threshold strictly between the smallest and the largest,
/// and the assertion is that both representations were exercised.
#[test]
fn a_world_that_crosses_the_density_threshold_saves_and_continues_identically() {
    const RADIUS: u32 = 20;
    const INTERVAL: u64 = 100;
    const SAVES: usize = 12;
    let cells = 64 * 64;

    // Pass one: what does the schedule do? The threshold plays no part in
    // the tick, so this run is the same world the second pass runs.
    let mut probe = advance(patch_config(RADIUS, INTERVAL, ONE_Q16), INTERVAL);
    let mut counts = Vec::new();
    for _ in 0..SAVES {
        for _ in 0..INTERVAL {
            probe.step();
        }
        counts.push(
            probe
                .worldmod_state()
                .expect("section")
                .layer_len(LAYER_CAPACITY_SCALE),
        );
    }
    let low = *counts.iter().min().expect("saves");
    let high = *counts.iter().max().expect("saves");
    assert!(
        low < high,
        "the patch covered exactly {low} cells at every save, so no threshold \
         could put saves on both sides of it"
    );
    // Strictly between the two, expressed as a Q16 fraction of the cell
    // count: a layer with `low` overrides encodes sparse and one with `high`
    // encodes dense.
    let midpoint = (low + high) / 2;
    let threshold = ((midpoint as u64 * 65_536) / cells as u64) as u32;
    assert!(threshold <= ONE_Q16);

    let mut world = advance(patch_config(RADIUS, INTERVAL, threshold), INTERVAL);
    let mut seen_sparse = 0;
    let mut seen_dense = 0;
    for save in 0..SAVES {
        for _ in 0..INTERVAL {
            world.step();
        }
        let state = world.export_state();
        let checksum = world.state_checksum();
        let bytes = encode(&state, checksum);
        let flags = section(&bytes, 13).0;
        if flags & (1 << LAYER_CAPACITY_SCALE) == 0 {
            seen_sparse += 1;
        } else {
            seen_dense += 1;
        }

        let (info, decoded) = decode_snapshot(&bytes).expect("decode");
        assert_eq!(info.format_version, FORMAT_VERSION);
        let mut restored = World::from_state(decoded).expect("restore");
        assert_eq!(
            restored.state_checksum(),
            checksum,
            "save {save} restored to a different checksum"
        );
        assert_eq!(
            restored.composed_terrain_checksum(),
            world.composed_terrain_checksum()
        );
        // Continue on both sides, across at least one further relocation, so
        // the check covers the schedule resuming rather than only the state
        // landing.
        let mut reference = World::from_state(state).expect("logical restore");
        for _ in 0..(INTERVAL + 25) {
            restored.step();
            reference.step();
        }
        assert_eq!(
            restored.state_checksum(),
            reference.state_checksum(),
            "save {save} diverged after restore"
        );
    }
    assert!(
        seen_sparse > 0 && seen_dense > 0,
        "the run encoded {seen_sparse} sparse and {seen_dense} dense saves, so it never \
         crossed the threshold and this test checked one representation twelve times"
    );
}

// --- C12.5 clause 5: the registered migration -------------------------------

/// A format 3 file loads through the registered transform and yields exactly
/// what the format 3 reader yields.
///
/// **The comparison routes through the new codec, which is the only version
/// of it worth running.** The transform decodes with the legacy reader and
/// re-encodes at format 4; this decodes *that* with the current reader and
/// compares against the legacy reader's own output. A test that compared the
/// transform's intermediate state against the legacy reader would be
/// comparing a function against the function it calls.
#[test]
fn the_format_3_migration_is_byte_identical_to_a_format_3_load() {
    // A world with the section disabled, because that is the only kind of
    // world a format 3 file can describe.
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    let world = advance(config, 400);
    assert!(world.worldmod_state().is_none());
    let state = world.export_state();
    let checksum = world.state_checksum();

    let legacy =
        encode_snapshot_format3(&state, 7, 3, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 3");
    assert_eq!(
        u16::from_le_bytes(legacy[OFFSET_FORMAT..OFFSET_FORMAT + 2].try_into().unwrap()),
        FORMAT_VERSION_3
    );
    assert_eq!(
        u16::from_le_bytes(
            legacy[OFFSET_SAVE_STATE_VERSION..OFFSET_SAVE_STATE_VERSION + 2]
                .try_into()
                .unwrap()
        ),
        SAVE_STATE_VERSION_3
    );

    // The current reader refuses it, which is what makes the registry
    // load-bearing rather than decorative.
    assert_eq!(
        decode_snapshot(&legacy).err(),
        Some(CodecError::UnsupportedFormat(FORMAT_VERSION_3))
    );

    let migration = migration_for(FORMAT_VERSION_3)
        .expect("format 3 is registered")
        .expect("format 3 needs a transform");
    assert_eq!(migration.from_format, FORMAT_VERSION_3);
    assert_eq!(migration.to_format, FORMAT_VERSION);
    assert_eq!(
        migration.expected_loss, "",
        "the 3 to 4 transform invents nothing and must not claim to"
    );
    let migrated = (migration.transform)(&legacy).expect("the transform runs");
    assert_eq!(migrated.source.format_version, FORMAT_VERSION_3);
    assert_eq!(migrated.source.world_id, 7);
    assert_eq!(migrated.source.parent_world_id, 3);

    // The byte-identity requirement, stated as `SaveState` equality because
    // `SaveState` derives `PartialEq` and covers every field of the logical
    // state including the config.
    let (legacy_info, legacy_state) = decode_snapshot_format3(&legacy).expect("legacy decode");
    let (migrated_info, migrated_state) =
        decode_snapshot(&migrated.bytes).expect("the migrated file decodes at format 4");
    assert_eq!(migrated_info.format_version, FORMAT_VERSION);
    assert_eq!(migrated_state, legacy_state, "the migrated state differs");
    assert_eq!(
        migrated_state, state,
        "neither path reproduced the original"
    );
    assert_eq!(migrated_state.worldmod, None);
    assert_eq!(
        migrated_state.composed_terrain_checksum, migrated_state.terrain_checksum,
        "a file that predates the layer must compose to its own baseline"
    );
    assert_eq!(legacy_info.state_checksum, migrated_info.state_checksum);
    assert_eq!(legacy_info.terrain_checksum, migrated_info.terrain_checksum);
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
        "a migrated world diverged from the world a format 3 load produces"
    );
}

/// Formats 1 and 2 have no migration and every unknown version fails closed.
#[test]
fn the_registry_registers_exactly_one_transform_and_refuses_everything_else() {
    assert!(
        migration_for(FORMAT_VERSION)
            .expect("the current format is not an error")
            .is_none(),
        "the current format must not route through a transform"
    );
    assert!(
        migration_for(FORMAT_VERSION_3)
            .expect("registered")
            .is_some()
    );
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

/// A file whose header says format 3 and whose body carries a format 4
/// section is refused rather than read leniently.
///
/// The header is not checksummed - only the payload is - so rewriting two
/// version words is all it takes to produce this file, and a reader that
/// parsed sections without regard to the version it was told would happily
/// accept it and then migrate a world that never existed.
#[test]
fn a_format_3_file_carrying_a_format_4_section_is_refused() {
    let world = modified_world(20, 200, ONE_Q16, 600);
    let state = world.export_state();
    let mut forged = encode(&state, world.state_checksum());
    forged[OFFSET_FORMAT..OFFSET_FORMAT + 2].copy_from_slice(&FORMAT_VERSION_3.to_le_bytes());
    forged[OFFSET_SAVE_STATE_VERSION..OFFSET_SAVE_STATE_VERSION + 2]
        .copy_from_slice(&SAVE_STATE_VERSION_3.to_le_bytes());
    assert_eq!(
        decode_snapshot_format3(&forged).err(),
        Some(CodecError::SectionNotInFormat {
            tag: 13,
            format: FORMAT_VERSION_3
        })
    );
    let migration = migration_for(FORMAT_VERSION_3)
        .expect("registered")
        .expect("some");
    assert!(
        matches!(
            (migration.transform)(&forged),
            Err(StoreError::Codec(CodecError::SectionNotInFormat { .. }))
        ),
        "the transform accepted a file that lies about its own version"
    );

    // The writer refuses the mirror image: a state carrying a modification
    // section cannot be written as format 3, because format 3 cannot express
    // one and dropping it silently is the "never alter meaning" rule broken
    // on the way out.
    assert!(matches!(
        encode_snapshot_format3(
            &state,
            1,
            0,
            world.state_checksum(),
            sim_persist::BUILD_VERSION,
            0,
            None
        ),
        Err(CodecError::SectionNotInFormat { .. })
    ));
}

// --- C12.8 at the file level ------------------------------------------------

/// **A world with the section disabled encodes exactly the payload format 3
/// wrote for it**: the two files differ in their format and logical-state
/// version words and in nothing else.
///
/// This is C12.8's "a disabled world encodes as it always did" stated as an
/// assertion rather than an argument, and it is also what makes the migration
/// test above possible: without it there would be no way to produce a real
/// format 3 file to migrate, and the test would have to migrate a file it had
/// synthesized to match its own expectations.
#[test]
fn a_disabled_world_encodes_the_payload_format_3_wrote() {
    // The widest world that generates cleanly, so the payload being compared
    // carries as many optional sections as possible. Seed 3 and the default
    // map size for the reason `config_round_trip.rs` gives: roughly a quarter
    // of seeds produce a map with no `Arid` cells, which world generation
    // correctly refuses, and a shrunken map refuses far more often than that.
    let mut config = SimConfig::phase2_default(3);
    config.initial_organisms = 200;
    config.climate.enabled = true;
    config.climate.worldgen_version = sim_core::WorldgenVersion::V2;
    config.contest.enabled = true;
    config.physiology.enabled = true;
    let world = advance(config, 300);
    assert!(world.worldmod_state().is_none());
    let state = world.export_state();
    let checksum = world.state_checksum();

    let current = encode(&state, checksum);
    let legacy =
        encode_snapshot_format3(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None)
            .expect("encode format 3");
    assert_eq!(current.len(), legacy.len());
    let differing: Vec<usize> = (0..current.len())
        .filter(|index| current[*index] != legacy[*index])
        .collect();
    assert_eq!(
        differing,
        vec![OFFSET_FORMAT, OFFSET_SAVE_STATE_VERSION],
        "a format 4 snapshot of a world with no modification section must differ from \
         the format 3 snapshot of the same world in the two version words alone"
    );
}

// --- Standing rule 2: the adversarial decode sweep --------------------------

/// Every declared count **inside** the modification section is bounded before
/// allocation, in both representations, and for every layer.
///
/// `config_round_trip.rs`'s sweep patches the first count word of each
/// count-led section, which here is layer 0's. This one reaches layers 1 and
/// 2 - the counts a per-layer loop makes easy to leave unguarded, and the
/// ones that carry the actual data.
#[test]
fn every_per_layer_count_in_the_modification_section_is_bounded() {
    for threshold in [ONE_Q16, 0] {
        let world = modified_world(20, 200, threshold, 1_200);
        let state = world.export_state();
        let bytes = encode(&state, world.state_checksum());
        let (_, body_start, body_len) = section(&bytes, 13);
        assert!(
            decode_snapshot(&bytes).is_ok(),
            "the unpatched snapshot must decode, or every refusal below is vacuous"
        );

        // Each layer's count word, located by walking the body the way the
        // decoder does rather than by assuming a layout.
        let mut offset = body_start;
        for layer in 0..3_usize {
            for count in [u64::MAX, u64::MAX / 8, body_len as u64] {
                let mut patched = bytes.clone();
                patched[offset..offset + 8].copy_from_slice(&count.to_le_bytes());
                reseal(&mut patched, body_start, body_len);
                assert!(
                    decode_snapshot(&patched).is_err(),
                    "layer {layer} admitted a declared count of {count} against a \
                     {body_len}-byte body (threshold {threshold})"
                );
            }
            let declared =
                u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
            let dense = section(&bytes, 13).0 & (1 << layer) != 0;
            offset += 8 + declared * if dense { 8 } else { 12 };
        }
        // The walk must have landed exactly on the trailing scalars: the
        // i128 sink and nine u64 counters.
        assert_eq!(
            offset,
            body_start + body_len - (16 + 9 * 8),
            "the layout walk disagrees with the encoder, so the offsets patched above \
             were not the counts"
        );
    }
}

/// **Twenty thousand seeded corruptions of the modification section, every
/// one of them resealed, and not one produces a world that claims to be the
/// saved one.**
///
/// The reseal is the whole point and it is what the phase plan's "corruption
/// sweep" has to mean here. `persistence.rs`'s existing sweep flips bits and
/// lets CRC32 catch them, which measures CRC32; these flips land inside the
/// section body and both checksums are recomputed afterwards, so the bytes
/// reach the parser, the bounds, the ordering check, the domain check, and
/// the composed checksum in turn. A corruption survives only by being caught
/// by one of those or by the state checksum the store compares - and every
/// case must be caught by *something*.
///
/// Zero panics is asserted by the test completing: a panic in a decoder is a
/// fail-open into a crash, because the caller never sees the typed error it
/// is supposed to handle (D-091).
#[test]
fn twenty_thousand_corruptions_of_the_modification_section_never_pass_as_the_original() {
    let world = modified_world(20, 200, ONE_Q16, 1_200);
    let state = world.export_state();
    let recorded = world.state_checksum();
    let valid = encode(&state, recorded);
    let (_, body_start, body_len) = section(&valid, 13);

    let mut rng = 0x00de_fec8_ab1e_5eed_u64;
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
        for _ in 0..1 + next() % 4 {
            let position = body_start + (next() % body_len as u64) as usize;
            bytes[position] ^= 1 << (next() % 8);
        }
        if bytes == valid {
            // Two flips of the same bit. Not a corruption at all, counted so
            // the tallies below add up rather than being quietly absorbed.
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
        // The last line of defence, and the one the store applies: a world
        // that restored must not claim the checksum of the world that was
        // saved. This is where a corrupted *counter* lands - it changes no
        // terrain and breaks no invariant, and it is still a different world.
        assert_ne!(
            restored.state_checksum(),
            info.state_checksum,
            "a corrupted modification section restored to a world that passes as the \
             one that was saved"
        );
        checksum_caught += 1;
    }
    // Every stage must actually fire. A sweep where one number is zero is a
    // sweep that tested one mechanism twenty thousand times.
    assert!(decode_refused > 0, "no corruption reached a decode bound");
    assert!(
        restore_refused > 0,
        "no corruption reached the ordering, domain, or composed checks"
    );
    assert!(
        checksum_caught > 0,
        "no corruption survived to the state checksum, so that clause is untested"
    );
    println!(
        "worldmod corruption sweep: decode refused {decode_refused}, restore refused \
         {restore_refused}, caught by state checksum {checksum_caught}, no-op {identical}"
    );
}

/// The section flags word is validated against a per-tag whitelist rather
/// than read and discarded.
///
/// Every section has carried this word since format 1 and no reader had ever
/// looked at it: any value was accepted and ignored. It carries the
/// modification section's representation now, so a bit nobody defined has to
/// be a decode failure - otherwise a file could claim a representation the
/// reader silently disagrees with, which is the one thing a recorded
/// representation exists to prevent.
#[test]
fn a_section_flag_nobody_defined_is_refused() {
    let world = modified_world(20, 200, ONE_Q16, 600);
    let bytes = encode(&world.export_state(), world.state_checksum());
    assert!(decode_snapshot(&bytes).is_ok());

    // A section that defines no flags at all: the organism table. The flags
    // word lives inside the payload, so the payload CRC is resealed too -
    // otherwise every rejection here would be a checksum failure and the test
    // would say nothing about the whitelist.
    let mut patched = bytes.clone();
    let (_, organisms_body, organisms_len) = section(&patched, 3);
    patched[organisms_body - 10..organisms_body - 8].copy_from_slice(&1_u16.to_le_bytes());
    reseal(&mut patched, organisms_body, organisms_len);
    assert_eq!(
        decode_snapshot(&patched).err(),
        Some(CodecError::UnknownSectionFlags { tag: 3, flags: 1 })
    );

    // ...and the modification section, whose three defined bits are the
    // three layers. Bit 3 is one past them.
    let mut patched = bytes.clone();
    let (flags, worldmod_body, worldmod_len) = section(&patched, 13);
    let bad = flags | (1 << 3);
    patched[worldmod_body - 10..worldmod_body - 8].copy_from_slice(&bad.to_le_bytes());
    reseal(&mut patched, worldmod_body, worldmod_len);
    assert_eq!(
        decode_snapshot(&patched).err(),
        Some(CodecError::UnknownSectionFlags {
            tag: 13,
            flags: bad
        })
    );

    // A *defined* bit on an empty layer is not an error - it is the dense
    // encoding of an empty layer, which is a legal thing to write - so the
    // rejection above is about the undefined bit and not about any change to
    // the word.
    let mut patched = bytes.clone();
    patched[worldmod_body - 10..worldmod_body - 8].copy_from_slice(&(flags | 1).to_le_bytes());
    reseal(&mut patched, worldmod_body, worldmod_len);
    // Layer 0's body is a sparse count of zero, which a dense reader reads as
    // a cell count of zero: an empty dense field. It decodes, and it decodes
    // to the same set.
    let (_, decoded) = decode_snapshot(&patched).expect("a defined flag bit is legal");
    let (_, clean) = decode_snapshot(&bytes).expect("decode");
    assert_eq!(decoded.worldmod, clean.worldmod);
}
