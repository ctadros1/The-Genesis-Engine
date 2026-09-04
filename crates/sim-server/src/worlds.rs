//! World lifecycle: building a runtime, its tick thread, its durable
//! saves, and the JSON and Prometheus renderings that report it.
//!
//! One tick thread per world (ADR-0039). Threads share nothing but the
//! process-level `Hub`, so a control on one world cannot reach another and
//! a stopped world's thread exits without disturbing the rest.

use crate::json::escape;
use crate::state::{
    CheckpointMode, Control, Hub, PRIMARY_WORLD_ID, Pacing, WorldRuntime, now_unix_ms,
};
use crate::stream;
use sim_core::World;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A runtime around an already-built world. The caller has validated the
/// config and assigned the id; this only wires up the bookkeeping.
#[allow(clippy::too_many_arguments)]
pub fn new_runtime(
    id: u64,
    name: String,
    preset: String,
    parent_world_id: u64,
    world_epoch: u64,
    world: World,
    paused: bool,
    speed_q16: u32,
) -> WorldRuntime {
    let dt_ms = u64::from(world.config().dt_ms);
    WorldRuntime {
        id,
        name,
        preset,
        created_unix_ms: now_unix_ms(),
        parent_world_id,
        world_epoch,
        world: Mutex::new(world),
        control: Mutex::new(Control { paused, speed_q16 }),
        clients: Mutex::new(Vec::new()),
        stopped: AtomicBool::new(false),
        tick_samples_us: Mutex::new(VecDeque::with_capacity(4_096)),
        ticks_total: AtomicU64::new(0),
        ticks_per_second_milli: AtomicU64::new(0),
        dt_ms,
        saves_total: AtomicU64::new(0),
        save_failures_total: AtomicU64::new(0),
        last_save_duration_us: AtomicU64::new(0),
        last_save_bytes: AtomicU64::new(0),
        checkpoints_skipped: AtomicU64::new(0),
        last_capture_us: AtomicU64::new(0),
        last_control_ms: AtomicU64::new(0),
    }
}

/// Register a runtime and start its tick thread. `run_ticks` belongs to
/// world 1 only: it ends the *process*, which is what A5.1 measures.
pub fn start(hub: &Arc<Hub>, runtime: Arc<WorldRuntime>, run_ticks: Option<u64>) {
    hub.worlds
        .lock()
        .expect("worlds")
        .insert(runtime.id, Arc::clone(&runtime));
    let hub = Arc::clone(hub);
    std::thread::spawn(move || tick_loop(hub, runtime, run_ticks));
}

// --- Durable saves ----------------------------------------------------------

/// Perform one durable save (checkpoint or named) for one world. State
/// capture happens under that world's lock; encoding, compression, and
/// fsync happen outside it so only the export clone stalls other readers.
pub fn perform_save(
    hub: &Hub,
    runtime: &WorldRuntime,
    name: &str,
    kind: &str,
) -> Result<sim_persist::SaveRecord, String> {
    let Some(store) = hub.store.as_ref() else {
        return Err("no --data-dir configured".to_owned());
    };
    let started = Instant::now();
    let (state, checksum) = {
        let world = runtime.world.lock().expect("world");
        (world.export_state(), world.state_checksum())
    };
    let result = store.lock().expect("store").save(
        &state,
        checksum,
        runtime.id,
        runtime.parent_world_id,
        name,
        kind,
        0,
        Some(3),
    );
    match result {
        Ok(record) => {
            runtime.saves_total.fetch_add(1, Ordering::Relaxed);
            runtime
                .last_save_duration_us
                .store(started.elapsed().as_micros() as u64, Ordering::Relaxed);
            runtime.last_save_bytes.store(record.bytes, Ordering::Relaxed);
            if kind == "checkpoint" {
                prune_checkpoints(hub, runtime.id);
            }
            Ok(record)
        }
        Err(error) => {
            runtime.save_failures_total.fetch_add(1, Ordering::Relaxed);
            Err(error.to_string())
        }
    }
}

