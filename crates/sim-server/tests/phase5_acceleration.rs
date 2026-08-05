//! Phase 5 acceptance criterion A5.1: acceleration is result-neutral.
//!
//! > For a fixed seed and config, the final state checksum after T ticks is
//! > identical when run at 1x pacing, at maximum headless speed, with an
//! > observer attached, and with an observer attached and then detached
//! > mid-run. Four executions, one checksum.
//!
//! The claim being tested is structural: the kernel reads no clock, so how
//! fast the host chooses to call `step`, and whether anyone is watching,
//! cannot reach a result. That is easy to assert and easy to break — a
//! frame-rate-dependent broadcast that touched world state, or a pacing
//! path that skipped a tick under load, would both show up here as a
//! checksum difference and nowhere else.
//!
//! Each execution is a real process on real sockets, exactly like the other
//! server integration tests.

use sim_protocol::{
    Frame, FrameMeta, LAYER_METRICS, LAYER_ORGANISMS, LAYER_TERRAIN, PROTOCOL_MAJOR,
    PROTOCOL_MINOR, Viewport, encode,
};
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

const OBSERVER_TOKEN: &str = "phase5-observer-token";
const ADMIN_TOKEN: &str = "phase5-admin-token";
/// Long enough that an observer can attach and detach inside the run at the
/// paced speeds below, short enough to keep the suite quick.
const TICKS: u64 = 100;
const ORGANISMS: &str = "150";

static PORT_OFFSET: AtomicU16 = AtomicU16::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Observer {
    None,
    Attached,
    AttachedThenDetached,
}

#[derive(Debug)]
struct Outcome {
    state_checksum: String,
    terrain_checksum: String,
    config_hash: String,
    final_tick: u64,
    pacing: String,
    wall: Duration,
}

fn json_field(line: &str, key: &str) -> String {
    // The summary line is a flat, machine-generated object; a targeted
    // extraction keeps this test free of a JSON dependency, matching the
    // other CLI-output assertions in this repository.
    let needle = format!("\"{key}\":");
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in {line}"))
        + needle.len();
    let rest = &line[start..];
    let rest = rest.strip_prefix('"').unwrap_or(rest);
    let end = rest
        .find(['"', ',', '}'])
        .unwrap_or_else(|| panic!("unterminated {key} in {line}"));
    rest[..end].to_owned()
}

fn run_server(pacing: &str, speed: &str, observer: Observer) -> Outcome {
    let base =
        24_000 + (std::process::id() % 15_000) as u16 + PORT_OFFSET.fetch_add(2, Ordering::Relaxed);
    let rest_port = base;
    let ws_port = base + 1;
    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_lifesim-server"))
        .env("LIFESIM_OBSERVER_TOKEN", OBSERVER_TOKEN)
        .env("LIFESIM_ADMIN_TOKEN", ADMIN_TOKEN)
        .arg("--rest-port")
        .arg(rest_port.to_string())
        .arg("--ws-port")
        .arg(ws_port.to_string())
        .arg("--organisms")
        .arg(ORGANISMS)
        .arg("--pacing")
        .arg(pacing)
        .arg("--speed")
        .arg(speed)
        .arg("--run-ticks")
        .arg(TICKS.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server");

    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let boot = Instant::now();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || boot.elapsed() > Duration::from_secs(15)
        {
            let _ = child.kill();
            panic!("server did not start");
        }
        if line.contains("REST on") {
            break;
        }
        // A run this short can finish before the banner is consumed.
        if line.contains("server_run_schema_version") {
            let _ = child.wait();
            return parse_outcome(&line, started.elapsed());
        }
    }

    let watcher = match observer {
        Observer::None => None,
        Observer::Attached | Observer::AttachedThenDetached => {
            let mut socket = ws_connect(ws_port);
            subscribe(&mut socket);
            let detach_after = if observer == Observer::AttachedThenDetached {
                Some(Duration::from_millis(400))
            } else {
                None
            };
            Some(std::thread::spawn(move || {
                let opened = Instant::now();
                let mut frames = 0_u64;
                loop {
                    if let Some(after) = detach_after
                        && opened.elapsed() >= after
                    {
                        // Drop the socket mid-run: the tick thread must
                        // notice the dead subscriber without altering the
                        // world it is stepping.
                        let _ = socket.close(None);
                        drop(socket);
                        return frames;
                    }
                    match socket.read() {
                        Ok(Message::Binary(_)) => frames += 1,
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(error))
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                || error.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(_) => return frames,
                    }
                    if opened.elapsed() > Duration::from_secs(60) {
                        return frames;
                    }
                }
            }))
        }
    };

    let summary = loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            let _ = child.kill();
            panic!("server exited without a run summary");
        }
        if line.contains("server_run_schema_version") {
            break line.clone();
        }
        if started.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            panic!("run did not finish");
        }
    };
    let _ = child.wait();
    if let Some(watcher) = watcher {
        let frames = watcher.join().unwrap_or(0);
        if observer != Observer::None {
            assert!(
                frames > 0,
                "the observer received no frames, so this execution did not test \
                 an attached observer at all"
            );
        }
    }
    parse_outcome(&summary, started.elapsed())
}

