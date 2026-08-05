//! Phase 6 benchmark: the environment phase with each climate field enabled
//! independently, reclassification cost, and founder generation at tick 0.
//!
//! `#[ignore]`; run in release by `scripts/run-phase6-benchmarks.sh`.
//!
//! The Phase 1 record identified the environment phase as the dominant fixed
//! cost — about 200 microseconds for the 65,536-cell logistic regrowth scan —
//! so Phase 6 lands directly on the known hot path and the measurement is
//! the point rather than a formality. No target is declared here.

use sim_core::{OriginMode, SimConfig, TickObserver, TickPhase, World, WorldgenVersion};
use std::time::Instant;

const SEED: u64 = 7;

#[derive(Default)]
struct PhaseTimer {
    started: Option<Instant>,
    environment_us: Vec<f64>,
}

impl TickObserver for PhaseTimer {
    fn phase_started(&mut self, phase: TickPhase) {
        if phase == TickPhase::Environment {
            self.started = Some(Instant::now());
        }
    }
    fn phase_finished(&mut self, phase: TickPhase) {
        if phase == TickPhase::Environment
            && let Some(started) = self.started.take()
        {
            self.environment_us
                .push(started.elapsed().as_secs_f64() * 1_000_000.0);
        }
    }
}

fn percentiles(samples: &mut [f64]) -> (f64, f64) {
    samples.sort_by(f64::total_cmp);
    let pick = |f: f64| samples[((samples.len() - 1) as f64 * f).ceil() as usize];
    (pick(0.5), pick(0.95))
}

fn measure(label: &str, config: SimConfig, ticks: u64) {
    let mut world = World::new(config).expect("world");
    let mut timer = PhaseTimer::default();
    for _ in 0..500 {
        world.step_with_observer(&mut timer);
    }
    timer.environment_us.clear();
    let started = Instant::now();
    for _ in 0..ticks {
        world.step_with_observer(&mut timer);
    }
    let wall = started.elapsed().as_secs_f64();
    let (p50, p95) = percentiles(&mut timer.environment_us);
    println!(
        "PHASE6-BENCH environment label={label} cells={} ticks={ticks} \
         environment_p50_us={p50:.2} environment_p95_us={p95:.2} \
         ticks_per_second={:.1}",
        config.cells_x * config.cells_y,
        ticks as f64 / wall
    );
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase6-benchmarks.sh"]
fn environment_phase_cost_by_field() {
    let base = |seed: u64| {
        let mut config = SimConfig::phase2_default(seed);
        config.cells_x = 256;
        config.cells_y = 256;
        config.initial_organisms = 500;
        config.max_entities = 5_000;
        config
    };

    // Baseline: the Phase 1/2 environment phase, logistic regrowth only.
    measure("climate-off", base(SEED), 3_000);

    // Climate on, reclassification effectively never, so this isolates the
    // per-tick moisture exchange.
    let mut moisture_only = base(SEED);
    moisture_only.climate.enabled = true;
    moisture_only.climate.worldgen_version = WorldgenVersion::V2;
    moisture_only.climate.reclassify_interval_ticks = 1_000_000;
    measure("moisture-only", moisture_only, 3_000);

    // Default cadence.
    let mut default_cadence = moisture_only;
    default_cadence.climate.reclassify_interval_ticks = 100;
    measure("moisture-plus-reclassify-100", default_cadence, 3_000);

    // Reclassify every tick: the cost the cadence exists to avoid.
    let mut every_tick = moisture_only;
    every_tick.climate.reclassify_interval_ticks = 1;
    measure("reclassify-every-tick", every_tick, 1_000);
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase6-benchmarks.sh"]
fn founder_generation_cost_at_tick_zero() {
    // One-time cost by origin mode. Measured separately from the tick loop
    // because it is paid once and would otherwise be invisible.
    let mut random = SimConfig::phase2_default(SEED);
    random.cells_x = 256;
    random.cells_y = 256;
    random.initial_organisms = 500;
    random.max_entities = 5_000;

    let mut demes = random;
    demes.origin.deme_count = 4;
    demes.origin.deme_radius_m = 256;
    demes.origin.deme_min_separation_m = 192;

    let mut seeded = random;
    seeded.climate.enabled = true;
    seeded.climate.worldgen_version = WorldgenVersion::V2;
    seeded.origin.mode = OriginMode::Seeded;
    seeded.origin.archetype_count = 2;
    seeded.origin.archetypes[0].id = 1;
    seeded.origin.archetypes[0].biome_affinity = sim_core::all_biomes_mask();
    seeded.origin.archetypes[1].id = 2;
    seeded.origin.archetypes[1].biome_affinity = sim_core::all_biomes_mask();

    for (label, config) in [
        ("random-one-deme", random),
        ("random-four-demes", demes),
        ("seeded-two-archetypes", seeded),
    ] {
        let mut samples = Vec::new();
        for _ in 0..10 {
            let started = Instant::now();
            let world = World::new(config);
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert!(world.is_ok(), "{label} failed to generate");
        }
        let (p50, p95) = percentiles(&mut samples);
        println!(
            "PHASE6-BENCH founder-generation label={label} organisms={} \
             world_new_p50_ms={p50:.3} world_new_p95_ms={p95:.3}",
            config.initial_organisms
        );
    }
}