/// Retain `--checkpoint-keep` checkpoints for `world_id`.
///
/// `SnapshotStore::prune_checkpoints` keeps the newest N checkpoints in the
/// whole catalog, which is correct for the one-world server it was written
/// for and destructive here: world 1's automatic checkpoints would delete
/// another world's final one. Until the store grows a per-world prune, the
/// prune runs only when every checkpoint in the catalog belongs to this
/// world. Retaining too many checkpoints costs disk; deleting another
/// world's last state costs the world.
fn prune_checkpoints(hub: &Hub, world_id: u64) {
    let Some(store) = hub.store.as_ref() else {
        return;
    };
    let store = store.lock().expect("store");
    let Ok(records) = store.list() else { return };
    let ours = records
        .iter()
        .filter(|record| record.kind == "checkpoint")
        .all(|record| record.world_id == world_id);
    if ours {
        let _ = store.prune_checkpoints(hub.checkpoint_keep);
    }
}

/// Capture state on the tick thread and hand the write to the background
/// writer. Capture is the only cost the tick thread pays.
fn submit_checkpoint(
    hub: &Hub,
    runtime: &WorldRuntime,
    checkpointer: &sim_persist::AsyncCheckpointer,
    tick: u64,
) {
    let capture_started = Instant::now();
    let (state, checksum) = {
        let world = runtime.world.lock().expect("world");
        (world.export_state(), world.state_checksum())
    };
    runtime.last_capture_us.store(
        capture_started.elapsed().as_micros() as u64,
        Ordering::Relaxed,
    );
    let request = sim_persist::CheckpointRequest {
        state,
        state_checksum: checksum,
        world_id: runtime.id,
        parent_world_id: runtime.parent_world_id,
        name: "auto".to_owned(),
        kind: "checkpoint".to_owned(),
        event_log_offset: 0,
        compression_level: Some(3),
        prune_keep: Some(hub.checkpoint_keep),
    };
    if checkpointer.submit(request) == sim_persist::SubmitResult::Busy {
        // Refused, counted, and audited. The checkpoint interval is shorter
        // than a checkpoint takes, and pretending otherwise would make the
        // interval a lie.
        runtime.checkpoints_skipped.fetch_add(1, Ordering::Relaxed);
        hub.record_audit(
            runtime.id,
            "service",
            "checkpoint",
            false,
            "skipped: previous checkpoint still writing",
            tick,
            "",
        );
    }
}

// --- Tick loop --------------------------------------------------------------

