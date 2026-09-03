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
    ChecksumMismatch {
        recorded: u64,
        restored: u64,
    },
    UnknownSave(i64),
    /// No registered transform for the file's declared format version.
    Migration(String),
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
            Self::Migration(reason) => write!(formatter, "migration: {reason}"),
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
        // Through the registry, not through the current-format reader: a
        // restored backup set can legitimately contain a file this build no
        // longer writes, and verifying it is exactly what this function is
        // for.
        let (info, state) = decode_snapshot_migrating(&bytes)?;
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
        let (info, state) = decode_snapshot_migrating(&bytes)?;
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

/// The result of running a registered migration: the source file's header,
/// the logical state in the current shape, and the re-encoded current-format
/// bytes.
///
/// All three, because a migration has two consumers with different needs. A
/// loader wants the state and nothing else. A tool that rewrites a save
/// archive wants the bytes. And the acceptance test wants to decode the bytes
/// back through the **current** reader and compare the result against what
/// the legacy reader produced from the original file - which is the only
/// version of "byte-identical to a format 3 load" that exercises the new
/// codec rather than asserting the transform equals itself.
#[derive(Clone, Debug)]
pub struct MigratedSave {
    /// The header of the file as it was found, so a caller reports the
    /// provenance of the save it actually read.
    pub source: SnapshotInfo,
    pub state: SaveState,
    /// The same state re-encoded at `codec::FORMAT_VERSION`.
    pub bytes: Vec<u8>,
}

/// One registered format transform.
///
/// `migration_for` used to return `Result<(), String>` and had no transform
/// type at all, so "a registered migration" was not expressible: the registry
/// could say yes or no to a version and nothing else. The type is the point
/// of this struct; the fields around it are what a migration has to declare
/// under `specifications/world-save-format.md` - source format, target
/// format, and what is lost.
pub struct Migration {
    pub from_format: u16,
    pub to_format: u16,
    /// What the transform invents, if anything. `""` means nothing: the
    /// 3-to-4 transform writes an absent modification section and a composed
    /// checksum equal to the baseline, and both are *identities* for a file
    /// with no overrides rather than data conjured to fill a field.
    pub expected_loss: &'static str,
    pub transform: fn(&[u8]) -> Result<MigratedSave, StoreError>,
}

impl fmt::Debug for Migration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Migration")
            .field("from_format", &self.from_format)
            .field("to_format", &self.to_format)
            .field("expected_loss", &self.expected_loss)
            .finish_non_exhaustive()
    }
}

/// Format 3 to the current format.
///
/// **Named for its source and not for its target, because the target moves.**
/// It was `FORMAT3_TO_FORMAT4` and its `to_format` was already
/// `codec::FORMAT_VERSION`, so the day format 5 landed the constant said 5
/// while the name said 4 - a transform whose name is one version stale is the
/// kind of thing that gets read rather than checked. Every registered
/// transform lands on the current format in one hop; there is no chaining,
/// because `decode_snapshot_migrating` applies exactly one.
///
/// Formats 1 and 2 have none, by design and by physics: a format 1 file
/// cannot say what its climate settings were, and a format 2 schema-2 file
/// does not contain the per-organism state format 3 added, because format 2
/// drove that loop from a count that is zero in a schema-2 world.
/// Transforming either would mean inventing state, which is exactly the
/// "never alter meaning during load" rule this crate exists to keep. They
/// fail closed, permanently, and the actionable advice is to keep the file
/// and use a build that reads it.
///
/// Format 3 is different in kind: everything in a format 3 file is still
/// present in a current file with the same meaning, and all three later
/// additions have honest values for a world that predates them. An absent
/// modification section is what a world with no overrides has,
/// `composed := baseline` is not a guess but the identity
/// `TerrainModState::composed_checksum` is written to satisfy for the empty
/// set, and rule 0 was a no-op in every build that could write the file.
/// Nothing is inferred and nothing is lost.
static FORMAT3_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_3,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format3_to_current,
};

