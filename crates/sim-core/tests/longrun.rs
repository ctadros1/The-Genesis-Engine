//! Long-run stability tests: invariants must hold and state must stay
//! bounded across many ticks.
//!
//! The 24-hour-equivalent test (864,000 ticks at dt = 100 ms) is `#[ignore]`
//! because it takes minutes; run it explicitly with:
//! `cargo test --release -p sim-core --test longrun -- --ignored`

use sim_core::{SimConfig, World};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

#[test]
fn ten_thousand_ticks_hold_invariants() {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    let mut world = World::new(config).unwrap();
    for tick in 1..=10_000_u64 {
        world.step();
        if tick % 500 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    // The world must remain observable whether or not the population
    // survived; either way the accounting has to balance.
    let metrics = world.metrics();
    assert_eq!(
        metrics.population,
        u64::from(world.config().initial_organisms) + metrics.births_total
            - metrics.deaths_starvation_total
            - metrics.deaths_old_age_total
    );
}

#[test]
fn extinct_world_stays_valid_and_pausable() {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 30;
    config.intake_rate_milli_per_s = 0; // nothing assimilates
    config.initial_energy_milli = 100;
    config.reproduction_enabled = false;
    let mut world = World::new(config).unwrap();
    for _ in 0..50 {
        world.step();
    }
    assert!(world.is_extinct());
    assert_eq!(world.population(), 0);
    world.check_invariants().unwrap();
    // Extinction is latched and emitted exactly once.
    let extinctions = world.counters();
    assert_eq!(
        extinctions.deaths_starvation_total,
        u64::from(world.config().initial_organisms)
    );
    // Still steppable and pausable.
    world.step();
    world.set_paused(true);
    world.step();
    world.check_invariants().unwrap();
}

/// 24-hour-equivalent stability run on the documented 500-organism default
/// world: 864,000 ticks at dt = 100 ms. Checks invariants periodically and
/// verifies bounded population and event buffers throughout.
#[test]
#[ignore = "multi-minute release-mode long-run; run explicitly"]
fn long_run_24h_equivalent_holds_invariants() {
    let config = SimConfig::phase1_default(SEED);
    let mut world = World::new(config).unwrap();
    let mut max_population = world.population();
    for tick in 1..=864_000_u64 {
        world.step();
        max_population = max_population.max(world.population());
        assert!(world.events().len() <= 4_096, "event buffer exceeded bound");
        if tick % 10_000 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    world.check_invariants().unwrap();
    assert!(max_population <= world.config().max_entities as usize);
    // Record trajectory facts in test output for the completion report.
    let metrics = world.metrics();
    eprintln!(
        "24h-equivalent: final population {} births {} starvation {} old_age {} capacity_rejections {} max_population {} state_checksum 0x{:016x}",
        metrics.population,
        metrics.births_total,
        metrics.deaths_starvation_total,
        metrics.deaths_old_age_total,
        metrics.capacity_rejections_total,
        max_population,
        world.state_checksum()
    );
}
