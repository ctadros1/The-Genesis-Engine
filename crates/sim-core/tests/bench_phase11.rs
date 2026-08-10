//! Phase 11 benchmark and long run: C11.6's ledger horizon and C11.7's
//! `learn`-phase cost.
//!
//! `#[ignore]`; run in release by `scripts/run-phase11-benchmarks.sh`.
//!
//! Two things here that the wave-B work left open.
//!
//! **C11.6 asks for a 10^6-tick run.** The kernel test
//! `phase11_learning::the_energy_ledger_stays_exact_with_plasticity_costs_
//! flowing_through_it` is the exact assertion - it pins the debit to one
//! edge per organism per tick against a matched control on tick 1, which a
//! long run cannot do because the two populations diverge - but it runs
//! 5,000 ticks. Exactness over *duration* is a different claim from
//! exactness at a moment, and it is the one the criterion states, so it is
//! measured here at the horizon the criterion names.
//!
//! **C11.7 asks for `learn` p50/p95 across a range of plastic-edge
//! fractions.** The snapshot half is in `sim-persist`; the phase-timing half
//! had no target at all. Note the plan asks for p95 and no existing bench
//! test computes one - `median` is the only statistic the Phase 6 to 10
//! harnesses use - so a percentile over timing samples is added here.

use sim_core::{
    Genome2, GenomeCaps, LearnedEdgeSave, LocusKind, PlasticityBudget, PlasticityGenes,
    RULE_HEBBIAN, SimConfig, TickObserver, TickPhase, World, compile_network_with_budget,
};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const TIERS: [u32; 2] = [500, 2_000];
const WARMUP_TICKS: u64 = 200;
const SAMPLE_TICKS: u64 = 2_000;

#[derive(Default)]
struct PhaseTimer {
    started: Option<Instant>,
    current: Option<TickPhase>,
    samples: [Vec<f64>; TickPhase::ALL.len()],
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

/// Percentile over timing samples. The existing harnesses only ever take a
/// median; C11.7 asks for p95, and a tail is the whole point of asking - a
/// learn phase whose median is flat and whose p95 is not is a learn phase
/// that stalls on the organisms that actually learned.
fn percentile(samples: &mut [f64], milli: u32) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() as u64 - 1) * u64::from(milli) / 1_000) as usize]
}

fn bench_config(seed: u64, organisms: u32) -> SimConfig {
    let mut config = SimConfig::phase11_default(seed);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = organisms;
    // Population is the independent variable here, so the guard is set to the
    // tier rather than left to the ecology - the opposite of the campaign
    // discipline, and deliberate, exactly as `bench_phase9_snapshot` records.
    config.max_entities = organisms;
    config.plasticity.plastic_edge_cost_milli_per_s = 20;
    config
}

fn plastic_edge_ids(genome: &Genome2, budget: PlasticityBudget) -> Vec<u32> {
    compile_network_with_budget(&genome.express_network(), budget)
        .expect("a rewritten genome compiles")
        .plastic_edges
        .iter()
        .map(|edge| edge.homology_id)
        .collect()
}

/// Flag the first `count` edges of every founder plastic.
///
/// Nothing in the engine writes `EDGE_FLAG_PLASTIC` except point mutation,
/// and waiting for mutation to produce a given plastic-edge fraction would
/// make this a benchmark of the mutation rate. The fraction is authored
/// through the public save path so it is the independent variable.
fn world_with_plastic_edges(config: SimConfig, count_per_organism: usize) -> World {
    let world = World::new(config).expect("world");
    if count_per_organism == 0 {
        return world;
    }
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    let budget = state.config.plasticity_budget();
    let mut rows: Vec<Vec<LearnedEdgeSave>> = Vec::new();
    let schema2 = state.schema2.as_mut().expect("a schema-2 world");
    for index in 0..schema2.genomes.len() {
        let mut genome =
            Genome2::decode(&schema2.genomes[index], &caps).expect("a live genome decodes");
        let mut flagged = 0_usize;
        for haplotype in &mut genome.haplotypes {
            for chromosome in &mut haplotype.chromosomes {
                for locus in chromosome.iter_mut() {
                    if flagged >= count_per_organism {
                        break;
                    }
                    if let LocusKind::Edge {
                        flags, plasticity, ..
                    } = &mut locus.kind
                    {
                        *flags |= sim_core::EDGE_FLAG_PLASTIC;
                        *plasticity = PlasticityGenes {
                            rule_id: RULE_HEBBIAN,
                            eta: 0.01,
                            coefficients: [1.0, 0.0, 0.0, 0.0],
                            decay: 0.0,
                            modulator_node: 0,
                        };
                        flagged += 1;
                    }
                }
            }
        }
        rows.push(
            plastic_edge_ids(&genome, budget)
                .into_iter()
                .map(|edge_homology_id| LearnedEdgeSave {
                    edge_homology_id,
                    learned_q16: 0,
                    trace_q16: 0,
                })
                .collect(),
        );
        schema2.genomes[index] = genome.encode();
    }
    if let Some(learn) = state.learn.as_mut() {
        learn.edges = rows;
    }
    World::from_state(state).expect("the rewritten genomes restore")
}

