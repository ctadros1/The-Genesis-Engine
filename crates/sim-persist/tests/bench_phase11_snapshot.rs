//! Phase 11 snapshot budget: C11.7's storage half.
//!
//! `#[ignore]`; run in release by `scripts/run-phase11-benchmarks.sh`.
//!
//! Lives here rather than beside a kernel-side Phase 11 benchmark for the
//! reason `bench_phase9_snapshot.rs` gives: it needs the codec, and
//! `sim-core` is deliberately dependency-free.
//!
//! # What C11.7 actually has to answer
//!
//! `PlasticityConfig::max_plastic_edges` shipped at 32 with an explicit note
//! that it is **provisional** and must be restated from measurement, exactly
//! as the genome caps were restated once by C9.8. The question is not "how
//! big is a snapshot" but "how big does a snapshot get per plastic edge, and
//! what does that make the cap worth". Sparse storage is the design bet - the
//! spec argues that storing a dense learned copy of every weight would
//! roughly double snapshot size against the Phase 4 record - and this is
//! where the bet is settled or lost.
//!
//! # Three conditions, because two of them would lie
//!
//! - `off`: the plasticity section disabled. The baseline the "on versus off"
//!   number is against.
//! - `evolved`: the section enabled and `mutation.plasticity_enabled` on, run
//!   long enough for point mutation to flip flags. **This is the realistic
//!   level and it may be near zero**, because nothing in the founder is
//!   plastic and the flag has to be discovered - which is the phase's named
//!   most-likely failure, not a benchmark defect. The measured plastic-edge
//!   fraction is printed beside every byte count so a reader can tell which
//!   of those two a small number is.
//! - `seeded`: every founder edge flagged plastic, through the same public
//!   save path the Phase 11 tests use. This is the **upper bound**, and it is
//!   the condition the cap has to be set against: a budget justified by a
//!   population that never evolved any plasticity is a budget for a world
//!   that does not need one.
//!
//! Reporting `evolved` alone would understate the cost by however much
//! evolution happened not to find; reporting `seeded` alone would overstate
//! it as a certainty. Both are printed, labelled, with the fraction that
//! produced each.

