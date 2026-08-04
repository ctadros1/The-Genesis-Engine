//! Multi-generation long-run stability for Phase 2.
//!
//! Assertions are state-health checks (bounded population and memory,
//! valid records, exact accounting, finite controller state), never
//! scripted ecological outcomes. Diagnostics are reported for the record.

use sim_core::{SimConfig, World, analyze};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

#[test]
fn multi_generation_run_stays_bounded_and_valid() {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    let mut world = World::new(config).unwrap();
    for tick in 1..=8_000_u64 {
        world.step();
        assert!(world.events().len() <= 4_096);
        if tick % 500 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    let metrics = world.metrics();
    assert!(metrics.population <= u64::from(world.config().max_entities));
    assert_eq!(metrics.controller_faults_total, 0);
    // Either the population persists or the world reports a valid empty
    // outcome; both are acceptable long-run results.
    if metrics.population == 0 {
        assert!(metrics.extinct);
    }
    eprintln!(
        "phase2 8k ticks: population {} paired_births {} depth {} diversity {:?}",
        metrics.population,
        metrics.paired_births_total,
        metrics.max_ancestry_depth,
        analyze(&world).map(|report| report.mean_pairwise_distance)
    );
}

/// Release-mode multi-generation scenario on the default 500-organism
/// Phase 2 world: 200,000 ticks (5.5 simulated hours). Run explicitly:
/// `cargo test --release -p sim-core --test phase2_longrun -- --ignored`
#[test]
#[ignore = "multi-minute release-mode long-run; run explicitly"]
fn long_multi_generation_run_holds_invariants() {
    let config = SimConfig::phase2_default(SEED);
    let mut world = World::new(config).unwrap();
    let mut max_population = world.population();
    for tick in 1..=200_000_u64 {
        world.step();
        max_population = max_population.max(world.population());
        assert!(world.events().len() <= 4_096);
        if tick % 10_000 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    world.check_invariants().unwrap();
    let metrics = world.metrics();
    assert!(max_population <= world.config().max_entities as usize);
    assert_eq!(metrics.controller_faults_total, 0);
    let report = analyze(&world);
    eprintln!(
        "phase2 200k ticks: population {} (max {max_population}) paired_births {} rejections c/p/e {}/{}/{} depth {} old_age {} starvation {} diversity {:?} clusters {:?} checksum 0x{:016x}",
        metrics.population,
        metrics.paired_births_total,
        metrics.pair_rejected_capacity_total,
        metrics.pair_rejected_placement_total,
        metrics.pair_rejected_energy_total,
        metrics.max_ancestry_depth,
        metrics.deaths_old_age_total,
        metrics.deaths_starvation_total,
        report.as_ref().map(|r| r.mean_pairwise_distance),
        report.as_ref().map(|r| r.cluster_count),
        world.state_checksum()
    );
}