/// C11.7: `learn` phase p50/p95 at both tiers across plastic-edge fractions.
#[test]
#[ignore = "long run"]
fn phase11_learn_phase_cost_across_plastic_edge_fractions() {
    let learn_index = TickPhase::ALL
        .iter()
        .position(|phase| *phase == TickPhase::Learn)
        .expect("the learn phase exists");
    for tier in TIERS {
        for edges in [0_usize, 1, 2] {
            let mut world = world_with_plastic_edges(bench_config(SEED, tier), edges);
            for _ in 0..WARMUP_TICKS {
                world.step();
            }
            let mut timer = PhaseTimer::default();
            for _ in 0..SAMPLE_TICKS {
                world.step_with_observer(&mut timer);
            }
            let metrics = world.metrics();
            assert!(
                world.population() > 0,
                "tier {tier} with {edges} plastic edges went extinct, so the \
                 timings below are of an empty world"
            );
            // A zero-edge arm must genuinely have none, and a nonzero arm
            // must genuinely have some, or the sweep has no independent
            // variable and the numbers are three copies of one measurement.
            if edges == 0 {
                assert_eq!(metrics.plastic_edges_total, 0);
                assert_eq!(metrics.plasticity_updates_total, 0);
            } else {
                assert!(metrics.plastic_edges_total > 0);
                assert!(
                    metrics.plasticity_updates_total > 0,
                    "the learn phase never ran, so its timing is the cost of \
                     an empty loop"
                );
            }
            let mut samples = timer.samples[learn_index].clone();
            let p50 = percentile(&mut samples, 500);
            let p95 = percentile(&mut samples, 950);
            let mut whole: f64 = 0.0;
            for index in 0..TickPhase::ALL.len() {
                let mut phase_samples = timer.samples[index].clone();
                whole += percentile(&mut phase_samples, 500);
            }
            println!(
                "PHASE11-BENCH learn tier={tier} plastic_edges_per_organism={edges} \
                 population={} plastic_edges_total={} updates={} faults={} saturations={} \
                 learn_p50_us={p50:.3} learn_p95_us={p95:.3} whole_tick_p50_us={whole:.2} \
                 learn_share_milli={}",
                metrics.population,
                metrics.plastic_edges_total,
                metrics.plasticity_updates_total,
                metrics.plasticity_faults_total,
                metrics.plasticity_saturations_total,
                if whole > 0.0 {
                    (p50 * 1_000.0 / whole) as i64
                } else {
                    0
                },
            );
        }
    }
}

/// C11.6 at the horizon the criterion names: ledger exactness with
/// plasticity costs flowing through it over a 10^6-tick run.
///
/// `check_invariants` reconstructs the energy and biomass ledgers from first
/// principles and compares with **no tolerance**, so it is the assertion.
/// Run at a small world, because what is being tested is exactness over
/// duration rather than exactness at scale.
#[test]
#[ignore = "long run"]
fn phase11_ledger_exact_over_a_million_ticks_with_plasticity() {
    // **Mirrors C10.9's world**, which is the one known to sustain a
    // population for 10^6 ticks: 128x128 at a cell capacity of 240,000, with
    // physiology on. The first cut of this test invented a thinner world -
    // 64x64 at the default capacity - and it went extinct with and without
    // plasticity, so the debit was not the cause and the population guard
    // below is what said so. Reusing the known-viable ecology keeps this a
    // test of ledger exactness over duration rather than a re-derivation of
    // carrying capacity.
    let mut config = SimConfig::phase11_default(31);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = 400;
    config.max_entities = 400;
    config.cell_capacity_milli = 240_000;
    config.physiology.enabled = true;
    config.physiology.extrinsic_hazard_q16_per_s = 13;
    config.genome2.mutation.duplication_q16 = 6_554;
    config.genome2.mutation.deletion_q16 = 655;
    config.genome2.mutation.transposition_q16 = 0;
    // The per-tick debit is `cost_milli_per_s * dt_ms / 1000` in whole
    // milli-units, so at the default `dt_ms = 100` anything below 10 milli/s
    // truncates to **zero** and the cost clause would be vacuous. 10 is the
    // cheapest rate that charges anything at all.
    config.plasticity.plastic_edge_cost_milli_per_s = 10;
    let mut world = world_with_plastic_edges(config, 1);
    let mut checks = 0_u64;
    for tick in 1..=1_000_000_u64 {
        world.step();
        if tick % 10_000 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
            checks += 1;
        }
    }
    world.check_invariants().expect("final invariants");
    let metrics = world.metrics();
    assert!(
        world.population() > 0,
        "the world went extinct, so a million ticks of ledger exactness is \
         mostly a statement about an empty world"
    );
    assert!(
        metrics.births_total > 0,
        "nothing reproduced, so the debit never crossed the birth path"
    );
    // The plasticity debit must have actually accrued, or this is C10.9 run
    // again under a different name.
    assert!(
        metrics.plasticity_cost_milli > 0,
        "no plasticity energy was spent, so the ledger clause this test \
         exists for was never exercised"
    );
    assert!(metrics.plasticity_updates_total > 0);
    println!(
        "PHASE11-BENCH ledger ticks=1000000 checks={checks} population={} births={} \
         plastic_edges_total={} updates={} faults={} saturations={} plasticity_cost_milli={} \
         energy_milli={} biomass_milli={}",
        metrics.population,
        metrics.births_total,
        metrics.plastic_edges_total,
        metrics.plasticity_updates_total,
        metrics.plasticity_faults_total,
        metrics.plasticity_saturations_total,
        metrics.plasticity_cost_milli,
        metrics.total_energy_milli,
        metrics.total_biomass_milli,
    );
}
