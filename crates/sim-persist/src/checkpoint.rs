//! Asynchronous checkpoint writer (Phase 5).
//!
//! Phase 4 wrote checkpoints synchronously on the tick thread: the world
//! lock was released after `export_state`, but encoding, zstd compression,
//! `fsync`, and the catalog insert all still ran before the next tick could
//! start. That is the stall A5.5 measures and removes.
//!
//! The split here is deliberate about what stays on the tick thread and
//! what leaves it:
//!
//! - The tick thread captures `SaveState`, an owned deep copy taken at a
//!   tick boundary, and hands it over. Capture is the only cost it keeps.
//! - Encoding, compression, the atomic write, the `fsync`, the catalog
//!   commit, and pruning all happen on the writer thread.
//!
//! Because the writer only ever sees an owned, immutable capture, it cannot
//! observe a torn world: there is no path by which a snapshot is encoded
//! from live arrays. That is what makes the asynchronous path safe rather
//! than merely faster, and the durability ordering inside `SnapshotStore`
//! is untouched, so an interrupted asynchronous write leaves exactly the
//! same evidence an interrupted synchronous one did.
//!
//! The queue holds at most one request. A checkpoint requested while
//! another is still in flight is **refused and counted**, never queued and
//! never silently discarded: an unbounded queue under a slow disk would
//! turn a latency problem into a memory problem, and a silent drop would
//! make the checkpoint interval a lie.

use crate::store::{SaveRecord, SnapshotStore, StoreError};
use sim_core::SaveState;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

/// One durable-save request handed to the writer thread.
#[derive(Clone, Debug)]
pub struct CheckpointRequest {
    pub state: SaveState,
    pub state_checksum: u64,
    pub world_id: u64,
    pub parent_world_id: u64,
    pub name: String,
    pub kind: String,
    pub event_log_offset: u64,
    pub compression_level: Option<i32>,
    /// When set, prune checkpoints to this many after a successful write.
    pub prune_keep: Option<usize>,
}

/// What happened to one completed request.
#[derive(Clone, Debug)]
pub struct CheckpointOutcome {
    pub tick: u64,
    pub kind: String,
    pub record: Option<SaveRecord>,
    pub error: Option<String>,
    /// Wall-clock cost on the writer thread, not on the tick thread.
    pub duration_us: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitResult {
    Accepted,
    /// A checkpoint is still in flight; this one was refused and counted.
    Busy,
    /// The writer thread is gone.
    Stopped,
}

#[derive(Default)]
struct Slot {
    pending: Option<CheckpointRequest>,
    in_flight: bool,
    stopping: bool,
    outcomes: Vec<CheckpointOutcome>,
}

struct Inner {
    slot: Mutex<Slot>,
    signal: Condvar,
    skipped: AtomicU64,
    stopped: AtomicBool,
}

pub struct AsyncCheckpointer {
    inner: Arc<Inner>,
    handle: Option<JoinHandle<()>>,
}

impl AsyncCheckpointer {
    /// Start the writer thread. The store handle is shared rather than
    /// moved so the REST surface keeps its synchronous named-save path.
    pub fn spawn(store: Arc<Mutex<SnapshotStore>>) -> Self {
        let inner = Arc::new(Inner {
            slot: Mutex::new(Slot::default()),
            signal: Condvar::new(),
            skipped: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
        });
        let worker = Arc::clone(&inner);
        let handle = std::thread::Builder::new()
            .name("checkpoint-writer".to_owned())
            .spawn(move || writer_loop(&worker, &store))
            .expect("spawn checkpoint writer");
        Self {
            inner,
            handle: Some(handle),
        }
    }

