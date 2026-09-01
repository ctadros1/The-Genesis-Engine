//! Phase 13 benchmarks: what the social channel costs per tick, emitted as
//! `PHASE13-BENCH` markers collected by `scripts/run-phase13-benchmarks.sh`.
//!
//! The arms isolate the seam from the work, as every phase's benchmark
//! does: `disabled` is the Phase 12 artifact world; `quiet` turns the
//! section on with no founder binding any social channel; `perceiving`
//! binds one neighbour cue per founder; `emitting` binds one emission
//! channel; `full` is the `--social` trace's script. The K and density
//! sweeps answer the plan's stated question - how sense cost scales with
//! `perception_k` and with crowding.
//!
//! **The plan asks whether "unbound channels are not gathered" holds in
//! practice.** For the *controller gather* it does - an unbound channel is
//! never requested. For the *sense-phase cue scan* it does not: with
//! `perception_enabled` the K-nearest scan and cue fill run for every
//! organism whatever its bindings, so the quiet arm pays the scan. The
//! numbers here are the measurement of that; the census-gated skip is a
//! recorded backlog item, to be taken only if the quiet delta matters at
//! campaign scale.

use sim_core::{
    Activation, CHANNEL_NEIGHBOUR_BASE, CHANNEL_SIGNAL_EMIT_BASE, Genome2, GenomeCaps, Locus,
    LocusKind, NodeRole, STRUCTURAL_HOMOLOGY_BASE, SimConfig, World,
};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const WARMUP_TICKS: u64 = 100;
const SAMPLE_TICKS: u64 = 500;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

fn base_config(social: bool, founders: u32, k: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = founders;
    config.max_entities = 4_000;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.contest.enabled = true;
    config.artifact.enabled = true;
    config.artifact.action_cost_milli = 6;
    config.artifact.strike_cost_milli = 12;
    config.social.enabled = social;
    if social {
        config.social.perception_k = k;
        config.social.signal_cost_milli = 2;
    }
    config.validate().expect("validates");
    config
}

fn bind_always_on(genome: &mut Genome2, channel: u16, salt: u32) {
    let node_id = STRUCTURAL_HOMOLOGY_BASE + 50_000 + salt * 10;
    for haplotype in &mut genome.haplotypes {
        let chromosome = &mut haplotype.chromosomes[0];
        chromosome.push(Locus {
            homology_id: node_id,
            gene_lineage_id: u64::from(node_id),
            mutation_event_id: 0,
            kind: LocusKind::Node {
                role: NodeRole::Output,
                activation_id: Activation::TanhApprox.id(),
                bias: 8.0,
                time_constant: 0,
            },
        });
        chromosome.push(Locus {
            homology_id: node_id + 1,
            gene_lineage_id: u64::from(node_id + 1),
            mutation_event_id: 0,
            kind: LocusKind::IoBinding {
                node: node_id,
                channel_id: channel,
                gain: 1.0,
            },
        });
        chromosome.sort_unstable_by_key(|locus| locus.homology_id);
    }
}

/// Every founder bound always-on to `channels`, through the save path.
fn scripted_world(config: SimConfig, channels: &[u16]) -> World {
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    let schema2 = state.schema2.as_mut().expect("schema 2");
    for index in 0..schema2.genomes.len() {
        let mut genome = Genome2::decode(&schema2.genomes[index], &caps).expect("decodes");
        for (salt, &channel) in channels.iter().enumerate() {
            bind_always_on(&mut genome, channel, salt as u32);
        }
        genome.validate_structure(&caps).expect("validates");
        schema2.genomes[index] = genome.encode();
        for _ in 0..channels.len() {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    World::from_state(state).expect("restores")
}

fn tick_cost_of(mut world: World) -> (f64, World) {
    for _ in 0..WARMUP_TICKS {
        world.step();
    }
    let mut samples = Vec::with_capacity(SAMPLE_TICKS as usize);
    for _ in 0..SAMPLE_TICKS {
        let started = Instant::now();
        world.step();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    (median(&mut samples), world)
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn social_tick_cost_disabled_quiet_perceiving_emitting_full() {
    let (disabled_us, disabled) = tick_cost_of(World::new(base_config(false, 200, 4)).expect("w"));
    let (quiet_us, quiet) = tick_cost_of(World::new(base_config(true, 200, 4)).expect("w"));
    let (perceiving_us, _) = tick_cost_of(scripted_world(
        base_config(true, 200, 4),
        &[CHANNEL_NEIGHBOUR_BASE],
    ));
    let (emitting_us, emitting) = tick_cost_of(scripted_world(
        base_config(true, 200, 4),
        &[CHANNEL_SIGNAL_EMIT_BASE],
    ));
    let (full_us, full) = tick_cost_of(scripted_world(
        base_config(true, 200, 4),
        &[CHANNEL_NEIGHBOUR_BASE, CHANNEL_SIGNAL_EMIT_BASE],
    ));
    let quiet_counters = quiet.social_counters().expect("section on");
    assert_eq!(
        quiet_counters.signals_emitted_total, 0,
        "the quiet arm emitted: {quiet_counters:?}"
    );
    let emitting_counters = emitting.social_counters().expect("section on");
    assert!(
        emitting_counters.signals_emitted_total > 0,
        "the emitting arm never emitted"
    );
    let full_counters = full.social_counters().expect("section on");
    println!(
        "PHASE13-BENCH social-tick cells=16384 founders=200 k=4 \
         disabled_us={disabled_us:.1} disabled_population={} \
         quiet_us={quiet_us:.1} quiet_population={} \
         perceiving_us={perceiving_us:.1} \
         emitting_us={emitting_us:.1} emitting_signals={} \
         full_us={full_us:.1} full_signals={} full_population={}",
        disabled.metrics().population,
        quiet.metrics().population,
        emitting_counters.signals_emitted_total,
        full_counters.signals_emitted_total,
        full.metrics().population,
    );
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn sense_cost_scales_with_k_and_density() {
    for founders in [200_u32, 1_000] {
        let (disabled_us, world) =
            tick_cost_of(World::new(base_config(false, founders, 4)).expect("w"));
        println!(
            "PHASE13-BENCH sense-scaling founders={founders} k=off \
             disabled_us={disabled_us:.1} population={}",
            world.metrics().population,
        );
        for k in [1_u32, 4] {
            let (quiet_us, world) =
                tick_cost_of(World::new(base_config(true, founders, k)).expect("w"));
            println!(
                "PHASE13-BENCH sense-scaling founders={founders} k={k} \
                 quiet_us={quiet_us:.1} population={}",
                world.metrics().population,
            );
        }
    }
}
