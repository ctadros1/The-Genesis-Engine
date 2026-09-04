//! Spawn harness shared by the multi-world integration tests.
//!
//! The process-spawning half is copied from `tests/integration.rs`, whose
//! own copy stays where it is: that file is the compatibility test (S5) and
//! it has to keep passing unmodified, so it cannot be refactored into this
//! one without weakening what it proves. The REST helper here differs on
//! purpose - it can send a request body, which world creation needs.

#![allow(dead_code)]

use sim_protocol::{Frame, FrameMeta, PROTOCOL_MAJOR, PROTOCOL_MINOR, Viewport, decode, encode};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

pub const OBSERVER_TOKEN: &str = "test-observer-token";
pub const ADMIN_TOKEN: &str = "test-admin-token";

/// Backstop for a server that is alive but silent, not an assertion about
/// how fast a healthy server boots.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);

static PORT_OFFSET: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

pub struct ServerGuard {
    child: Child,
    pub rest_port: u16,
    pub ws_port: u16,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn spawn_server(extra_args: &[&str]) -> ServerGuard {
    // A range above the one `integration.rs` and `phase5_acceleration.rs`
    // draw from, so two test binaries running at once cannot collide.
    let base = 40_000
        + (std::process::id() % 15_000) as u16
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
        .stderr(Stdio::null());
    let mut child = command.spawn().expect("spawn server");

    let stdout = child.stdout.take().expect("stdout");
    let (lines_tx, lines_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    let _ = lines_tx.send(line);
                }
            }
        }
    });
    let started = Instant::now();
    loop {
        let remaining = STARTUP_TIMEOUT.saturating_sub(started.elapsed());
        match lines_rx.recv_timeout(remaining) {
            Ok(line) if line.contains("REST on") => break,
            Ok(_) => {}
            Err(RecvTimeoutError::Disconnected) => {
                let _ = child.kill();
                panic!("server stdout closed before the banner");
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                panic!("server did not start within {STARTUP_TIMEOUT:?}");
            }
        }
    }
    ServerGuard {
        child,
        rest_port,
        ws_port,
    }
}

pub fn http(guard: &ServerGuard, method: &str, path: &str, token: &str) -> (u16, String) {
    request(guard, method, path, Some(token), None, None)
}

pub fn post_json(
    guard: &ServerGuard,
    path: &str,
    token: &str,
    body: &str,
) -> (u16, String) {
    request(guard, "POST", path, Some(token), None, Some(body))
}

pub fn request(
    guard: &ServerGuard,
    method: &str,
    path: &str,
    token: Option<&str>,
    idempotency: Option<&str>,
    body: Option<&str>,
) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", guard.rest_port)).expect("connect rest");
    let mut head =
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    if let Some(token) = token {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some(key) = idempotency {
        head.push_str(&format!("Idempotency-Key: {key}\r\n"));
    }
    let body = body.unwrap_or("");
    head.push_str("Content-Type: application/json\r\n");
    head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    head.push_str(body);
    stream.write_all(head.as_bytes()).expect("write");
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

/// Read one unsigned JSON number field out of a response body. The bodies
/// this server writes are flat, so a scan is enough and keeps the tests
/// free of a JSON dependency the workspace does not have.
pub fn number_field(body: &str, key: &str) -> u64 {
    text_field(body, key)
        .parse()
        .unwrap_or_else(|_| panic!("field {key} is not a number in {body}"))
}

/// Read one field's text, whether it was written as a string or a bare
/// value.
pub fn text_field(body: &str, key: &str) -> String {
    let needle = format!("\"{key}\":");
    let start = body
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in {body}"))
        + needle.len();
    let rest = &body[start..];
    let rest = rest.strip_prefix('"').unwrap_or(rest);
    let end = rest
        .find(['"', ',', '}'])
        .unwrap_or_else(|| panic!("unterminated {key} in {body}"));
    rest[..end].to_owned()
}

pub fn ws_connect(guard: &ServerGuard, path: &str, token: &str) -> WebSocket<TcpStream> {
    let stream = TcpStream::connect(("127.0.0.1", guard.ws_port)).expect("connect ws");
    let (mut socket, _) = tungstenite::client(
        format!("ws://127.0.0.1:{}{path}", guard.ws_port),
        stream,
    )
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

pub fn send(socket: &mut WebSocket<TcpStream>, frame: &Frame) {
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

pub fn read_frame(
    socket: &mut WebSocket<TcpStream>,
    deadline: Duration,
) -> Option<(FrameMeta, Frame)> {
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

pub fn full_viewport() -> Viewport {
    Viewport {
        x0_fp: 0,
        y0_fp: 0,
        x1_fp: 1_048_575,
        y1_fp: 1_048_575,
        lod: 0,
    }
}
