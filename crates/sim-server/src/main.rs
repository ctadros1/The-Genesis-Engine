//! Private observer/control server for one authoritative world.
//!
//! Threads: one tick loop (owns pacing; never blocks on client I/O), one
//! REST acceptor, one WebSocket acceptor, one thread per WS session.
//! Security posture: private LAN boundary; every request needs a bearer
//! token (observer or admin role). Tokens come from the environment or are
//! generated at startup and printed once; nothing is committed. TLS and
//! reverse-proxy choices remain deployment decisions per the security
//! model. This binary deploys nothing and touches no infrastructure.

mod state;
mod stream;

use sim_core::{SimConfig, World, analyze};
use state::{CheckpointMode, Control, MAX_SPEED_Q16, Pacing, Role, Shared, now_unix_ms};
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpListener;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_SEED: u64 = 0x5eed_cafe_f00d_beef;
const DEFAULT_REST_PORT: u16 = 8940;
const DEFAULT_WS_PORT: u16 = 8941;
/// Minimum interval between accepted control mutations.
const CONTROL_RATE_LIMIT_MS: u64 = 100;

fn main() {
    if let Err(error) = run() {
        eprintln!("lifesim-server: {error}");
        std::process::exit(2);
    }
}

struct Options {
    seed: u64,
    organisms: u32,
    phase2: bool,
    rest_port: u16,
    ws_port: u16,
    speed_q16: u32,
    paused: bool,
    data_dir: Option<std::path::PathBuf>,
    checkpoint_interval_secs: u64,
    checkpoint_keep: usize,
    checkpoint_mode: CheckpointMode,
    load_save: Option<std::path::PathBuf>,
    pacing: Pacing,
    /// Stop after this many ticks and print a fixture line. Exists so
    /// acceleration neutrality (A5.1) is an automated test rather than a
    /// manual procedure.
    run_ticks: Option<u64>,
}

fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        seed: DEFAULT_SEED,
        organisms: 500,
        phase2: true,
        rest_port: DEFAULT_REST_PORT,
        ws_port: DEFAULT_WS_PORT,
        speed_q16: 1 << 16,
        paused: false,
        data_dir: None,
        checkpoint_interval_secs: 0,
        checkpoint_keep: 4,
        checkpoint_mode: CheckpointMode::Async,
        load_save: None,
        pacing: Pacing::Realtime,
        run_ticks: None,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--no-phase2" => {
                options.phase2 = false;
                index += 1;
                continue;
            }
            "--paused" => {
                options.paused = true;
                index += 1;
                continue;
            }
            name => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {name}"))?;
                match name {
                    "--seed" => {
                        options.seed = value
                            .strip_prefix("0x")
                            .map_or_else(|| value.parse(), |hex| u64::from_str_radix(hex, 16))
                            .map_err(|_| format!("invalid seed {value}"))?;
                    }
                    "--organisms" => {
                        options.organisms = value
                            .parse()
                            .map_err(|_| format!("invalid count {value}"))?;
                    }
                    "--rest-port" => {
                        options.rest_port =
                            value.parse().map_err(|_| format!("invalid port {value}"))?;
                    }
                    "--ws-port" => {
                        options.ws_port =
                            value.parse().map_err(|_| format!("invalid port {value}"))?;
                    }
                    "--speed" => {
                        let speed: f64 = value
                            .parse()
                            .map_err(|_| format!("invalid speed {value}"))?;
                        if !(0.0..=64.0).contains(&speed) {
                            return Err("speed must be in [0, 64]".to_owned());
                        }
                        options.speed_q16 = (speed * 65_536.0) as u32;
                    }
                    "--data-dir" => {
                        options.data_dir = Some(std::path::PathBuf::from(value));
                    }
                    "--checkpoint-interval-secs" => {
                        options.checkpoint_interval_secs = value
                            .parse()
                            .map_err(|_| format!("invalid interval {value}"))?;
                    }
                    "--checkpoint-keep" => {
                        options.checkpoint_keep =
                            value.parse().map_err(|_| format!("invalid keep {value}"))?;
                    }
                    "--checkpoint-mode" => {
                        options.checkpoint_mode = match value.as_str() {
                            "sync" => CheckpointMode::Sync,
                            "async" => CheckpointMode::Async,
                            _ => {
                                return Err(format!(
                                    "checkpoint mode must be sync or async, got {value}"
                                ));
                            }
                        };
                    }
                    "--load-save" => {
                        options.load_save = Some(std::path::PathBuf::from(value));
                    }
                    "--pacing" => {
                        options.pacing = match value.as_str() {
                            "realtime" => Pacing::Realtime,
                            "headless" => Pacing::Headless,
                            _ => {
                                return Err(format!(
                                    "pacing must be realtime or headless, got {value}"
                                ));
                            }
                        };
                    }
                    "--run-ticks" => {
                        options.run_ticks = Some(
                            value
                                .parse()
                                .map_err(|_| format!("invalid tick count {value}"))?,
                        );
                    }
                    _ => return Err(format!("unknown option {name}")),
                }
                index += 2;
            }
        }
    }
    Ok(options)
}

