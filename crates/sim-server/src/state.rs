//! Shared server state: the authoritative world, control settings, audit
//! log, subscriber registry, and stream statistics.
//!
//! The kernel stays pure; every clock, socket, and thread lives here. The
//! tick thread must never block on client I/O: subscribers receive frames
//! through bounded latest-wins queues and slow clients get resynced with a
//! fresh keyframe instead of an unbounded backlog.

use sim_core::World;
use std::collections::VecDeque;
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

pub struct Control {
    pub paused: bool,
    pub speed_q16: u32,
}

/// How the tick thread paces itself.
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

pub struct Shared {
    pub world: Mutex<World>,
    pub control: Mutex<Control>,
    pub audit: Mutex<Vec<AuditRecord>>,
    pub clients: Mutex<Vec<Arc<ClientSlot>>>,
    pub next_client_id: AtomicU64,
    pub next_audit_id: AtomicU64,
    pub world_epoch: u64,
    pub observer_token: String,
    pub admin_token: String,
    /// Ring of recent tick durations (microseconds) for benchmarking.
    pub tick_samples_us: Mutex<VecDeque<f64>>,
    pub ticks_total: AtomicU64,
    /// Snapshot store (None disables persistence endpoints/checkpoints).
    /// Shared rather than owned so the asynchronous checkpoint writer and
    /// the REST save endpoints can hold the same catalog.
    pub store: Option<Arc<Mutex<sim_persist::SnapshotStore>>>,
    /// Wall-clock seconds between automatic checkpoints (0 disables).
    pub checkpoint_interval_secs: u64,
    pub checkpoint_keep: usize,
    pub checkpoint_mode: CheckpointMode,
    pub pacing: Pacing,
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

impl Shared {
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

    pub fn record_audit(
        &self,
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
