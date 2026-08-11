//! Server integration tests: real process, real sockets. Each test spawns
//! the release-or-debug binary on ephemeral ports with fixed tokens.

use sim_protocol::{
    Frame, FrameMeta, LAYER_METRICS, LAYER_ORGANISMS, LAYER_TERRAIN, PROTOCOL_MAJOR,
    PROTOCOL_MINOR, Viewport, decode, encode,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

const OBSERVER_TOKEN: &str = "test-observer-token";
const ADMIN_TOKEN: &str = "test-admin-token";

/// Backstop for a server that is alive but silent, not an assertion about how
/// fast a healthy server boots. A server that *fails* to start is detected by
/// its stdout closing and reported with its exit status immediately, so no
/// real failure waits this out; the only thing this bounds is a hang, and it
/// has to survive a machine busy with several concurrent release builds. The
/// previous ten seconds was neither: it timed out honest slow starts and could
/// not fire at all on the hang it was nominally there for.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);

/// How many pre-banner stdout lines a failure message quotes back.
const STARTUP_LINES_QUOTED: usize = 20;

struct ServerGuard {
    child: Child,
    rest_port: u16,
    ws_port: u16,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_server(extra_args: &[&str]) -> ServerGuard {
    match try_spawn_server(extra_args) {
        Ok(guard) => guard,
        Err(failure) => panic!("{failure}"),
    }
}

/// Spawn the server and wait for its readiness banner, returning the reason on
/// failure instead of panicking so the failure path itself can be tested.
fn try_spawn_server(extra_args: &[&str]) -> Result<ServerGuard, String> {
    // Ephemeral, collision-resistant ports per test process/case.
    let base = 20_000
        + (std::process::id() % 20_000) as u16
        + PORT_OFFSET.fetch_add(2, std::sync::atomic::Ordering::Relaxed);
    let rest_port = base;
    let ws_port = base + 1;
    let mut command = Command::new(env!("CARGO_BIN_EXE_lifesim-server"));
    command
        .env("LIFESIM_OBSERVER_TOKEN", OBSERVER_TOKEN)
        .env("LIFESIM_ADMIN_TOKEN", ADMIN_TOKEN)
        .arg("--rest-port")
        .arg(rest_port.to_string())
        .arg("--ws-port")
        .arg(ws_port.to_string())
        .arg("--organisms")
        .arg("150")
        .arg("--speed")
        .arg("16")
        .args(extra_args)
        .stdout(Stdio::piped())
        // Piped rather than discarded: the reason a server refused to start
        // ("rest bind: Address already in use" for a port another test process
        // already holds) is written here and then lost, which is what left the
        // old failure with nothing to say but "did not start". A thread drains
        // it continuously, so the pipe can neither fill nor close early.
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn server");
    let complaints = drain_stderr(child.stderr.take().expect("stderr"));

    // Wait for the startup banner. Two unrelated failures used to share one
    // clock and one message: a server that died, and a server that is merely
    // slow. Only the second is worth waiting for, so they are separated here.
    // Stdout closing means the process is gone and is reported at once with
    // its exit status, which is why the deadline can afford to be generous.
    //
    // The read runs on its own thread because a blocking `read_line` cannot be
    // interrupted, and the old loop therefore consulted the clock only when a
    // line happened to arrive. That made the deadline simultaneously
    // unenforceable and unfair: a silent server blocked forever, while a
    // banner that arrived late was read and then discarded as "did not start".
    // The thread also keeps draining stdout for the server's lifetime, so a
    // later write can never meet a closed pipe.
    let stdout = child.stdout.take().expect("stdout");
    let (lines_tx, lines_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                // EOF, or an error this thread cannot report; either way there
                // are no more lines, and dropping the sender is what tells the
                // waiter the process has stopped talking.
                Ok(0) | Err(_) => return,
                // A send error only means nobody is listening any more. Keep
                // reading rather than leaving the pipe to fill.
                Ok(_) => {
                    let _ = lines_tx.send(line);
                }
            }
        }
    });

    let started = Instant::now();
    let mut quoted: Vec<String> = Vec::new();
    loop {
        let remaining = STARTUP_TIMEOUT.saturating_sub(started.elapsed());
        match lines_rx.recv_timeout(remaining) {
            Ok(line) if line.contains("REST on") => break,
            Ok(line) => {
                if quoted.len() < STARTUP_LINES_QUOTED {
                    quoted.push(line);
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(startup_failure(
                    &mut child,
                    "stdout closed before the banner",
                    started.elapsed(),
                    &quoted,
                    &complaints,
                ));
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err(startup_failure(
                    &mut child,
                    "no banner within the timeout",
                    started.elapsed(),
                    &quoted,
                    &complaints,
                ));
            }
        }
    }
    Ok(ServerGuard {
        child,
        rest_port,
        ws_port,
    })
}

