//! Phase 4 persistence tests: golden snapshots, hostile input, crash
//! simulation, catalog consistency, and restore-from-backup.

use sim_core::{SimConfig, World};
use sim_persist::{
    CodecError, FORMAT_VERSION, SnapshotStore, StoreError, decode_snapshot, encode_snapshot,
    migration_for, read_info,
};
use std::fs;
use std::path::PathBuf;

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn scratch_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("lifesim-persist-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

fn phase2_world(ticks: u64) -> World {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 60;
    config.max_entities = 600;
    let mut world = World::new(config).unwrap();
    for _ in 0..ticks {
        world.step();
    }
    world
}

fn snapshot_bytes(world: &World, compression: Option<i32>) -> Vec<u8> {
    encode_snapshot(
        &world.export_state(),
        1,
        0,
        world.state_checksum(),
        "test-build",
        0,
        compression,
    )
    .expect("encode")
}

#[test]
fn golden_snapshot_round_trips_compressed_and_uncompressed() {
    let world = phase2_world(300);
    let checksum = world.state_checksum();
    for compression in [None, Some(3)] {
        let bytes = snapshot_bytes(&world, compression);
        let info = read_info(&bytes).expect("header");
        assert_eq!(info.format_version, FORMAT_VERSION);
        assert_eq!(info.compressed, compression.is_some());
        assert_eq!(info.tick, world.tick_number());
        assert_eq!(info.seed, SEED);
        assert_eq!(info.state_checksum, checksum);
        assert_eq!(info.build_version, "test-build");

        let (_, state) = decode_snapshot(&bytes).expect("decode");
        let restored = World::from_state(state).expect("restore");
        assert_eq!(restored.state_checksum(), checksum);
        assert_eq!(restored.population(), world.population());
    }
    // Encoding is deterministic for the same state.
    assert_eq!(snapshot_bytes(&world, None), snapshot_bytes(&world, None));
}

#[test]
fn corrupt_truncated_and_oversized_snapshots_fail_closed() {
    let world = phase2_world(50);
    let valid = snapshot_bytes(&world, Some(3));

    // Truncations at every boundary class.
    for cut in [0, 3, 20, 111, valid.len() - 1] {
        assert!(read_info(&valid[..cut.min(valid.len())]).is_err());
    }

    // Magic corruption.
    let mut bad_magic = valid.clone();
    bad_magic[0] = b'X';
    assert!(matches!(read_info(&bad_magic), Err(CodecError::BadMagic)));

    // Unknown format version.
    let mut bad_format = valid.clone();
    bad_format[4..6].copy_from_slice(&99_u16.to_le_bytes());
    assert!(matches!(
        read_info(&bad_format),
        Err(CodecError::UnsupportedFormat(99))
    ));

    // Payload bit flip is caught by the payload checksum.
    let mut flipped = valid.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 0x01;
    assert!(matches!(
        read_info(&flipped),
        Err(CodecError::PayloadChecksumMismatch)
    ));

    // Oversized declared lengths are rejected before allocation.
    let mut oversized = valid.clone();
    oversized[64..72].copy_from_slice(&u64::MAX.to_le_bytes()); // uncompressed_len
    assert!(matches!(
        read_info(&oversized),
        Err(CodecError::UncompressedTooLarge(_)) | Err(CodecError::LengthMismatch { .. })
    ));

    // Seeded corruption sweep across the whole file.
    let mut state = 0x00de_fec8_ab1e_5eed_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut rejected = 0;
    for _ in 0..2_000 {
        let mut bytes = valid.clone();
        for _ in 0..1 + next() % 4 {
            let position = (next() % bytes.len() as u64) as usize;
            bytes[position] ^= 1 << (next() % 8);
        }
        if decode_snapshot(&bytes).is_err() {
            rejected += 1;
        }
    }
    assert!(rejected > 1_990, "rejected only {rejected}/2000");
}

#[test]
fn migration_registry_fails_closed_for_unknown_versions() {
    assert!(migration_for(FORMAT_VERSION).is_ok());
    assert!(migration_for(0).is_err());
    assert!(migration_for(FORMAT_VERSION + 1).is_err());
}

#[test]
fn store_saves_atomically_and_interrupted_writes_never_win() {
    let directory = scratch_dir("crash");
    let (store, _) = SnapshotStore::open(&directory).unwrap();
    let world = phase2_world(200);

    // Checkpoint 1: valid.
    let record = store
        .save(
            &world.export_state(),
            world.state_checksum(),
            1,
            0,
            "auto",
            "checkpoint",
            0,
            Some(3),
        )
        .unwrap();

    // Simulate an interrupted second save: a partial temp file appears but
    // no rename or catalog commit ever happens.
    let partial = snapshot_bytes(&world, Some(3));
    fs::write(
        directory.join("world1-tick999999999999-crash.alif.tmp"),
        &partial[..partial.len() / 3],
    )
    .unwrap();
    // Also simulate a catalog row whose file was later corrupted.
    let corrupt_record = store
        .save(
            &world.export_state(),
            world.state_checksum(),
            1,
            0,
            "auto",
            "checkpoint",
            0,
            Some(3),
        )
        .unwrap();
    fs::write(directory.join(&corrupt_record.path), b"garbage").unwrap();

    // Recovery: reopen the store.
    drop(store);
    let (store, report) = SnapshotStore::open(&directory).unwrap();
    assert_eq!(report.removed_temp_files, 1);
    assert_eq!(report.broken_saves, 1);
    assert_eq!(report.valid_saves, 1);

    // The last valid checkpoint is still the first save.
    let latest = store.latest_checkpoint().unwrap().expect("checkpoint");
    assert_eq!(latest.save_id, record.save_id);
    assert_eq!(latest.state_checksum, world.state_checksum());
    // And it verifies through an isolated restore.
    let verify = store.verify(latest.save_id).unwrap();
    assert_eq!(verify.state_checksum, world.state_checksum());
    assert_eq!(verify.tick, world.tick_number());
}

#[test]
fn restore_from_backup_set_preserves_provenance_and_leaves_source_untouched() {
    // Build a "backup set": snapshot files plus catalog, copied wholesale
    // to an isolated restore target, per the backup runbook.
    let source = scratch_dir("backup-source");
    let (store, _) = SnapshotStore::open(&source).unwrap();
    let mut world = phase2_world(400);
    let record = store
        .save(
            &world.export_state(),
            world.state_checksum(),
            1,
            0,
            "named-backup",
            "manual",
            0,
            Some(3),
        )
        .unwrap();
    let source_listing: Vec<String> = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();

    // Transfer the whole recovery set to an isolated destination.
    let target = scratch_dir("backup-target");
    for entry in fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }

    // Isolated restore and provenance comparison.
    let (restored_store, report) = SnapshotStore::open(&target).unwrap();
    assert_eq!(report.valid_saves, 1);
    let restored_record = restored_store.list().unwrap().remove(0);
    assert_eq!(restored_record.state_checksum, record.state_checksum);
    let verify = restored_store.verify(restored_record.save_id).unwrap();
    assert_eq!(verify.world_id, 1);
    assert_eq!(verify.tick, world.tick_number());
    assert_eq!(verify.seed, SEED);
    assert_eq!(verify.config_hash, world.config_hash());
    assert_eq!(verify.state_checksum, world.state_checksum());

    // The restored world continues identically to the original.
    let (_, mut branched) = SnapshotStore::load_world(&target.join(&restored_record.path)).unwrap();
    for _ in 0..100 {
        world.step();
        branched.step();
    }
    assert_eq!(branched.state_checksum(), world.state_checksum());

    // Source set unchanged by the test restore.
    let after_listing: Vec<String> = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(source_listing.len(), after_listing.len());
}

