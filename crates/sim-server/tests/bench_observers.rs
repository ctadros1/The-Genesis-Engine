//! Phase 3 stream benchmark: tick percentiles with 0, 1, and 4 observers,
//! plus per-client bandwidth and dropped-update behavior.
//!
//! `#[ignore]` because it runs for tens of seconds; invoked by
//! `scripts/run-phase3-benchmarks.sh` in release mode with
//! LIFESIM_BENCH_OUTPUT pointing at the raw record directory.

use sim_protocol::{
    Frame, FrameMeta, LAYER_METRICS, LAYER_ORGANISMS, LAYER_TERRAIN, PROTOCOL_MAJOR,
    PROTOCOL_MINOR, Viewport, decode, encode,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tungstenite::Message;

const OBSERVER_TOKEN: &str = "bench-observer";
const ADMIN_TOKEN: &str = "bench-admin";
const REST_PORT: u16 = 8960;
const WS_PORT: u16 = 8961;
const MEASURE_SECONDS: u64 = 15;

fn http_get(path: &str, token: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", REST_PORT)).expect("rest");
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).expect("write");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("read");
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default()
}

fn observer_thread(stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let stream = TcpStream::connect(("127.0.0.1", WS_PORT)).expect("ws");
        let (mut socket, _) =
            tungstenite::client(format!("ws://127.0.0.1:{WS_PORT}/"), stream).expect("handshake");
        socket
            .get_ref()
            .set_read_timeout(Some(Duration::from_millis(100)))
            .expect("timeout");
        let meta = FrameMeta {
            world_epoch: 0,
            sequence: 0,
            checksummed: false,
        };
        socket
            .send(Message::Binary(encode(
                &Frame::Hello {
                    major: PROTOCOL_MAJOR,
                    minor: PROTOCOL_MINOR,
                    capabilities: 0,
                    token: OBSERVER_TOKEN.as_bytes().to_vec(),
                },
                meta,
            )))
            .expect("hello");
        socket
            .send(Message::Binary(encode(
                &Frame::Subscribe {
                    viewport: Viewport {
                        x0_fp: 0,
                        y0_fp: 0,
                        x1_fp: 1_048_575,
                        y1_fp: 1_048_575,
                        lod: 0,
                    },
                    layers: LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS,
                    max_rate_hz: 20,
                },
                meta,
            )))
            .expect("subscribe");
        // Read constantly and acknowledge, like a healthy live client.
        while !stop.load(Ordering::Relaxed) {
            match socket.read() {
                Ok(Message::Binary(bytes)) => {
                    if let Ok((frame_meta, Frame::Delta { .. } | Frame::Keyframe { .. })) =
                        decode(&bytes)
                    {
                        let _ = socket.send(Message::Binary(encode(
                            &Frame::Ack {
                                applied_sequence: frame_meta.sequence,
                            },
                            meta,
                        )));
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(error))
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
        }
    })
}

fn measure_scenario(observers: usize) -> (String, Vec<(u64, u64)>) {
    let stop = Arc::new(AtomicBool::new(false));
    let handles: Vec<_> = (0..observers)
        .map(|_| observer_thread(Arc::clone(&stop)))
        .collect();
    // Settle, then reset the sample ring and measure a fixed window.
    std::thread::sleep(Duration::from_secs(2));
    let _ = http_get("/api/benchmarks/ticks?reset=1", OBSERVER_TOKEN);
    std::thread::sleep(Duration::from_secs(MEASURE_SECONDS));
    let stats = http_get("/api/benchmarks/ticks", OBSERVER_TOKEN);
    // Per-client byte counters before teardown.
    let clients: Vec<(u64, u64)> = stats
        .split("\"bytes_sent\":")
        .skip(1)
        .map(|chunk| {
            let bytes: u64 = chunk
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let dropped: u64 = chunk
                .split("\"dropped_updates\":")
                .nth(1)
                .and_then(|rest| {
                    rest.split(|c: char| !c.is_ascii_digit())
                        .next()
                        .and_then(|value| value.parse().ok())
                })
                .unwrap_or(0);
            (bytes, dropped)
        })
        .collect();
    stop.store(true, Ordering::Relaxed);
    for handle in handles {
        let _ = handle.join();
    }
    std::thread::sleep(Duration::from_millis(500));
    (stats, clients)
}

struct ServerGuard(Child);
impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "multi-scenario timed benchmark; run via scripts/run-phase3-benchmarks.sh"]
fn stream_benchmark_with_observer_fanout() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lifesim-server"))
        .env("LIFESIM_OBSERVER_TOKEN", OBSERVER_TOKEN)
        .env("LIFESIM_ADMIN_TOKEN", ADMIN_TOKEN)
        .args([
            "--rest-port",
            &REST_PORT.to_string(),
            "--ws-port",
            &WS_PORT.to_string(),
            "--organisms",
            "500",
            "--speed",
            "16",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server");
    {
        let stdout = child.stdout.take().expect("stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            assert!(reader.read_line(&mut line).unwrap_or(0) > 0, "startup");
            if line.contains("REST on") {
                break;
            }
        }
    }
    let _guard = ServerGuard(child);
    // Warm the ecology so frames carry a realistic population.
    std::thread::sleep(Duration::from_secs(5));

    let mut scenarios = String::from("[");
    for (index, observers) in [0_usize, 1, 4].into_iter().enumerate() {
        let started = Instant::now();
        let (stats, clients) = measure_scenario(observers);
        let elapsed = started.elapsed().as_secs_f64();
        let mut client_json = String::from("[");
        for (client_index, (bytes, dropped)) in clients.iter().enumerate() {
            if client_index > 0 {
                client_json.push(',');
            }
            client_json.push_str(&format!(
                "{{\"bytes_sent\":{bytes},\"dropped_updates\":{dropped},\"bytes_per_second\":{:.0}}}",
                *bytes as f64 / elapsed
            ));
        }
        client_json.push(']');
        if index > 0 {
            scenarios.push(',');
        }
        scenarios.push_str(&format!(
            "{{\"observers\":{observers},\"measure_seconds\":{MEASURE_SECONDS},\"server_stats\":{stats},\"clients\":{client_json}}}"
        ));
        eprintln!("scenario observers={observers}: {stats}");
    }
    scenarios.push(']');

    let record = format!(
        concat!(
            "{{\n  \"benchmark_schema_version\": 2,\n",
            "  \"benchmark_kind\": \"phase3-stream\",\n",
            "  \"scenario\": {{\"initial_organisms\": 500, \"phase2\": true, \"speed\": 16, ",
            "\"subscription\": \"full world, all layers, 20 Hz\"}},\n",
            "  \"scenarios\": {},\n",
            "  \"limitations\": [\"local loopback only, not deployment network\", ",
            "\"synthetic Rust observers, not browsers\", ",
            "\"population follows the live trajectory during measurement\"]\n}}\n"
        ),
        scenarios
    );
    if let Ok(output_dir) = std::env::var("LIFESIM_BENCH_OUTPUT") {
        std::fs::create_dir_all(&output_dir).expect("output dir");
        std::fs::write(
            std::path::Path::new(&output_dir).join("phase3-stream-summary.json"),
            &record,
        )
        .expect("write record");
    }
    eprintln!("{record}");
}
