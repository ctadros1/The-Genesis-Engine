//! Deterministic replay tests (same build, same process).
//! Clean-process determinism is covered by `scripts/verify-phase1-determinism.sh`
//! and the CLI integration tests, which spawn separate processes.

use sim_core::{SimConfig, World};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn test_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = 200;
    config.max_entities = 2_000;
    config
}

#[test]
fn same_seed_and_config_repeat_exactly() {
    let mut first = World::new(test_config()).unwrap();
    let mut second = World::new(test_config()).unwrap();
    for tick in 0..500 {
        first.step();
        second.step();
        if tick % 100 == 0 {
            assert_eq!(
                first.state_checksum(),
                second.state_checksum(),
                "diverged at tick {tick}"
            );
        }
    }
    assert_eq!(first.state_checksum(), second.state_checksum());
    assert_eq!(first.population(), second.population());
    first.check_invariants().unwrap();
}

#[test]
fn different_seed_diverges() {
    let mut config = test_config();
    let mut first = World::new(config).unwrap();
    config.world_seed = SEED + 1;
    let mut second = World::new(config).unwrap();
    for _ in 0..200 {
        first.step();
        second.step();
    }
    assert_ne!(first.state_checksum(), second.state_checksum());
}

#[test]
fn config_change_changes_hash_and_trajectory() {
    let base = test_config();
    let mut changed = base;
    changed.growth_rate_q16_per_s += 1;
    assert_ne!(base.stable_hash(), changed.stable_hash());

    let mut first = World::new(base).unwrap();
    let mut second = World::new(changed).unwrap();
    for _ in 0..300 {
        first.step();
        second.step();
    }
    // Different config hash feeds the state checksum, so these must differ
    // even if trajectories happened to coincide.
    assert_ne!(first.state_checksum(), second.state_checksum());
}

#[test]
fn pause_and_resume_do_not_perturb_the_trajectory() {
    let mut uninterrupted = World::new(test_config()).unwrap();
    let mut interrupted = World::new(test_config()).unwrap();

    for _ in 0..100 {
        uninterrupted.step();
        interrupted.step();
    }
    interrupted.set_paused(true);
    for _ in 0..50 {
        interrupted.step(); // must be no-ops
    }
    interrupted.set_paused(false);
    for _ in 0..100 {
        uninterrupted.step();
        interrupted.step();
    }
    assert_eq!(uninterrupted.tick_number(), interrupted.tick_number());
    assert_eq!(uninterrupted.state_checksum(), interrupted.state_checksum());
}

#[test]
fn checksum_covers_paused_flag() {
    let mut world = World::new(test_config()).unwrap();
    let unpaused = world.state_checksum();
    world.set_paused(true);
    assert_ne!(world.state_checksum(), unpaused);
    world.set_paused(false);
    assert_eq!(world.state_checksum(), unpaused);
}
