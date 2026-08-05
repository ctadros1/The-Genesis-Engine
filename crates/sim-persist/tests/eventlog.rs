//! Phase 5 acceptance criteria A5.3 and A5.4: the event log is complete and
//! replayable, and its decoder is hostile-input safe.
//!
//! > **A5.3** For a run of at least 10^6 ticks, the event log contains every
//! > event the kernel emitted with zero drops, or, if the bounded per-tick
//! > buffer dropped events, the recorded drop counter matches the gap
//! > exactly. Reading the log back reconstructs the counters in the final
//! > snapshot exactly.
//!
//! > **A5.4** A seeded corruption sweep of at least 20,000 cases produces
//! > zero panics and typed rejections, matching the discipline of the
//! > existing protocol and snapshot sweeps.
//!
//! The 10^6-tick run is `#[ignore]`d and release-only, following the
//! existing long-run convention; a 10^4-tick version of the same assertions
//! runs by default so a regression is caught without a slow suite.

use sim_core::{SimConfig, World};
use sim_persist::{
    EventLogInfo, EventLogRecorder, EventLogWriter, decode_log, decode_log_events,
    decode_log_prefix, encode_snapshot, read_log_info,
};
use std::fs;
use std::path::PathBuf;

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn scratch_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("lifesim-eventlog-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

fn log_info(world: &World) -> EventLogInfo {
    EventLogInfo {
        format_version: sim_persist::EVENT_LOG_FORMAT_VERSION,
        world_id: 1,
        seed: world.config().world_seed,
        config_hash: world.config_hash(),
        event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
        max_events_per_tick: sim_core::MAX_EVENTS_PER_TICK as u32,
        start_tick: 0,
        build_version: sim_persist::BUILD_VERSION.to_owned(),
    }
}

/// A world that keeps producing events for the whole run.
///
/// This matters more than it looks. An earlier version of this test used a
/// small phase2 world, which goes extinct after a few hundred ticks: the run
/// still executed 10^6 ticks and the criterion still passed, on a log
/// containing 62 events. The criterion would have been satisfied by a log
/// that proved essentially nothing. These parameters sustain a population
/// past 10^6 ticks (about 1,900 organisms and 4.5 million events), and the
/// assertions below refuse a run that goes extinct or produces a thin log,
/// so the same hole cannot reopen silently.
fn sustained_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 256;
    config.cells_y = 256;
    config.initial_organisms = 500;
    config.max_entities = 5_000;
    config
}

/// Same shape at a smaller scale, for the default-speed test.
fn sustained_small_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = 200;
    config.max_entities = 3_000;
    config
}

/// Run a world for `ticks`, recording every tick's events, and return the
/// log bytes alongside the finished world.
fn run_and_record_with(config: SimConfig, ticks: u64, directory: &str) -> (World, Vec<u8>) {
    let directory = scratch_dir(directory);
    let path = directory.join("run.alev");
    let mut world = World::new(config).expect("world");

    let writer = EventLogWriter::create(&path, &log_info(&world)).expect("create log");
    let mut recorder = EventLogRecorder::new(writer);
    for _ in 0..ticks {
        world.step();
        recorder.record(&world).expect("record tick");
    }
    recorder.writer_mut().sync().expect("sync");
    let bytes = fs::read(&path).expect("read log");
    (world, bytes)
}

/// Convenience for the tests that only need a log, not a specific world.
fn run_and_record(ticks: u64, directory: &str) -> (World, Vec<u8>) {
    run_and_record_with(sustained_small_config(), ticks, directory)
}

