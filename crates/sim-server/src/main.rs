//! Private observer/control server. Hosts many authoritative worlds in one
//! process (ADR-0039): world 1 comes from the command-line flags, the rest
//! are created over REST from a preset plus named settings.
//!
//! Threads: one tick loop per world (each owns its pacing; none blocks on
//! client I/O), one REST acceptor, one WebSocket acceptor, one thread per WS
//! session. Security posture: private LAN boundary; every request needs a
//! bearer token (observer or admin role). Tokens come from the environment
//! or are generated at startup and printed once; nothing is committed. TLS
//! and reverse-proxy choices remain deployment decisions per the security
//! model. This binary deploys nothing and touches no infrastructure.

mod json;
mod schema;
mod state;
mod stream;
mod worlds;

use crate::json::escape as json_escape;
use sim_core::{SimConfig, World, analyze};
use state::{
    CheckpointMode, Hub, MAX_SPEED_Q16, PRIMARY_WORLD_ID, Pacing, Role, WorldRuntime, now_unix_ms,
};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::net::TcpListener;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const DEFAULT_SEED: u64 = 0x5eed_cafe_f00d_beef;
const DEFAULT_REST_PORT: u16 = 8940;
const DEFAULT_WS_PORT: u16 = 8941;
/// Minimum interval between accepted control mutations, per world.
const CONTROL_RATE_LIMIT_MS: u64 = 100;
/// Default bound on hosted worlds (`--max-worlds`).
const DEFAULT_MAX_WORLDS: usize = 8;

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
    /// Address both listeners bind. Loopback by default (the documented
    /// posture: Caddy is the browser-facing boundary); a LAN address is a
    /// developer-instance choice made explicitly on the command line.
    bind: String,
    ws_port: u16,
    speed_q16: u32,
    paused: bool,
    data_dir: Option<std::path::PathBuf>,
    checkpoint_interval_secs: u64,
    checkpoint_keep: usize,
    checkpoint_mode: CheckpointMode,
    load_save: Option<std::path::PathBuf>,
    pacing: Pacing,
    max_worlds: usize,
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
        bind: "127.0.0.1".to_owned(),
        ws_port: DEFAULT_WS_PORT,
        speed_q16: 1 << 16,
        paused: false,
        data_dir: None,
        checkpoint_interval_secs: 0,
        checkpoint_keep: 4,
        checkpoint_mode: CheckpointMode::Async,
        load_save: None,
        pacing: Pacing::Realtime,
        max_worlds: DEFAULT_MAX_WORLDS,
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
                    "--bind" => {
                        options.bind = value.to_owned();
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
                    "--max-worlds" => {
                        let count: usize = value
                            .parse()
                            .map_err(|_| format!("invalid world count {value}"))?;
                        if count == 0 {
                            return Err("max worlds must be at least 1".to_owned());
                        }
                        options.max_worlds = count;
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

    let hub = Arc::new(Hub {
        observer_token: resolve_token("LIFESIM_OBSERVER_TOKEN"),
        admin_token: resolve_token("LIFESIM_ADMIN_TOKEN"),
        audit: Mutex::new(Vec::new()),
        next_audit_id: AtomicU64::new(1),
        next_client_id: AtomicU64::new(1),
        idempotency: Mutex::new(HashMap::new()),
        store,
        data_dir: options.data_dir.clone(),
        checkpoint_interval_secs: options.checkpoint_interval_secs,
        checkpoint_keep: options.checkpoint_keep,
        checkpoint_mode: options.checkpoint_mode,
        pacing: options.pacing,
        max_worlds: options.max_worlds,
        next_world_id: AtomicU64::new(PRIMARY_WORLD_ID + 1),
        worlds: Mutex::new(BTreeMap::new()),
    });

    let primary = Arc::new(worlds::new_runtime(
        PRIMARY_WORLD_ID,
        "primary".to_owned(),
        if options.load_save.is_some() {
            "loaded".to_owned()
        } else {
            "flags".to_owned()
        },
        0,
        world_epoch,
        world,
        options.paused,
        options.speed_q16,
    ));

    // Both listeners bind before the banner prints. The banner is the
    // readiness signal every integration test waits on, so printing it
    // while the REST port is still unbound makes it a lie that fails as an
    // intermittent connection-refused in whichever test connects fastest.
    let ws_listener = TcpListener::bind((options.bind.as_str(), options.ws_port))
        .map_err(|error| format!("ws bind: {error}"))?;
    let server = tiny_http::Server::http((options.bind.as_str(), options.rest_port))
        .map_err(|error| format!("rest bind: {error}"))?;
    println!(
        "lifesim-server: REST on {bind}:{} WS on {bind}:{} ({})",
        options.rest_port,
        options.ws_port,
        if options.bind == "127.0.0.1" || options.bind == "localhost" || options.bind == "::1" {
            "private, loopback only"
        } else {
            "private LAN boundary: bearer tokens in the clear, developer instance only"
        },
        bind = options.bind
    );

    {
        let hub = Arc::clone(&hub);
        std::thread::spawn(move || stream::websocket_listener(hub, ws_listener));
    }
    worlds::start(&hub, primary, options.run_ticks);

    for request in server.incoming_requests() {
        handle_request(&hub, request);
    }
    Ok(())
}

/// One-line summary printed when `--run-ticks` completes. Deliberately the
/// same shape as the CLI fixture line so the two are directly comparable,
/// which is what makes A5.1 a checksum equality rather than a description.
pub fn print_run_summary(hub: &Hub, runtime: &WorldRuntime, requested: u64) {
    let world = runtime.world.lock().expect("world");
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
        hub.pacing.name(),
        hub.checkpoint_mode.name(),
        requested,
        world.tick_number(),
        world.config().world_seed,
        world.config_hash(),
        metrics.population,
        metrics.births_total,
        metrics.extinct,
        runtime.checkpoints_skipped.load(Ordering::Relaxed),
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

fn respond_error(request: tiny_http::Request, status: u16, message: &str) {
    respond_json(
        request,
        status,
        format!("{{\"error\":\"{}\"}}", json_escape(message)),
    );
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

/// Read a bounded request body. Bodies are read before anything is
/// allocated from them, and a body larger than the cap is refused rather
/// than truncated into a shape that happens to parse.
fn read_body(request: &mut tiny_http::Request) -> Result<String, String> {
    if let Some(length) = request.body_length()
        && length > json::MAX_BODY_BYTES
    {
        return Err(format!(
            "body larger than {} bytes",
            json::MAX_BODY_BYTES
        ));
    }
    let mut body = String::new();
    request
        .as_reader()
        .take(json::MAX_BODY_BYTES as u64 + 1)
        .read_to_string(&mut body)
        .map_err(|_| "body is not UTF-8 text".to_owned())?;
    if body.len() > json::MAX_BODY_BYTES {
        return Err(format!(
            "body larger than {} bytes",
            json::MAX_BODY_BYTES
        ));
    }
    Ok(body)
}

fn handle_request(hub: &Arc<Hub>, mut request: tiny_http::Request) {
    let url = request.url().to_owned();
    let path = url.split('?').next().unwrap_or("").to_owned();
    let method = request.method().as_str().to_owned();

    if method == "OPTIONS" {
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
                    "GET, POST, DELETE, OPTIONS".as_bytes(),
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
    let role = bearer_token(&request).and_then(|token| hub.role_for(&token));
    let Some(role) = role else {
        respond_error(request, 401, "missing or invalid bearer token");
        return;
    };

    let segments: Vec<&str> = path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    match (method.as_str(), segments.as_slice()) {
        ("GET", ["api", "schema"]) => {
            respond_json(request, 200, schema::schema_json(hub.max_worlds));
        }
        ("POST", ["api", "schema", "preview"]) => {
            let body = match read_body(&mut request) {
                Ok(body) => body,
                Err(error) => return respond_error(request, 400, &error),
            };
            match schema::preview_json(&body, generated_seed(hub)) {
                Ok(body) => respond_json(request, 200, body),
                Err(bad) => respond_json(request, 400, bad.to_json()),
            }
        }
        ("GET", ["api", "worlds"]) => {
            let mut body = String::from("[");
            for (index, runtime) in hub.all_worlds().iter().enumerate() {
                if index > 0 {
                    body.push(',');
                }
                body.push_str(&worlds::summary_json(runtime));
            }
            body.push(']');
            respond_json(request, 200, body);
        }
        ("POST", ["api", "worlds"]) => {
            if role != Role::Admin {
                return deny_admin(hub, request, 0, "create-world");
            }
            create_world(hub, request);
        }
        ("GET", ["api", "worlds", id]) => match resolve(hub, id) {
            Some(runtime) => respond_json(request, 200, worlds::summary_json(&runtime)),
            None => respond_error(request, 404, "unknown world"),
        },
        ("DELETE", ["api", "worlds", id]) => {
            if role != Role::Admin {
                return deny_admin(hub, request, parse_id(id).unwrap_or(0), "delete-world");
            }
            delete_world(hub, request, id);
        }
        ("POST", ["api", "worlds", id, "control"]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            if role != Role::Admin {
                hub.record_audit(
                    runtime.id,
                    "observer",
                    &format!("control {url}"),
                    false,
                    "admin role required",
                    runtime.tick_number(),
                    &header_value(&request, "Idempotency-Key").unwrap_or_default(),
                );
                return respond_error(request, 403, "admin role required");
            }
            handle_control(hub, &runtime, request, &url);
        }
        ("POST", ["api", "worlds", id, "branch"]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            if role != Role::Admin {
                return deny_admin(hub, request, runtime.id, "branch");
            }
            branch_world(hub, request, &runtime, &url);
        }
        ("GET", ["api", "worlds", id, "saves"]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            list_saves(hub, request, runtime.id);
        }
        ("POST", ["api", "worlds", id, "saves"]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            if role != Role::Admin {
                return deny_admin(hub, request, runtime.id, "save");
            }
            create_save(hub, request, &runtime, &url);
        }
        ("POST", ["api", "worlds", id, "saves", save_id, "verify"]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            if role != Role::Admin {
                return deny_admin(hub, request, runtime.id, "verify-save");
            }
            verify_save(hub, request, &runtime, save_id);
        }
        ("GET", ["api", "worlds", id, "organisms", organism_id]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            let Ok(organism_id) = organism_id.parse::<u64>() else {
                return respond_error(request, 400, "invalid organism id");
            };
            let body = {
                let world = runtime.world.lock().expect("world");
                world
                    .organism_detail(organism_id)
                    .map(|detail| organism_json(&detail))
            };
            match body {
                Some(body) => respond_json(request, 200, body),
                None => respond_error(request, 404, "organism not found"),
            }
        }
        ("GET", ["api", "worlds", id, "analysis"]) => {
            let Some(runtime) = resolve(hub, id) else {
                return respond_error(request, 404, "unknown world");
            };
            let body = {
                let world = runtime.world.lock().expect("world");
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
                None => respond_error(request, 409, "analysis requires a phase2 world"),
            }
        }
        ("GET", ["metrics"]) => {
            respond(
                request,
                200,
                "text/plain; version=0.0.4",
                worlds::metrics_text(hub),
            );
        }
        ("GET", ["api", "benchmarks", "ticks"]) => {
            let id = query_param(&url, "world").unwrap_or_else(|| PRIMARY_WORLD_ID.to_string());
            let Some(runtime) = resolve(hub, &id) else {
                return respond_error(request, 404, "unknown world");
            };
            let samples: Vec<f64> = {
                let samples = runtime.tick_samples_us.lock().expect("samples");
                samples.iter().copied().collect()
            };
            if query_param(&url, "reset").as_deref() == Some("1") {
                runtime.tick_samples_us.lock().expect("samples").clear();
            }
            respond_json(request, 200, worlds::tick_stats_json(&runtime, &samples));
        }
        ("GET", ["api", "audit"]) => {
            if role != Role::Admin {
                return respond_error(request, 403, "admin role required");
            }
            let audit = hub.audit.lock().expect("audit");
            let mut body = String::from("[");
            for (index, record) in audit.iter().rev().take(100).enumerate() {
                if index > 0 {
                    body.push(',');
                }
                body.push_str(&format!(
                    concat!(
                        "{{\"id\":{},\"unix_ms\":{},\"world_id\":{},\"role\":\"{}\",",
                        "\"action\":\"{}\",\"accepted\":{},\"detail\":\"{}\",\"tick\":{},",
                        "\"idempotency_key\":\"{}\"}}"
                    ),
                    record.id,
                    record.unix_ms,
                    record.world_id,
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
        _ => respond_error(request, 404, "not found"),
    }
}

/// A world id from a path segment, and the runtime it names if it is still
/// hosted. An unknown id is a 404 everywhere, which is what keeps a control
/// aimed at a deleted world from landing on a live one.
fn parse_id(segment: &str) -> Option<u64> {
    segment.parse().ok()
}

fn resolve(hub: &Hub, segment: &str) -> Option<Arc<WorldRuntime>> {
    parse_id(segment).and_then(|id| hub.world(id))
}

fn deny_admin(hub: &Hub, request: tiny_http::Request, world_id: u64, action: &str) {
    let key = header_value(&request, "Idempotency-Key").unwrap_or_default();
    hub.record_audit(
        world_id,
        "observer",
        action,
        false,
        "admin role required",
        0,
        &key,
    );
    respond_error(request, 403, "admin role required");
}

/// A seed for a world whose request named none: the clock mixed with the
/// id the world is about to take, so two worlds created in the same
/// millisecond do not share a world.
fn generated_seed(hub: &Hub) -> u64 {
    let mut hasher = sim_core::Fnv1a64::new();
    hasher.update(b"lifesim-server-world-seed-v1");
    hasher.update_u64(now_unix_ms());
    hasher.update_u64(hub.next_world_id.load(Ordering::Relaxed));
    hasher.finish()
}

// --- World lifecycle routes -------------------------------------------------

fn create_world(hub: &Arc<Hub>, mut request: tiny_http::Request) {
    let key = header_value(&request, "Idempotency-Key").unwrap_or_default();
    if let Some((status, body)) = replay(hub, 0, &key) {
        return respond_json(request, status, body);
    }
    let body = match read_body(&mut request) {
        Ok(body) => body,
        Err(error) => {
            hub.record_audit(0, "admin", "create-world", false, &error, 0, &key);
            return respond_error(request, 400, &error);
        }
    };
    let requested = match schema::parse_create(&body, generated_seed(hub)) {
        Ok(request) => request,
        Err(bad) => {
            hub.record_audit(0, "admin", "create-world", false, &bad.message, 0, &key);
            return respond_json(request, 400, bad.to_json());
        }
    };
    if hub.worlds.lock().expect("worlds").len() >= hub.max_worlds {
        let detail = format!("at the --max-worlds bound of {}", hub.max_worlds);
        hub.record_audit(0, "admin", "create-world", false, &detail, 0, &key);
        return respond_error(request, 409, &detail);
    }
    // Settings are applied to the config, and only then is the world built:
    // the config hash the summary reports is the hash of the world that
    // exists, not of one that was edited afterwards.
    let config = match schema::build_config(&requested.preset, requested.seed, &requested.settings)
    {
        Ok(config) => config,
        Err(bad) => {
            hub.record_audit(0, "admin", "create-world", false, &bad.message, 0, &key);
            return respond_json(request, 400, bad.to_json());
        }
    };
    let world = match World::new(config) {
        Ok(world) => world,
        Err(error) => {
            let message = error.to_string();
            hub.record_audit(0, "admin", "create-world", false, &message, 0, &key);
            return respond_error(request, 400, &message);
        }
    };
    let id = hub.next_world_id.fetch_add(1, Ordering::Relaxed);
    let runtime = Arc::new(worlds::new_runtime(
        id,
        requested.name.clone(),
        requested.preset.clone(),
        0,
        1,
        world,
        requested.paused,
        requested.speed_q16,
    ));
    let summary = worlds::summary_json(&runtime);
    worlds::start(hub, runtime, None);
    hub.record_audit(
        id,
        "admin",
        "create-world",
        true,
        &format!(
            "world {id} preset {} seed 0x{:016x} settings {}",
            requested.preset,
            requested.seed,
            requested.settings.len()
        ),
        0,
        &key,
    );
    remember(hub, 0, &key, 201, &summary);
    respond_json(request, 201, summary);
}

fn delete_world(hub: &Arc<Hub>, request: tiny_http::Request, id: &str) {
    let key = header_value(&request, "Idempotency-Key").unwrap_or_default();
    let Some(runtime) = resolve(hub, id) else {
        return respond_error(request, 404, "unknown world");
    };
    if !runtime.stopped.load(Ordering::Relaxed) {
        hub.record_audit(
            runtime.id,
            "admin",
            "delete-world",
            false,
            "world is running",
            runtime.tick_number(),
            &key,
        );
        return respond_error(request, 409, "stop the world before deleting it");
    }
    let tick = runtime.tick_number();
    hub.worlds.lock().expect("worlds").remove(&runtime.id);
    hub.record_audit(
        runtime.id,
        "admin",
        "delete-world",
        true,
        "removed from the registry; saves kept",
        tick,
        &key,
    );
    respond_json(
        request,
        200,
        format!("{{\"world_id\":{},\"deleted\":true}}", runtime.id),
    );
}

/// `POST /api/worlds/{id}/branch?save_id=N&name=`: a new world loaded from
/// one of this world's saves.
///
/// A branch starts paused. The state it carries is the save's, and that is
/// the only thing anyone can check about it; a branch that started running
/// would have stepped past the state it was branched from before its
/// creator could look at it.
fn branch_world(
    hub: &Arc<Hub>,
    request: tiny_http::Request,
    parent: &Arc<WorldRuntime>,
    url: &str,
) {
    let key = header_value(&request, "Idempotency-Key").unwrap_or_default();
    if let Some((status, body)) = replay(hub, parent.id, &key) {
        return respond_json(request, status, body);
    }
    let Some(store) = hub.store.as_ref() else {
        return respond_error(request, 409, "no data dir configured");
    };
    let Some(data_dir) = hub.data_dir.as_ref() else {
        return respond_error(request, 409, "no data dir configured");
    };
    let save_id: Option<i64> = query_param(url, "save_id").and_then(|value| value.parse().ok());
    let Some(save_id) = save_id else {
        return respond_error(request, 400, "save_id is required");
    };
    let record = match store.lock().expect("store").list() {
        Ok(records) => records
            .into_iter()
            .find(|record| record.save_id == save_id && record.world_id == parent.id),
        Err(error) => return respond_error(request, 500, &error.to_string()),
    };
    let Some(record) = record else {
        return respond_error(request, 404, "unknown save for this world");
    };
    if hub.worlds.lock().expect("worlds").len() >= hub.max_worlds {
        return respond_error(
            request,
            409,
            &format!("at the --max-worlds bound of {}", hub.max_worlds),
        );
    }
    let world = match sim_persist::SnapshotStore::load_world(&data_dir.join(&record.path)) {
        Ok((_, world)) => world,
        Err(error) => {
            let message = error.to_string();
            hub.record_audit(parent.id, "admin", "branch", false, &message, 0, &key);
            return respond_error(request, 422, &message);
        }
    };
    let name = schema::sanitize_name(&query_param(url, "name").unwrap_or_default());
    let id = hub.next_world_id.fetch_add(1, Ordering::Relaxed);
    let runtime = Arc::new(worlds::new_runtime(
        id,
        name,
        parent.preset.clone(),
        parent.id,
        2,
        world,
        true,
        1 << 16,
    ));
    let summary = worlds::summary_json(&runtime);
    let tick = runtime.tick_number();
    worlds::start(hub, runtime, None);
    hub.record_audit(
        id,
        "admin",
        "branch",
        true,
        &format!("world {id} from save {save_id} of world {}", parent.id),
        tick,
        &key,
    );
    remember(hub, parent.id, &key, 201, &summary);
    respond_json(request, 201, summary);
}

// --- Saves ------------------------------------------------------------------

fn list_saves(hub: &Hub, request: tiny_http::Request, world_id: u64) {
    let Some(store) = hub.store.as_ref() else {
        return respond_error(request, 409, "no data dir configured");
    };
    let records = store.lock().expect("store").list();
    match records {
        Ok(records) => {
            let mut body = String::from("[");
            for (written, record) in records
                .iter()
                .filter(|record| record.world_id == world_id)
                .enumerate()
            {
                if written > 0 {
                    body.push(',');
                }
                body.push_str(&format!(
                    concat!(
                        "{{\"save_id\":{},\"world_id\":{},\"name\":\"{}\",\"kind\":\"{}\",",
                        "\"tick\":{},\"bytes\":{},\"compressed\":{},\"format_version\":{},",
                        "\"config_hash\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",",
                        "\"created_unix_ms\":{},\"verified\":{}}}"
                    ),
                    record.save_id,
                    record.world_id,
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
        Err(error) => respond_error(request, 500, &error.to_string()),
    }
}

fn create_save(hub: &Hub, request: tiny_http::Request, runtime: &WorldRuntime, url: &str) {
    let name = query_param(url, "name").unwrap_or_else(|| "manual".to_owned());
    let safe_name: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let tick = runtime.tick_number();
    match worlds::perform_save(hub, runtime, &safe_name, "manual") {
        Ok(record) => {
            hub.record_audit(
                runtime.id,
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
                    concat!(
                        "{{\"save_id\":{},\"world_id\":{},\"tick\":{},\"bytes\":{},",
                        "\"state_checksum\":\"0x{:016x}\"}}"
                    ),
                    record.save_id, record.world_id, record.tick, record.bytes,
                    record.state_checksum
                ),
            );
        }
        Err(error) => {
            hub.record_audit(runtime.id, "admin", "save", false, &error, tick, "");
            respond_error(request, 500, &error);
        }
    }
}

fn verify_save(hub: &Hub, request: tiny_http::Request, runtime: &WorldRuntime, save_id: &str) {
    let Some(store) = hub.store.as_ref() else {
        return respond_error(request, 409, "no data dir configured");
    };
    let Ok(save_id) = save_id.parse::<i64>() else {
        return respond_error(request, 400, "invalid save id");
    };
    // A save belongs to one world; verifying another world's save through
    // this world's path would make the id in the path decorative.
    let belongs = match store.lock().expect("store").list() {
        Ok(records) => records
            .iter()
            .any(|record| record.save_id == save_id && record.world_id == runtime.id),
        Err(error) => return respond_error(request, 500, &error.to_string()),
    };
    if !belongs {
        return respond_error(request, 404, "unknown save for this world");
    }
    let tick = runtime.tick_number();
    // Isolated verification: rebuilds a throwaway world, never the live one.
    let result = store.lock().expect("store").verify(save_id);
    match result {
        Ok(report) => {
            hub.record_audit(
                runtime.id,
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
                        "{{\"save_id\":{},\"world_id\":{},\"tick\":{},\"seed\":\"0x{:016x}\",",
                        "\"config_hash\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",",
                        "\"population\":{},\"build_version\":\"{}\",\"result\":\"ok\"}}"
                    ),
                    report.save_id,
                    report.world_id,
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
            hub.record_audit(
                runtime.id,
                "admin",
                "verify-save",
                false,
                &error.to_string(),
                tick,
                "",
            );
            respond_error(request, 422, &error.to_string());
        }
    }
}

// --- Controls ---------------------------------------------------------------

/// Recorded response for a keyed mutation, if this key has already been
/// answered for this world.
fn replay(hub: &Hub, world_id: u64, key: &str) -> Option<(u16, String)> {
    if key.is_empty() {
        return None;
    }
    hub.idempotency
        .lock()
        .expect("idempotency")
        .get(&format!("{world_id}:{key}"))
        .cloned()
}

fn remember(hub: &Hub, world_id: u64, key: &str, status: u16, body: &str) {
    if key.is_empty() {
        return;
    }
    hub.idempotency
        .lock()
        .expect("idempotency")
        .insert(format!("{world_id}:{key}"), (status, body.to_owned()));
}

fn handle_control(
    hub: &Arc<Hub>,
    runtime: &Arc<WorldRuntime>,
    request: tiny_http::Request,
    url: &str,
) {
    let key = header_value(&request, "Idempotency-Key").unwrap_or_default();
    if let Some((status, body)) = replay(hub, runtime.id, &key) {
        respond_json(request, status, body);
        return;
    }
    let now = now_unix_ms();
    let last = runtime.last_control_ms.load(Ordering::Relaxed);
    if now.saturating_sub(last) < CONTROL_RATE_LIMIT_MS {
        respond_error(request, 429, "control rate limit");
        return;
    }

    let action = query_param(url, "action").unwrap_or_default();
    let tick = runtime.tick_number();
    let stopped = runtime.stopped.load(Ordering::Relaxed);
    let (status, body, accepted, detail) = match action.as_str() {
        // A stopped world has no tick thread to obey a pause or a speed, so
        // accepting one would report a state the world will never be in.
        "pause" | "resume" | "speed" if stopped => (
            409,
            "{\"error\":\"world is stopped\"}".to_owned(),
            false,
            "world is stopped".to_owned(),
        ),
        "pause" => {
            runtime.control.lock().expect("control").paused = true;
            (
                200,
                worlds::control_state_json(runtime),
                true,
                "paused".to_owned(),
            )
        }
        "resume" => {
            runtime.control.lock().expect("control").paused = false;
            (
                200,
                worlds::control_state_json(runtime),
                true,
                "resumed".to_owned(),
            )
        }
        "speed" => {
            let requested: Option<f64> =
                query_param(url, "multiplier").and_then(|value| value.parse().ok());
            match requested {
                Some(multiplier) if (0.0..=64.0).contains(&multiplier) => {
                    let speed_q16 = ((multiplier * 65_536.0) as u32).min(MAX_SPEED_Q16);
                    runtime.control.lock().expect("control").speed_q16 = speed_q16;
                    (
                        200,
                        worlds::control_state_json(runtime),
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
        "stop" => {
            runtime.stopped.store(true, Ordering::Relaxed);
            (
                200,
                worlds::control_state_json(runtime),
                true,
                "stopped".to_owned(),
            )
        }
        other => (
            400,
            "{\"error\":\"unknown action\"}".to_owned(),
            false,
            format!("unknown action {other}"),
        ),
    };
    if accepted {
        runtime.last_control_ms.store(now, Ordering::Relaxed);
    }
    hub.record_audit(
        runtime.id,
        "admin",
        &format!("control {action}"),
        accepted,
        &detail,
        tick,
        &key,
    );
    remember(hub, runtime.id, &key, status, &body);
    respond_json(request, status, body);
}

// --- JSON rendering ---------------------------------------------------------

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