/// Collect the server's stderr on a thread for the life of the process. The
/// thread keeps reading after the cap is reached, because the point is to leave
/// the pipe drained; only storage is bounded.
fn drain_stderr(stderr: std::process::ChildStderr) -> Arc<Mutex<Vec<String>>> {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&collected);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    let mut sink = sink.lock().expect("stderr sink");
                    if sink.len() < STARTUP_LINES_QUOTED {
                        sink.push(line);
                    }
                }
            }
        }
    });
    collected
}

/// Explain a server that never announced itself, naming which of the two
/// failures happened. A server that died on a bad flag or an occupied port
/// reports its exit status and its own complaint here, so it fails fast with a
/// cause instead of being indistinguishable from a slow boot.
fn startup_failure(
    child: &mut Child,
    reason: &str,
    waited: Duration,
    quoted: &[String],
    complaints: &Mutex<Vec<String>>,
) -> String {
    // A process on its way out needs a moment before its status can be read; a
    // wedged one must not hold the suite up, so the wait is bounded and a
    // survivor is killed rather than left holding its ports for the rest of
    // the run.
    let state = match reap(child, Duration::from_secs(2)) {
        Some(status) => format!("process exited: {status}"),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            "process still running, killed".to_owned()
        }
    };
    // The dying process wrote its complaint just before exiting; give the
    // stderr thread a moment to have taken it off the pipe before quoting.
    std::thread::sleep(Duration::from_millis(50));
    let said = match complaints.lock().expect("stderr sink").as_slice() {
        [] => "nothing on stderr".to_owned(),
        lines => format!("stderr: {lines:?}"),
    };
    let stdout = if quoted.is_empty() {
        "nothing on stdout".to_owned()
    } else {
        format!("stdout so far: {quoted:?}")
    };
    format!("server did not start: {reason} after {waited:.1?}; {state}; {said}; {stdout}")
}