    /// Hand a captured state to the writer. Never blocks on I/O.
    pub fn submit(&self, request: CheckpointRequest) -> SubmitResult {
        if self.inner.stopped.load(Ordering::Acquire) {
            return SubmitResult::Stopped;
        }
        let mut slot = self.inner.slot.lock().expect("checkpoint slot");
        if slot.stopping {
            return SubmitResult::Stopped;
        }
        if slot.pending.is_some() || slot.in_flight {
            drop(slot);
            self.inner.skipped.fetch_add(1, Ordering::Relaxed);
            return SubmitResult::Busy;
        }
        slot.pending = Some(request);
        self.inner.signal.notify_all();
        SubmitResult::Accepted
    }

    /// Number of checkpoints refused because a write was still in flight.
    /// A nonzero value means the configured interval is shorter than the
    /// time a checkpoint takes, and it is reported, not hidden.
    pub fn skipped(&self) -> u64 {
        self.inner.skipped.load(Ordering::Relaxed)
    }

    pub fn busy(&self) -> bool {
        let slot = self.inner.slot.lock().expect("checkpoint slot");
        slot.pending.is_some() || slot.in_flight
    }

    /// Take every outcome recorded since the last call.
    pub fn drain_outcomes(&self) -> Vec<CheckpointOutcome> {
        let mut slot = self.inner.slot.lock().expect("checkpoint slot");
        std::mem::take(&mut slot.outcomes)
    }

    /// Block until nothing is queued or in flight.
    pub fn wait_idle(&self) {
        let mut slot = self.inner.slot.lock().expect("checkpoint slot");
        while slot.pending.is_some() || slot.in_flight {
            slot = self.inner.signal.wait(slot).expect("checkpoint slot");
        }
    }

    /// Finish any in-flight write, stop the thread, and return the final
    /// outcomes. Called at shutdown so a checkpoint in progress is never
    /// abandoned half-written.
    pub fn shutdown(mut self) -> Vec<CheckpointOutcome> {
        self.stop_and_join();
        let mut slot = self.inner.slot.lock().expect("checkpoint slot");
        std::mem::take(&mut slot.outcomes)
    }

    fn stop_and_join(&mut self) {
        {
            let mut slot = self.inner.slot.lock().expect("checkpoint slot");
            slot.stopping = true;
            self.inner.signal.notify_all();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        self.inner.stopped.store(true, Ordering::Release);
    }
}

impl Drop for AsyncCheckpointer {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.stop_and_join();
        }
    }
}

fn writer_loop(inner: &Arc<Inner>, store: &Arc<Mutex<SnapshotStore>>) {
    loop {
        let request = {
            let mut slot = inner.slot.lock().expect("checkpoint slot");
            loop {
                if let Some(request) = slot.pending.take() {
                    slot.in_flight = true;
                    break request;
                }
                if slot.stopping {
                    return;
                }
                slot = inner.signal.wait(slot).expect("checkpoint slot");
            }
        };

        let started = Instant::now();
        let result = write_one(store, &request);
        let duration_us = started.elapsed().as_micros() as u64;

        let outcome = match result {
            Ok(record) => CheckpointOutcome {
                tick: request.state.tick,
                kind: request.kind.clone(),
                bytes: record.bytes,
                record: Some(record),
                error: None,
                duration_us,
            },
            Err(error) => CheckpointOutcome {
                tick: request.state.tick,
                kind: request.kind.clone(),
                record: None,
                error: Some(error.to_string()),
                duration_us,
                bytes: 0,
            },
        };

        let mut slot = inner.slot.lock().expect("checkpoint slot");
        slot.in_flight = false;
        // Bound the outcome buffer; a host that never drains must not grow
        // this without limit.
        if slot.outcomes.len() >= 256 {
            slot.outcomes.remove(0);
        }
        slot.outcomes.push(outcome);
        inner.signal.notify_all();
    }
}