#[test]
fn checkpoint_pruning_keeps_newest_and_named_saves() {
    let directory = scratch_dir("prune");
    let (store, _) = SnapshotStore::open(&directory).unwrap();
    let mut world = phase2_world(10);
    let mut checkpoint_ids = Vec::new();
    for _ in 0..5 {
        for _ in 0..20 {
            world.step();
        }
        let record = store
            .save(
                &world.export_state(),
                world.state_checksum(),
                1,
                0,
                "auto",
                "checkpoint",
                0,
                Some(1),
            )
            .unwrap();
        checkpoint_ids.push(record.save_id);
    }
    let named = store
        .save(
            &world.export_state(),
            world.state_checksum(),
            1,
            0,
            "keep-me",
            "manual",
            0,
            Some(1),
        )
        .unwrap();

    let removed = store.prune_checkpoints(2).unwrap();
    assert_eq!(removed, 3);
    let remaining = store.list().unwrap();
    let checkpoints: Vec<i64> = remaining
        .iter()
        .filter(|record| record.kind == "checkpoint")
        .map(|record| record.save_id)
        .collect();
    assert_eq!(checkpoints.len(), 2);
    // Newest two checkpoints survive.
    assert!(checkpoints.contains(&checkpoint_ids[4]));
    assert!(checkpoints.contains(&checkpoint_ids[3]));
    // Named saves are never pruned.
    assert!(
        remaining
            .iter()
            .any(|record| record.save_id == named.save_id)
    );
}

#[test]
fn verify_detects_checksum_forgery() {
    let directory = scratch_dir("forgery");
    let (store, _) = SnapshotStore::open(&directory).unwrap();
    let world = phase2_world(100);
    // Record a deliberately wrong checksum: verification must fail even
    // though the file itself decodes.
    let record = store
        .save(
            &world.export_state(),
            world.state_checksum() ^ 1,
            1,
            0,
            "forged",
            "manual",
            0,
            None,
        )
        .unwrap();
    assert!(matches!(
        store.verify(record.save_id),
        Err(StoreError::ChecksumMismatch { .. })
    ));
}