/// Format 4 to the current format.
///
/// The one field format 5 adds is `plasticity.live_rule_zero`, and a
/// format-4 world ran with it `false` - not by default but by construction,
/// since no build that wrote a format-4 file had a live rule 0 to switch on.
/// So this transform invents nothing and `expected_loss` is the empty string,
/// exactly as it is for format 3.
static FORMAT4_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_4,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format4_to_current,
};

fn migrate_format3_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format3(bytes)?;
    // **Written out rather than inherited from the decoder.** The format 3
    // reader already resolves all three fields this way, and stating them
    // here anyway is what makes this a transform with a policy instead of a
    // wrapper around a reader: if the reader's resolution ever changed, the
    // migration's contract would not silently change with it.
    //
    // None of the three is observable by a test today, for the reason set out
    // on `migrate_format4_to_current` below - the reader already agrees with
    // every one of them, so deleting a line changes no output. Recorded there
    // rather than claimed away here.
    state.worldmod = None;
    state.composed_terrain_checksum = state.terrain_checksum;
    state.config.plasticity.live_rule_zero = false;
    resolve_format7_defaults(&mut state);
    resolve_format8_defaults(&mut state);
    resolve_format9_defaults(&mut state);
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 5 to the current format.
///
/// The one field format 6 adds is `plasticity.price_moved_edges_only`, and a
/// format-5 world priced every flagged edge - not by default but by
/// construction, since no build that wrote a format-5 file had another
/// pricing to select. So this transform invents nothing either.
static FORMAT5_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_5,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format5_to_current,
};

fn migrate_format5_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format5(bytes)?;
    // Unobservable for the reason recorded on `migrate_format4_to_current`:
    // the reader already resolves it this way. Kept, and kept documented as
    // unbacked rather than credited to a test that cannot see it.
    state.config.plasticity.price_moved_edges_only = false;
    resolve_format7_defaults(&mut state);
    resolve_format8_defaults(&mut state);
    resolve_format9_defaults(&mut state);
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 6 to current (7). What format 7 adds is the artifact section, the
/// `bind` operator's rate and counter, and the object table; a format-6
/// world had none of them - not by default but by construction, since no
/// build that wrote a format-6 file had an object in it or a `bind` to run.
/// So this transform invents nothing, and `expected_loss` is empty.
static FORMAT6_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_6,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format6_to_current,
};

fn migrate_format6_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format6(bytes)?;
    resolve_format7_defaults(&mut state);
    resolve_format8_defaults(&mut state);
    resolve_format9_defaults(&mut state);
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 7 to current (8). What format 8 adds is the social section; a
/// format-7 world had none of it - not by default but by construction,
/// since no build that wrote a format-7 file could perceive a conspecific
/// or emit a signal. So this transform invents nothing, and
/// `expected_loss` is empty.
static FORMAT7_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_7,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format7_to_current,
};

fn migrate_format7_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format7(bytes)?;
    resolve_format8_defaults(&mut state);
    resolve_format9_defaults(&mut state);
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 8 to current (9). What format 9 adds is the physiology-v2
/// ontogeny config block and the ontogeny progress section; a format-8
/// world had neither - not by default but by construction, since no build
/// that wrote a format-8 file grew a body over a lifetime. So this
/// transform invents nothing, and `expected_loss` is empty.
static FORMAT8_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_8,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format8_to_current,
};

fn migrate_format8_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format8(bytes)?;
    resolve_format9_defaults(&mut state);
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 9 to current (10). What format 10 adds is the two mate-choice
/// gates and the counters section; a format-9 world had neither - not by
/// default but by construction, since no build that wrote a format-9 file
/// chose a mate by anything but distance. So this transform invents
/// nothing, and `expected_loss` is empty.
static FORMAT9_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_9,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format9_to_current,
};

fn migrate_format9_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format9(bytes)?;
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 10 to current: what formats 11 and 12 add are the chemistry
/// and microbial sections; a format-10 world had neither - by
/// construction. Invents nothing; `expected_loss` empty.
static FORMAT10_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_10,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format10_to_current,
};

fn migrate_format10_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format10(bytes)?;
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 11 to current (12): what format 12 adds is the microbial
/// section; a format-11 world had none - by construction. Invents
/// nothing; `expected_loss` empty.
static FORMAT11_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_11,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format11_to_current,
};

