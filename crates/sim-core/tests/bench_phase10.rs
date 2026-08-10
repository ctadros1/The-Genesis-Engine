//! Phase 10 benchmark and long run: C10.8's caps measurement and C10.9's
//! ledger.
//!
//! `#[ignore]`; run in release by `scripts/run-phase10-benchmarks.sh`.
//!
//! The plan is explicit that **caps are set from this measurement, not
//! before it**, and asks for the per-organism cost against module count *as
//! a distribution* rather than as a mean, since evolved sizes will be
//! skewed. It also asks for the interaction with controller cost, because
//! neural modules drive the node budget and the two skews multiply.

use sim_core::{LatticeKind, SimConfig, TickObserver, TickPhase, World};
use std::time::Instant;

const TIERS: [u32; 2] = [500, 2_000];
const EVOLVE_TICKS: u64 = 30_000;

#[derive(Default)]
struct PhaseTimer {
    started: Option<Instant>,
    current: Option<TickPhase>,
    // Widened from 8 by Phase 11's `learn` phase. Sized from
    // `TickPhase::ALL` rather than restated, so the next phase to be added
    // is a compile error here instead of an index panic at run time.
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

fn median(samples: &mut [f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(f64::total_cmp);
    samples[(samples.len() - 1) / 2]
}

fn percentile(sorted: &[u32], milli: u32) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() as u64 - 1) * u64::from(milli) / 1_000) as usize]
}

fn campaign_config(seed: u64, organisms: u32, morphology: bool) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = organisms;
    // The guard binds so a tier is a population rather than a label, exactly
    // as the Phase 9 benchmark does and for the same reason.
    config.max_entities = organisms;
    config.cell_capacity_milli = 240_000;
    config.genome2.enabled = true;
    config.physiology.enabled = true;
    config.physiology.extrinsic_hazard_q16_per_s = 13;
    config.genome2.mutation.duplication_q16 = 6_554;
    config.genome2.mutation.deletion_q16 = 655;
    config.genome2.mutation.transposition_q16 = 0;
    config.morphology.enabled = morphology;
    config
}

/// Module-count distribution and what the caps permit against it.
#[test]
#[ignore = "benchmark"]
fn phase10_caps_and_distribution() {
    for lattice in [LatticeKind::Square, LatticeKind::Hex] {
        let mut config = campaign_config(11, 2_000, true);
        config.morphology.lattice = lattice;
        let mut world = World::new(config).expect("world");
        for _ in 0..EVOLVE_TICKS {
            world.step();
        }
        let census = world.morphology_census();
        let mut modules: Vec<u32> = census.iter().map(|sample| sample.modules).collect();
        modules.sort_unstable();
        let metrics = world.metrics();
        let caps = config.morphology.caps;
        println!(
            "PHASE10-BENCH distribution lattice={} population={} \
             modules_p50={} modules_p90={} modules_p99={} modules_max={} \
             distinct={} nonviable={} refused_node_budget={} \
             cap_max_modules={} cap_lattice_radius={} cap_max_growth_steps={}",
            lattice.name(),
            metrics.population,
            percentile(&modules, 500),
            percentile(&modules, 900),
            percentile(&modules, 990),
            modules.last().copied().unwrap_or(0),
            metrics.distinct_morphologies,
            metrics.nonviable_bodies,
            metrics.refused_node_budget,
            caps.max_modules,
            caps.lattice_radius,
            caps.max_growth_steps,
        );
    }
}

/// Tick cost with and without morphology, at both tiers.
#[test]
#[ignore = "benchmark"]
fn phase10_tick_cost() {
    for tier in TIERS {
        for (label, morphology) in [("schema2", false), ("morphology", true)] {
            let mut world = World::new(campaign_config(11, tier, morphology)).expect("world");
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
            for index in 0..TickPhase::ALL.len() {
                whole += median(&mut timer.samples[index]);
            }
            let metrics = world.metrics();
            println!(
                "PHASE10-BENCH tick tier={tier} label={label} whole_tick_p50_us={whole:.2} \
                 mean_population={mean_population:.0} ticks_per_second={:.0} \
                 mean_modules_milli={} bodies_grown={}",
                1_000_000.0 / whole.max(0.001),
                metrics.mean_modules_milli,
                metrics.bodies_grown,
            );
        }
    }
}

/// C10.9: ledger exactness with growth energy flowing through it over a
/// 10^6-tick run.
///
/// The invariant check is the assertion; it reconstructs the energy and
/// biomass ledgers from first principles and fails if a single milli-unit
/// has gone missing. Run at a small world so a million ticks is affordable,
/// because what is being tested is exactness over duration rather than
/// exactness at scale.
#[test]
#[ignore = "long run"]
fn phase10_ledger_exact_over_a_million_ticks() {
    let mut config = campaign_config(29, 400, true);
    config.cells_x = 64;
    config.cells_y = 64;
    config.max_entities = 4_000;
    let mut world = World::new(config).expect("world");
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
    assert!(metrics.births_total > 0 && metrics.bodies_grown > 0);
    println!(
        "PHASE10-BENCH ledger ticks=1000000 checks={checks} population={} births={} \
         bodies_grown={} nonviable={} mean_modules_milli={} distinct={} \
         energy_milli={} biomass_milli={}",
        metrics.population,
        metrics.births_total,
        metrics.bodies_grown,
        metrics.nonviable_bodies,
        metrics.mean_modules_milli,
        metrics.distinct_morphologies,
        metrics.total_energy_milli,
        metrics.total_biomass_milli,
    );
}