/// The child's exit status if it is reaped within `grace`, otherwise `None`.
fn reap(child: &mut Child, grace: Duration) -> Option<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if started.elapsed() >= grace {
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

static PORT_OFFSET: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

fn http(
    guard: &ServerGuard,
    method: &str,
    path: &str,
    token: Option<&str>,
    idempotency: Option<&str>,
) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", guard.rest_port)).expect("connect rest");
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    if let Some(token) = token {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some(key) = idempotency {
        request.push_str(&format!("Idempotency-Key: {key}\r\n"));
    }
    request.push_str("Content-Length: 0\r\n\r\n");
    stream.write_all(request.as_bytes()).expect("write");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("read");
    let status: u16 = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("status");
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

fn ws_connect(guard: &ServerGuard, token: &str) -> WebSocket<TcpStream> {
    let stream = TcpStream::connect(("127.0.0.1", guard.ws_port)).expect("connect ws");
    let (mut socket, _) = tungstenite::client(format!("ws://127.0.0.1:{}/", guard.ws_port), stream)
        .expect("ws handshake");
    socket
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("timeout");
    send(
        &mut socket,
        &Frame::Hello {
            major: PROTOCOL_MAJOR,
            minor: PROTOCOL_MINOR,
            capabilities: 0,
            token: token.as_bytes().to_vec(),
        },
    );
    socket
}

fn send(socket: &mut WebSocket<TcpStream>, frame: &Frame) {
    let bytes = encode(
        frame,
        FrameMeta {
            world_epoch: 0,
            sequence: 0,
            checksummed: false,
        },
    );
    socket.send(Message::Binary(bytes)).expect("send");
}

fn read_frame(socket: &mut WebSocket<TcpStream>, deadline: Duration) -> Option<(FrameMeta, Frame)> {
    let started = Instant::now();
    while started.elapsed() < deadline {
        match socket.read() {
            Ok(Message::Binary(bytes)) => {
                return Some(decode(&bytes).expect("well-formed server frame"));
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return None,
        }
    }
    None
}

fn full_viewport() -> Viewport {
    Viewport {
        x0_fp: 0,
        y0_fp: 0,
        x1_fp: 1_048_575,
        y1_fp: 1_048_575,
        lod: 0,
    }
}

/// The startup wait has to tell a dead server from a slow one. A server that
/// cannot start must be reported by its exit status well inside
/// `STARTUP_TIMEOUT`; if it were not, every genuine startup failure would cost
/// the suite the full timeout, and the timeout could not be raised to a length
/// that survives a loaded machine.
#[test]
fn a_server_that_cannot_start_is_reported_by_its_exit_status_not_by_the_timeout() {
    let started = Instant::now();
    let failure = match try_spawn_server(&["--not-a-real-flag"]) {
        // Dropping the guard kills the server the flag should have refused.
        Ok(_) => panic!("a rejected flag must not produce a running server"),
        Err(failure) => failure,
    };
    let elapsed = started.elapsed();
    assert!(
        failure.contains("stdout closed before the banner") && failure.contains("process exited:"),
        "expected a died-at-startup diagnosis, got: {failure}"
    );
    assert!(
        elapsed < STARTUP_TIMEOUT / 4,
        "a crashed server took {elapsed:.1?}, which is timeout-shaped rather than fast"
    );
}

/// The failure this harness is most likely to actually meet: the REST port is
/// already held by another test process, whose port range overlaps this one and
/// is keyed on the same pid. The server refuses to start, and the reason has to
/// survive to the panic message -- an occupied port and a slow boot used to
/// produce the same four words, which is why the flake was diagnosed as a
/// timeout in the first place.
#[test]
fn an_occupied_port_is_reported_as_a_bind_failure_not_as_a_slow_start() {
    let squatter = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("squat a port");
    let taken = squatter.local_addr().expect("addr").port();
    let started = Instant::now();
    // A later --rest-port wins, so this forces the collision deterministically.
    let failure = match try_spawn_server(&["--rest-port", &taken.to_string()]) {
        Ok(_) => panic!("port {taken} is held; the server must not have bound it"),
        Err(failure) => failure,
    };
    assert!(
        failure.contains("process exited:") && failure.contains("rest bind:"),
        "expected the bind failure to name itself, got: {failure}"
    );
    assert!(
        started.elapsed() < STARTUP_TIMEOUT / 4,
        "a refused bind took {:.1?}, which is timeout-shaped rather than fast",
        started.elapsed()
    );
}

#[test]
fn websocket_rejects_bad_token() {
    let guard = spawn_server(&[]);
    let mut socket = ws_connect(&guard, "wrong-token");
    let frame = read_frame(&mut socket, Duration::from_secs(5));
    match frame {
        Some((_, Frame::Error { code, .. })) => assert_eq!(code, 401),
        other => panic!("expected 401 error frame, got {other:?}"),
    }
}

#[test]
fn subscribe_yields_welcome_subscribed_keyframe_then_deltas() {
    let guard = spawn_server(&[]);
    let mut socket = ws_connect(&guard, OBSERVER_TOKEN);

    let (_, welcome) = read_frame(&mut socket, Duration::from_secs(5)).expect("welcome");
    let Frame::Welcome {
        major,
        cells_x,
        phase2,
        ..
    } = welcome
    else {
        panic!("expected welcome, got {welcome:?}");
    };
    assert_eq!(major, PROTOCOL_MAJOR);
    assert_eq!(cells_x, 256);
    assert!(phase2);

    send(
        &mut socket,
        &Frame::Subscribe {
            viewport: full_viewport(),
            layers: LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS,
            max_rate_hz: 30,
        },
    );
    let (_, subscribed) = read_frame(&mut socket, Duration::from_secs(5)).expect("subscribed");
    let Frame::Subscribed {
        layers,
        max_rate_hz,
        ..
    } = subscribed
    else {
        panic!("expected subscribed, got {subscribed:?}");
    };
    assert_eq!(layers, LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS);
    assert!(max_rate_hz <= 30);

    // First state frame is a self-contained keyframe with terrain tiles.
    let mut got_keyframe = false;
    let mut got_delta = false;
    let mut got_metrics = false;
    let mut sequences = Vec::new();
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) && !(got_keyframe && got_delta && got_metrics)
    {
        match read_frame(&mut socket, Duration::from_secs(5)) {
            Some((
                meta,
                Frame::Keyframe {
                    tiles, entities, ..
                },
            )) => {
                if !got_keyframe {
                    assert!(tiles.is_some(), "keyframe must carry terrain tiles");
                    assert!(!entities.is_empty(), "world starts populated");
                }
                got_keyframe = true;
                sequences.push(meta.sequence);
            }
            Some((meta, Frame::Delta { .. })) => {
                assert!(got_keyframe, "delta before first keyframe");
                got_delta = true;
                sequences.push(meta.sequence);
                send(
                    &mut socket,
                    &Frame::Ack {
                        applied_sequence: meta.sequence,
                    },
                );
            }
            Some((_, Frame::MetricsSample { population, .. })) => {
                assert!(population > 0);
                got_metrics = true;
            }
            Some(_) => {}
            None => break,
        }
    }
    assert!(got_keyframe && got_delta && got_metrics);
    // Sequences are strictly increasing.
    for window in sequences.windows(2) {
        assert!(window[1] > window[0]);
    }
}

#[test]
fn viewport_is_clamped_and_unknown_layers_are_masked() {
    let guard = spawn_server(&[]);
    let mut socket = ws_connect(&guard, OBSERVER_TOKEN);
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("welcome");
    send(
        &mut socket,
        &Frame::Subscribe {
            viewport: Viewport {
                x0_fp: -500,
                y0_fp: -500,
                x1_fp: i32::MAX,
                y1_fp: i32::MAX,
                lod: 250,
            },
            layers: 0xffff_ffff,
            max_rate_hz: 255,
        },
    );
    let (_, subscribed) = read_frame(&mut socket, Duration::from_secs(5)).expect("subscribed");
    let Frame::Subscribed {
        viewport,
        layers,
        max_rate_hz,
    } = subscribed
    else {
        panic!("expected subscribed");
    };
    assert!(viewport.x0_fp >= 0 && viewport.y0_fp >= 0);
    assert!(viewport.x1_fp < 1_048_576 && viewport.y1_fp < 1_048_576);
    assert!(viewport.lod <= 3);
    assert_eq!(layers, LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS);
    assert!(max_rate_hz <= 30);
}

#[test]
fn slow_client_gets_resynced_with_keyframe_not_unbounded_backlog() {
    let guard = spawn_server(&[]);
    let mut socket = ws_connect(&guard, OBSERVER_TOKEN);
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("welcome");
    send(
        &mut socket,
        &Frame::Subscribe {
            viewport: full_viewport(),
            layers: LAYER_ORGANISMS,
            max_rate_hz: 30,
        },
    );
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("subscribed");
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("first keyframe");

    // Simulate a stalled client: stop reading for a while; never ack.
    std::thread::sleep(Duration::from_secs(4));

    // The server must have collapsed the backlog; after resuming reads, a
    // fresh keyframe arrives (resync) rather than an endless delta chain.
    let mut saw_keyframe_after_stall = false;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        match read_frame(&mut socket, Duration::from_secs(5)) {
            Some((_, Frame::Keyframe { .. })) => {
                saw_keyframe_after_stall = true;
                break;
            }
            Some(_) => {}
            None => break,
        }
    }
    assert!(saw_keyframe_after_stall, "expected keyframe resync");

    // Server-side counters recorded the dropped updates.
    let (status, body) = http(
        &guard,
        "GET",
        "/api/benchmarks/ticks",
        Some(OBSERVER_TOKEN),
        None,
    );
    assert_eq!(status, 200);
    assert!(body.contains("dropped_updates"));
}

#[test]
fn rest_auth_roles_and_audit_are_enforced() {
    let guard = spawn_server(&[]);

    let (status, _) = http(&guard, "GET", "/api/worlds/1", None, None);
    assert_eq!(status, 401);

    let (status, body) = http(&guard, "GET", "/api/worlds/1", Some(OBSERVER_TOKEN), None);
    assert_eq!(status, 200);
    assert!(body.contains("\"world_id\":1"));

    // Observer cannot mutate; the denial is audited.
    let (status, _) = http(
        &guard,
        "POST",
        "/api/worlds/1/control?action=pause",
        Some(OBSERVER_TOKEN),
        Some("obs-key"),
    );
    assert_eq!(status, 403);

    // Admin pause works and is audited; idempotent replay returns the
    // recorded response without reapplying.
    let (status, first_body) = http(
        &guard,
        "POST",
        "/api/worlds/1/control?action=pause",
        Some(ADMIN_TOKEN),
        Some("pause-key"),
    );
    assert_eq!(status, 200);
    assert!(first_body.contains("\"paused\":true"));
    let (status, replay_body) = http(
        &guard,
        "POST",
        "/api/worlds/1/control?action=pause",
        Some(ADMIN_TOKEN),
        Some("pause-key"),
    );
    assert_eq!(status, 200);
    assert_eq!(first_body, replay_body);

    // While paused the tick counter must stop advancing.
    let (_, before) = http(&guard, "GET", "/api/worlds/1", Some(OBSERVER_TOKEN), None);
    std::thread::sleep(Duration::from_millis(600));
    let (_, after) = http(&guard, "GET", "/api/worlds/1", Some(OBSERVER_TOKEN), None);
    let tick_of = |body: &str| -> u64 {
        body.split("\"tick\":")
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .and_then(|value| value.parse().ok())
            .expect("tick field")
    };
    assert_eq!(tick_of(&before), tick_of(&after));

    // Audit shows both the denial and the acceptance; observers may not
    // read it.
    let (status, _) = http(&guard, "GET", "/api/audit", Some(OBSERVER_TOKEN), None);
    assert_eq!(status, 403);
    let (status, audit) = http(&guard, "GET", "/api/audit", Some(ADMIN_TOKEN), None);
    assert_eq!(status, 200);
    assert!(audit.contains("\"accepted\":false"));
    assert!(audit.contains("\"accepted\":true"));
    assert!(audit.contains("pause-key"));

    // Invalid control input is rejected.
    std::thread::sleep(Duration::from_millis(150));
    let (status, _) = http(
        &guard,
        "POST",
        "/api/worlds/1/control?action=speed&multiplier=1000",
        Some(ADMIN_TOKEN),
        Some("bad-speed"),
    );
    assert_eq!(status, 400);
}

#[test]
fn organism_detail_is_bounded_and_never_includes_controller_weights() {
    let guard = spawn_server(&[]);
    // Find a living organism via a keyframe.
    let mut socket = ws_connect(&guard, OBSERVER_TOKEN);
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("welcome");
    send(
        &mut socket,
        &Frame::Subscribe {
            viewport: full_viewport(),
            layers: LAYER_ORGANISMS,
            max_rate_hz: 10,
        },
    );
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("subscribed");
    let id = loop {
        match read_frame(&mut socket, Duration::from_secs(5)).expect("frame") {
            (_, Frame::Keyframe { entities, .. }) if !entities.is_empty() => {
                break entities[0].id;
            }
            _ => {}
        }
    };
    let (status, body) = http(
        &guard,
        "GET",
        &format!("/api/worlds/1/organisms/{id}"),
        Some(OBSERVER_TOKEN),
        None,
    );
    assert_eq!(status, 200);
    assert!(body.contains("\"trait_genes\":["));
    assert!(body.contains("\"parents\":["));
    assert!(body.contains("\"phenotype\":"));
    // 14 trait genes only; the 696-value controller matrix never appears.
    let genes = body
        .split("\"trait_genes\":[")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("genes array");
    assert_eq!(genes.split(',').count(), 14);
    assert!(body.len() < 4_096, "detail response must stay bounded");

    let (status, _) = http(
        &guard,
        "GET",
        "/api/worlds/1/organisms/999999999",
        Some(OBSERVER_TOKEN),
        None,
    );
    assert_eq!(status, 404);
}

#[test]
fn malformed_websocket_frames_get_an_error_not_a_crash() {
    let guard = spawn_server(&[]);
    let mut socket = ws_connect(&guard, OBSERVER_TOKEN);
    let _ = read_frame(&mut socket, Duration::from_secs(5)).expect("welcome");
    socket
        .send(Message::Binary(vec![0xde, 0xad, 0xbe, 0xef]))
        .expect("send garbage");
    let mut got_error = false;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        if let Some((_, Frame::Error { code, .. })) =
            read_frame(&mut socket, Duration::from_secs(2))
        {
            assert_eq!(code, 400);
            got_error = true;
            break;
        }
    }
    assert!(got_error);
    // The connection and the server both remain usable.
    send(
        &mut socket,
        &Frame::Subscribe {
            viewport: full_viewport(),
            layers: LAYER_ORGANISMS,
            max_rate_hz: 5,
        },
    );
    assert!(matches!(
        read_frame(&mut socket, Duration::from_secs(5)),
        Some((_, Frame::Subscribed { .. }))
    ));
}

