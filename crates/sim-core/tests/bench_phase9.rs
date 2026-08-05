//! Phase 9 benchmark: what a variable topology costs, and what it costs to
//! store.
//!
//! `#[ignore]`; run in release by `scripts/run-phase9-benchmarks.sh`.
//!
//! This is C9.8's measurement. The structural caps shipped provisional
//! (`GenomeCaps::provisional`) with an explicit obligation attached: "Caps
//! chosen before the measurement are provisional and must be restated
//! afterward." Restating them needs three numbers that did not exist -
//! snapshot bytes per organism under an **evolved** topology distribution
//! rather than under founders, the same figure at the caps rather than at
//! the observed distribution, and the checkpoint cost at both tiers.
//!
//! The plan also asks for the controller cost as a function of node and edge
//! count, and for the **distribution** rather than the mean, "because
//! evolved topology sizes will be skewed". They are: the tail is what sets a
//! cap, and a mean over a population that is mostly founders says nothing
//! about it.

use sim_core::{GenomeCaps, SimConfig, TickObserver, TickPhase, World};
use std::time::Instant;

/// The two tiers every prior phase reported against
/// (`docs/13-performance-strategy.md`).
const TIERS: [u32; 2] = [500, 2_000];

/// The confirmatory campaign's own mutation regime, so the topology
/// distribution measured here is the one the campaign produced rather than
/// a synthetic one.
const CAMPAIGN_DUPLICATION_Q16: u32 = 6_554;
const CAMPAIGN_DELETION_Q16: u32 = 655;
const CAMPAIGN_POINT_Q16: u32 = 6_554;

/// Long enough for structure to have diversified. The ecology sweep put 60k
/// ticks at ~61 generations; 30k is ~30, which is past the point where the
/// distribution has a tail and keeps the benchmark affordable.
const EVOLVE_TICKS: u64 = 30_000;

#[derive(Default)]
struct PhaseTimer {
    started: Option<Instant>,
    current: Option<TickPhase>,
    samples: [Vec<f64>; 8],
}

impl TickObserver for PhaseTimer {
    fn phase_started(&mut self, phase: TickPhase) {
        self.current = Some(phase);
        self.started = Some(Instant::now());
    }
    fn phase_finished(&mut self, phase: TickPhase) {
        if let (Some(started), Some(current)) = (self.started.take(), self.current.take())
            && current == phase
        {
            let index = TickPhase::ALL
                .iter()
                .position(|candidate| *candidate == phase)
                .expect("known phase");
            self.samples[index].push(started.elapsed().as_secs_f64() * 1_000_000.0);
        }
    }
}

