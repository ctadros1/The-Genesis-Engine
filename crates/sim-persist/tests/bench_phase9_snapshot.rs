//! Phase 9 snapshot budget: C9.8's storage half.
//!
//! `#[ignore]`; run in release by `scripts/run-phase9-benchmarks.sh`.
//!
//! Lives here rather than beside the rest of the Phase 9 benchmark because
//! it needs the codec, and `sim-core` is deliberately dependency-free. The
//! topology distribution it reports is duplicated from the kernel-side
//! benchmark on purpose: a snapshot figure without the distribution that
//! produced it cannot be checked against a cap.

use sim_core::{SimConfig, World};
use std::time::Instant;

const TIERS: [u32; 2] = [500, 2_000];
const CAMPAIGN_DUPLICATION_Q16: u32 = 6_554;
const CAMPAIGN_DELETION_Q16: u32 = 655;
const CAMPAIGN_POINT_Q16: u32 = 6_554;
const EVOLVE_TICKS: u64 = 30_000;

fn percentile(sorted: &[u32], milli: u32) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() as u64 - 1) * u64::from(milli) / 1_000) as usize;
    sorted[index]
}

fn evolved_config(seed: u64, organisms: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = organisms;
    // **The guard binds here on purpose**, which is the opposite of the
    // C8.3 discipline every campaign follows. A campaign must let ecology
    // set the population, or it is reporting a memory limit; this benchmark
    // has population as its *independent variable*, and at a fixed carrying
    // capacity both tiers equilibrate to the same few thousand organisms,
    // which would make "at both tiers" a label on one measurement.
    config.max_entities = organisms;
    config.cell_capacity_milli = 240_000;
    config.genome2.enabled = true;
    config.physiology.enabled = true;
    config.physiology.extrinsic_hazard_q16_per_s = 13;
    config.genome2.mutation.duplication_q16 = CAMPAIGN_DUPLICATION_Q16;
    config.genome2.mutation.deletion_q16 = CAMPAIGN_DELETION_Q16;
    config.genome2.mutation.point_q16 = CAMPAIGN_POINT_Q16;
    config.genome2.mutation.transposition_q16 = 0;
    config
}

/// Snapshot budget and topology distribution at one tier.
#[test]
#[ignore = "benchmark"]
fn phase9_snapshot_budget() {
    for tier in TIERS {
        let mut world = World::new(evolved_config(11, tier)).expect("world");
        for _ in 0..EVOLVE_TICKS {
            world.step();
        }
        let population = world.population();
        assert!(
            population > 0,
            "tier {tier} went extinct before measurement"
        );

        let census = world.structure_census();
        let mut nodes: Vec<u32> = census.iter().map(|sample| sample.nodes).collect();
        let mut edges: Vec<u32> = census.iter().map(|sample| sample.edges).collect();
        let mut bytes: Vec<u32> = census.iter().map(|sample| sample.genome_bytes).collect();
        nodes.sort_unstable();
        edges.sort_unstable();
        bytes.sort_unstable();
        let total_genome_bytes: u64 = bytes.iter().map(|value| u64::from(*value)).sum();

        // The whole snapshot, through the same path a checkpoint takes, so
        // the figure includes framing and every other section rather than
        // only the genome arrays.
        let checksum = world.state_checksum();
        let state = world.export_state();
        let started = Instant::now();
        let encoded =
            sim_persist::encode_snapshot(&state, 1, 0, checksum, "bench", 0, None).expect("encode");
        let encode_us = started.elapsed().as_secs_f64() * 1_000_000.0;
        let started = Instant::now();
        let (_, decoded) = sim_persist::decode_snapshot(&encoded).expect("decode");
        let decode_us = started.elapsed().as_secs_f64() * 1_000_000.0;
        // Restore all the way back into a world and check the checksum, so a
        // snapshot that encodes fast by dropping something is not reported as
        // a cheap snapshot.
        let started = Instant::now();
        let restored = sim_core::World::from_state(decoded).expect("restore");
        let restore_us = started.elapsed().as_secs_f64() * 1_000_000.0;
        assert_eq!(restored.state_checksum(), checksum);

        println!(
            "PHASE9-BENCH snapshot tier={tier} population={population} \
             snapshot_bytes={} bytes_per_organism={} genome_bytes_total={} \
             genome_bytes_share_milli={} encode_us={encode_us:.1} decode_us={decode_us:.1} \
             restore_us={restore_us:.1}",
            encoded.len(),
            (encoded.len() / population.max(1)) as u64,
            total_genome_bytes,
            total_genome_bytes * 1_000 / (encoded.len() as u64).max(1),
        );
        println!(
            "PHASE9-BENCH topology tier={tier} population={population} \
             nodes_p50={} nodes_p90={} nodes_p99={} nodes_max={} \
             edges_p50={} edges_p90={} edges_p99={} edges_max={} \
             genome_bytes_p50={} genome_bytes_p90={} genome_bytes_p99={} genome_bytes_max={}",
            percentile(&nodes, 500),
            percentile(&nodes, 900),
            percentile(&nodes, 990),
            nodes.last().copied().unwrap_or(0),
            percentile(&edges, 500),
            percentile(&edges, 900),
            percentile(&edges, 990),
            edges.last().copied().unwrap_or(0),
            percentile(&bytes, 500),
            percentile(&bytes, 900),
            percentile(&bytes, 990),
            bytes.last().copied().unwrap_or(0),
        );
    }
}