/// The whole of A5.3 for a given run length.
fn assert_log_reconstructs_the_world(config: SimConfig, ticks: u64, directory: &str) {
    let (world, bytes) = run_and_record_with(config, ticks, directory);

    let scan = decode_log(&bytes).expect("log decodes");
    assert_eq!(scan.bytes_consumed, bytes.len(), "trailing bytes");
    assert_eq!(scan.info.seed, SEED);
    assert_eq!(scan.info.config_hash, world.config_hash());

    // The kernel's own drop counter and the log's recorded drops agree, and
    // every counter in the final state is reproduced by replaying the log.
    let counters = world.counters();
    let phase2 = world.phase2_enabled().then(|| world.phase2_counters());
    assert_eq!(
        scan.dropped, counters.dropped_events_total,
        "the log's recorded drop total does not match the kernel's"
    );
    scan.reconcile(&counters, phase2.as_ref())
        .expect("replayed counters must reproduce the world's");

    // "the counters in the final snapshot" is the literal wording, so check
    // against the snapshot's own copy rather than only the live world.
    let state = world.export_state();
    let snapshot = encode_snapshot(
        &state,
        1,
        0,
        world.state_checksum(),
        sim_persist::BUILD_VERSION,
        bytes.len() as u64,
        Some(3),
    )
    .expect("encode snapshot");
    let info = read_log_info(&bytes).expect("header").0;
    assert_eq!(info.config_hash, world.config_hash());
    let (snapshot_info, decoded) = sim_persist::decode_snapshot(&snapshot).expect("decode");
    assert_eq!(
        snapshot_info.event_log_offset,
        bytes.len() as u64,
        "the snapshot must point at the end of the log it was taken beside"
    );
    scan.reconcile(
        &decoded.counters,
        decoded.phase2.as_ref().map(|p2| &p2.counters),
    )
    .expect("replayed counters must reproduce the snapshot's");

    // Materializing the events must agree with the streaming count.
    let (again, events) = decode_log_events(&bytes).expect("decode events");
    assert_eq!(again.counters, scan.counters);
    assert_eq!(events.len() as u64, scan.events);
    assert!(
        events.windows(2).all(|pair| pair[0].tick <= pair[1].tick),
        "events must be in ascending tick order"
    );

    // Guards against a passing-but-empty run. Without these, a world that
    // goes extinct in the first few hundred ticks satisfies every equality
    // above on a log of a few dozen events, and the criterion means nothing.
    assert!(
        !world.is_extinct(),
        "the world went extinct, so most of the {ticks}-tick run logged nothing"
    );
    // One event per four ticks is a floor, not a target: these configs
    // produce between 0.5 and 4.5 events per tick. It exists to make the
    // 62-events-per-million-ticks case fail loudly rather than pass.
    assert!(
        scan.events >= ticks / 4,
        "only {} events over {ticks} ticks; the log is too thin to demonstrate \
         completeness",
        scan.events
    );
    let last = scan.last_tick.expect("at least one event");
    assert!(
        last * 100 >= ticks * 99,
        "the last event was at tick {last} of {ticks}; events must span the run, \
         not stop early"
    );
}

#[test]
fn a5_3_event_log_reconstructs_the_final_counters() {
    assert_log_reconstructs_the_world(sustained_small_config(), 10_000, "short");
}

#[test]
#[ignore = "long run: 10^6 ticks, about six minutes; run with --release --ignored"]
fn a5_3_event_log_reconstructs_the_final_counters_over_a_million_ticks() {
    assert_log_reconstructs_the_world(sustained_config(), 1_000_000, "million");
}

#[test]
fn event_log_growth_rate_is_reported_not_assumed() {
    // Not a threshold: the phase records the measured growth rate rather
    // than asserting one, because a target declared before measurement is
    // exactly the unmeasured claim AGENTS.md forbids. The assertion is only
    // that the file is bounded by the framing arithmetic.
    let (_, bytes) = run_and_record(10_000, "growth");
    let scan = decode_log(&bytes).expect("decode");
    let per_segment_overhead = 28_u64; // magic+tick+count+dropped+len+crc
    let floor = scan.segments * per_segment_overhead;
    assert!(
        bytes.len() as u64 >= floor,
        "file smaller than its own framing overhead"
    );
    eprintln!(
        "event log: {} bytes for {} segments and {} events over 10^4 ticks \
         ({} bytes per 10^6 ticks at this rate)",
        bytes.len(),
        scan.segments,
        scan.events,
        bytes.len() as u64 * 100
    );
}

