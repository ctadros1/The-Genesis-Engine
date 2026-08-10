//! Phase 8 benchmark: the per-organism cost demography adds, and where.
//!
//! `#[ignore]`; run in release by `scripts/run-phase8-benchmarks.sh`.
//!
//! The plan asks for this one specifically because ADR-0025 moved the phase
//! earlier: "the per-organism cost delta against Phase 7, and the resulting
//! change in ticks per second per world, since that number is the honest
//! price of moving this phase earlier and it now applies to every campaign
//! that follows rather than only to the last few."
//!
//! As in Phase 7, demography changes the population, so per-phase
//! microseconds are normalized per 1,000 organisms and each mechanism is
//! measured with the hazards switched off where possible, so cost is not
//! confounded with the demographic effect.

use sim_core::{SimConfig, TickObserver, TickPhase, World};
use std::time::Instant;

const SEED: u64 = 3;
const WARMUP_TICKS: u64 = 300;
const MEASURED_TICKS: u64 = 2_000;

#[derive(Default)]
struct PhaseTimer {
    started: Option<Instant>,
    current: Option<TickPhase>,
    // Widened from 8 by Phase 11's `learn` phase. Sized from
    // `TickPhase::ALL` rather than restated, so the next phase to be added
    // is a compile error here instead of an index panic at run time.
    samples: [Vec<f64>; TickPhase::ALL.len()],
    population_sum: u128,
    ticks: u64,
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

fn measure(label: &str, config: SimConfig) {
    let mut world = World::new(config).expect("world");
    let mut timer = PhaseTimer::default();
    for _ in 0..WARMUP_TICKS {
        world.step_with_observer(&mut timer);
    }
    for samples in &mut timer.samples {
        samples.clear();
    }
    let started = Instant::now();
    for _ in 0..MEASURED_TICKS {
        world.step_with_observer(&mut timer);
        timer.population_sum += world.population() as u128;
        timer.ticks += 1;
    }
    let wall = started.elapsed().as_secs_f64();
    let mean_population = (timer.population_sum / u128::from(timer.ticks.max(1))) as f64;

    let mut whole = 0.0;
    for (index, phase) in TickPhase::ALL.iter().enumerate() {
        let value = median(&mut timer.samples[index]);
        whole += value;
        println!(
            "PHASE8-BENCH phase label={label} phase={} p50_us={value:.2} \
             mean_population={mean_population:.1} us_per_1000_organisms={:.3}",
            phase.name(),
            value * 1_000.0 / mean_population.max(1.0),
        );
    }
    println!(
        "PHASE8-BENCH tick label={label} whole_tick_p50_us={whole:.2} \
         ticks_per_second={:.1} mean_population={mean_population:.1} final_population={}",
        MEASURED_TICKS as f64 / wall,
        world.population(),
    );
}

/// The campaign configuration, so the numbers are the ones that actually
/// applied to Phase 8's own campaign rather than a synthetic tier.
fn campaign_config() -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.max_entities = 40_000;
    config.cell_capacity_milli = 120_000;
    config.climate.enabled = true;
    config.climate.worldgen_version = sim_core::WorldgenVersion::V2;
    config
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase8-benchmarks.sh"]
fn demography_cost_by_mechanism() {
    measure("phase7-baseline", campaign_config());

    // Each mechanism alone, with the hazards off so the population stays
    // comparable to the baseline's and the figure is cost, not consequence.
    let mut allometry = campaign_config();
    allometry.physiology.enabled = true;
    allometry.physiology.thermoregulation_enabled = false;
    allometry.physiology.senescence_enabled = false;
    allometry.physiology.extrinsic_hazard_q16_per_s = 0;
    measure("allometry-only", allometry);

    let mut thermal = allometry;
    thermal.physiology.allometry_enabled = false;
    thermal.physiology.thermoregulation_enabled = true;
    measure("thermoregulation-only", thermal);

    let mut hazard = campaign_config();
    hazard.physiology.enabled = true;
    hazard.physiology.allometry_enabled = false;
    hazard.physiology.thermoregulation_enabled = false;
    hazard.physiology.senescence_enabled = true;
    hazard.physiology.extrinsic_hazard_q16_per_s = 0;
    measure("hazard-draws-only", hazard);

    // Everything on, at the campaign's own mortality regime. This one is
    // not cost-comparable -- its population is far lower -- and is the
    // number a campaign planner actually needs.
    let mut full = campaign_config();
    full.physiology.enabled = true;
    full.physiology.extrinsic_hazard_q16_per_s = 65;
    measure("campaign-condition-A", full);
}