/// Token from the environment, or a random one generated from the OS
/// entropy source and printed once at startup (never persisted).
fn resolve_token(env_name: &str) -> String {
    if let Ok(value) = std::env::var(env_name)
        && !value.is_empty()
    {
        return value;
    }
    let mut bytes = [0_u8; 24];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .is_err()
    {
        // Extremely private fallback; still unpredictable enough for a
        // LAN-local development boundary.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos())
            .unwrap_or(0);
        bytes[..4].copy_from_slice(&nanos.to_le_bytes());
        bytes[4..8].copy_from_slice(&std::process::id().to_le_bytes());
    }
    let mut token = String::with_capacity(48);
    for byte in bytes {
        token.push_str(&format!("{byte:02x}"));
    }
    println!("{env_name} not set; generated token: {token}");
    token
}

fn run() -> Result<(), String> {
    let options = parse_options()?;
    // Start fresh from flags, or branch from a validated snapshot (the
    // recorded configuration wins; the branch gets a new world epoch).
    let (world, world_epoch) = match &options.load_save {
        Some(path) => {
            let (info, world) = sim_persist::SnapshotStore::load_world(path)
                .map_err(|error| format!("load-save: {error}"))?;
            println!(
                "branched from save: tick {} config 0x{:016x} build {}",
                info.tick, info.config_hash, info.build_version
            );
            (world, 2)
        }
        None => {
            let mut config = SimConfig::phase1_default(options.seed);
            config.initial_organisms = options.organisms;
            config.phase2.enabled = options.phase2;
            config.validate().map_err(|error| error.to_string())?;
            (World::new(config).map_err(|error| error.to_string())?, 1)
        }
    };
    let dt_ms = u64::from(world.config().dt_ms);

    let store = match &options.data_dir {
        Some(directory) => {
            let (store, recovery) = sim_persist::SnapshotStore::open(directory)
                .map_err(|error| format!("data dir: {error}"))?;
            println!(
                "snapshot store: {} valid, {} broken, {} temp files removed",
                recovery.valid_saves, recovery.broken_saves, recovery.removed_temp_files
            );
            Some(Arc::new(Mutex::new(store)))
        }
        None => None,
    };

    let shared = Arc::new(Shared {
        world: Mutex::new(world),
        control: Mutex::new(Control {
            paused: options.paused,
            speed_q16: options.speed_q16,
        }),
        audit: Mutex::new(Vec::new()),
        clients: Mutex::new(Vec::new()),
        next_client_id: AtomicU64::new(1),
        next_audit_id: AtomicU64::new(1),
        world_epoch,
        observer_token: resolve_token("LIFESIM_OBSERVER_TOKEN"),
        admin_token: resolve_token("LIFESIM_ADMIN_TOKEN"),
        tick_samples_us: Mutex::new(std::collections::VecDeque::with_capacity(4_096)),
        ticks_total: AtomicU64::new(0),
        store,
        checkpoint_interval_secs: options.checkpoint_interval_secs,
        checkpoint_keep: options.checkpoint_keep,
        checkpoint_mode: options.checkpoint_mode,
        pacing: options.pacing,
        saves_total: AtomicU64::new(0),
        save_failures_total: AtomicU64::new(0),
        last_save_duration_us: AtomicU64::new(0),
        last_save_bytes: AtomicU64::new(0),
        checkpoints_skipped: AtomicU64::new(0),
        last_capture_us: AtomicU64::new(0),
    });

    // Both listeners bind before the banner prints. The banner is the
    // readiness signal every integration test waits on, so printing it
    // while the REST port is still unbound makes it a lie that fails as an
    // intermittent connection-refused in whichever test connects fastest.
    let ws_listener = TcpListener::bind(("127.0.0.1", options.ws_port))
        .map_err(|error| format!("ws bind: {error}"))?;
    let server = tiny_http::Server::http(("127.0.0.1", options.rest_port))
        .map_err(|error| format!("rest bind: {error}"))?;
    println!(
        "lifesim-server: REST on 127.0.0.1:{} WS on 127.0.0.1:{} (private, loopback only)",
        options.rest_port, options.ws_port
    );

    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || stream::websocket_listener(shared, ws_listener));
    }
    {
        let shared = Arc::clone(&shared);
        let run_ticks = options.run_ticks;
        std::thread::spawn(move || tick_loop(shared, dt_ms, run_ticks));
    }

    let idempotency: Mutex<HashMap<String, (u16, String)>> = Mutex::new(HashMap::new());
    let last_control_ms = AtomicU64::new(0);
    for request in server.incoming_requests() {
        handle_request(&shared, request, &idempotency, &last_control_ms);
    }
    Ok(())
}

// --- Tick loop --------------------------------------------------------------

