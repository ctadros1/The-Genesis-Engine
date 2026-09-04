//! Shared server state, split in two by ADR-0039: the process-level `Hub`
//! and the per-world `WorldRuntime`.
//!
//! The kernel stays pure; every clock, socket, and thread lives here. A
//! world's tick thread must never block on client I/O: subscribers receive
//! frames through bounded latest-wins queues and slow clients get resynced
//! with a fresh keyframe instead of an unbounded backlog.
//!
//! Lock order, process-wide: `Hub::worlds` is held only long enough to
//! clone an `Arc<WorldRuntime>` or to insert/remove one, and never while a
//! world lock is held. Nothing else nests a world lock inside another
//! world's, so two worlds cannot deadlock each other.

use sim_core::World;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hard server-side subscription limits (documented in the API spec).
pub const MAX_RATE_HZ: u8 = 30;
pub const MIN_RATE_HZ: u8 = 1;
pub const MAX_PENDING_FRAMES: usize = 16;
/// Terrain tile refresh cadence in sent state frames.
pub const TILE_REFRESH_INTERVAL: u64 = 20;
/// Maximum speed multiplier (Q16) an admin may set: 64x.
pub const MAX_SPEED_Q16: u32 = 64 << 16;
/// The world built from the command-line flags. Every route, test and
/// deployment that predates ADR-0039 addresses this id.
pub const PRIMARY_WORLD_ID: u64 = 1;

pub struct Control {
    pub paused: bool,
    pub speed_q16: u32,
}

/// How a tick thread paces itself.
///
/// Acceleration is a host concern only. The kernel reads no clock, so
/// pacing cannot reach a result; A5.1 is the test that proves the claim
/// rather than asserting it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pacing {
    /// Sleep so one tick takes `dt / speed` of wall-clock time.
    Realtime,
    /// Never sleep. The speed multiplier becomes meaningless and is
    /// ignored; pause still works, because pausing is world state.
    Headless,
}

impl Pacing {
    pub fn name(self) -> &'static str {
        match self {
            Pacing::Realtime => "realtime",
            Pacing::Headless => "headless",
        }
    }
}

/// Whether a checkpoint blocks the tick thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointMode {
    /// Phase 4 behavior: encode, compress, and fsync on the tick thread.
    /// Kept available so the Phase 4 path can be measured and rolled back to.
    Sync,
    /// Phase 5 behavior: capture on the tick thread, write on another.
    Async,
}