fn tick_loop(hub: Arc<Hub>, runtime: Arc<WorldRuntime>, run_ticks: Option<u64>) {
    let dt_ms = runtime.dt_ms;
    let mut scratch = Vec::new();
    let mut next_deadline = Instant::now();
    let mut last_metrics = Instant::now();
    let mut last_checkpoint = Instant::now();
    let mut rate_window = (Instant::now(), 0_u64);
    // Automatic checkpoints follow the process's checkpoint flags, which
    // describe the world those flags built. A created world is checkpointed
    // when it is asked to be (a save, or the final one written when it
    // stops), never on a schedule nobody set for it.
    let scheduled_checkpoints =
        runtime.id == PRIMARY_WORLD_ID && hub.checkpoint_interval_secs > 0 && hub.store.is_some();
    // The asynchronous writer exists only when both a store and the
    // asynchronous mode are configured; otherwise the Phase 4 synchronous
    // path runs unchanged.
    let checkpointer = match (hub.store.as_ref(), hub.checkpoint_mode, scheduled_checkpoints) {
        (Some(store), CheckpointMode::Async, true) => {
            Some(sim_persist::AsyncCheckpointer::spawn(Arc::clone(store)))
        }
        _ => None,
    };
    loop {
        if runtime.stopped.load(Ordering::Relaxed) {
            if let Some(checkpointer) = checkpointer {
                drain_outcomes(&hub, &runtime, &checkpointer);
                let _ = checkpointer.shutdown();
            }
            finish_stopped_world(&hub, &runtime);
            return;
        }
        if let Some(limit) = run_ticks
            && runtime.ticks_total.load(Ordering::Relaxed) >= limit
        {
            // Finish any checkpoint still in flight before reporting, so
            // the summary describes a settled world.
            if let Some(checkpointer) = checkpointer {
                let outcomes = checkpointer.shutdown();
                for outcome in outcomes.iter().filter(|outcome| outcome.error.is_some()) {
                    eprintln!(
                        "checkpoint at tick {} failed: {}",
                        outcome.tick,
                        outcome.error.as_deref().unwrap_or("")
                    );
                }
            }
            crate::print_run_summary(&hub, &runtime, limit);
            std::process::exit(0);
        }
        let (paused, speed_q16) = {
            let control = runtime.control.lock().expect("control");
            (control.paused, control.speed_q16)
        };
        // A paused world advances zero ticks in either pacing mode: pausing
        // is world state, not a pacing policy.
        if paused || (hub.pacing == Pacing::Realtime && speed_q16 == 0) {
            std::thread::sleep(Duration::from_millis(20));
            next_deadline = Instant::now();
            update_rate(&runtime, &mut rate_window);
            continue;
        }
        // Real-time pacing: interval = dt / speed. Headless pacing never
        // sleeps, so the speed multiplier is ignored entirely rather than
        // being reinterpreted as a large one.
        let interval_us = (dt_ms * 1_000 * 65_536) / u64::from(speed_q16).max(1);
        let started = Instant::now();
        {
            let mut world = runtime.world.lock().expect("world");
            world.step();
        }
        let elapsed_us = started.elapsed().as_secs_f64() * 1_000_000.0;
        {
            let mut samples = runtime.tick_samples_us.lock().expect("samples");
            if samples.len() >= 4_096 {
                samples.pop_front();
            }
            samples.push_back(elapsed_us);
        }
        runtime.ticks_total.fetch_add(1, Ordering::Relaxed);
        update_rate(&runtime, &mut rate_window);

        // Stream state frames (per-client rate limiting inside).
        stream::broadcast(&runtime, &mut scratch);
        if last_metrics.elapsed() >= Duration::from_secs(1) {
            stream::broadcast_metrics(&runtime);
            last_metrics = Instant::now();
        }
        // Automatic checkpoints at completed tick boundaries.
        if scheduled_checkpoints
            && last_checkpoint.elapsed() >= Duration::from_secs(hub.checkpoint_interval_secs)
        {
            let tick = runtime.ticks_total.load(Ordering::Relaxed);
            match checkpointer.as_ref() {
                Some(checkpointer) => submit_checkpoint(&hub, &runtime, checkpointer, tick),
                None => match perform_save(&hub, &runtime, "auto", "checkpoint") {
                    Ok(record) => hub.record_audit(
                        runtime.id,
                        "service",
                        "checkpoint",
                        true,
                        &format!("save_id {} bytes {}", record.save_id, record.bytes),
                        tick,
                        "",
                    ),
                    Err(error) => {
                        hub.record_audit(
                            runtime.id,
                            "service",
                            "checkpoint",
                            false,
                            &error,
                            tick,
                            "",
                        );
                    }
                },
            }
            last_checkpoint = Instant::now();
        }
        // Completed asynchronous writes are reported here rather than on
        // the writer thread, so audit ordering stays on one thread.
        if let Some(checkpointer) = checkpointer.as_ref() {
            drain_outcomes(&hub, &runtime, checkpointer);
        }

        if hub.pacing == Pacing::Headless {
            // Nothing to wait for: the kernel reads no clock, so running
            // free cannot change a result. A5.1 is the proof.
            continue;
        }
        next_deadline += Duration::from_micros(interval_us);
        let now = Instant::now();
        if next_deadline > now {
            std::thread::sleep(next_deadline - now);
        } else {
            // Falling behind (turbo speed or heavy load): never sleep-debt.
            next_deadline = now;
        }
    }
}

fn drain_outcomes(hub: &Hub, runtime: &WorldRuntime, checkpointer: &sim_persist::AsyncCheckpointer) {
    for outcome in checkpointer.drain_outcomes() {
        match (&outcome.record, &outcome.error) {
            (Some(record), _) => {
                runtime.saves_total.fetch_add(1, Ordering::Relaxed);
                runtime
                    .last_save_duration_us
                    .store(outcome.duration_us, Ordering::Relaxed);
                runtime
                    .last_save_bytes
                    .store(outcome.bytes, Ordering::Relaxed);
                hub.record_audit(
                    runtime.id,
                    "service",
                    "checkpoint",
                    true,
                    &format!("save_id {} bytes {}", record.save_id, record.bytes),
                    outcome.tick,
                    "",
                );
            }
            (None, Some(error)) => {
                runtime.save_failures_total.fetch_add(1, Ordering::Relaxed);
                hub.record_audit(
                    runtime.id,
                    "service",
                    "checkpoint",
                    false,
                    error,
                    outcome.tick,
                    "",
                );
            }
            (None, None) => {}
        }
    }
}