/// Perform one durable save (checkpoint or named). State capture happens
/// under the world lock; encoding, compression, and fsync happen outside
/// it so only the export clone stalls other world readers.
fn perform_save(
    shared: &Shared,
    name: &str,
    kind: &str,
) -> Result<sim_persist::SaveRecord, String> {
    let Some(store) = shared.store.as_ref() else {
        return Err("no --data-dir configured".to_owned());
    };
    let started = Instant::now();
    let (state, checksum) = {
        let world = shared.world.lock().expect("world");
        (world.export_state(), world.state_checksum())
    };
    let result = store.lock().expect("store").save(
        &state,
        checksum,
        1,
        if shared.world_epoch > 1 { 1 } else { 0 },
        name,
        kind,
        0,
        Some(3),
    );
    match result {
        Ok(record) => {
            shared.saves_total.fetch_add(1, Ordering::Relaxed);
            shared
                .last_save_duration_us
                .store(started.elapsed().as_micros() as u64, Ordering::Relaxed);
            shared
                .last_save_bytes
                .store(record.bytes, Ordering::Relaxed);
            if kind == "checkpoint" {
                let _ = store
                    .lock()
                    .expect("store")
                    .prune_checkpoints(shared.checkpoint_keep);
            }
            Ok(record)
        }
        Err(error) => {
            shared.save_failures_total.fetch_add(1, Ordering::Relaxed);
            Err(error.to_string())
        }
    }
}

