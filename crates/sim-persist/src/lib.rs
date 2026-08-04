//! Persistence adapter (Phase 4): versioned ALIF snapshots, atomic file
//! writes, SQLite catalog, recovery scan, isolated restore verification,
//! and the explicit migration registry.
//!
//! Boundary rules: this crate encodes/decodes documented logical state
//! only (`sim_core::SaveState`) and never alters meaning during load;
//! the kernel stays free of files and clocks.

mod codec;
mod store;

pub use codec::{
    CodecError, FLAG_ZSTD, FORMAT_VERSION, MAX_STORED_LEN, MAX_UNCOMPRESSED_LEN, SNAPSHOT_MAGIC,
    SnapshotInfo, crc32, decode_snapshot, encode_snapshot, read_info,
};
pub use store::{
    BUILD_VERSION, RecoveryReport, SaveRecord, SnapshotStore, StoreError, VerifyReport,
    migration_for,
};