#[test]
fn a5_4_corruption_sweep_never_panics_and_always_rejects_typed() {
    let (_, valid) = run_and_record(2_000, "sweep");
    assert!(valid.len() > 1_000, "sweep needs a non-trivial log");
    decode_log(&valid).expect("the unmodified log must decode");

    // xorshift, seeded, so a failure reproduces exactly.
    let mut state = 0x00de_fec8_ab1e_5eed_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut rejected = 0_u32;
    let mut accepted = 0_u32;
    const CASES: u32 = 20_000;
    for _ in 0..CASES {
        let mut bytes = valid.clone();
        // Mix of single-bit flips, byte stomps, and truncations, so the
        // sweep exercises framing, checksums, and length caps rather than
        // only one rejection path.
        match next() % 4 {
            0 => {
                let position = (next() % bytes.len() as u64) as usize;
                bytes[position] ^= 1 << (next() % 8);
            }
            1 => {
                let position = (next() % bytes.len() as u64) as usize;
                bytes[position] = (next() % 256) as u8;
            }
            2 => {
                let cut = (next() % bytes.len() as u64) as usize;
                bytes.truncate(cut);
            }
            _ => {
                for _ in 0..1 + next() % 6 {
                    let position = (next() % bytes.len() as u64) as usize;
                    bytes[position] ^= 1 << (next() % 8);
                }
            }
        }

        // The contract is: a typed result, never a panic and never an
        // out-of-bounds read. Both decoders are exercised because the
        // prefix scanner has its own bounds arithmetic.
        match decode_log(&bytes) {
            Ok(_) => accepted += 1,
            Err(_) => rejected += 1,
        }
        let _ = decode_log_prefix(&bytes);
    }

    // A truncation that lands inside the header, or a mutation of a
    // reserved byte, can leave a still-valid shorter log, so the bar is
    // "overwhelmingly rejected", matching the existing snapshot sweep.
    assert!(
        rejected + accepted == CASES,
        "every case must produce a typed result"
    );
    assert!(
        rejected > CASES - CASES / 100,
        "only {rejected}/{CASES} corruptions were rejected"
    );
}

#[test]
fn a_log_truncated_mid_segment_reports_its_valid_prefix() {
    let (_, valid) = run_and_record(2_000, "torn");
    // Cut inside the final segment, the shape a crash between write and
    // sync produces.
    let torn = &valid[..valid.len() - 5];
    assert!(
        decode_log(torn).is_err(),
        "a torn log must not decode whole"
    );

    let (prefix, error) = decode_log_prefix(torn).expect("header is intact");
    assert!(error.is_some(), "the reason the prefix ended must be typed");
    assert!(
        prefix.segments > 0,
        "the intact prefix must still be readable"
    );
    assert!(
        prefix.bytes_consumed < torn.len(),
        "the reported valid length must exclude the torn tail"
    );

    // Every segment in the prefix is fully decoded or absent; nothing is
    // half-admitted.
    let whole = &valid[..prefix.bytes_consumed];
    let reparsed = decode_log(whole).expect("the reported prefix is itself a valid log");
    assert_eq!(reparsed.segments, prefix.segments);
    assert_eq!(reparsed.events, prefix.events);
    assert_eq!(reparsed.counters, prefix.counters);
}

#[test]
fn a_log_from_a_different_world_is_still_readable_but_self_describing() {
    // Provenance is carried, not enforced: the log records which seed and
    // config hash produced it so an analysis can refuse a mismatch itself.
    let (world, bytes) = run_and_record(500, "provenance");
    let scan = decode_log(&bytes).expect("decode");
    assert_eq!(scan.info.seed, world.config().world_seed);
    assert_eq!(scan.info.config_hash, world.config_hash());
    assert_eq!(scan.info.build_version, sim_persist::BUILD_VERSION);
    assert_eq!(
        scan.info.event_schema_version,
        sim_core::EVENT_SCHEMA_VERSION
    );
}