fn tick_loop(shared: Arc<Shared>, dt_ms: u64, run_ticks: Option<u64>) {
    let mut scratch = Vec::new();
    let mut next_deadline = Instant::now();
    let mut last_metrics = Instant::now();
    let mut last_checkpoint = Instant::now();
    // The asynchronous writer exists only when both a store and the
    // asynchronous mode are configured; otherwise the Phase 4 synchronous
    // path runs unchanged.
    let checkpointer = match (shared.store.as_ref(), shared.checkpoint_mode) {
        (Some(store), CheckpointMode::Async) => {
            Some(sim_persist::AsyncCheckpointer::spawn(Arc::clone(store)))
        }
        _ => None,
    };
    loop {
        if let Some(limit) = run_ticks
            && shared.ticks_total.load(Ordering::Relaxed) >= limit
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
            print_run_summary(&shared, limit);
            std::process::exit(0);
        }
        let (paused, speed_q16) = {
            let control = shared.control.lock().expect("control");
            (control.paused, control.speed_q16)
        };
        // A paused world advances zero ticks in either pacing mode: pausing
        // is world state, not a pacing policy.
        if paused || (shared.pacing == Pacing::Realtime && speed_q16 == 0) {
            std::thread::sleep(Duration::from_millis(20));
            next_deadline = Instant::now();
            continue;
        }
        // Real-time pacing: interval = dt / speed. Headless pacing never
        // sleeps, so the speed multiplier is ignored entirely rather than
        // being reinterpreted as a large one.
        let interval_us = (dt_ms * 1_000 * 65_536) / u64::from(speed_q16).max(1);
        let started = Instant::now();
        {
            let mut world = shared.world.lock().expect("world");
            world.step();
        }
        let elapsed_us = started.elapsed().as_secs_f64() * 1_000_000.0;
        {
            let mut samples = shared.tick_samples_us.lock().expect("samples");
            if samples.len() >= 4_096 {
                samples.pop_front();
            }
            samples.push_back(elapsed_us);
        }
        shared.ticks_total.fetch_add(1, Ordering::Relaxed);

        // Stream state frames (per-client rate limiting inside).
        stream::broadcast(&shared, &mut scratch);
        if last_metrics.elapsed() >= Duration::from_secs(1) {
            stream::broadcast_metrics(&shared);
            last_metrics = Instant::now();
        }
        // Automatic checkpoints at completed tick boundaries.
        if shared.checkpoint_interval_secs > 0
            && shared.store.is_some()
            && last_checkpoint.elapsed() >= Duration::from_secs(shared.checkpoint_interval_secs)
        {
            let tick = shared.ticks_total.load(Ordering::Relaxed);
            match checkpointer.as_ref() {
                Some(checkpointer) => submit_checkpoint(&shared, checkpointer, tick),
                None => match perform_save(&shared, "auto", "checkpoint") {
                    Ok(record) => shared.record_audit(
                        "service",
                        "checkpoint",
                        true,
                        &format!("save_id {} bytes {}", record.save_id, record.bytes),
                        tick,
                        "",
                    ),
                    Err(error) => {
                        shared.record_audit("service", "checkpoint", false, &error, tick, "");
                    }
                },
            }
            last_checkpoint = Instant::now();
        }
        // Completed asynchronous writes are reported here rather than on
        // the writer thread, so audit ordering stays on one thread.
        if let Some(checkpointer) = checkpointer.as_ref() {
            for outcome in checkpointer.drain_outcomes() {
                match (&outcome.record, &outcome.error) {
                    (Some(record), _) => {
                        shared.saves_total.fetch_add(1, Ordering::Relaxed);
                        shared
                            .last_save_duration_us
                            .store(outcome.duration_us, Ordering::Relaxed);
                        shared
                            .last_save_bytes
                            .store(outcome.bytes, Ordering::Relaxed);
                        shared.record_audit(
                            "service",
                            "checkpoint",
                            true,
                            &format!("save_id {} bytes {}", record.save_id, record.bytes),
                            outcome.tick,
                            "",
                        );
                    }
                    (None, Some(error)) => {
                        shared.save_failures_total.fetch_add(1, Ordering::Relaxed);
                        shared.record_audit(
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

        if shared.pacing == Pacing::Headless {
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

/// Capture state on the tick thread and hand the write to the background
/// writer. Capture is the only cost the tick thread pays.
fn submit_checkpoint(
    shared: &Arc<Shared>,
    checkpointer: &sim_persist::AsyncCheckpointer,
    tick: u64,
) {
    let capture_started = Instant::now();
    let (state, checksum) = {
        let world = shared.world.lock().expect("world");
        (world.export_state(), world.state_checksum())
    };
    shared.last_capture_us.store(
        capture_started.elapsed().as_micros() as u64,
        Ordering::Relaxed,
    );
    let request = sim_persist::CheckpointRequest {
        state,
        state_checksum: checksum,
        world_id: 1,
        parent_world_id: if shared.world_epoch > 1 { 1 } else { 0 },
        name: "auto".to_owned(),
        kind: "checkpoint".to_owned(),
        event_log_offset: 0,
        compression_level: Some(3),
        prune_keep: Some(shared.checkpoint_keep),
    };
    if checkpointer.submit(request) == sim_persist::SubmitResult::Busy {
        // Refused, counted, and audited. The checkpoint interval is shorter
        // than a checkpoint takes, and pretending otherwise would make the
        // interval a lie.
        shared.checkpoints_skipped.fetch_add(1, Ordering::Relaxed);
        shared.record_audit(
            "service",
            "checkpoint",
            false,
            "skipped: previous checkpoint still writing",
            tick,
            "",
        );
    }
}

/// One-line summary printed when `--run-ticks` completes. Deliberately the
/// same shape as the CLI fixture line so the two are directly comparable,
/// which is what makes A5.1 a checksum equality rather than a description.
fn print_run_summary(shared: &Arc<Shared>, requested: u64) {
    let world = shared.world.lock().expect("world");
    let metrics = world.metrics();
    println!(
        concat!(
            "{{\"server_run_schema_version\":1,\"pacing\":\"{}\",",
            "\"checkpoint_mode\":\"{}\",\"ticks_requested\":{},\"final_tick\":{},",
            "\"seed\":\"0x{:016x}\",\"config_hash\":\"0x{:016x}\",",
            "\"population\":{},\"births_total\":{},\"extinct\":{},",
            "\"checkpoints_skipped\":{},",
            "\"terrain_checksum\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\"}}"
        ),
        shared.pacing.name(),
        shared.checkpoint_mode.name(),
        requested,
        world.tick_number(),
        world.config().world_seed,
        world.config_hash(),
        metrics.population,
        metrics.births_total,
        metrics.extinct,
        shared.checkpoints_skipped.load(Ordering::Relaxed),
        world.terrain().terrain_checksum,
        world.state_checksum()
    );
}

// --- REST -------------------------------------------------------------------

fn bearer_token(request: &tiny_http::Request) -> Option<String> {
    for header in request.headers() {
        if header.field.equiv("Authorization") {
            let value = header.value.as_str();
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Some(token.trim().to_owned());
            }
        }
    }
    None
}

fn header_value(request: &tiny_http::Request, name: &str) -> Option<String> {
    for header in request.headers() {
        if header.field.as_str().as_str().eq_ignore_ascii_case(name) {
            return Some(header.value.as_str().to_owned());
        }
    }
    None
}

fn respond(request: tiny_http::Request, status: u16, content_type: &str, body: String) {
    // Private-LAN CORS: the observer app runs on a different local origin
    // and authenticates with bearer tokens (no cookies), so a permissive
    // origin does not widen the authorization boundary.
    let response = tiny_http::Response::from_string(body)
        .with_status_code(status)
        .with_header(
            tiny_http::Header::from_bytes("Content-Type".as_bytes(), content_type.as_bytes())
                .expect("header"),
        )
        .with_header(
            tiny_http::Header::from_bytes("Access-Control-Allow-Origin".as_bytes(), "*".as_bytes())
                .expect("header"),
        );
    let _ = request.respond(response);
}

fn respond_json(request: tiny_http::Request, status: u16, body: String) {
    respond(request, status, "application/json", body);
}

fn query_param(url: &str, name: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name {
            return Some(value.to_owned());
        }
    }
    None
}

fn handle_request(
    shared: &Arc<Shared>,
    request: tiny_http::Request,
    idempotency: &Mutex<HashMap<String, (u16, String)>>,
    last_control_ms: &AtomicU64,
) {
    let url = request.url().to_owned();
    let path = url.split('?').next().unwrap_or("").to_owned();
    let method = request.method().clone();

    if method.as_str() == "OPTIONS" {
        let response = tiny_http::Response::empty(204)
            .with_header(
                tiny_http::Header::from_bytes(
                    "Access-Control-Allow-Origin".as_bytes(),
                    "*".as_bytes(),
                )
                .expect("header"),
            )
            .with_header(
                tiny_http::Header::from_bytes(
                    "Access-Control-Allow-Methods".as_bytes(),
                    "GET, POST, OPTIONS".as_bytes(),
                )
                .expect("header"),
            )
            .with_header(
                tiny_http::Header::from_bytes(
                    "Access-Control-Allow-Headers".as_bytes(),
                    "Authorization, Idempotency-Key, Content-Type".as_bytes(),
                )
                .expect("header"),
            );
        let _ = request.respond(response);
        return;
    }

    if path == "/api/health" {
        respond_json(request, 200, "{\"status\":\"ok\"}".to_owned());
        return;
    }

    // Everything else requires a role.
    let role = bearer_token(&request).and_then(|token| shared.role_for(&token));
    let Some(role) = role else {
        respond_json(
            request,
            401,
            "{\"error\":\"missing or invalid bearer token\"}".to_owned(),
        );
        return;
    };

    match (method.as_str(), path.as_str()) {
        ("GET", "/api/worlds") => {
            respond_json(request, 200, format!("[{}]", world_summary_json(shared)));
        }
        ("GET", "/api/worlds/1") => {
            respond_json(request, 200, world_summary_json(shared));
        }
        ("GET", "/metrics") => {
            respond(
                request,
                200,
                "text/plain; version=0.0.4",
                metrics_text(shared),
            );
        }
        ("GET", "/api/worlds/1/analysis") => {
            let body = {
                let world = shared.world.lock().expect("world");
                analyze(&world).map(|report| {
                    let mut sizes = String::new();
                    for (index, size) in report.cluster_sizes.iter().enumerate() {
                        if index > 0 {
                            sizes.push(',');
                        }
                        sizes.push_str(&size.to_string());
                    }
                    format!(
                        concat!(
                            "{{\"algorithm\":\"{}\",\"analysis_tick\":{},",
                            "\"population\":{},\"sampled\":{},\"cluster_count\":{},",
                            "\"cluster_sizes\":[{}],\"mean_pairwise_distance\":{:.6}}}"
                        ),
                        report.algorithm,
                        report.analysis_tick,
                        report.population,
                        report.sampled,
                        report.cluster_count,
                        sizes,
                        report.mean_pairwise_distance
                    )
                })
            };
            match body {
                Some(body) => respond_json(request, 200, body),
                None => respond_json(
                    request,
                    409,
                    "{\"error\":\"analysis requires a phase2 world\"}".to_owned(),
                ),
            }
        }
        ("GET", "/api/benchmarks/ticks") => {
            let samples: Vec<f64> = {
                let samples = shared.tick_samples_us.lock().expect("samples");
                samples.iter().copied().collect()
            };
            if query_param(&url, "reset").as_deref() == Some("1") {
                shared.tick_samples_us.lock().expect("samples").clear();
            }
            respond_json(request, 200, tick_stats_json(shared, &samples));
        }
        ("GET", "/api/audit") => {
            if role != Role::Admin {
                respond_json(
                    request,
                    403,
                    "{\"error\":\"admin role required\"}".to_owned(),
                );
                return;
            }
            let audit = shared.audit.lock().expect("audit");
            let mut body = String::from("[");
            for (index, record) in audit.iter().rev().take(100).enumerate() {
                if index > 0 {
                    body.push(',');
                }
                body.push_str(&format!(
                    concat!(
                        "{{\"id\":{},\"unix_ms\":{},\"role\":\"{}\",\"action\":\"{}\",",
                        "\"accepted\":{},\"detail\":\"{}\",\"tick\":{},\"idempotency_key\":\"{}\"}}"
                    ),
                    record.id,
                    record.unix_ms,
                    record.role,
                    json_escape(&record.action),
                    record.accepted,
                    json_escape(&record.detail),
                    record.tick,
                    json_escape(&record.idempotency_key)
                ));
            }
            body.push(']');
            respond_json(request, 200, body);
        }
        ("GET", "/api/worlds/1/saves") => {
            let Some(store) = shared.store.as_ref() else {
                respond_json(
                    request,
                    409,
                    "{\"error\":\"no data dir configured\"}".to_owned(),
                );
                return;
            };
            let records = store.lock().expect("store").list();
            match records {
                Ok(records) => {
                    let mut body = String::from("[");
                    for (index, record) in records.iter().enumerate() {
                        if index > 0 {
                            body.push(',');
                        }
                        body.push_str(&format!(
                            concat!(
                                "{{\"save_id\":{},\"name\":\"{}\",\"kind\":\"{}\",\"tick\":{},",
                                "\"bytes\":{},\"compressed\":{},\"format_version\":{},",
                                "\"config_hash\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",",
                                "\"created_unix_ms\":{},\"verified\":{}}}"
                            ),
                            record.save_id,
                            json_escape(&record.name),
                            json_escape(&record.kind),
                            record.tick,
                            record.bytes,
                            record.compressed,
                            record.format_version,
                            record.config_hash,
                            record.state_checksum,
                            record.created_unix_ms,
                            record.verified_unix_ms.is_some()
                        ));
                    }
                    body.push(']');
                    respond_json(request, 200, body);
                }
                Err(error) => respond_json(
                    request,
                    500,
                    format!("{{\"error\":\"{}\"}}", json_escape(&error.to_string())),
                ),
            }
        }
        ("POST", "/api/worlds/1/saves") => {
            if role != Role::Admin {
                respond_json(
                    request,
                    403,
                    "{\"error\":\"admin role required\"}".to_owned(),
                );
                return;
            }
            let name = query_param(&url, "name").unwrap_or_else(|| "manual".to_owned());
            let safe_name: String = name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .take(64)
                .collect();
            let tick = current_tick(shared);
            match perform_save(shared, &safe_name, "manual") {
                Ok(record) => {
                    shared.record_audit(
                        "admin",
                        "save",
                        true,
                        &format!("save_id {} name {safe_name}", record.save_id),
                        tick,
                        &header_value(&request, "Idempotency-Key").unwrap_or_default(),
                    );
                    respond_json(
                        request,
                        200,
                        format!(
                            "{{\"save_id\":{},\"tick\":{},\"bytes\":{},\"state_checksum\":\"0x{:016x}\"}}",
                            record.save_id, record.tick, record.bytes, record.state_checksum
                        ),
                    );
                }
                Err(error) => {
                    shared.record_audit("admin", "save", false, &error, tick, "");
                    respond_json(
                        request,
                        500,
                        format!("{{\"error\":\"{}\"}}", json_escape(&error)),
                    );
                }
            }
        }
        ("POST", _) if path.starts_with("/api/worlds/1/saves/") && path.ends_with("/verify") => {
            if role != Role::Admin {
                respond_json(
                    request,
                    403,
                    "{\"error\":\"admin role required\"}".to_owned(),
                );
                return;
            }
            let Some(store) = shared.store.as_ref() else {
                respond_json(
                    request,
                    409,
                    "{\"error\":\"no data dir configured\"}".to_owned(),
                );
                return;
            };
            let save_id: Option<i64> = path
                .trim_end_matches("/verify")
                .rsplit('/')
                .next()
                .and_then(|value| value.parse().ok());
            let Some(save_id) = save_id else {
                respond_json(request, 400, "{\"error\":\"invalid save id\"}".to_owned());
                return;
            };
            let tick = current_tick(shared);
            // Isolated verification: rebuilds a throwaway world, never the
            // live one.
            let result = store.lock().expect("store").verify(save_id);
            match result {
                Ok(report) => {
                    shared.record_audit(
                        "admin",
                        "verify-save",
                        true,
                        &format!("save_id {save_id}"),
                        tick,
                        "",
                    );
                    respond_json(
                        request,
                        200,
                        format!(
                            concat!(
                                "{{\"save_id\":{},\"tick\":{},\"seed\":\"0x{:016x}\",",
                                "\"config_hash\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",",
                                "\"population\":{},\"build_version\":\"{}\",\"result\":\"ok\"}}"
                            ),
                            report.save_id,
                            report.tick,
                            report.seed,
                            report.config_hash,
                            report.state_checksum,
                            report.population,
                            json_escape(&report.build_version)
                        ),
                    );
                }
                Err(error) => {
                    shared.record_audit(
                        "admin",
                        "verify-save",
                        false,
                        &error.to_string(),
                        tick,
                        "",
                    );
                    respond_json(
                        request,
                        422,
                        format!("{{\"error\":\"{}\"}}", json_escape(&error.to_string())),
                    );
                }
            }
        }
        ("POST", "/api/worlds/1/control") => {
            if role != Role::Admin {
                shared.record_audit(
                    "observer",
                    &format!("control {url}"),
                    false,
                    "admin role required",
                    current_tick(shared),
                    &header_value(&request, "Idempotency-Key").unwrap_or_default(),
                );
                respond_json(
                    request,
                    403,
                    "{\"error\":\"admin role required\"}".to_owned(),
                );
                return;
            }
            handle_control(shared, request, &url, idempotency, last_control_ms);
        }
        ("GET", _) if path.starts_with("/api/worlds/1/organisms/") => {
            let id: Option<u64> = path.rsplit('/').next().and_then(|value| value.parse().ok());
            let Some(id) = id else {
                respond_json(
                    request,
                    400,
                    "{\"error\":\"invalid organism id\"}".to_owned(),
                );
                return;
            };
            let body = {
                let world = shared.world.lock().expect("world");
                world
                    .organism_detail(id)
                    .map(|detail| organism_json(&detail))
            };
            match body {
                Some(body) => respond_json(request, 200, body),
                None => respond_json(
                    request,
                    404,
                    "{\"error\":\"organism not found\"}".to_owned(),
                ),
            }
        }
        _ => respond_json(request, 404, "{\"error\":\"not found\"}".to_owned()),
    }
}

fn current_tick(shared: &Shared) -> u64 {
    shared.world.lock().expect("world").tick_number()
}

fn handle_control(
    shared: &Arc<Shared>,
    request: tiny_http::Request,
    url: &str,
    idempotency: &Mutex<HashMap<String, (u16, String)>>,
    last_control_ms: &AtomicU64,
) {
    let key = header_value(&request, "Idempotency-Key").unwrap_or_default();
    if !key.is_empty()
        && let Some((status, body)) = idempotency.lock().expect("idempotency").get(&key).cloned()
    {
        respond_json(request, status, body);
        return;
    }
    let now = now_unix_ms();
    let last = last_control_ms.load(Ordering::Relaxed);
    if now.saturating_sub(last) < CONTROL_RATE_LIMIT_MS {
        respond_json(
            request,
            429,
            "{\"error\":\"control rate limit\"}".to_owned(),
        );
        return;
    }

    let action = query_param(url, "action").unwrap_or_default();
    let tick = current_tick(shared);
    let (status, body, accepted, detail) = match action.as_str() {
        "pause" => {
            shared.control.lock().expect("control").paused = true;
            (200, control_state_json(shared), true, "paused".to_owned())
        }
        "resume" => {
            shared.control.lock().expect("control").paused = false;
            (200, control_state_json(shared), true, "resumed".to_owned())
        }
        "speed" => {
            let requested: Option<f64> =
                query_param(url, "multiplier").and_then(|value| value.parse().ok());
            match requested {
                Some(multiplier) if (0.0..=64.0).contains(&multiplier) => {
                    let speed_q16 = ((multiplier * 65_536.0) as u32).min(MAX_SPEED_Q16);
                    shared.control.lock().expect("control").speed_q16 = speed_q16;
                    (
                        200,
                        control_state_json(shared),
                        true,
                        format!("speed {multiplier}"),
                    )
                }
                _ => (
                    400,
                    "{\"error\":\"multiplier must be in [0, 64]\"}".to_owned(),
                    false,
                    "invalid speed".to_owned(),
                ),
            }
        }
        other => (
            400,
            "{\"error\":\"unknown action\"}".to_owned(),
            false,
            format!("unknown action {other}"),
        ),
    };
    if accepted {
        last_control_ms.store(now, Ordering::Relaxed);
    }
    shared.record_audit(
        "admin",
        &format!("control {action}"),
        accepted,
        &detail,
        tick,
        &key,
    );
    if !key.is_empty() {
        idempotency
            .lock()
            .expect("idempotency")
            .insert(key, (status, body.clone()));
    }
    respond_json(request, status, body);
}

// --- JSON / metrics rendering ----------------------------------------------

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn control_state_json(shared: &Shared) -> String {
    let control = shared.control.lock().expect("control");
    format!(
        "{{\"paused\":{},\"speed_multiplier\":{:.4}}}",
        control.paused,
        f64::from(control.speed_q16) / 65_536.0
    )
}

fn world_summary_json(shared: &Shared) -> String {
    let world = shared.world.lock().expect("world");
    let metrics = world.metrics();
    let control = shared.control.lock().expect("control");
    format!(
        concat!(
            "{{\"world_id\":1,\"world_epoch\":{},\"tick\":{},\"population\":{},",
            "\"births_total\":{},\"deaths_starvation_total\":{},\"deaths_old_age_total\":{},",
            "\"paired_births_total\":{},\"max_ancestry_depth\":{},\"extinct\":{},",
            "\"phase2\":{},\"paused\":{},\"speed_multiplier\":{:.4},",
            "\"config_hash\":\"0x{:016x}\",\"seed\":\"0x{:016x}\",",
            "\"cells_x\":{},\"cells_y\":{},\"cell_size_m\":{},\"dt_ms\":{},",
            "\"total_biomass_milli\":{},\"total_energy_milli\":{}}}"
        ),
        shared.world_epoch,
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
        metrics.total_energy_milli
    )
}

fn organism_json(detail: &sim_core::OrganismDetail) -> String {
    let mut body = format!(
        concat!(
            "{{\"id\":{},\"x_fp\":{},\"y_fp\":{},\"energy_milli\":{},",
            "\"age_ticks\":{},\"cooldown_ticks\":{}"
        ),
        detail.id,
        detail.x_fp,
        detail.y_fp,
        detail.energy_milli,
        detail.age_ticks,
        detail.cooldown_ticks
    );
    if let Some(phase2) = &detail.phase2 {
        let mut genes = String::new();
        for (index, gene) in phase2.trait_genes.iter().enumerate() {
            if index > 0 {
                genes.push(',');
            }
            genes.push_str(&format!("{gene:.6}"));
        }
        body.push_str(&format!(
            concat!(
                ",\"heading_bam\":{},\"speed_milli\":{},\"trait_genes\":[{}],",
                "\"parents\":[{},{}],\"ancestry_depth\":{},\"child_count\":{},",
                "\"birth_tick\":{},\"genome_hash\":\"0x{:016x}\",",
                "\"phenotype\":{{\"body_scale_milli\":{},\"max_speed_milli\":{},",
                "\"sensor_range_milli\":{},\"basal_mult_milli\":{},\"intake_mult_milli\":{},",
                "\"maturity_ticks\":{},\"invest_milli\":{},\"cooldown_ticks\":{}}}"
            ),
            phase2.heading_bam,
            phase2.speed_milli,
            genes,
            phase2.parents[0],
            phase2.parents[1],
            phase2.ancestry_depth,
            phase2.child_count,
            phase2.birth_tick,
            phase2.genome_hash,
            phase2.phenotype.body_scale_milli,
            phase2.phenotype.max_speed_milli,
            phase2.phenotype.sensor_range_milli,
            phase2.phenotype.basal_mult_milli,
            phase2.phenotype.intake_mult_milli,
            phase2.phenotype.maturity_ticks,
            phase2.phenotype.invest_milli,
            phase2.phenotype.cooldown_ticks
        ));
    }
    body.push('}');
    body
}

fn tick_stats_json(shared: &Shared, samples: &[f64]) -> String {
    let clients = shared.clients.lock().expect("clients");
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
            "{{\"samples\":0,\"ticks_total\":{},\"clients\":{client_stats}}}",
            shared.ticks_total.load(Ordering::Relaxed)
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
            "{{\"samples\":{},\"ticks_total\":{},\"tick_microseconds\":",
            "{{\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"min\":{:.3},\"max\":{:.3}}},",
            "\"clients\":{}}}"
        ),
        sorted.len(),
        shared.ticks_total.load(Ordering::Relaxed),
        percentile(0.50),
        percentile(0.95),
        percentile(0.99),
        sorted[0],
        sorted[sorted.len() - 1],
        client_stats
    )
}

fn metrics_text(shared: &Shared) -> String {
    let metrics = {
        let world = shared.world.lock().expect("world");
        world.metrics()
    };
    let mut text = String::new();
    let world_label = "server";
    text.push_str("# TYPE lifesim_organisms gauge\n");
    text.push_str(&format!(
        "lifesim_organisms{{world=\"{world_label}\",life_state=\"alive\"}} {}\n",
        metrics.population
    ));
    text.push_str("# TYPE lifesim_births_total counter\n");
    text.push_str(&format!(
        "lifesim_births_total{{world=\"{world_label}\"}} {}\n",
        metrics.births_total
    ));
    text.push_str("# TYPE lifesim_deaths_total counter\n");
    text.push_str(&format!(
        "lifesim_deaths_total{{world=\"{world_label}\",cause=\"starvation\"}} {}\n",
        metrics.deaths_starvation_total
    ));
    text.push_str(&format!(
        "lifesim_deaths_total{{world=\"{world_label}\",cause=\"old_age\"}} {}\n",
        metrics.deaths_old_age_total
    ));
    text.push_str("# TYPE lifesim_ticks_total counter\n");
    text.push_str(&format!(
        "lifesim_ticks_total{{world=\"{world_label}\"}} {}\n",
        shared.ticks_total.load(Ordering::Relaxed)
    ));
    // Stream metrics for connected observers.
    let clients = shared.clients.lock().expect("clients");
    let total_bytes: u64 = clients
        .iter()
        .map(|slot| slot.bytes_sent.load(Ordering::Relaxed))
        .sum();
    let total_dropped: u64 = clients
        .iter()
        .map(|slot| slot.dropped_updates.load(Ordering::Relaxed))
        .sum();
    text.push_str("# TYPE lifesim_stream_bytes_total counter\n");
    text.push_str(&format!(
        "lifesim_stream_bytes_total{{world=\"{world_label}\",client_class=\"observer\"}} {total_bytes}\n"
    ));
    text.push_str("# TYPE lifesim_observer_dropped_updates_total counter\n");
    text.push_str(&format!(
        "lifesim_observer_dropped_updates_total{{world=\"{world_label}\",reason=\"backpressure\"}} {total_dropped}\n"
    ));
    text.push_str("# TYPE lifesim_observers gauge\n");
    text.push_str(&format!(
        "lifesim_observers{{world=\"{world_label}\"}} {}\n",
        clients.len()
    ));
    drop(clients);
    // Save metrics (zero when persistence is disabled).
    text.push_str("# TYPE lifesim_saves_total counter\n");
    text.push_str(&format!(
        "lifesim_saves_total{{world=\"{world_label}\",result=\"ok\"}} {}\n",
        shared.saves_total.load(Ordering::Relaxed)
    ));
    text.push_str(&format!(
        "lifesim_saves_total{{world=\"{world_label}\",result=\"error\"}} {}\n",
        shared.save_failures_total.load(Ordering::Relaxed)
    ));
    text.push_str("# TYPE lifesim_save_duration_seconds gauge\n");
    text.push_str(&format!(
        "lifesim_save_duration_seconds{{world=\"{world_label}\",result=\"last\"}} {:.6}\n",
        shared.last_save_duration_us.load(Ordering::Relaxed) as f64 / 1_000_000.0
    ));
    text.push_str("# TYPE lifesim_save_bytes gauge\n");
    text.push_str(&format!(
        "lifesim_save_bytes{{world=\"{world_label}\"}} {}\n",
        shared.last_save_bytes.load(Ordering::Relaxed)
    ));
    // Phase 5 checkpoint instrumentation. `capture` is the only part of a
    // checkpoint the tick thread pays for in asynchronous mode, so it is
    // exported separately from the total write duration above.
    text.push_str("# TYPE lifesim_checkpoint_capture_seconds gauge\n");
    text.push_str(&format!(
        "lifesim_checkpoint_capture_seconds{{world=\"{world_label}\",mode=\"{}\"}} {:.6}\n",
        shared.checkpoint_mode.name(),
        shared.last_capture_us.load(Ordering::Relaxed) as f64 / 1_000_000.0
    ));
    text.push_str("# TYPE lifesim_checkpoints_skipped_total counter\n");
    text.push_str(&format!(
        "lifesim_checkpoints_skipped_total{{world=\"{world_label}\"}} {}\n",
        shared.checkpoints_skipped.load(Ordering::Relaxed)
    ));
    text
}
