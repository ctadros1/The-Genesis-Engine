//! Phase 5 benchmark: headless throughput, scheduler scaling and host
//! contention, event-log write cost and growth, and the checkpoint stall on
//! the tick thread.
//!
//! `#[ignore]`; run in release mode by `scripts/run-phase5-benchmarks.sh`
//! with LIFESIM_BENCH_OUTPUT set.
//!
//! Nothing here declares a target. The plan is explicit that
//! "no acceptance criterion of the form 'achieves X worlds at Y ticks per
//! second'" exists, because declaring a target before measuring is the
//! unmeasured scale claim AGENTS.md forbids. The one threshold that is
//! asserted is A5.5's, and it is a budget the configuration already fixes
//! (the tick interval), not a performance goal invented here.

use sim_core::{SimConfig, World};
use sim_experiment::{Campaign, SchedulerOptions, run_campaign};
use sim_persist::{
    AsyncCheckpointer, CheckpointRequest, EventLogInfo, EventLogRecorder, EventLogWriter,
    SnapshotStore, SubmitResult,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// The documented supported tiers.
const TIERS: [u32; 2] = [500, 2_000];
/// Warm the ecology so measurements reflect a live world, matching the
/// Phase 4 record's method.
const WARMUP_TICKS: u64 = 2_000;

fn percentiles(samples: &mut [f64]) -> (f64, f64, f64) {
    samples.sort_by(f64::total_cmp);
    let pick =
        |fraction: f64| -> f64 { samples[((samples.len() - 1) as f64 * fraction).ceil() as usize] };
    (pick(0.5), pick(0.95), pick(0.99))
}

fn tier_config(organisms: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.initial_organisms = organisms;
    config.max_entities = organisms * 10;
    config
}

fn scratch_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("lifesim-bench5-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

fn emit(section: &str, body: &str) {
    println!("PHASE5-BENCH {section} {body}");
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase5-benchmarks.sh"]
fn headless_throughput_per_world_at_both_tiers() {
    for organisms in TIERS {
        let mut world = World::new(tier_config(organisms)).unwrap();
        for _ in 0..WARMUP_TICKS {
            world.step();
        }
        let measured = 5_000_u64;
        let mut samples = Vec::with_capacity(measured as usize);
        let started = Instant::now();
        for _ in 0..measured {
            let tick_started = Instant::now();
            world.step();
            samples.push(tick_started.elapsed().as_secs_f64() * 1_000.0);
        }
        let wall = started.elapsed().as_secs_f64();
        let (p50, p95, p99) = percentiles(&mut samples);
        emit(
            "headless-throughput",
            &format!(
                "tier={organisms} population={} ticks={measured} wall_s={wall:.3} \
                 ticks_per_second={:.1} tick_p50_ms={p50:.4} tick_p95_ms={p95:.4} \
                 tick_p99_ms={p99:.4}",
                world.population(),
                measured as f64 / wall
            ),
        );
    }
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase5-benchmarks.sh"]
fn scheduler_scaling_and_host_contention() {
    // 16 independent worlds of the small tier, run at increasing worker
    // counts. Per-world throughput degradation as workers increase is the
    // host-contention number the plan asks for.
    let campaign = Campaign::parse(
        "campaign bench-scaling\nticks 3000\nseeds 1..16\nbase preset phase2\n\
         base cells_x 128\nbase cells_y 128\nbase initial_organisms 200\n\
         base max_entities 2000\ncondition only\noutput events off\noutput snapshots off\n",
    )
    .expect("campaign");
    let total_ticks = campaign.ticks * campaign.run_count() as u64;

    let mut single_world_rate = 0.0_f64;
    for workers in [1_usize, 2, 4, 8] {
        let started = Instant::now();
        let results = run_campaign(&campaign, &SchedulerOptions::in_memory(workers));
        let wall = started.elapsed().as_secs_f64();
        assert!(results.iter().all(|result| result.is_ok()));
        let aggregate = total_ticks as f64 / wall;
        let per_world = aggregate / workers as f64;
        if workers == 1 {
            single_world_rate = aggregate;
        }
        emit(
            "scheduler-scaling",
            &format!(
                "workers={workers} worlds={} ticks_per_world={} wall_s={wall:.3} \
                 aggregate_ticks_per_second={aggregate:.1} \
                 per_worker_ticks_per_second={per_world:.1} \
                 speedup_versus_1_worker={:.3} \
                 per_world_degradation_percent={:.2}",
                campaign.run_count(),
                campaign.ticks,
                aggregate / single_world_rate,
                if single_world_rate > 0.0 {
                    (1.0 - per_world / single_world_rate) * 100.0
                } else {
                    0.0
                }
            ),
        );
    }
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase5-benchmarks.sh"]
fn event_log_write_cost_and_growth_rate() {
    let directory = scratch_dir("eventlog");
    // Alternating repetitions rather than one run of each. A single
    // with-versus-without comparison on this host produced a *negative*
    // overhead, which is not a finding about logging: it is run-to-run
    // variance exceeding the effect. Alternating and reporting the median
    // of each side keeps the comparison honest about which is which, and
    // the reported spread shows when the effect is below the noise floor.
    const REPEATS: usize = 5;
    for organisms in TIERS {
        let config = tier_config(organisms);
        let measured = 5_000_u64;
        let mut without_samples = Vec::with_capacity(REPEATS);
        let mut with_samples = Vec::with_capacity(REPEATS);
        let mut bytes = 0_u64;
        let mut events = 0_u64;
        let mut plain_checksum = 0_u64;
        let mut logged_checksum = 0_u64;

        for repeat in 0..REPEATS {
            let mut plain = World::new(config).unwrap();
            for _ in 0..WARMUP_TICKS {
                plain.step();
            }
            let started = Instant::now();
            for _ in 0..measured {
                plain.step();
            }
            without_samples.push(started.elapsed().as_secs_f64());
            plain_checksum = plain.state_checksum();

            let mut logged = World::new(config).unwrap();
            for _ in 0..WARMUP_TICKS {
                logged.step();
            }
            let path = directory.join(format!("tier{organisms}-{repeat}.alev"));
            let writer = EventLogWriter::create(
                &path,
                &EventLogInfo {
                    format_version: sim_persist::EVENT_LOG_FORMAT_VERSION,
                    world_id: 1,
                    seed: SEED,
                    config_hash: logged.config_hash(),
                    event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
                    max_events_per_tick: sim_core::MAX_EVENTS_PER_TICK as u32,
                    start_tick: logged.tick_number(),
                    build_version: sim_persist::BUILD_VERSION.to_owned(),
                },
            )
            .expect("create log");
            let mut recorder = EventLogRecorder::new(writer);
            let started = Instant::now();
            for _ in 0..measured {
                logged.step();
                recorder.record(&logged).expect("record");
            }
            with_samples.push(started.elapsed().as_secs_f64());
            recorder.writer_mut().sync().expect("sync");
            bytes = recorder.writer().offset();
            events = recorder.writer().events();
            logged_checksum = logged.state_checksum();
        }

        // The two worlds must still agree: logging is an observation.
        assert_eq!(
            logged_checksum, plain_checksum,
            "recording an event log changed the world"
        );

        without_samples.sort_by(f64::total_cmp);
        with_samples.sort_by(f64::total_cmp);
        let without = without_samples[REPEATS / 2];
        let with = with_samples[REPEATS / 2];
        let spread = |samples: &[f64]| (samples[samples.len() - 1] / samples[0] - 1.0) * 100.0;
        emit(
            "event-log-cost",
            &format!(
                "tier={organisms} ticks={measured} repeats={REPEATS} \
                 median_wall_without_log_s={without:.4} median_wall_with_log_s={with:.4} \
                 median_overhead_percent={:.3} without_spread_percent={:.2} \
                 with_spread_percent={:.2} events={events} bytes={bytes} \
                 bytes_per_million_ticks={} bytes_per_event={:.2}",
                (with / without - 1.0) * 100.0,
                spread(&without_samples),
                spread(&with_samples),
                bytes * 1_000_000 / measured,
                if events > 0 {
                    bytes as f64 / events as f64
                } else {
                    0.0
                }
            ),
        );
    }
    let _ = std::fs::remove_dir_all(&directory);
}

/// A5.5: measured tick p95 during checkpointing, synchronous versus
/// asynchronous, at both tiers.
#[test]
#[ignore = "timed benchmark; run via scripts/run-phase5-benchmarks.sh"]
fn a5_5_checkpoint_stall_on_the_tick_thread() {
    // The tick budget is the configured tick interval; at dt_ms = 100 that
    // is 100 ms. This is a budget the configuration fixes, not a target
    // chosen for the benchmark.
    let budget_ms = f64::from(SimConfig::phase2_default(SEED).dt_ms);

    for organisms in TIERS {
        let directory = scratch_dir(&format!("checkpoint{organisms}"));
        let config = tier_config(organisms);
        let mut world = World::new(config).unwrap();
        for _ in 0..WARMUP_TICKS {
            world.step();
        }
        let population = world.population();

        // Checkpoint every `interval` ticks so several land inside the
        // measured window in both modes.
        let measured = 2_000_u64;
        let interval = 200_u64;

        // --- Synchronous: the Phase 4 path, encode and fsync inline. ---
        let sync_store = Arc::new(Mutex::new(
            SnapshotStore::open(&directory.join("sync"))
                .expect("store")
                .0,
        ));
        let mut sync_world = World::from_state(world.export_state()).expect("clone world");
        let mut sync_samples = Vec::with_capacity(measured as usize);
        // Checkpoint ticks are separated out because they are the only ones
        // that can show a stall. With a checkpoint every 200 ticks, they are
        // 0.5 percent of the sample, so a p95 over all ticks cannot see them
        // by construction; reporting only p95 would make any stall
        // invisible and the comparison meaningless.
        let mut sync_checkpoint_ticks = Vec::new();
        let mut sync_checkpoints = 0_u32;
        for tick in 0..measured {
            let started = Instant::now();
            sync_world.step();
            let is_checkpoint = tick % interval == 0;
            if is_checkpoint {
                let state = sync_world.export_state();
                let checksum = sync_world.state_checksum();
                sync_store
                    .lock()
                    .expect("store")
                    .save(&state, checksum, 1, 0, "auto", "checkpoint", 0, Some(3))
                    .expect("save");
                sync_checkpoints += 1;
            }
            // The tick thread's cost is the step plus whatever the
            // checkpoint made it wait for.
            let cost = started.elapsed().as_secs_f64() * 1_000.0;
            sync_samples.push(cost);
            if is_checkpoint {
                sync_checkpoint_ticks.push(cost);
            }
        }
        let (sync_p50, sync_p95, sync_p99) = percentiles(&mut sync_samples);
        let sync_max = sync_samples.last().copied().unwrap_or(0.0);
        let (sync_cp_p50, _, _) = percentiles(&mut sync_checkpoint_ticks);
        let sync_cp_max = sync_checkpoint_ticks.last().copied().unwrap_or(0.0);

        // --- Asynchronous: capture inline, write elsewhere. ---
        let async_store = Arc::new(Mutex::new(
            SnapshotStore::open(&directory.join("async"))
                .expect("store")
                .0,
        ));
        let checkpointer = AsyncCheckpointer::spawn(Arc::clone(&async_store));
        let mut async_world = World::from_state(world.export_state()).expect("clone world");
        let mut async_samples = Vec::with_capacity(measured as usize);
        let mut async_checkpoint_ticks = Vec::new();
        let mut async_checkpoints = 0_u32;
        let mut refused = 0_u32;
        for tick in 0..measured {
            let started = Instant::now();
            async_world.step();
            let is_checkpoint = tick % interval == 0;
            if is_checkpoint {
                let state = async_world.export_state();
                let checksum = async_world.state_checksum();
                let outcome = checkpointer.submit(CheckpointRequest {
                    state,
                    state_checksum: checksum,
                    world_id: 1,
                    parent_world_id: 0,
                    name: "auto".to_owned(),
                    kind: "checkpoint".to_owned(),
                    event_log_offset: 0,
                    compression_level: Some(3),
                    prune_keep: None,
                });
                match outcome {
                    SubmitResult::Accepted => async_checkpoints += 1,
                    SubmitResult::Busy => refused += 1,
                    SubmitResult::Stopped => panic!("writer stopped"),
                }
            }
            let cost = started.elapsed().as_secs_f64() * 1_000.0;
            async_samples.push(cost);
            if is_checkpoint {
                async_checkpoint_ticks.push(cost);
            }
        }
        checkpointer.wait_idle();
        let write_durations: Vec<u64> = checkpointer
            .drain_outcomes()
            .iter()
            .map(|outcome| outcome.duration_us)
            .collect();
        let outcomes = checkpointer.shutdown();
        assert!(
            outcomes.iter().all(|outcome| outcome.error.is_none()),
            "an asynchronous checkpoint failed"
        );
        let (async_p50, async_p95, async_p99) = percentiles(&mut async_samples);
        let async_max = async_samples.last().copied().unwrap_or(0.0);
        let (async_cp_p50, _, _) = percentiles(&mut async_checkpoint_ticks);
        let async_cp_max = async_checkpoint_ticks.last().copied().unwrap_or(0.0);
        // What the write actually cost, off the tick thread.
        let write_max_ms = write_durations.iter().copied().max().unwrap_or(0) as f64 / 1_000.0;

        // Both worlds must still be the same world.
        assert_eq!(
            sync_world.state_checksum(),
            async_world.state_checksum(),
            "checkpoint mode changed the world"
        );

        emit(
            "checkpoint-stall",
            &format!(
                "tier={organisms} population={population} ticks={measured} \
                 checkpoint_interval_ticks={interval} budget_ms={budget_ms:.1} \
                 sync_checkpoints={sync_checkpoints} sync_tick_p50_ms={sync_p50:.4} \
                 sync_tick_p95_ms={sync_p95:.4} sync_tick_p99_ms={sync_p99:.4} \
                 sync_tick_max_ms={sync_max:.4} \
                 sync_checkpoint_tick_p50_ms={sync_cp_p50:.4} \
                 sync_checkpoint_tick_max_ms={sync_cp_max:.4} \
                 async_checkpoints={async_checkpoints} async_refused={refused} \
                 async_tick_p50_ms={async_p50:.4} async_tick_p95_ms={async_p95:.4} \
                 async_tick_p99_ms={async_p99:.4} async_tick_max_ms={async_max:.4} \
                 async_checkpoint_tick_p50_ms={async_cp_p50:.4} \
                 async_checkpoint_tick_max_ms={async_cp_max:.4} \
                 async_write_max_ms={write_max_ms:.4}"
            ),
        );

        // A5.5's assertion: tick p95 during checkpointing is within the
        // configured tick interval. Asserted on the checkpoint ticks
        // themselves as well, because those are the only ticks that can
        // exceed the budget and a p95 over all ticks cannot see them.
        assert!(
            async_p95 < budget_ms,
            "tier {organisms}: asynchronous tick p95 {async_p95:.3} ms exceeds the \
             {budget_ms:.1} ms tick budget"
        );
        assert!(
            async_cp_max < budget_ms,
            "tier {organisms}: the slowest asynchronous checkpoint tick took \
             {async_cp_max:.3} ms, over the {budget_ms:.1} ms tick budget"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }
}