fn write_one(
    store: &Arc<Mutex<SnapshotStore>>,
    request: &CheckpointRequest,
) -> Result<SaveRecord, StoreError> {
    let store = store.lock().expect("store");
    let record = store.save(
        &request.state,
        request.state_checksum,
        request.world_id,
        request.parent_world_id,
        &request.name,
        &request.kind,
        request.event_log_offset,
        request.compression_level,
    )?;
    if let Some(keep) = request.prune_keep {
        store.prune_checkpoints(keep)?;
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{SimConfig, World};
    use std::path::PathBuf;

    fn scratch_dir(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("lifesim-checkpoint-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("scratch dir");
        directory
    }

    fn small_world(ticks: u64) -> World {
        let mut config = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
        config.cells_x = 32;
        config.cells_y = 32;
        config.initial_organisms = 20;
        config.max_entities = 200;
        let mut world = World::new(config).unwrap();
        for _ in 0..ticks {
            world.step();
        }
        world
    }

    fn request(world: &World, name: &str) -> CheckpointRequest {
        CheckpointRequest {
            state: world.export_state(),
            state_checksum: world.state_checksum(),
            world_id: 1,
            parent_world_id: 0,
            name: name.to_owned(),
            kind: "checkpoint".to_owned(),
            event_log_offset: 0,
            compression_level: Some(3),
            prune_keep: None,
        }
    }

    #[test]
    fn asynchronous_checkpoints_are_durable_and_restore_identically() {
        let directory = scratch_dir("durable");
        let (store, _) = SnapshotStore::open(&directory).unwrap();
        let store = Arc::new(Mutex::new(store));
        let world = small_world(120);
        let checksum = world.state_checksum();

        let checkpointer = AsyncCheckpointer::spawn(Arc::clone(&store));
        assert_eq!(
            checkpointer.submit(request(&world, "auto")),
            SubmitResult::Accepted
        );
        checkpointer.wait_idle();
        let outcomes = checkpointer.shutdown();
        assert_eq!(outcomes.len(), 1);
        let outcome = &outcomes[0];
        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        let record = outcome.record.as_ref().expect("record");
        assert_eq!(record.state_checksum, checksum);

        let path = directory.join(&record.path);
        let (_, restored) = SnapshotStore::load_world(&path).unwrap();
        assert_eq!(restored.state_checksum(), checksum);
    }

    #[test]
    fn a_request_arriving_during_a_write_is_refused_and_counted() {
        let directory = scratch_dir("busy");
        let (store, _) = SnapshotStore::open(&directory).unwrap();
        let store = Arc::new(Mutex::new(store));
        let world = small_world(60);
        let checkpointer = AsyncCheckpointer::spawn(Arc::clone(&store));

        // Hold the store so the first request cannot complete, guaranteeing
        // the second arrives while the first is in flight.
        let guard = store.lock().expect("store");
        assert_eq!(
            checkpointer.submit(request(&world, "first")),
            SubmitResult::Accepted
        );
        // Wait until the writer has actually claimed the request.
        while !checkpointer.busy() {
            std::thread::yield_now();
        }
        let mut refused = 0;
        for _ in 0..8 {
            if checkpointer.submit(request(&world, "second")) == SubmitResult::Busy {
                refused += 1;
            }
        }
        drop(guard);
        checkpointer.wait_idle();

        assert!(
            refused > 0,
            "no request was refused while one was in flight"
        );
        assert_eq!(checkpointer.skipped(), refused);
        let outcomes = checkpointer.shutdown();
        // Exactly one write happened; the refusals wrote nothing.
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].record.as_ref().unwrap().name, "first");
    }

    #[test]
    fn a_failing_write_is_reported_not_swallowed() {
        let directory = scratch_dir("failure");
        let (store, _) = SnapshotStore::open(&directory).unwrap();
        let store = Arc::new(Mutex::new(store));
        let world = small_world(30);
        let checkpointer = AsyncCheckpointer::spawn(Arc::clone(&store));

        // Remove the store directory so the atomic write cannot land.
        std::fs::remove_dir_all(&directory).unwrap();
        assert_eq!(
            checkpointer.submit(request(&world, "doomed")),
            SubmitResult::Accepted
        );
        checkpointer.wait_idle();
        let outcomes = checkpointer.shutdown();
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].error.is_some());
        assert!(outcomes[0].record.is_none());
    }
}