fn parse_outcome(line: &str, wall: Duration) -> Outcome {
    Outcome {
        state_checksum: json_field(line, "state_checksum"),
        terrain_checksum: json_field(line, "terrain_checksum"),
        config_hash: json_field(line, "config_hash"),
        final_tick: json_field(line, "final_tick").parse().expect("final_tick"),
        pacing: json_field(line, "pacing"),
        wall,
    }
}

fn ws_connect(ws_port: u16) -> WebSocket<TcpStream> {
    let stream = TcpStream::connect(("127.0.0.1", ws_port)).expect("connect ws");
    let (mut socket, _) =
        tungstenite::client(format!("ws://127.0.0.1:{ws_port}/"), stream).expect("ws handshake");
    socket
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(50)))
        .expect("timeout");
    let bytes = encode(
        &Frame::Hello {
            major: PROTOCOL_MAJOR,
            minor: PROTOCOL_MINOR,
            capabilities: 0,
            token: OBSERVER_TOKEN.as_bytes().to_vec(),
        },
        FrameMeta {
            world_epoch: 0,
            sequence: 0,
            checksummed: false,
        },
    );
    socket.send(Message::Binary(bytes)).expect("hello");
    socket
}

fn subscribe(socket: &mut WebSocket<TcpStream>) {
    let bytes = encode(
        &Frame::Subscribe {
            viewport: Viewport {
                x0_fp: 0,
                y0_fp: 0,
                x1_fp: 1_048_575,
                y1_fp: 1_048_575,
                lod: 0,
            },
            layers: LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS,
            max_rate_hz: 30,
        },
        FrameMeta {
            world_epoch: 0,
            sequence: 0,
            checksummed: false,
        },
    );
    socket.send(Message::Binary(bytes)).expect("subscribe");
}

#[test]
fn a5_1_acceleration_and_observers_are_result_neutral() {
    // Four executions of the same world under four host conditions.
    let executions = [
        (
            "1x realtime pacing",
            run_server("realtime", "1", Observer::None),
        ),
        (
            "maximum headless speed",
            run_server("headless", "1", Observer::None),
        ),
        (
            "observer attached throughout",
            run_server("realtime", "8", Observer::Attached),
        ),
        (
            "observer attached then detached mid-run",
            run_server("realtime", "8", Observer::AttachedThenDetached),
        ),
    ];

    for (label, outcome) in &executions {
        assert_eq!(
            outcome.final_tick, TICKS,
            "{label} ran {} ticks, not {TICKS}",
            outcome.final_tick
        );
    }

    let (first_label, first) = &executions[0];
    for (label, outcome) in &executions[1..] {
        assert_eq!(
            outcome.state_checksum, first.state_checksum,
            "state checksum differs between '{first_label}' and '{label}': \
             {} versus {}",
            first.state_checksum, outcome.state_checksum
        );
        assert_eq!(outcome.terrain_checksum, first.terrain_checksum, "{label}");
        assert_eq!(outcome.config_hash, first.config_hash, "{label}");
    }

    // The executions must actually have been paced differently, or the test
    // proves nothing: 1x pacing of 100 ticks at dt=100ms takes about ten
    // seconds of wall clock, while headless has no such floor.
    let realtime_1x = &executions[0].1;
    let headless = &executions[1].1;
    assert_eq!(realtime_1x.pacing, "realtime");
    assert_eq!(headless.pacing, "headless");
    assert!(
        headless.wall < realtime_1x.wall,
        "headless ({:?}) was not faster than 1x pacing ({:?}); the pacing modes \
         are not actually different and the equality above is vacuous",
        headless.wall,
        realtime_1x.wall
    );
}