/// A stopped world's last act: one checkpoint, so the state it stopped at
/// can be branched later. A stop that dropped the world would make "a
/// stopped world stays readable and saveable" true only until the process
/// ends.
fn finish_stopped_world(hub: &Hub, runtime: &WorldRuntime) {
    if hub.store.is_none() {
        return;
    }
    let tick = runtime.tick_number();
    match perform_save(hub, runtime, "final", "checkpoint") {
        Ok(record) => hub.record_audit(
            runtime.id,
            "service",
            "checkpoint",
            true,
            &format!("save_id {} bytes {} final", record.save_id, record.bytes),
            tick,
            "",
        ),
        Err(error) => hub.record_audit(runtime.id, "service", "checkpoint", false, &error, tick, ""),
    }
}

/// Refresh the measured tick rate once a second.
fn update_rate(runtime: &WorldRuntime, window: &mut (Instant, u64)) {
    let elapsed = window.0.elapsed();
    if elapsed < Duration::from_secs(1) {
        return;
    }
    let ticks = runtime.ticks_total.load(Ordering::Relaxed);
    let advanced = ticks.saturating_sub(window.1);
    let milli = (advanced as f64 * 1_000.0 / elapsed.as_secs_f64()) as u64;
    runtime.ticks_per_second_milli.store(milli, Ordering::Relaxed);
    *window = (Instant::now(), ticks);
}

// --- Rendering --------------------------------------------------------------

pub fn control_state_json(runtime: &WorldRuntime) -> String {
    let status = runtime.status();
    let control = runtime.control.lock().expect("control");
    format!(
        "{{\"paused\":{},\"speed_multiplier\":{:.4},\"status\":\"{status}\"}}",
        control.paused,
        f64::from(control.speed_q16) / 65_536.0
    )
}

/// One world's summary. Every field world 1 reported before ADR-0039 keeps
/// its name and shape; the identity fields the console needs are added.
pub fn summary_json(runtime: &WorldRuntime) -> String {
    let status = runtime.status();
    let tick_mean_us = runtime.tick_mean_us();
    let ticks_per_second =
        runtime.ticks_per_second_milli.load(Ordering::Relaxed) as f64 / 1_000.0;
    let world = runtime.world.lock().expect("world");
    let metrics = world.metrics();
    let control = runtime.control.lock().expect("control");
    format!(
        concat!(
            "{{\"world_id\":{},\"world_epoch\":{},\"tick\":{},\"population\":{},",
            "\"births_total\":{},\"deaths_starvation_total\":{},\"deaths_old_age_total\":{},",
            "\"paired_births_total\":{},\"max_ancestry_depth\":{},\"extinct\":{},",
            "\"phase2\":{},\"paused\":{},\"speed_multiplier\":{:.4},",
            "\"config_hash\":\"0x{:016x}\",\"seed\":\"0x{:016x}\",",
            "\"cells_x\":{},\"cells_y\":{},\"cell_size_m\":{},\"dt_ms\":{},",
            "\"total_biomass_milli\":{},\"total_energy_milli\":{},",
            "\"name\":\"{}\",\"preset\":\"{}\",\"status\":\"{}\",",
            "\"created_unix_ms\":{},\"parent_world_id\":{},",
            "\"tick_mean_us\":{:.3},\"ticks_per_second\":{:.3}}}"
        ),
        runtime.id,
        runtime.world_epoch,
        metrics.tick,
        metrics.population,
        metrics.births_total,
        metrics.deaths_starvation_total,
        metrics.deaths_old_age_total,
        metrics.paired_births_total,
        metrics.max_ancestry_depth,
        metrics.extinct,
        metrics.phase2_enabled,
        control.paused,
        f64::from(control.speed_q16) / 65_536.0,
        world.config_hash(),
        world.config().world_seed,
        world.config().cells_x,
        world.config().cells_y,
        world.config().cell_size_m,
        world.config().dt_ms,
        metrics.total_biomass_milli,
        metrics.total_energy_milli,
        escape(&runtime.name),
        escape(&runtime.preset),
        status,
        runtime.created_unix_ms,
        runtime.parent_world_id,
        tick_mean_us,
        ticks_per_second
    )
}