use sim_core::{SimConfig, World};
use sim_persist::{AsyncCheckpointer, CheckpointRequest, SnapshotStore, SubmitResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const TIERS: [u32; 2] = [500, 2_000];
const CAMPAIGN_DUPLICATION_Q16: u32 = 6_554;
const CAMPAIGN_DELETION_Q16: u32 = 655;
const CAMPAIGN_POINT_Q16: u32 = 6_554;
const EVOLVE_TICKS: u64 = 30_000;

/// The Phase 9 snapshot benchmark's world, so the two records are comparable.
///
/// C11.7 is stated "against the Phase 9 record", which only means something
/// if the world underneath is the same one. Grid, tier handling, capacity and
/// mutation rates are copied deliberately; the plasticity section is the only
/// thing that varies across the conditions below.
fn base_config(seed: u64, organisms: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = organisms;
    // The guard binds on purpose, as in Phase 9: population is this
    // benchmark's independent variable, and at a fixed carrying capacity both
    // tiers equilibrate to the same few thousand organisms, which would make
    // "at both tiers" a label on one measurement.
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

fn plastic_config(seed: u64, organisms: u32) -> SimConfig {
    let mut config = base_config(seed, organisms);
    config.genome2.mutation.plasticity_enabled = true;
    config.plasticity.enabled = true;
    config
}

/// Flag every founder edge plastic, through the public save path.
///
/// The same construction `sim-core`'s `phase11_learning.rs` and
/// `sim-persist`'s `config_round_trip.rs` use, and duplicated for the same
/// reason: nothing in the engine writes `EDGE_FLAG_PLASTIC`, so a benchmark
/// that waited for evolution to produce an upper bound would be measuring the
/// mutation operator instead of the snapshot.
///
/// The learn rows are rewritten to name the edges the rewritten plans mark
/// plastic, because `World::from_state` refuses a section that does not.
fn seeded_world(config: SimConfig) -> World {
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let budget = state.config.plasticity_budget();
    let mut rows = Vec::new();
    let schema2 = state.schema2.as_mut().expect("a schema-2 world");
    for encoded in schema2.genomes.iter_mut() {
        let mut genome = sim_core::Genome2::decode(encoded, &caps).expect("a live genome decodes");
        for haplotype in &mut genome.haplotypes {
            for chromosome in &mut haplotype.chromosomes {
                for locus in chromosome.iter_mut() {
                    if let sim_core::LocusKind::Edge {
                        flags, plasticity, ..
                    } = &mut locus.kind
                    {
                        *flags |= sim_core::EDGE_FLAG_PLASTIC;
                        *plasticity = sim_core::PlasticityGenes {
                            rule_id: sim_core::RULE_HEBBIAN,
                            eta: 0.002,
                            coefficients: [1.0, 0.0, 0.0, 0.0],
                            decay: 0.01,
                            modulator_node: 0,
                        };
                    }
                }
            }
        }
        rows.push(
            sim_core::compile_network_with_budget(&genome.express_network(), budget)
                .expect("a rewritten genome compiles")
                .plastic_edges
                .iter()
                .map(|edge| sim_core::LearnedEdgeSave {
                    edge_homology_id: edge.homology_id,
                    learned_q16: 0,
                    trace_q16: 0,
                })
                .collect::<Vec<_>>(),
        );
        *encoded = genome.encode();
    }
    state
        .learn
        .as_mut()
        .expect("a plasticity save section")
        .edges = rows;
    World::from_state(state).expect("the rewritten genomes restore")
}

/// Bytes the learn section contributes, computed from the record rather than
/// diffed against a snapshot of a different world.
///
/// A difference between two snapshots is a difference between two *worlds*:
/// enabling plasticity changes the config hash, the trajectory, the
/// population and every genome, so subtracting one total from the other
/// measures ecology at least as much as storage. The section's own framing is
/// exact and is what the cap has to be set against.
fn learn_section_bytes(state: &sim_core::SaveState) -> u64 {
    let Some(learn) = state.learn.as_ref() else {
        return 0;
    };
    // Section framing (tag, flags, length, crc) + organism count word.
    let mut bytes = 2 + 2 + 8 + 4 + 8_u64;
    for row in &learn.edges {
        // Per-organism plastic-edge count word, the records, the fault word.
        bytes += 4 + 4 + 12 * row.len() as u64;
    }
    // Six counters and the cost accumulator.
    bytes += 6 * 8 + 16;
    bytes
}

fn measure(label: &str, tier: u32, mut world: World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
    let population = world.population();
    assert!(population > 0, "{label} tier {tier} went extinct");

    let census = world.learned_census();
    let plastic_edges: u64 = census
        .iter()
        .map(|sample| u64::from(sample.plastic_edges))
        .sum();
    let learners = census
        .iter()
        .filter(|sample| sample.plastic_edges > 0)
        .count();
    let mut per_organism: Vec<u32> = census.iter().map(|sample| sample.plastic_edges).collect();
    per_organism.sort_unstable();
    let metrics = world.metrics();

    let checksum = world.state_checksum();
    let state = world.export_state();
    let section_bytes = learn_section_bytes(&state);
    let started = Instant::now();
    let encoded =
        sim_persist::encode_snapshot(&state, 1, 0, checksum, "bench", 0, None).expect("encode");
    let encode_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    let started = Instant::now();
    let (_, decoded) = sim_persist::decode_snapshot(&encoded).expect("decode");
    let decode_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    // Restore all the way back into a world and check the checksum, so a
    // snapshot that encodes fast by dropping the learn section is not reported
    // as a cheap snapshot.
    let started = Instant::now();
    let restored = World::from_state(decoded).expect("restore");
    let restore_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    assert_eq!(restored.state_checksum(), checksum);

    let percentile = |sorted: &[u32], milli: u32| -> u32 {
        if sorted.is_empty() {
            return 0;
        }
        sorted[((sorted.len() as u64 - 1) * u64::from(milli) / 1_000) as usize]
    };
    println!(
        "PHASE11-BENCH snapshot tier={tier} label={label} population={population} \
         snapshot_bytes={} bytes_per_organism={} learn_section_bytes={section_bytes} \
         learn_bytes_per_organism={} learn_share_milli={} plastic_edges_total={plastic_edges} \
         learners={learners} plastic_edges_p50={} plastic_edges_p90={} plastic_edges_max={} \
         mean_plastic_fraction_milli={} mean_abs_learned_milli={} \
         encode_us={encode_us:.1} decode_us={decode_us:.1} restore_us={restore_us:.1}",
        encoded.len(),
        (encoded.len() / population.max(1)) as u64,
        section_bytes / population.max(1) as u64,
        section_bytes * 1_000 / (encoded.len() as u64).max(1),
        percentile(&per_organism, 500),
        percentile(&per_organism, 900),
        per_organism.last().copied().unwrap_or(0),
        metrics.mean_plastic_fraction_milli,
        metrics.mean_abs_learned_milli,
    );
}

/// Snapshot **size** at both tiers, plasticity off, evolved, and seeded.
///
/// The checkpoint-stall half of C11.7 is
/// `phase11_checkpoint_stall_through_the_async_checkpointer` below. This doc
/// comment claimed both for the whole time only one was measured, which is
/// the shape of stale status text the phase plan warns about: a name that
/// covers a gap reads as coverage.
#[test]
#[ignore = "benchmark"]
fn phase11_snapshot_budget() {
    for tier in TIERS {
        measure(
            "off",
            tier,
            World::new(base_config(11, tier)).expect("world"),
            EVOLVE_TICKS,
        );
        measure(
            "evolved",
            tier,
            World::new(plastic_config(11, tier)).expect("world"),
            EVOLVE_TICKS,
        );
        measure(
            "seeded",
            tier,
            seeded_world(plastic_config(11, tier)),
            EVOLVE_TICKS,
        );
    }
}

// --- C11.7's second half: the stall, through the mechanism the plan names ---

fn percentiles(samples: &mut [f64]) -> (f64, f64, f64) {
    samples.sort_by(f64::total_cmp);
    let pick =
        |fraction: f64| -> f64 { samples[((samples.len() - 1) as f64 * fraction).ceil() as usize] };
    (pick(0.5), pick(0.95), pick(0.99))
}

fn scratch_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("lifesim-bench11-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

/// One condition's stall, synchronous against asynchronous.
///
/// Methodology is A5.5's, deliberately, so the two records are comparable:
/// same tiers, same checkpoint interval, same split between all-tick and
/// checkpoint-tick samples. What is new is that the world carries evolved or
/// seeded plastic edges, which is the clause C11.7 asks for and A5.5 could
/// not have supplied - Phase 5 predates plasticity.
///
/// **Checkpoint ticks are sampled separately, and that is not a refinement.**
/// At one checkpoint every 200 ticks they are 0.5 percent of the sample, so a
/// p95 over all ticks cannot see them by construction. Reporting only the
/// all-tick p95 would make any stall invisible and the comparison
/// meaningless - which is exactly how a benchmark reports "no stall" for a
/// world that stalls.
fn measure_stall(label: &str, tier: u32, world: &World, budget_ms: f64) {
    let directory = scratch_dir(&format!("{label}{tier}"));
    let measured = 2_000_u64;
    let interval = 200_u64;

    // --- Synchronous: encode, compress and fsync inline on the tick thread.
    let sync_store = Arc::new(Mutex::new(
        SnapshotStore::open(&directory.join("sync"))
            .expect("store")
            .0,
    ));
    let mut sync_world = World::from_state(world.export_state()).expect("clone world");
    let mut sync_samples = Vec::with_capacity(measured as usize);
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

    // --- Asynchronous: capture inline, everything else on the writer thread.
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
            match checkpointer.submit(CheckpointRequest {
                state,
                state_checksum: checksum,
                world_id: 1,
                parent_world_id: 0,
                name: "auto".to_owned(),
                kind: "checkpoint".to_owned(),
                event_log_offset: 0,
                compression_level: Some(3),
                prune_keep: None,
            }) {
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
        "{label} tier {tier}: an asynchronous checkpoint failed"
    );
    let (async_p50, async_p95, async_p99) = percentiles(&mut async_samples);
    let async_max = async_samples.last().copied().unwrap_or(0.0);
    let (async_cp_p50, _, _) = percentiles(&mut async_checkpoint_ticks);
    let async_cp_max = async_checkpoint_ticks.last().copied().unwrap_or(0.0);
    let write_max_ms = write_durations.iter().copied().max().unwrap_or(0) as f64 / 1_000.0;

    // **The guard that makes every number above mean something.** A
    // checkpoint mode that changed the world would make the two columns
    // measurements of different simulations, and a cheaper one would look
    // like a faster one. This is A5.5's assertion and it is repeated here
    // because it is repeated for the same reason.
    assert_eq!(
        sync_world.state_checksum(),
        async_world.state_checksum(),
        "{label} tier {tier}: checkpoint mode changed the world"
    );

    let census = async_world.learned_census();
    let plastic_edges: u64 = census
        .iter()
        .map(|sample| u64::from(sample.plastic_edges))
        .sum();
    let population = async_world.population();
    // Printed beside every timing, so a small stall cannot be read as "sparse
    // learned state is cheap to checkpoint" when it means "nothing was
    // plastic". This is the same guard the storage half prints its
    // plastic-edge fraction for.
    let edges_per_organism_milli = plastic_edges * 1_000 / (population as u64).max(1);

    println!(
        "PHASE11-BENCH checkpoint-stall tier={tier} label={label} population={population} \
         plastic_edges_total={plastic_edges} \
         plastic_edges_per_organism_milli={edges_per_organism_milli} \
         ticks={measured} checkpoint_interval_ticks={interval} budget_ms={budget_ms:.1} \
         sync_checkpoints={sync_checkpoints} sync_tick_p50_ms={sync_p50:.4} \
         sync_tick_p95_ms={sync_p95:.4} sync_tick_p99_ms={sync_p99:.4} \
         sync_tick_max_ms={sync_max:.4} sync_checkpoint_tick_p50_ms={sync_cp_p50:.4} \
         sync_checkpoint_tick_max_ms={sync_cp_max:.4} \
         async_checkpoints={async_checkpoints} async_refused={refused} \
         async_tick_p50_ms={async_p50:.4} async_tick_p95_ms={async_p95:.4} \
         async_tick_p99_ms={async_p99:.4} async_tick_max_ms={async_max:.4} \
         async_checkpoint_tick_p50_ms={async_cp_p50:.4} \
         async_checkpoint_tick_max_ms={async_cp_max:.4} \
         writer_thread_max_ms={write_max_ms:.4}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// C11.7's checkpoint-stall clause, through `AsyncCheckpointer`.
///
/// **This is the gap the phase plan named and it is closed here.** What was
/// reported before was synchronous encode/decode/restore time on the tick
/// thread - a real number, and not the one the criterion asks for. The plan
/// calls Phase 5's asynchronous checkpointing a prerequisite for exactly this
/// measurement, so reporting the synchronous cost in its place was a
/// substitution rather than an answer, and the status text said so.
///
/// Three conditions per tier, the same three the storage half uses, because
/// "with realistic evolved plasticity levels" is a claim about the world and
/// not about the checkpointer: `evolved` is what a campaign actually carries
/// and may be near zero, and `seeded` is the upper bound the cap is set
/// against.
#[test]
#[ignore = "benchmark"]
fn phase11_checkpoint_stall_through_the_async_checkpointer() {
    // The tick budget is the configured tick interval, not a target chosen
    // for the benchmark.
    let budget_ms = f64::from(SimConfig::phase2_default(11).dt_ms);
    for tier in TIERS {
        // Evolved to the same horizon the storage half uses, so the two
        // halves describe the same worlds.
        let mut off = World::new(base_config(11, tier)).expect("world");
        let mut evolved = World::new(plastic_config(11, tier)).expect("world");
        let mut seeded = seeded_world(plastic_config(11, tier));
        for _ in 0..EVOLVE_TICKS {
            off.step();
            evolved.step();
            seeded.step();
        }
        measure_stall("off", tier, &off, budget_ms);
        measure_stall("evolved", tier, &evolved, budget_ms);
        measure_stall("seeded", tier, &seeded, budget_ms);
    }
}