fn median(samples: &mut [f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(f64::total_cmp);
    samples[(samples.len() - 1) / 2]
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

/// What the caps *permit*, which is the number a cap has to be chosen
/// against.
///
/// The observed distribution says what evolution produced at one mutation
/// regime; the caps say what the format must survive if a regime, a longer
/// run, or a later phase pushes harder. A cap set from the observed
/// distribution alone would be a cap set from one campaign's ecology.
#[test]
#[ignore = "benchmark"]
fn phase9_cap_worst_case() {
    let caps = GenomeCaps::provisional();
    let mut world = World::new(evolved_config(11, 500)).expect("world");
    for _ in 0..EVOLVE_TICKS {
        world.step();
    }
    let census = world.structure_census();

    // **Marginal** bytes per structural locus, by least squares over the
    // census, not "genome bytes divided by structural loci". The first draft
    // of this measurement did the latter and reported 290 bytes per locus,
    // which is wrong by a factor of seven: it charged the fixed header and
    // the fourteen trait loci to the five structural ones. What a cap needs
    // is the slope - what one more node or edge costs - and the intercept is
    // the founder's fixed cost, which no cap governs.
    let points: Vec<(f64, f64)> = census
        .iter()
        .map(|sample| {
            (
                f64::from(sample.nodes + sample.edges),
                f64::from(sample.genome_bytes),
            )
        })
        .collect();
    let count = points.len() as f64;
    let mean_x = points.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = points.iter().map(|(_, y)| y).sum::<f64>() / count;
    let covariance: f64 = points
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();
    let variance: f64 = points.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();
    let bytes_per_locus = if variance > 0.0 {
        covariance / variance
    } else {
        0.0
    };
    let intercept = mean_y - bytes_per_locus * mean_x;

    // What each cap permits, in the units of the others. The provisional
    // caps were each chosen on their own, and they are not consistent: the
    // byte cap binds long before the node, edge, or locus caps can be
    // reached, so those three never bind at all.
    let loci_allowed_by_bytes = if bytes_per_locus > 0.0 {
        ((f64::from(caps.max_genome_bytes) - intercept) / bytes_per_locus).max(0.0)
    } else {
        0.0
    };
    let loci_allowed_by_locus_caps =
        f64::from(caps.max_loci_per_chromosome * u32::from(caps.max_chromosomes) * 2);
    let loci_needed_by_node_edge_caps = f64::from(caps.max_nodes + caps.max_edges);
    println!(
        "PHASE9-BENCH caps max_chromosomes={} max_loci_per_chromosome={} max_nodes={} \
         max_edges={} max_edges_per_node={} max_genome_bytes={} min_nodes={} \
         marginal_bytes_per_locus={bytes_per_locus:.1} fixed_bytes={intercept:.0} \
         loci_allowed_by_byte_cap={loci_allowed_by_bytes:.0} \
         loci_allowed_by_locus_caps={loci_allowed_by_locus_caps:.0} \
         loci_needed_by_node_edge_caps={loci_needed_by_node_edge_caps:.0} \
         bytes_needed_by_node_edge_caps={:.0}",
        caps.max_chromosomes,
        caps.max_loci_per_chromosome,
        caps.max_nodes,
        caps.max_edges,
        caps.max_edges_per_node,
        caps.max_genome_bytes,
        caps.min_nodes,
        intercept + bytes_per_locus * loci_needed_by_node_edge_caps,
    );
    for tier in TIERS {
        println!(
            "PHASE9-BENCH cap-budget tier={tier} \
             snapshot_bytes_at_max_genome_bytes={} snapshot_mb_at_max_genome_bytes={:.1}",
            u64::from(caps.max_genome_bytes) * u64::from(tier),
            u64::from(caps.max_genome_bytes) as f64 * f64::from(tier) / 1_048_576.0,
        );
    }
}

/// Controller cost against evolved topology size, and the tick cost schema 2
/// carries against schema 1.
#[test]
#[ignore = "benchmark"]
fn phase9_controller_cost() {
    for tier in TIERS {
        for (label, genome2) in [("schema1", false), ("schema2", true)] {
            let mut config = evolved_config(11, tier);
            config.genome2.enabled = genome2;
            let mut world = World::new(config).expect("world");
            // Evolve first, then measure: the whole point is the cost of an
            // evolved distribution, and measuring at tick 0 would measure
            // founders.
            for _ in 0..EVOLVE_TICKS {
                world.step();
            }
            let mut timer = PhaseTimer::default();
            let mut population_sum = 0_u128;
            let ticks = 500_u64;
            for _ in 0..ticks {
                world.step_with_observer(&mut timer);
                population_sum += world.population() as u128;
            }
            let mean_population = (population_sum / u128::from(ticks)).max(1) as f64;
            let mut whole = 0.0;
            for (index, phase) in TickPhase::ALL.iter().enumerate() {
                let value = median(&mut timer.samples[index]);
                whole += value;
                println!(
                    "PHASE9-BENCH phase tier={tier} label={label} phase={phase:?} \
                     p50_us={value:.2} p50_us_per_1000={:.2}",
                    value * 1_000.0 / mean_population,
                );
            }
            let metrics = world.metrics();
            println!(
                "PHASE9-BENCH tick tier={tier} label={label} whole_tick_p50_us={whole:.2} \
                 mean_population={mean_population:.0} ticks_per_second={:.0} \
                 mean_nodes_milli={} mean_edges_milli={} distinct={}",
                1_000_000.0 / whole.max(0.001),
                metrics.mean_nodes_milli,
                metrics.mean_edges_milli,
                metrics.distinct_structures,
            );
        }
    }
}
