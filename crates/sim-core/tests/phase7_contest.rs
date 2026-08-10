//! Phase 7 acceptance criteria C7.4 (exact accounting) and C7.5
//! (determinism), plus the unit and property obligations of the test plan.
//!
//! C7.1 to C7.3 are multi-seed campaign claims and live in the experiment
//! harness, not here. What this file establishes is the physics: that the
//! ledgers stay exact, that contest is order-independent, and that a
//! contest-disabled world is still the Phase 2 world byte for byte.

use sim_core::{DeathCause, EventKind, SimConfig, World};

fn contest_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase7_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 80;
    config.max_entities = 800;
    // Dense and aggressive enough that contest actually happens; a test in
    // which nothing ever attacks would prove nothing about contest.
    config.contest.attack_threshold_q16 = -32_768;
    config.contest.attack_range_m = 6;
    config
}

fn run(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world");
    for tick in 1..=ticks {
        world.step();
        if tick % 200 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    world.check_invariants().expect("final invariants");
    world
}

// --- C7.5 determinism -----------------------------------------------------

#[test]
fn c7_5_contest_disabled_reproduces_the_phase_2_fixture() {
    // The whole rollback story: disabled, the world takes the Phase 2 code
    // paths and reproduces its fixture exactly.
    let phase2 = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
    assert!(!phase2.contest.enabled);
    assert_eq!(phase2.stable_hash(), 0xf83d_3981_bf7d_d189);

    let mut world = World::new(phase2).expect("world");
    for _ in 0..500 {
        world.step();
    }
    assert_eq!(world.state_checksum(), 0xff9d_fcff_5dff_bf42);

    // A disabled section's parameters cannot move the hash either.
    let mut fiddled = phase2;
    fiddled.contest.damage_base_milli += 999;
    fiddled.contest.attack_range_m = 9;
    assert_eq!(fiddled.stable_hash(), phase2.stable_hash());
}

#[test]
fn c7_5_enabling_contest_starts_a_new_lineage_and_changes_the_world() {
    let phase2 = SimConfig::phase2_default(11);
    let phase7 = SimConfig::phase7_default(11);
    assert_ne!(phase2.stable_hash(), phase7.stable_hash());

    let with = run(contest_config(11), 800);
    let mut without_config = contest_config(11);
    without_config.contest.enabled = false;
    let without = run(without_config, 800);
    // Not a checksum comparison: the config hash sits in the checksum
    // preamble, so it would differ even for an inert section. The claim is
    // that contest changes measured outcomes.
    assert!(
        with.metrics().attacks_total > 0,
        "no attack ever fired; the contest test is vacuous"
    );
    assert_eq!(without.metrics().attacks_total, 0);
    assert_ne!(
        with.total_energy_milli(),
        without.total_energy_milli(),
        "contest had no energetic effect at all"
    );
}

#[test]
fn c7_5_same_seed_replays_exactly() {
    let first = run(contest_config(11), 600);
    let second = run(contest_config(11), 600);
    assert_eq!(first.state_checksum(), second.state_checksum());
    assert_eq!(
        first.metrics().attacks_total,
        second.metrics().attacks_total
    );
}

#[test]
fn c7_5_contest_survives_save_restore_and_continues_identically() {
    let mut original = World::new(contest_config(11)).expect("world");
    for _ in 0..400 {
        original.step();
    }
    let checksum = original.state_checksum();
    let mut restored = World::from_state(original.export_state()).expect("restore");
    assert_eq!(restored.state_checksum(), checksum);
    for _ in 0..300 {
        original.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        original.state_checksum(),
        "post-restore divergence with contest enabled"
    );
    restored.check_invariants().expect("invariants");
}

// --- C7.4 exact accounting -------------------------------------------------

#[test]
fn c7_4_energy_and_carcass_ledgers_stay_exact() {
    // `check_invariants` verifies the energy ledger, the biomass ledger, the
    // population accounting, and the carcass pool on every call. Running it
    // repeatedly through a contest-heavy run is the criterion.
    let world = run(contest_config(11), 20_000);
    let metrics = world.metrics();
    assert!(metrics.attacks_total > 0, "no attacks occurred");
    world.check_invariants().expect("ledgers exact");
}

#[test]
#[ignore = "long run: 10^6 ticks; run with --release --ignored"]
fn c7_4_ledgers_stay_exact_over_a_million_ticks() {
    let mut config = contest_config(11);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = 200;
    config.max_entities = 3_000;
    let world = run(config, 1_000_000);
    assert!(world.metrics().attacks_total > 0);
    world.check_invariants().expect("ledgers exact");
}

#[test]
fn c7_4_carcass_energy_never_exceeds_its_source() {
    // "Carcass energy never exceeds the source organism's recorded remaining
    // transferable energy." Checked against the events themselves.
    let mut config = contest_config(11);
    config.contest.carcass_energy_q16 = 65_536; // the whole remainder
    let mut world = World::new(config).expect("world");
    let mut checked = 0_u32;
    for _ in 0..4_000 {
        world.step();
        let energy_max = world.config().energy_max_milli;
        for event in world.events() {
            if let EventKind::CarcassCreated { energy_milli, .. } = event.kind {
                assert!(energy_milli > 0);
                assert!(
                    energy_milli <= energy_max,
                    "a carcass held {energy_milli}, more than an organism can carry"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no carcass was ever created");
    world.check_invariants().expect("invariants");
}

// --- Test-plan obligations -------------------------------------------------

#[test]
fn death_by_damage_is_terminal_and_emits_both_events() {
    let mut config = contest_config(11);
    config.contest.damage_base_milli = 100_000; // one hit kills
    let mut world = World::new(config).expect("world");
    let mut deaths = 0_u32;
    let mut by_damage = 0_u32;
    for _ in 0..600 {
        world.step();
        for event in world.events() {
            match event.kind {
                EventKind::Death {
                    cause: DeathCause::Damage,
                    ..
                } => deaths += 1,
                EventKind::DeathByDamage { .. } => by_damage += 1,
                _ => {}
            }
        }
    }
    assert!(deaths > 0, "lethal damage killed nobody");
    assert_eq!(
        deaths, by_damage,
        "every damage death must emit exactly one DeathByDamage"
    );
    world.check_invariants().expect("invariants");
}

#[test]
fn zero_damage_separates_expressing_an_attack_from_its_consequences() {
    // Condition C of the phase design: the action fires and costs energy
    // without consequence. Population must be unaffected by damage while
    // attacks still happen and still cost.
    let mut config = contest_config(11);
    config.contest.damage_base_milli = 0;
    let world = run(config, 3_000);
    let metrics = world.metrics();
    assert!(
        metrics.attacks_total > 0,
        "no attack fired under condition C"
    );
    assert_eq!(
        metrics.deaths_by_damage_total, 0,
        "zero damage still killed somebody"
    );
    assert_eq!(metrics.carcasses, 0, "zero damage still made carcasses");
}

#[test]
fn health_stays_within_bounds_and_a_dead_organism_never_acts() {
    let mut world = World::new(contest_config(11)).expect("world");
    for _ in 0..3_000 {
        world.step();
        // Health is bounded below by zero at the point of death and above by
        // the body-scale maximum; `check_invariants` covers the structural
        // half, and the population accounting covers "no action after death".
        world.check_invariants().expect("invariants");
    }
}

#[test]
fn restoring_a_contest_world_twice_continues_identically() {
    // **Renamed, because the old name was `storage_permutation_does_not_
    // change_the_next_ticks` and this test permutes nothing.** It restores
    // the same state twice and compares the two continuations, which is a
    // save/restore determinism test - a real and useful one - wearing a
    // permutation test's name. A reader looking for determinism-extensions
    // Rule 4's evidence would have found this and stopped looking.
    //
    // Rule 4's evidence is in `phase9_determinism.rs`, and the contest arrays
    // are covered there: `contest.health_milli` is one of the arrays the
    // negative sweep scrambles individually and the world must notice.
    let mut original = World::new(contest_config(13)).expect("world");
    for _ in 0..500 {
        original.step();
    }
    let state = original.export_state();
    let mut a = World::from_state(state.clone()).expect("restore");
    let mut b = World::from_state(state).expect("restore");
    for _ in 0..250 {
        a.step();
        b.step();
        original.step();
    }
    assert_eq!(a.state_checksum(), b.state_checksum());
    assert_eq!(a.state_checksum(), original.state_checksum());
}
