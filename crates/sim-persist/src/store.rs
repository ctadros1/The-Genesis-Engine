//! Durable snapshot store: atomic file writes, SQLite catalog, recovery
//! scan, and the migration registry.
//!
//! Transaction ordering: the snapshot file is written to a temporary name,
//! flushed, atomically renamed, and the directory synced BEFORE the
//! catalog row commits. A catalog row therefore never references a file
//! that is not durable, and an interrupted save leaves only a `.tmp` file
//! that recovery ignores and removes.

use crate::codec::{self, CodecError, SnapshotInfo};
use rusqlite::Connection;
use sim_core::{SaveState, World};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const BUILD_VERSION: &str = concat!("lifesim-", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub enum StoreError {
    Io(String),
    Catalog(String),
    Codec(CodecError),
    Restore(sim_core::RestoreError),
    ChecksumMismatch { recorded: u64, restored: u64 },
    UnknownSave(i64),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::Catalog(error) => write!(formatter, "catalog: {error}"),
            Self::Codec(error) => write!(formatter, "codec: {error}"),
            Self::Restore(error) => write!(formatter, "restore: {error}"),
            Self::ChecksumMismatch { recorded, restored } => write!(
                formatter,
                "state checksum mismatch: recorded 0x{recorded:016x}, restored 0x{restored:016x}"
            ),
            Self::UnknownSave(id) => write!(formatter, "unknown save id {id}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<CodecError> for StoreError {
    fn from(error: CodecError) -> Self {
        Self::Codec(error)
    }
}

fn io_error(error: std::io::Error) -> StoreError {
    StoreError::Io(error.to_string())
}

fn catalog_error(error: rusqlite::Error) -> StoreError {
    StoreError::Catalog(error.to_string())
}

/// One catalog row describing a durable snapshot.
#[derive(Clone, Debug)]
pub struct SaveRecord {
    pub save_id: i64,
    pub world_id: u64,
    pub name: String,
    pub kind: String,
    pub path: String,
    pub tick: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub state_checksum: u64,
    pub format_version: u16,
    pub bytes: u64,
    pub compressed: bool,
    pub created_unix_ms: u64,
    pub verified_unix_ms: Option<u64>,
}

pub struct SnapshotStore {
    directory: PathBuf,
    catalog: Connection,
}

impl SnapshotStore {
    /// Open (or create) a store rooted at `directory`, then run the
    /// recovery scan: leftover temporary files are removed and catalog
    /// rows pointing at missing/invalid files are marked broken so the
    /// latest valid checkpoint stays authoritative.
    pub fn open(directory: &Path) -> Result<(Self, RecoveryReport), StoreError> {
        fs::create_dir_all(directory).map_err(io_error)?;
        let catalog = Connection::open(directory.join("catalog.sqlite")).map_err(catalog_error)?;
        catalog
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS saves (
                    save_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    world_id INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    path TEXT NOT NULL,
                    tick INTEGER NOT NULL,
                    seed TEXT NOT NULL,
                    config_hash TEXT NOT NULL,
                    state_checksum TEXT NOT NULL,
                    format_version INTEGER NOT NULL,
                    bytes INTEGER NOT NULL,
                    compressed INTEGER NOT NULL,
                    created_unix_ms INTEGER NOT NULL,
                    verified_unix_ms INTEGER,
                    status TEXT NOT NULL DEFAULT 'valid'
                );",
            )
            .map_err(catalog_error)?;
        let store = Self {
            directory: directory.to_owned(),
            catalog,
        };
        let report = store.recover()?;
        Ok((store, report))
    }

    fn recover(&self) -> Result<RecoveryReport, StoreError> {
        let mut report = RecoveryReport::default();
        // 1. Remove interrupted temporary files.
        for entry in fs::read_dir(&self.directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "tmp") {
                let _ = fs::remove_file(&path);
                report.removed_temp_files += 1;
            }
        }
        // 2. Validate every catalog row's file header cheaply.
        let rows: Vec<(i64, String)> = {
            let mut statement = self
                .catalog
                .prepare("SELECT save_id, path FROM saves WHERE status = 'valid'")
                .map_err(catalog_error)?;
            let mapped = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(catalog_error)?;
            mapped.collect::<Result<_, _>>().map_err(catalog_error)?
        };
        for (save_id, path) in rows {
            let valid = fs::read(self.directory.join(&path))
                .ok()
                .and_then(|bytes| codec::read_info(&bytes).ok())
                .is_some();
            if valid {
                report.valid_saves += 1;
            } else {
                self.catalog
                    .execute(
                        "UPDATE saves SET status = 'broken' WHERE save_id = ?1",
                        [save_id],
                    )
                    .map_err(catalog_error)?;
                report.broken_saves += 1;
            }
        }
        Ok(report)
    }

    /// Save a captured state durably. Returns the catalog record. The
    /// file becomes durable before the catalog row exists.
    #[allow(clippy::too_many_arguments)]
    pub fn save(
        &self,
        state: &SaveState,
        state_checksum: u64,
        world_id: u64,
        parent_world_id: u64,
        name: &str,
        kind: &str,
        event_log_offset: u64,
        compression_level: Option<i32>,
    ) -> Result<SaveRecord, StoreError> {
        let created = now_ms();
        let file_name = format!("world{world_id}-tick{:012}-{created}.alif", state.tick);
        let bytes = codec::encode_snapshot(
            state,
            world_id,
            parent_world_id,
            state_checksum,
            BUILD_VERSION,
            event_log_offset,
            compression_level,
        )?;

        // Atomic write: temp file in the destination directory.
        let final_path = self.directory.join(&file_name);
        let temp_path = self.directory.join(format!("{file_name}.tmp"));
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
                .map_err(io_error)?;
            file.write_all(&bytes).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
        }
        fs::rename(&temp_path, &final_path).map_err(io_error)?;
        // Sync the directory so the rename itself is durable.
        if let Ok(directory) = File::open(&self.directory) {
            let _ = directory.sync_all();
        }

        // Only now commit catalog metadata.
        self.catalog
            .execute(
                "INSERT INTO saves (world_id, name, kind, path, tick, seed, config_hash,
                    state_checksum, format_version, bytes, compressed, created_unix_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    world_id as i64,
                    name,
                    kind,
                    file_name,
                    state.tick as i64,
                    format!("0x{:016x}", state.config.world_seed),
                    format!("0x{:016x}", state.config.stable_hash()),
                    format!("0x{state_checksum:016x}"),
                    codec::FORMAT_VERSION,
                    bytes.len() as i64,
                    compression_level.is_some(),
                    created as i64,
                ],
            )
            .map_err(catalog_error)?;
        let save_id = self.catalog.last_insert_rowid();
        Ok(SaveRecord {
            save_id,
            world_id,
            name: name.to_owned(),
            kind: kind.to_owned(),
            path: file_name,
            tick: state.tick,
            seed: state.config.world_seed,
            config_hash: state.config.stable_hash(),
            state_checksum,
            format_version: codec::FORMAT_VERSION,
            bytes: bytes.len() as u64,
            compressed: compression_level.is_some(),
            created_unix_ms: created,
            verified_unix_ms: None,
        })
    }

    pub fn list(&self) -> Result<Vec<SaveRecord>, StoreError> {
        let mut statement = self
            .catalog
            .prepare(
                "SELECT save_id, world_id, name, kind, path, tick, seed, config_hash,
                        state_checksum, format_version, bytes, compressed, created_unix_ms,
                        verified_unix_ms
                 FROM saves WHERE status = 'valid' ORDER BY save_id DESC",
            )
            .map_err(catalog_error)?;
        let rows = statement
            .query_map([], |row| {
                let seed: String = row.get(6)?;
                let config_hash: String = row.get(7)?;
                let checksum: String = row.get(8)?;
                Ok(SaveRecord {
                    save_id: row.get(0)?,
                    world_id: row.get::<_, i64>(1)? as u64,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    path: row.get(4)?,
                    tick: row.get::<_, i64>(5)? as u64,
                    seed: parse_hex(&seed),
                    config_hash: parse_hex(&config_hash),
                    state_checksum: parse_hex(&checksum),
                    format_version: row.get::<_, i64>(9)? as u16,
                    bytes: row.get::<_, i64>(10)? as u64,
                    compressed: row.get(11)?,
                    created_unix_ms: row.get::<_, i64>(12)? as u64,
                    verified_unix_ms: row.get::<_, Option<i64>>(13)?.map(|value| value as u64),
                })
            })
            .map_err(catalog_error)?;
        rows.collect::<Result<_, _>>().map_err(catalog_error)
    }

    pub fn latest_checkpoint(&self) -> Result<Option<SaveRecord>, StoreError> {
        Ok(self
            .list()?
            .into_iter()
            .find(|record| record.kind == "checkpoint"))
    }

    /// Retain only the newest `keep` checkpoints (named saves are kept).
    pub fn prune_checkpoints(&self, keep: usize) -> Result<u64, StoreError> {
        let checkpoints: Vec<SaveRecord> = self
            .list()?
            .into_iter()
            .filter(|record| record.kind == "checkpoint")
            .collect();
        let mut removed = 0;
        for record in checkpoints.into_iter().skip(keep) {
            let _ = fs::remove_file(self.directory.join(&record.path));
            self.catalog
                .execute(
                    "UPDATE saves SET status = 'pruned' WHERE save_id = ?1",
                    [record.save_id],
                )
                .map_err(catalog_error)?;
            removed += 1;
        }
        Ok(removed)
    }

    pub fn read_bytes(&self, record: &SaveRecord) -> Result<Vec<u8>, StoreError> {
        fs::read(self.directory.join(&record.path)).map_err(io_error)
    }

    /// Isolated restore verification: decode, rebuild a world, and compare
    /// the recorded state checksum and provenance. Never touches any live
    /// world; the verified world is dropped.
    pub fn verify(&self, save_id: i64) -> Result<VerifyReport, StoreError> {
        let record = self
            .list()?
            .into_iter()
            .find(|record| record.save_id == save_id)
            .ok_or(StoreError::UnknownSave(save_id))?;
        let bytes = self.read_bytes(&record)?;
        let (info, state) = codec::decode_snapshot(&bytes)?;
        let world = World::from_state(state).map_err(StoreError::Restore)?;
        let restored_checksum = world.state_checksum();
        if restored_checksum != info.state_checksum {
            return Err(StoreError::ChecksumMismatch {
                recorded: info.state_checksum,
                restored: restored_checksum,
            });
        }
        self.catalog
            .execute(
                "UPDATE saves SET verified_unix_ms = ?1 WHERE save_id = ?2",
                rusqlite::params![now_ms() as i64, save_id],
            )
            .map_err(catalog_error)?;
        Ok(VerifyReport {
            save_id,
            world_id: info.world_id,
            tick: info.tick,
            seed: info.seed,
            config_hash: info.config_hash,
            state_checksum: restored_checksum,
            terrain_checksum: info.terrain_checksum,
            build_version: info.build_version,
            compressed: info.compressed,
            population: world.population(),
        })
    }

    /// Load a snapshot file into a live world (used to branch/start a
    /// server from a save). Fail-closed; checksum-verified.
    pub fn load_world(path: &Path) -> Result<(SnapshotInfo, World), StoreError> {
        let bytes = fs::read(path).map_err(io_error)?;
        let (info, state) = codec::decode_snapshot(&bytes)?;
        let world = World::from_state(state).map_err(StoreError::Restore)?;
        let restored = world.state_checksum();
        if restored != info.state_checksum {
            return Err(StoreError::ChecksumMismatch {
                recorded: info.state_checksum,
                restored,
            });
        }
        Ok((info, world))
    }
}

fn parse_hex(value: &str) -> u64 {
    u64::from_str_radix(value.trim_start_matches("0x"), 16).unwrap_or(0)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RecoveryReport {
    pub valid_saves: u64,
    pub broken_saves: u64,
    pub removed_temp_files: u64,
}

#[derive(Clone, Debug)]
pub struct VerifyReport {
    pub save_id: i64,
    pub world_id: u64,
    pub tick: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub state_checksum: u64,
    pub terrain_checksum: u64,
    pub build_version: String,
    pub compressed: bool,
    pub population: usize,
}

/// Explicit migration registry. Format 1 is current; every other version
/// fails closed with an actionable reason. Transforms register here when a
/// future format exists — nothing migrates implicitly.
pub fn migration_for(format_version: u16) -> Result<(), String> {
    match format_version {
        codec::FORMAT_VERSION => Ok(()),
        older if older < codec::FORMAT_VERSION => Err(format!(
            "no registered migration from format {older} to {}; preserve the save read-only \
             and use a compatible binary",
            codec::FORMAT_VERSION
        )),
        newer => Err(format!(
            "save format {newer} is newer than this build's {}; upgrade the binary",
            codec::FORMAT_VERSION
        )),
    }
}
