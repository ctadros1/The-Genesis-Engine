//! Persistence adapter (Phase 4): versioned ALIF snapshots, atomic file
//! writes, SQLite catalog, recovery scan, isolated restore verification,
//! and the explicit migration registry.
//!
//! Boundary rules: this crate encodes/decodes documented logical state
//! only (`sim_core::SaveState`) and never alters meaning during load;
//! the kernel stays free of files and clocks.

mod actionlog;
mod checkpoint;
mod codec;
mod eventlog;
mod founders;
mod spatial;
mod store;

pub use actionlog::{
    ACTION_LOG_FORMAT_VERSION, ACTION_LOG_MAGIC, ActionLogError, ActionLogInfo, ActionLogScan,
    ActionLogWriter, ActionRecord, ActionSampleSet, decode_action, decode_action_prefix,
    encode_segment as encode_action_segment, policy_hash as action_policy_hash, read_action_info,
};
pub use checkpoint::{AsyncCheckpointer, CheckpointOutcome, CheckpointRequest, SubmitResult};

pub use codec::{
    CodecError, FLAG_ZSTD, FORMAT_VERSION, FORMAT_VERSION_3, FORMAT_VERSION_4, FORMAT_VERSION_5,
    FORMAT_VERSION_6, FORMAT_VERSION_7, FORMAT_VERSION_8, FORMAT_VERSION_9, FORMAT_VERSION_10,
    FORMAT_VERSION_11, FORMAT_VERSION_12, FORMAT_VERSION_13, FORMAT_VERSION_14, FORMAT_VERSION_15,
    FORMAT_VERSION_16,
    FORMAT7_CONFIG_BYTES,
    FORMAT8_CONFIG_BYTES, FORMAT9_CONFIG_BYTES, FORMAT10_CONFIG_BYTES, FORMAT11_CONFIG_BYTES,
    FORMAT12_CONFIG_BYTES, FORMAT13_CONFIG_BYTES, FORMAT14_CONFIG_BYTES, FORMAT15_CONFIG_BYTES,
    FORMAT16_CONFIG_BYTES,
    MAX_STORED_LEN, MAX_UNCOMPRESSED_LEN, SAVE_STATE_VERSION_3,
    SNAPSHOT_MAGIC, SnapshotInfo, crc32, decode_snapshot, decode_snapshot_format3,
    decode_snapshot_format4, decode_snapshot_format5, decode_snapshot_format6,
    decode_snapshot_format7, decode_snapshot_format8, decode_snapshot_format9,
    decode_snapshot_format10, decode_snapshot_format11, decode_snapshot_format12,
    decode_snapshot_format13, decode_snapshot_format14, decode_snapshot_format15, encode_snapshot,
    encode_snapshot_format3, encode_snapshot_format4,
    encode_snapshot_format5, encode_snapshot_format6, encode_snapshot_format7,
    encode_snapshot_format8, encode_snapshot_format9, encode_snapshot_format10,
    encode_snapshot_format11, encode_snapshot_format12, encode_snapshot_format13,
    encode_snapshot_format14, encode_snapshot_format15,
    peek_format_version,
    read_info,
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
    BUILD_VERSION, MigratedSave, Migration, RecoveryReport, SaveRecord, SnapshotStore, StoreError,
    VerifyReport, decode_snapshot_migrating, migration_for,
};