pub fn tick_stats_json(runtime: &WorldRuntime, samples: &[f64]) -> String {
    let clients = runtime.clients.lock().expect("clients");
    let mut client_stats = String::from("[");
    for (index, slot) in clients.iter().enumerate() {
        if index > 0 {
            client_stats.push(',');
        }
        client_stats.push_str(&format!(
            "{{\"client_id\":{},\"bytes_sent\":{},\"dropped_updates\":{}}}",
            slot.id,
            slot.bytes_sent.load(Ordering::Relaxed),
            slot.dropped_updates.load(Ordering::Relaxed)
        ));
    }
    client_stats.push(']');
    if samples.is_empty() {
        return format!(
            "{{\"world_id\":{},\"samples\":0,\"ticks_total\":{},\"clients\":{client_stats}}}",
            runtime.id,
            runtime.ticks_total.load(Ordering::Relaxed)
        );
    }
    let mut sorted: Vec<f64> = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| -> f64 {
        let index = ((sorted.len() - 1) as f64 * fraction).ceil() as usize;
        sorted[index]
    };
    format!(
        concat!(
            "{{\"world_id\":{},\"samples\":{},\"ticks_total\":{},\"tick_microseconds\":",
            "{{\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"min\":{:.3},\"max\":{:.3}}},",
            "\"clients\":{}}}"
        ),
        runtime.id,
        sorted.len(),
        runtime.ticks_total.load(Ordering::Relaxed),
        percentile(0.50),
        percentile(0.95),
        percentile(0.99),
        sorted[0],
        sorted[sorted.len() - 1],
        client_stats
    )
}