fn migrate_format11_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format11(bytes)?;
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 12 to current (15): what format 13 adds is the two coupling
/// fractions, zero in every format-12 world - by construction; what format
/// 14 further adds is the transition config and section, off and absent in
/// every format-12 world on the same terms; what format 15 further adds is
/// the two consumption fields and `consumed_milli` (ADR-0034), zero/off on
/// the same terms again. Invents nothing; `expected_loss` empty.
static FORMAT12_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_12,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format12_to_current,
};

fn migrate_format12_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format12(bytes)?;
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 13 to current (15): what format 14 adds is the transition config
/// block and `SECTION_TRANSITION`; what format 15 further adds is the two
/// consumption fields and `consumed_milli` (ADR-0034). A format-13 world
/// had none of it - not by default but by construction, since no build
/// that wrote a format-13 file had a transition policy to run, a scratch
/// origin to start from, or a mouth to consume with. Invents nothing;
/// `expected_loss` empty.
static FORMAT13_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_13,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format13_to_current,
};

fn migrate_format13_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format13(bytes)?;
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Format 14 to current (15): what format 15 adds is the two consumption
/// fields (ADR-0034) and one trailing i128 on `SECTION_CHEMISTRY`; a
/// format-14 world had neither - not by default but by construction, since
/// no build that wrote a format-14 file had a mouth to consume with.
/// Invents nothing; `expected_loss` empty.
static FORMAT14_TO_CURRENT: Migration = Migration {
    from_format: codec::FORMAT_VERSION_14,
    to_format: codec::FORMAT_VERSION,
    expected_loss: "",
    transform: migrate_format14_to_current,
};

fn migrate_format14_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format14(bytes)?;
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// The resolution every pre-15 transform states for what format 15 added
/// (ADR-0034): the two consumption fields at their defaults, and a
/// nonzero `consumed_milli` resolved to zero when a chemistry section is
/// present - not by default but by construction, since no build that
/// wrote a pre-15 file had a mouth to consume with.
fn resolve_format15_defaults(state: &mut sim_core::SaveState) {
    let defaults = sim_core::ChemistryConfig::chemistry_default();
    state.config.chemistry.consumption_fraction_q16 = defaults.consumption_fraction_q16;
    state.config.chemistry.consumption_yield_q16 = defaults.consumption_yield_q16;
    if let Some(chemistry) = state.chemistry.as_mut() {
        chemistry.consumed_milli = 0;
    }
}

/// The resolution every pre-14 transform states for what format 14 added.
fn resolve_format14_defaults(state: &mut sim_core::SaveState) {
    state.config.transition = sim_core::TransitionConfig::transition_default();
    state.transition = None;
}

/// The resolution every pre-13 transform states for what format 13 added.
fn resolve_format13_defaults(state: &mut sim_core::SaveState) {
    state.config.chemistry.excretion_fraction_q16 = 0;
    state.config.chemistry.remains_fraction_q16 = 0;
}

/// The resolution every pre-12 transform states for what format 12 added.
/// Field by field rather than the whole chemistry struct, because the rest
/// of that struct is format 11's and legitimately non-default there.
fn resolve_format12_defaults(state: &mut sim_core::SaveState) {
    let defaults = sim_core::ChemistryConfig::chemistry_default();
    let chemistry = &mut state.config.chemistry;
    chemistry.microbial_enabled = defaults.microbial_enabled;
    chemistry.replication_axis = defaults.replication_axis;
    chemistry.aggregation_axis = defaults.aggregation_axis;
    chemistry.growth_rate_low_q16 = defaults.growth_rate_low_q16;
    chemistry.growth_rate_high_q16 = defaults.growth_rate_high_q16;
    chemistry.growth_yield_q16 = defaults.growth_yield_q16;
    chemistry.death_q16 = defaults.death_q16;
    chemistry.death_waste_fraction_q16 = defaults.death_waste_fraction_q16;
    chemistry.mutation_q16 = defaults.mutation_q16;
    state.microbial = None;
}