impl CheckpointMode {
    pub fn name(self) -> &'static str {
        match self {
            CheckpointMode::Sync => "sync",
            CheckpointMode::Async => "async",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuditRecord {
    pub id: u64,
    pub unix_ms: u64,
    /// Which world the mutation addressed. 0 means the process itself
    /// (world creation, which has no world until it succeeds).
    pub world_id: u64,
    pub role: &'static str,
    pub action: String,
    pub accepted: bool,
    pub detail: String,
    pub tick: u64,
    pub idempotency_key: String,
}

/// One subscriber's bounded outbound queue. The tick thread pushes; the
/// connection thread drains. Overflow clears pending deltas and forces a
/// keyframe resync (`dropped` counts collapsed updates).
pub struct ClientSlot {
    pub id: u64,
    pub queue: Mutex<SlotQueue>,
    pub signal: Condvar,
    pub closed: AtomicBool,
    pub bytes_sent: AtomicU64,
    pub dropped_updates: AtomicU64,
}

pub struct SlotQueue {
    pub frames: VecDeque<Vec<u8>>,
    pub needs_keyframe: bool,
    /// Client subscription (world fixed-point bounds, layers, rate).
    pub viewport: Option<(i32, i32, i32, i32)>,
    pub layers: u32,
    pub rate_hz: u8,
    /// Wall-clock ms of the last state frame sent to this client.
    pub last_state_frame_ms: u64,
    /// Frames sent to this subscriber (drives tile refresh cadence).
    pub state_frames_sent: u64,
    /// Last acknowledged sequence from the client.
    pub acked_sequence: u64,
    /// Per-client view of entity records last sent, for delta diffing.
    pub last_entities: std::collections::HashMap<u64, sim_protocol::EntityRecord>,
    pub next_sequence: u64,
}

impl ClientSlot {
    pub fn new(id: u64) -> Arc<Self> {
        Arc::new(Self {
            id,
            queue: Mutex::new(SlotQueue {
                frames: VecDeque::new(),
                needs_keyframe: true,
                viewport: None,
                layers: 0,
                rate_hz: 10,
                last_state_frame_ms: 0,
                state_frames_sent: 0,
                acked_sequence: 0,
                last_entities: std::collections::HashMap::new(),
                next_sequence: 1,
            }),
            signal: Condvar::new(),
            closed: AtomicBool::new(false),
            bytes_sent: AtomicU64::new(0),
            dropped_updates: AtomicU64::new(0),
        })
    }

    /// Push an encoded frame; on overflow collapse the backlog and demand
    /// a keyframe resync instead of queueing without bound.
    pub fn push_frame(&self, bytes: Vec<u8>) {
        let mut queue = self.queue.lock().expect("slot queue");
        if queue.frames.len() >= MAX_PENDING_FRAMES {
            let collapsed = queue.frames.len() as u64;
            queue.frames.clear();
            queue.needs_keyframe = true;
            self.dropped_updates.fetch_add(collapsed, Ordering::Relaxed);
        }
        queue.frames.push_back(bytes);
        drop(queue);
        self.signal.notify_one();
    }
}

/// One hosted world: its state, its controls, its subscribers, its tick
/// thread's counters. Nothing here is shared with another world, which is
/// what makes "pausing one does not pause the others" a property of the
/// data layout rather than of the control handler's care.
pub struct WorldRuntime {
    pub id: u64,
    pub name: String,
    /// The preset the world was created from, or the parent's preset for a
    /// branch. Reported so a console can say what a world descends from.
    pub preset: String,
    pub created_unix_ms: u64,
    /// 0 for a world with no parent.
    pub parent_world_id: u64,
    pub world_epoch: u64,
    pub world: Mutex<World>,
    pub control: Mutex<Control>,
    pub clients: Mutex<Vec<Arc<ClientSlot>>>,
    /// Set by `control?action=stop`. The tick thread writes a final
    /// checkpoint and exits; open sessions answer 410 and close.
    pub stopped: AtomicBool,
    /// Ring of recent tick durations (microseconds) for benchmarking.
    pub tick_samples_us: Mutex<VecDeque<f64>>,
    pub ticks_total: AtomicU64,
    /// Measured tick rate, in thousandths of a tick per second, refreshed
    /// once a second by the tick thread. Measured rather than derived from
    /// the mean tick cost: a paused or speed-limited world costs the same
    /// per tick and advances at a different rate.
    pub ticks_per_second_milli: AtomicU64,
    pub dt_ms: u64,
    pub saves_total: AtomicU64,
    pub save_failures_total: AtomicU64,
    pub last_save_duration_us: AtomicU64,
    pub last_save_bytes: AtomicU64,
    /// Automatic checkpoints refused because the previous one was still
    /// being written. Reported, never hidden: a nonzero value means the
    /// configured interval is shorter than a checkpoint takes.
    pub checkpoints_skipped: AtomicU64,
    /// Wall-clock cost of the last state capture on the tick thread. This
    /// is the only part of a checkpoint the tick thread pays for in
    /// asynchronous mode, and A5.5 measures it directly.
    pub last_capture_us: AtomicU64,
    /// Wall-clock ms of the last accepted control on this world. Per world
    /// rather than per process, so a burst of controls on one world cannot
    /// refuse a control on another.
    pub last_control_ms: AtomicU64,
}

impl WorldRuntime {
    pub fn status(&self) -> &'static str {
        if self.stopped.load(Ordering::Relaxed) {
            "stopped"
        } else if self.control.lock().expect("control").paused {
            "paused"
        } else {
            "running"
        }
    }

    pub fn tick_number(&self) -> u64 {
        self.world.lock().expect("world").tick_number()
    }

    /// Mean of the recent tick-duration ring, in microseconds.
    pub fn tick_mean_us(&self) -> f64 {
        let samples = self.tick_samples_us.lock().expect("samples");
        if samples.is_empty() {
            return 0.0;
        }
        samples.iter().sum::<f64>() / samples.len() as f64
    }
}

/// Process-level state: the tokens, the audit log, the idempotency cache,
/// the snapshot store, and the registry of worlds.
pub struct Hub {
    pub observer_token: String,
    pub admin_token: String,
    pub audit: Mutex<Vec<AuditRecord>>,
    pub next_audit_id: AtomicU64,
    pub next_client_id: AtomicU64,
    /// Recorded responses for keyed mutations, keyed by world id and the
    /// client's `Idempotency-Key` so one key replayed against a different
    /// world cannot return the other world's answer.
    pub idempotency: Mutex<HashMap<String, (u16, String)>>,
    /// Snapshot store (None disables persistence endpoints/checkpoints).
    /// Shared rather than owned so the asynchronous checkpoint writer and
    /// the REST save endpoints can hold the same catalog.
    pub store: Option<Arc<Mutex<sim_persist::SnapshotStore>>>,
    /// Root of the store, needed to turn a catalog row's file name into a
    /// path when branching.
    pub data_dir: Option<std::path::PathBuf>,
    /// Wall-clock seconds between automatic checkpoints (0 disables).
    pub checkpoint_interval_secs: u64,
    pub checkpoint_keep: usize,
    pub checkpoint_mode: CheckpointMode,
    pub pacing: Pacing,
    pub max_worlds: usize,
    /// World 1 comes from the flags; created worlds start at 2.
    pub next_world_id: AtomicU64,
    pub worlds: Mutex<BTreeMap<u64, Arc<WorldRuntime>>>,
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Constant-time-ish token comparison (length leak acceptable on a
/// private LAN boundary; avoids early-exit content leaks).
pub fn token_matches(provided: &str, expected: &str) -> bool {
    if provided.len() != expected.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (a, b) in provided.bytes().zip(expected.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Observer,
    Admin,
}

impl Hub {
    /// Resolve a bearer token to a role. Admin implies observer access.
    pub fn role_for(&self, token: &str) -> Option<Role> {
        if token_matches(token, &self.admin_token) {
            Some(Role::Admin)
        } else if token_matches(token, &self.observer_token) {
            Some(Role::Observer)
        } else {
            None
        }
    }

    /// The registry entry for `id`, if the world still exists.
    pub fn world(&self, id: u64) -> Option<Arc<WorldRuntime>> {
        self.worlds.lock().expect("worlds").get(&id).cloned()
    }

    /// Every hosted world in id order, so listings are stable.
    pub fn all_worlds(&self) -> Vec<Arc<WorldRuntime>> {
        self.worlds
            .lock()
            .expect("worlds")
            .values()
            .cloned()
            .collect()
    }

    // One record, one argument each: bundling them into a struct would put
    // a type between every call site and the log it writes.
    #[allow(clippy::too_many_arguments)]
    pub fn record_audit(
        &self,
        world_id: u64,
        role: &'static str,
        action: &str,
        accepted: bool,
        detail: &str,
        tick: u64,
        idempotency_key: &str,
    ) {
        let record = AuditRecord {
            id: self.next_audit_id.fetch_add(1, Ordering::Relaxed),
            unix_ms: now_unix_ms(),
            world_id,
            role,
            action: action.to_owned(),
            accepted,
            detail: detail.to_owned(),
            tick,
            idempotency_key: idempotency_key.to_owned(),
        };
        let mut audit = self.audit.lock().expect("audit log");
        audit.push(record);
        // Bounded audit history in memory for this local phase.
        if audit.len() > 10_000 {
            let excess = audit.len() - 10_000;
            audit.drain(..excess);
        }
    }
}
