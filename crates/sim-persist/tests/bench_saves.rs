//! Phase 4 persistence benchmark: save/restore duration and size at the
//! documented tiers, comparing the uncompressed codec against zstd levels
//! (the explicitly bounded compressed-codec comparison ADR-0007 requires).
//!
//! `#[ignore]`; run in release mode by `scripts/run-phase4-benchmarks.sh`
//! with LIFESIM_BENCH_OUTPUT set.

use sim_core::{SimConfig, World};
use sim_persist::{decode_snapshot, encode_snapshot};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const SAMPLES: usize = 20;

fn percentiles(samples: &mut [f64]) -> (f64, f64) {
    samples.sort_by(f64::total_cmp);
    let pick =
        |fraction: f64| -> f64 { samples[((samples.len() - 1) as f64 * fraction).ceil() as usize] };
    (pick(0.5), pick(0.95))
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase4-benchmarks.sh"]
fn save_restore_benchmark_with_compression_comparison() {
    let mut scenarios = String::from("[");
    for (scenario_index, organisms) in [500_u32, 2_000].into_iter().enumerate() {
        let config = {
            let mut config = SimConfig::phase2_default(SEED);
            config.initial_organisms = organisms;
            config
        };
        let mut world = World::new(config).unwrap();
        // Warm the ecology so the snapshot carries realistic state.
        for _ in 0..2_000 {
            world.step();
        }
        let population = world.population();
        let checksum = world.state_checksum();
        let state = world.export_state();

        let mut variants = String::from("[");
        for (variant_index, (label, level)) in [
            ("uncompressed", None),
            ("zstd-1", Some(1)),
            ("zstd-3", Some(3)),
            ("zstd-9", Some(9)),
        ]
        .into_iter()
        .enumerate()
        {
            let mut encode_samples = Vec::with_capacity(SAMPLES);
            let mut decode_samples = Vec::with_capacity(SAMPLES);
            let mut restore_samples = Vec::with_capacity(SAMPLES);
            let mut bytes_len = 0_usize;
            for _ in 0..SAMPLES {
                let started = Instant::now();
                let bytes = encode_snapshot(&state, 1, 0, checksum, "bench", 0, level).unwrap();
                encode_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
                bytes_len = bytes.len();

                let started = Instant::now();
                let (_, decoded) = decode_snapshot(&bytes).unwrap();
                decode_samples.push(started.elapsed().as_secs_f64() * 1_000.0);

                let started = Instant::now();
                let restored = World::from_state(decoded).unwrap();
                restore_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
                assert_eq!(restored.state_checksum(), checksum);
            }
            let (encode_p50, encode_p95) = percentiles(&mut encode_samples);
            let (decode_p50, decode_p95) = percentiles(&mut decode_samples);
            let (restore_p50, restore_p95) = percentiles(&mut restore_samples);
            if variant_index > 0 {
                variants.push(',');
            }
            variants.push_str(&format!(
                concat!(
                    "{{\"codec\":\"{}\",\"bytes\":{},",
                    "\"encode_ms\":{{\"p50\":{:.3},\"p95\":{:.3}}},",
                    "\"decode_ms\":{{\"p50\":{:.3},\"p95\":{:.3}}},",
                    "\"rebuild_world_ms\":{{\"p50\":{:.3},\"p95\":{:.3}}}}}"
                ),
                label,
                bytes_len,
                encode_p50,
                encode_p95,
                decode_p50,
                decode_p95,
                restore_p50,
                restore_p95
            ));
            eprintln!(
                "{organisms} organisms {label}: {bytes_len} bytes, encode p50 {encode_p50:.2} ms, decode p50 {decode_p50:.2} ms, rebuild p50 {restore_p50:.2} ms"
            );
        }
        variants.push(']');
        if scenario_index > 0 {
            scenarios.push(',');
        }
        scenarios.push_str(&format!(
            "{{\"initial_organisms\":{organisms},\"population_at_save\":{population},\"tick\":{},\"state_checksum\":\"0x{checksum:016x}\",\"variants\":{variants}}}",
            state.tick
        ));
    }
    scenarios.push(']');

    let record = format!(
        concat!(
            "{{\n  \"benchmark_schema_version\": 2,\n",
            "  \"benchmark_kind\": \"phase4-persistence\",\n",
            "  \"method\": {{\"samples_per_variant\": {}, \"world\": \"phase2 default after 2,000 ticks\", ",
            "\"note\": \"save duration equals the tick-thread stall for synchronous checkpoints; ",
            "encode includes state capture serialization, rebuild includes terrain regeneration and invariant checks\"}},\n",
            "  \"scenarios\": {},\n",
            "  \"limitations\": [\"local development host, not deployment VM or storage\", ",
            "\"file system effects excluded (in-memory encode/decode); atomic-write fsync cost is workload-dependent\"]\n}}\n"
        ),
        SAMPLES, scenarios
    );
    if let Ok(output_dir) = std::env::var("LIFESIM_BENCH_OUTPUT") {
        std::fs::create_dir_all(&output_dir).expect("output dir");
        std::fs::write(
            std::path::Path::new(&output_dir).join("phase4-persistence-summary.json"),
            &record,
        )
        .expect("write record");
    }
    eprintln!("{record}");
}