#[test]
fn saves_checkpoints_and_isolated_verify_work_end_to_end() {
    let data_dir = std::env::temp_dir().join(format!("lifesim-server-p4-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    let data_dir_str = data_dir.to_string_lossy().into_owned();
    let guard = spawn_server(&[
        "--data-dir",
        &data_dir_str,
        "--checkpoint-interval-secs",
        "2",
        "--checkpoint-keep",
        "2",
    ]);

    // Observer cannot create saves.
    let (status, _) = http(
        &guard,
        "POST",
        "/api/worlds/1/saves?name=nope",
        Some(OBSERVER_TOKEN),
        None,
    );
    assert_eq!(status, 403);

    // Admin manual save.
    let (status, body) = http(
        &guard,
        "POST",
        "/api/worlds/1/saves?name=milestone-1",
        Some(ADMIN_TOKEN),
        Some("save-key"),
    );
    assert_eq!(status, 200, "save failed: {body}");
    let save_id: i64 = body
        .split("\"save_id\":")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|value| value.parse().ok())
        .expect("save id");

    // Isolated verify passes and is audited.
    let (status, verify_body) = http(
        &guard,
        "POST",
        &format!("/api/worlds/1/saves/{save_id}/verify"),
        Some(ADMIN_TOKEN),
        None,
    );
    assert_eq!(status, 200, "verify failed: {verify_body}");
    assert!(verify_body.contains("\"result\":\"ok\""));

    // Automatic checkpoints appear within the interval and are pruned.
    std::thread::sleep(Duration::from_secs(6));
    let (status, listing) = http(
        &guard,
        "GET",
        "/api/worlds/1/saves",
        Some(OBSERVER_TOKEN),
        None,
    );
    assert_eq!(status, 200);
    let checkpoints = listing.matches("\"kind\":\"checkpoint\"").count();
    assert!(
        (1..=2).contains(&checkpoints),
        "expected 1..=2 retained checkpoints, listing: {listing}"
    );
    assert!(listing.contains("milestone-1"));

    // Audit shows the service checkpoints and the admin save.
    let (_, audit) = http(&guard, "GET", "/api/audit", Some(ADMIN_TOKEN), None);
    assert!(audit.contains("\"action\":\"checkpoint\""));
    assert!(audit.contains("\"action\":\"save\""));
    drop(guard);

    // Branch a new server from the manual save; the world resumes at the
    // recorded tick with a new epoch.
    let saved_path = std::fs::read_dir(&data_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "alif")
        })
        .expect("saved snapshot file");
    let branch = spawn_server(&["--load-save", &saved_path.to_string_lossy()]);
    let (status, world_body) = http(&branch, "GET", "/api/worlds/1", Some(OBSERVER_TOKEN), None);
    assert_eq!(status, 200);
    assert!(world_body.contains("\"world_epoch\":2"));
    let _ = std::fs::remove_dir_all(&data_dir);
}