/// Prometheus exposition for every hosted world.
///
/// World 1's lines are byte-identical to the single-world server's, label
/// for label: dashboards and the metrics schema row that names them predate
/// this increment. Every other world repeats the same series with a
/// `world_id` label, and the series are grouped by family so each `# TYPE`
/// is written once.
pub fn metrics_text(hub: &Hub) -> String {
    let worlds = hub.all_worlds();
    let mut text = String::new();
    // `world="server"` is the pre-existing label value for the flag-built
    // world; the id distinguishes the rest without moving world 1's line.
    let labels = |runtime: &WorldRuntime, extra: &str| -> String {
        let mut labels = String::from("world=\"server\"");
        if runtime.id != PRIMARY_WORLD_ID {
            labels.push_str(&format!(",world_id=\"{}\"", runtime.id));
        }
        labels.push_str(extra);
        labels
    };
    let snapshots: Vec<(Arc<WorldRuntime>, sim_core::MetricsSnapshot)> = worlds
        .iter()
        .map(|runtime| {
            let metrics = runtime.world.lock().expect("world").metrics();
            (Arc::clone(runtime), metrics)
        })
        .collect();

    text.push_str("# TYPE lifesim_organisms gauge\n");
    for (runtime, metrics) in &snapshots {
        text.push_str(&format!(
            "lifesim_organisms{{{}}} {}\n",
            labels(runtime, ",life_state=\"alive\""),
            metrics.population
        ));
    }
    text.push_str("# TYPE lifesim_births_total counter\n");
    for (runtime, metrics) in &snapshots {
        text.push_str(&format!(
            "lifesim_births_total{{{}}} {}\n",
            labels(runtime, ""),
            metrics.births_total
        ));
    }
    text.push_str("# TYPE lifesim_deaths_total counter\n");
    for (runtime, metrics) in &snapshots {
        text.push_str(&format!(
            "lifesim_deaths_total{{{}}} {}\n",
            labels(runtime, ",cause=\"starvation\""),
            metrics.deaths_starvation_total
        ));
        text.push_str(&format!(
            "lifesim_deaths_total{{{}}} {}\n",
            labels(runtime, ",cause=\"old_age\""),
            metrics.deaths_old_age_total
        ));
    }
    text.push_str("# TYPE lifesim_ticks_total counter\n");
    for (runtime, _) in &snapshots {
        text.push_str(&format!(
            "lifesim_ticks_total{{{}}} {}\n",
            labels(runtime, ""),
            runtime.ticks_total.load(Ordering::Relaxed)
        ));
    }
    // Phase 13 social series (specifications/metrics-schema.md row 13),
    // rendered only for worlds whose section is enabled: a disabled world
    // exports no social series at all rather than a wall of zeros (D-014's
    // inert rule applied to observability). Signal content is never a label.
    if snapshots.iter().any(|(_, metrics)| metrics.social_enabled) {
        let social = || snapshots.iter().filter(|(_, m)| m.social_enabled);
        text.push_str("# TYPE lifesim_signals_emitted_total counter\n");
        for (runtime, metrics) in social() {
            text.push_str(&format!(
                "lifesim_signals_emitted_total{{{}}} {}\n",
                labels(runtime, ""),
                metrics.signals_emitted_total
            ));
        }
        text.push_str("# TYPE lifesim_signal_energy_spent_milli_total counter\n");
        for (runtime, metrics) in social() {
            text.push_str(&format!(
                "lifesim_signal_energy_spent_milli_total{{{}}} {}\n",
                labels(runtime, ""),
                metrics.signal_cost_milli_total
            ));
        }
        text.push_str("# TYPE lifesim_perceived_neighbours gauge\n");
        for (runtime, metrics) in social() {
            text.push_str(&format!(
                "lifesim_perceived_neighbours{{{}}} {}\n",
                labels(runtime, ""),
                metrics.perceived_neighbours
            ));
        }
        text.push_str("# TYPE lifesim_perception_faults_total counter\n");
        for (runtime, metrics) in social() {
            text.push_str(&format!(
                "lifesim_perception_faults_total{{{}}} {}\n",
                labels(runtime, ""),
                metrics.perception_faults_total
            ));
        }
    }
    // Stream metrics for connected observers.
    let stream_totals: Vec<(u64, u64, usize)> = worlds
        .iter()
        .map(|runtime| {
            let clients = runtime.clients.lock().expect("clients");
            (
                clients
                    .iter()
                    .map(|slot| slot.bytes_sent.load(Ordering::Relaxed))
                    .sum(),
                clients
                    .iter()
                    .map(|slot| slot.dropped_updates.load(Ordering::Relaxed))
                    .sum(),
                clients.len(),
            )
        })
        .collect();
    text.push_str("# TYPE lifesim_stream_bytes_total counter\n");
    for (runtime, totals) in worlds.iter().zip(&stream_totals) {
        text.push_str(&format!(
            "lifesim_stream_bytes_total{{{}}} {}\n",
            labels(runtime, ",client_class=\"observer\""),
            totals.0
        ));
    }
    text.push_str("# TYPE lifesim_observer_dropped_updates_total counter\n");
    for (runtime, totals) in worlds.iter().zip(&stream_totals) {
        text.push_str(&format!(
            "lifesim_observer_dropped_updates_total{{{}}} {}\n",
            labels(runtime, ",reason=\"backpressure\""),
            totals.1
        ));
    }
    text.push_str("# TYPE lifesim_observers gauge\n");
    for (runtime, totals) in worlds.iter().zip(&stream_totals) {
        text.push_str(&format!(
            "lifesim_observers{{{}}} {}\n",
            labels(runtime, ""),
            totals.2
        ));
    }
    // Save metrics (zero when persistence is disabled).
    text.push_str("# TYPE lifesim_saves_total counter\n");
    for runtime in &worlds {
        text.push_str(&format!(
            "lifesim_saves_total{{{}}} {}\n",
            labels(runtime, ",result=\"ok\""),
            runtime.saves_total.load(Ordering::Relaxed)
        ));
        text.push_str(&format!(
            "lifesim_saves_total{{{}}} {}\n",
            labels(runtime, ",result=\"error\""),
            runtime.save_failures_total.load(Ordering::Relaxed)
        ));
    }
    text.push_str("# TYPE lifesim_save_duration_seconds gauge\n");
    for runtime in &worlds {
        text.push_str(&format!(
            "lifesim_save_duration_seconds{{{}}} {:.6}\n",
            labels(runtime, ",result=\"last\""),
            runtime.last_save_duration_us.load(Ordering::Relaxed) as f64 / 1_000_000.0
        ));
    }
    text.push_str("# TYPE lifesim_save_bytes gauge\n");
    for runtime in &worlds {
        text.push_str(&format!(
            "lifesim_save_bytes{{{}}} {}\n",
            labels(runtime, ""),
            runtime.last_save_bytes.load(Ordering::Relaxed)
        ));
    }
    // Phase 5 checkpoint instrumentation. `capture` is the only part of a
    // checkpoint the tick thread pays for in asynchronous mode, so it is
    // exported separately from the total write duration above.
    text.push_str("# TYPE lifesim_checkpoint_capture_seconds gauge\n");
    for runtime in &worlds {
        text.push_str(&format!(
            "lifesim_checkpoint_capture_seconds{{{}}} {:.6}\n",
            labels(runtime, &format!(",mode=\"{}\"", hub.checkpoint_mode.name())),
            runtime.last_capture_us.load(Ordering::Relaxed) as f64 / 1_000_000.0
        ));
    }
    text.push_str("# TYPE lifesim_checkpoints_skipped_total counter\n");
    for runtime in &worlds {
        text.push_str(&format!(
            "lifesim_checkpoints_skipped_total{{{}}} {}\n",
            labels(runtime, ""),
            runtime.checkpoints_skipped.load(Ordering::Relaxed)
        ));
    }
    text
}
