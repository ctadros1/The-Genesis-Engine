//! Persistence adapter (Phase 4): versioned ALIF snapshots, atomic file
//! writes, SQLite catalog, recovery scan, isolated restore verification,
//! and the explicit migration registry.
//!
//! Boundary rules: this crate encodes/decodes documented logical state
//! only (`sim_core::SaveState`) and never alters meaning during load;
//! the kernel stays free of files and clocks.

mod checkpoint;
mod codec;
mod eventlog;
mod founders;
mod spatial;
mod store;

pub use checkpoint::{AsyncCheckpointer, CheckpointOutcome, CheckpointRequest, SubmitResult};

pub use codec::{
    CodecError, FLAG_ZSTD, FORMAT_VERSION, MAX_STORED_LEN, MAX_UNCOMPRESSED_LEN, SNAPSHOT_MAGIC,
    SnapshotInfo, crc32, decode_snapshot, encode_snapshot, read_info,
};
pub use eventlog::{
    EVENT_LOG_FORMAT_VERSION, EVENT_LOG_MAGIC, EventLogError, EventLogInfo, EventLogRecorder,
    EventLogScan, EventLogWriter, MAX_SEGMENT_BODY_LEN, ReconcileError, ReconstructedCounters,
    decode_log, decode_log_events, decode_log_prefix, encode_segment, read_log_info,
};
pub use founders::{
    FOUNDER_FORMAT_VERSION, FOUNDER_MAGIC, FounderError, FounderProvenance, FounderSet,
    MAX_FOUNDERS, decode_founders, encode_founders,
};
pub use spatial::{
    MAX_SAMPLE_ORGANISMS, SPATIAL_LOG_FORMAT_VERSION, SPATIAL_LOG_MAGIC, SpatialLogError,
    SpatialLogInfo, SpatialLogScan, SpatialLogWriter, SpatialSample, decode_spatial,
    decode_spatial_prefix, read_spatial_info,
};
pub use store::{
    BUILD_VERSION, RecoveryReport, SaveRecord, SnapshotStore, StoreError, VerifyReport,
    migration_for,
};
