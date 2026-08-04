//! Phase 2 deterministic replay and compatibility tests.

use sim_core::{EventKind, SimConfig, World};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn phase2_config() -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    config
}

#[test]
fn identical_seed_and_config_replay_exactly_including_events() {
    let mut first = World::new(phase2_config()).unwrap();
    let mut second = World::new(phase2_config()).unwrap();
    for tick in 0..800 {
        first.step();
        second.step();
        // Controller, pairing, variation, event, ancestry, and state
        // outcomes must all match; events capture the audit trail.
        assert_eq!(first.events(), second.events(), "events diverged at {tick}");
        if tick % 200 == 0 {
            assert_eq!(
                first.state_checksum(),
                second.state_checksum(),
                "state diverged at {tick}"
            );
        }
    }
    assert_eq!(first.state_checksum(), second.state_checksum());
    assert_eq!(
        first.phase2_counters().paired_births_total,
        second.phase2_counters().paired_births_total
    );
    first.check_invariants().unwrap();
}

#[test]
fn different_seed_diverges() {
    let mut config = phase2_config();
    let mut first = World::new(config).unwrap();
    config.world_seed = SEED + 1;
    let mut second = World::new(config).unwrap();
    for _ in 0..300 {
        first.step();
        second.step();
    }
    assert_ne!(first.state_checksum(), second.state_checksum());
}

#[test]
fn phase2_policy_field_changes_hash_and_lineage() {
    let base = phase2_config();
    let mut changed = base;
    changed.phase2.variation_probability_q16 += 1;
    assert_ne!(base.stable_hash(), changed.stable_hash());

    // Disabled Phase 2 sections do not contribute to the hash: a config
    // with different phase2 parameters but enabled=false hashes exactly
    // like the Phase 1 config (Phase 1 fixture preservation).
    let mut disabled_a = SimConfig::phase1_default(SEED);
    let mut disabled_b = SimConfig::phase1_default(SEED);
    disabled_a.phase2.variation_probability_q16 = 1;
    disabled_b.phase2.variation_probability_q16 = 60_000;
    assert!(!disabled_a.phase2.enabled && !disabled_b.phase2.enabled);
    assert_eq!(disabled_a.stable_hash(), disabled_b.stable_hash());
}

#[test]
fn phase2_disabled_world_matches_phase1_world_exactly() {
    let mut phase1_config = SimConfig::phase1_default(SEED);
    phase1_config.cells_x = 96;
    phase1_config.cells_y = 96;
    phase1_config.initial_organisms = 120;
    phase1_config.max_entities = 1_200;
    let mut disabled = phase1_config;
    disabled.phase2.cluster_sample_max = 7; // inert while disabled

    let mut first = World::new(phase1_config).unwrap();
    let mut second = World::new(disabled).unwrap();
    for _ in 0..500 {
        first.step();
        second.step();
    }
    assert_eq!(first.state_checksum(), second.state_checksum());
    assert!(!first.phase2_enabled());
}

#[test]
fn event_reading_and_analysis_do_not_change_state() {
    let mut reader = World::new(phase2_config()).unwrap();
    let mut non_reader = World::new(phase2_config()).unwrap();
    for _ in 0..300 {
        reader.step();
        let _ = reader.events();
        let _ = sim_core::analyze(&reader); // offline job on live state
        non_reader.step();
    }
    assert_eq!(reader.state_checksum(), non_reader.state_checksum());
}

#[test]
fn pause_and_resume_remain_trajectory_neutral() {
    let mut uninterrupted = World::new(phase2_config()).unwrap();
    let mut interrupted = World::new(phase2_config()).unwrap();
    for _ in 0..150 {
        uninterrupted.step();
        interrupted.step();
    }
    interrupted.set_paused(true);
    let paused_checksum = interrupted.state_checksum();
    for _ in 0..40 {
        interrupted.step();
    }
    assert_eq!(interrupted.state_checksum(), paused_checksum);
    interrupted.set_paused(false);
    for _ in 0..150 {
        uninterrupted.step();
        interrupted.step();
    }
    assert_eq!(uninterrupted.state_checksum(), interrupted.state_checksum());
}

#[test]
fn paired_birth_events_are_ordered_and_audited() {
    let mut world = World::new(phase2_config()).unwrap();
    let mut births = Vec::new();
    for _ in 0..3_000 {
        world.step();
        let mut last_sequence: Option<u64> = None;
        for event in world.events() {
            if let EventKind::PairedBirth {
                id,
                parent_a,
                parent_b,
                genome_hash,
                invest_a_milli,
                invest_b_milli,
                ..
            } = event.kind
            {
                // Stable ordering: birth IDs increase within a tick.
                if let Some(previous) = last_sequence {
                    assert!(id > previous);
                }
                last_sequence = Some(id);
                assert_ne!(parent_a, parent_b);
                assert!(parent_a < id && parent_b < id);
                assert!(invest_a_milli >= invest_b_milli);
                assert!(invest_a_milli - invest_b_milli <= 1);
                assert_ne!(genome_hash, 0);
                births.push((id, parent_a, parent_b, genome_hash));
            }
        }
        if births.len() >= 5 {
            break;
        }
    }
    assert!(
        !births.is_empty(),
        "expected paired births in the test horizon"
    );
    // Live ancestry matches the event audit trail for surviving children.
    for (id, parent_a, parent_b, genome_hash) in births {
        if let Some((parents, depth, _children, birth_tick, hash)) = world.ancestry_of(id) {
            assert_eq!(parents, [parent_a, parent_b]);
            assert_eq!(hash, genome_hash);
            assert!(depth >= 1);
            assert!(birth_tick > 0);
        }
    }
    world.check_invariants().unwrap();
}