/// The resolution every pre-11 transform states for what format 11 added.
fn resolve_format11_defaults(state: &mut sim_core::SaveState) {
    state.config.chemistry = sim_core::ChemistryConfig::chemistry_default();
    state.chemistry = None;
}

/// The resolution every pre-10 transform states for what format 10 added,
/// unobservable on the terms `resolve_format9_defaults` is.
fn resolve_format10_defaults(state: &mut sim_core::SaveState) {
    state.config.physiology.mate_choice_enabled = false;
    state.config.physiology.mate_choice_scramble = false;
    state.matechoice = None;
}

/// The resolution every pre-9 transform states for what format 9 added.
/// Unobservable in the same sense as `resolve_format8_defaults` - the
/// readers already resolve it this way - and kept for the same reason.
fn resolve_format9_defaults(state: &mut sim_core::SaveState) {
    let defaults = sim_core::PhysiologyConfig::physiology_default();
    state.config.physiology.ontogeny_enabled = defaults.ontogeny_enabled;
    state.config.physiology.birth_modules_min = defaults.birth_modules_min;
    state.config.physiology.growth_cost_milli_per_mass_milli =
        defaults.growth_cost_milli_per_mass_milli;
    state.config.physiology.growth_rate_milli_per_s = defaults.growth_rate_milli_per_s;
    state.ontogeny = None;
}

/// The resolution every pre-8 transform states for what format 8 added.
/// Unobservable in the same sense as `resolve_format7_defaults` - the
/// readers already resolve it this way - and kept for the same reason.
fn resolve_format8_defaults(state: &mut sim_core::SaveState) {
    state.config.social = sim_core::SocialConfig::social_default();
}

/// The resolution every pre-7 transform states for what format 7 added.
/// Unobservable in the same sense as the format-5 and format-6 lines above -
/// the readers already resolve these this way - and kept for the same
/// reason: the transform states its own resolution rather than borrowing
/// the reader's.
fn resolve_format7_defaults(state: &mut sim_core::SaveState) {
    state.config.artifact = sim_core::ArtifactConfig::artifact_default();
    state.config.genome2.mutation.binding_q16 = 0;
    if let Some(schema2) = state.schema2.as_mut() {
        schema2.counters.binding_applied = 0;
    }
    state.objects = None;
}

fn migrate_format4_to_current(bytes: &[u8]) -> Result<MigratedSave, StoreError> {
    let (source, mut state) = codec::decode_snapshot_format4(bytes)?;
    state.config.plasticity.price_moved_edges_only = false;
    // The one field format 5 adds, written out for the reason the three above
    // are: the transform states its own resolution rather than borrowing the
    // reader's. Everything else in a format 4 file is present in a format 5
    // file unchanged, which is why nothing else appears here.
    //
    // **This assignment is not observable by any test, and saying so is the
    // point.** A mutation run deleting it left the whole `sim-persist` suite
    // green, because `decode_config` at format 4 skips the byte and leaves the
    // field at its `false` default - so the line only ever writes `false` over
    // `false`. The same is true of the three assignments in
    // `migrate_format3_to_current`, so this is a property of the pattern and
    // not an oversight here. Setting it to `true` *is* caught, which is the
    // half that matters: the tests pin the transform's *output*, and what they
    // cannot pin is which of two agreeing paths produced it. Kept as
    // defence-in-depth against a future reader whose resolution changes, and
    // documented as unbacked rather than credited to a test that cannot see it.
    state.config.plasticity.live_rule_zero = false;
    state.config.plasticity.price_moved_edges_only = false;
    resolve_format7_defaults(&mut state);
    resolve_format8_defaults(&mut state);
    resolve_format9_defaults(&mut state);
    resolve_format10_defaults(&mut state);
    resolve_format11_defaults(&mut state);
    resolve_format12_defaults(&mut state);
    resolve_format13_defaults(&mut state);
    resolve_format14_defaults(&mut state);
    resolve_format15_defaults(&mut state);
    reencode(source, state)
}

