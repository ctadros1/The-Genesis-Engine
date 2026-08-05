//! Phase 7 benchmark: the per-phase cost of contest at both tiers, and the
//! carcass entity-count effect measured separately from the per-organism
//! cost.
//!
//! `#[ignore]`; run in release by `scripts/run-phase7-benchmarks.sh`.
//!
//! The phase plan predicts contest lands on two phases: `Apply` gains intent
//! resolution against neighbours, and `Sense` gains threat estimation.
//! Everything else should be unmoved, and a measurement that shows an
//! unrelated phase moving is a finding rather than noise to be averaged away.
//!
//! **Contest changes the population trajectory**, so a naive A/B at a fixed
//! tick count compares a world of 400 organisms against one of 90 and
//! attributes the difference to contest. Per-phase microseconds are
//! therefore reported alongside the mean population that produced them, and
//! the per-organism figure is what the comparison rests on. No target is
//! declared.

use sim_core::{SimConfig, TickObserver, TickPhase, World};
use std::time::Instant;

const SEED: u64 = 7;
const WARMUP_TICKS: u64 = 500;
const MEASURED_TICKS: u64 = 4_000;

#[derive(Default)]
struct PhaseTimer {
    started: Option<Instant>,
    current: Option<TickPhase>,
    samples: [Vec<f64>; 8],
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

impl PhaseTimer {
    fn reset(&mut self) {
        for samples in &mut self.samples {
            samples.clear();
        }
        self.population_sum = 0;
        self.ticks = 0;
    }

    fn percentiles(samples: &mut [f64]) -> (f64, f64) {
        if samples.is_empty() {
            return (0.0, 0.0);
        }
        samples.sort_by(f64::total_cmp);
        let pick = |f: f64| samples[((samples.len() - 1) as f64 * f).ceil() as usize];
        (pick(0.5), pick(0.95))
    }
}

fn measure(label: &str, tier: u32, config: SimConfig) {
    let mut world = World::new(config).expect("world");
    let mut timer = PhaseTimer::default();
    for _ in 0..WARMUP_TICKS {
        world.step_with_observer(&mut timer);
    }
    timer.reset();

    let started = Instant::now();
    for _ in 0..MEASURED_TICKS {
        world.step_with_observer(&mut timer);
        timer.population_sum += world.population() as u128;
        timer.ticks += 1;
    }
    let wall = started.elapsed().as_secs_f64();
    let mean_population = (timer.population_sum / u128::from(timer.ticks.max(1))) as f64;
    let metrics = world.metrics();

    let mut whole_tick_us = 0.0;
    for (index, phase) in TickPhase::ALL.iter().enumerate() {
        let (p50, p95) = PhaseTimer::percentiles(&mut timer.samples[index]);
        whole_tick_us += p50;
        println!(
            "PHASE7-BENCH phase label={label} tier={tier} phase={} \
             p50_us={p50:.2} p95_us={p95:.2} mean_population={mean_population:.1} \
             us_per_1000_organisms={:.3}",
            phase.name(),
            p50 * 1_000.0 / mean_population.max(1.0),
        );
    }
    println!(
        "PHASE7-BENCH tick label={label} tier={tier} whole_tick_p50_us={whole_tick_us:.2} \
         ticks_per_second={:.1} mean_population={mean_population:.1} final_population={} \
         attacks={} carcasses={} deaths_by_damage={}",
        MEASURED_TICKS as f64 / wall,
        metrics.population,
        metrics.attacks_total,
        metrics.carcasses,
        metrics.deaths_by_damage_total,
    );
}

fn tier_config(tier: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.initial_organisms = tier;
    config.max_entities = tier * 10;
    config
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase7-benchmarks.sh"]
fn contest_per_phase_cost_at_both_tiers() {
    for tier in [500_u32, 2_000] {
        measure("contest-disabled", tier, tier_config(tier));

        let mut enabled = tier_config(tier);
        enabled.contest.enabled = true;
        measure("contest-enabled", tier, enabled);

        // Attacks fire but do nothing, so the work is done at close to the
        // disabled condition's population. This is the comparison that
        // separates contest's *cost* from contest's *demographic effect*.
        let mut costed = tier_config(tier);
        costed.contest.enabled = true;
        costed.contest.damage_base_milli = 0;
        measure("contest-zero-damage", tier, costed);
    }
}

#[test]
#[ignore = "timed benchmark; run via scripts/run-phase7-benchmarks.sh"]
fn carcass_entity_count_effect() {
    // The plan asks for the carcass effect separately from the per-organism
    // cost, because carcasses are entities the tick has to scan and their
    // count is governed by decay rather than by population. Slowing decay
    // to zero lets the table fill; the contrast against fast decay isolates
    // what a carcass costs.
    for (label, decay_q16, cap) in [
        ("carcass-decay-fast", 32_768_u32, 4_096_u32),
        ("carcass-decay-default", 3_277, 4_096),
        ("carcass-decay-none", 0, 4_096),
        ("carcass-cap-small", 0, 64),
    ] {
        let mut config = tier_config(500);
        config.contest.enabled = true;
        config.contest.carcass_decay_q16_per_s = decay_q16;
        config.contest.max_carcasses = cap;
        measure(label, 500, config);
    }
}