/// Re-encode a migrated state at the current format, preserving provenance.
///
/// Shared by both transforms rather than written twice, because the header
/// fields a migration must carry through are a property of migrating and not
/// of any one source format - and the version this pair was written at, the
/// 3-to-4 transform's body was the only statement of them, so a second
/// transform would have had to restate them correctly from reading it.
fn reencode(source: SnapshotInfo, state: SaveState) -> Result<MigratedSave, StoreError> {
    // Compression is preserved as a property, not as a level: no file records
    // the level it was written at, so a migrated file may differ in size from
    // its source. Size is not part of the contract; the decoded state is.
    let compression = source.compressed.then_some(MIGRATION_COMPRESSION_LEVEL);
    let bytes = codec::encode_snapshot(
        &state,
        source.world_id,
        source.parent_world_id,
        source.state_checksum,
        &source.build_version,
        source.event_log_offset,
        compression,
    )?;
    Ok(MigratedSave {
        source,
        state,
        bytes,
    })
}

/// zstd level used when a migrated file's source was compressed. Matches the
/// level the checkpointer uses, so a migrated archive is not an outlier.
const MIGRATION_COMPRESSION_LEVEL: i32 = 3;

/// Explicit migration registry.
///
/// `Ok(None)` means the file is already at the current format and is decoded
/// natively. `Ok(Some(migration))` names a registered transform. `Err` is
/// fail-closed with an actionable reason, and it is what every unregistered
/// version gets - nothing migrates implicitly, and a version this build has
/// never heard of is refused rather than read hopefully.
pub fn migration_for(format_version: u16) -> Result<Option<&'static Migration>, String> {
    match format_version {
        codec::FORMAT_VERSION => Ok(None),
        codec::FORMAT_VERSION_3 => Ok(Some(&FORMAT3_TO_CURRENT)),
        codec::FORMAT_VERSION_4 => Ok(Some(&FORMAT4_TO_CURRENT)),
        codec::FORMAT_VERSION_5 => Ok(Some(&FORMAT5_TO_CURRENT)),
        codec::FORMAT_VERSION_6 => Ok(Some(&FORMAT6_TO_CURRENT)),
        codec::FORMAT_VERSION_7 => Ok(Some(&FORMAT7_TO_CURRENT)),
        codec::FORMAT_VERSION_8 => Ok(Some(&FORMAT8_TO_CURRENT)),
        codec::FORMAT_VERSION_9 => Ok(Some(&FORMAT9_TO_CURRENT)),
        codec::FORMAT_VERSION_10 => Ok(Some(&FORMAT10_TO_CURRENT)),
        codec::FORMAT_VERSION_11 => Ok(Some(&FORMAT11_TO_CURRENT)),
        codec::FORMAT_VERSION_12 => Ok(Some(&FORMAT12_TO_CURRENT)),
        codec::FORMAT_VERSION_13 => Ok(Some(&FORMAT13_TO_CURRENT)),
        codec::FORMAT_VERSION_14 => Ok(Some(&FORMAT14_TO_CURRENT)),
        older if older < codec::FORMAT_VERSION => Err(format!(
            "no registered migration from format {older} to {}; a format 1 or 2 file does not \
             contain the state later formats require and cannot be transformed without \
             inventing it. Preserve the save read-only and use a compatible binary",
            codec::FORMAT_VERSION
        )),
        newer => Err(format!(
            "save format {newer} is newer than this build's {}; upgrade the binary",
            codec::FORMAT_VERSION
        )),
    }
}

/// Decode a snapshot of any registered format, applying a migration when the
/// file needs one.
///
/// This is the function every loader should call. `codec::decode_snapshot` is
/// the current-format reader and refuses anything else, which is right for a
/// codec and wrong for a loader: the CLI's `verify-save` called `read_info`
/// first and `migration_for` second, so an old file failed with
/// `UnsupportedFormat` before the registry it was about to consult ever ran -
/// the registry was unreachable from the one command that existed to use it.
pub fn decode_snapshot_migrating(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), StoreError> {
    let format = codec::peek_format_version(bytes)?;
    match migration_for(format).map_err(StoreError::Migration)? {
        None => Ok(codec::decode_snapshot(bytes)?),
        Some(migration) => {
            let migrated = (migration.transform)(bytes)?;
            Ok((migrated.source, migrated.state))
        }
    }
}
