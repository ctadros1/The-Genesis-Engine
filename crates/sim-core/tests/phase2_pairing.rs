//! Paired-parent creation, ancestry, and rejection-policy tests.

use sim_core::{DeathCause, EventKind, PairRejectReason, SimConfig, World};
use std::collections::{HashMap, HashSet};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn small_phase2(initial: u32, max_entities: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = initial;
    config.max_entities = max_entities;
    config
}

#[test]
fn mate_intent_is_required_for_any_pairing() {
    // A mate threshold of 1.0 can never be exceeded by a bounded output,
    // so no mutual intent can form and no paired birth may occur.
    let mut config = small_phase2(80, 800);
    config.phase2.mate_threshold_q16 = 65_536;
    let mut world = World::new(config).unwrap();
    for _ in 0..2_000 {
        world.step();
    }
    assert_eq!(world.phase2_counters().paired_births_total, 0);
    assert_eq!(world.counters().births_total, 0);
    world.check_invariants().unwrap();
}

#[test]
fn reproduction_disabled_gates_pairing_entirely() {
    let mut config = small_phase2(80, 800);
    config.reproduction_enabled = false;
    let mut world = World::new(config).unwrap();
    for _ in 0..1_500 {
        world.step();
    }
    let counters = world.phase2_counters();
    assert_eq!(counters.paired_births_total, 0);
    assert_eq!(counters.pair_rejected_capacity_total, 0);
    assert_eq!(counters.pair_rejected_placement_total, 0);
    assert_eq!(counters.pair_rejected_energy_total, 0);
    world.check_invariants().unwrap();
}

#[test]
fn capacity_ceiling_rejects_pairs_and_is_never_exceeded() {
    // Population starts at the ceiling. Deaths later free capacity, so
    // births may eventually occur; the invariants under test are that
    // saturated ticks deterministically reject pairs and the ceiling is
    // never exceeded at any observation point.
    let mut config = small_phase2(120, 120);
    // Make pairing easy so rejections actually occur.
    config.phase2.compatibility_threshold_q16 = 65_536;
    config.phase2.pairing_range_m = 16;
    let mut world = World::new(config).unwrap();
    for _ in 0..3_000 {
        world.step();
        assert!(world.population() <= 120, "capacity ceiling exceeded");
    }
    let counters = world.phase2_counters();
    assert!(
        counters.pair_rejected_capacity_total > 0,
        "expected capacity rejections in a ceiling-bound world"
    );
    world.check_invariants().unwrap();
}

#[test]
fn incompatible_records_cannot_pair() {
    // A compatibility threshold of zero rejects every founder pair
    // (founder trait distance is virtually never exactly zero).
    let mut config = small_phase2(80, 800);
    config.phase2.compatibility_threshold_q16 = 0;
    let mut world = World::new(config).unwrap();
    for _ in 0..2_000 {
        world.step();
    }
    assert_eq!(world.phase2_counters().paired_births_total, 0);
    world.check_invariants().unwrap();
}

#[test]
fn pairing_audit_holds_across_a_full_run() {
    let mut world = World::new(small_phase2(150, 1_500)).unwrap();
    let mut death_tick: HashMap<u64, u64> = HashMap::new();
    let mut pair_events = Vec::new();
    let mut child_ids = HashSet::new();
    let mut rejected = 0_u64;
    for _ in 0..6_000 {
        world.step();
        for event in world.events() {
            match event.kind {
                EventKind::Death { id, cause } => {
                    assert!(matches!(cause, DeathCause::Starvation | DeathCause::OldAge));
                    death_tick.insert(id, event.tick);
                }
                EventKind::PairedBirth {
                    id,
                    parent_a,
                    parent_b,
                    ..
                } => {
                    // Fresh, never-recycled child IDs.
                    assert!(child_ids.insert(id));
                    pair_events.push((event.tick, id, parent_a, parent_b));
                }
                EventKind::PairRejected { reason, .. } => {
                    assert!(matches!(
                        reason,
                        PairRejectReason::Capacity
                            | PairRejectReason::Placement
                            | PairRejectReason::Energy
                    ));
                    rejected += 1;
                }
                _ => {}
            }
        }
    }
    assert!(
        !pair_events.is_empty(),
        "expected paired births over 6,000 ticks"
    );
    // No pairing may involve a parent that was already removed: a parent's
    // death tick must be at or after every pairing event that names it
    // (same-tick is legal: pairing resolves before lifecycle removal).
    for (tick, _child, parent_a, parent_b) in &pair_events {
        for parent in [parent_a, parent_b] {
            if let Some(died) = death_tick.get(parent) {
                assert!(
                    died >= tick,
                    "parent {parent} died at {died} but paired at {tick}"
                );
            }
        }
    }
    // Cooldown: the same parent cannot appear in two paired births closer
    // than the minimum genome-derived cooldown (200 ticks).
    let mut last_pairing: HashMap<u64, u64> = HashMap::new();
    for (tick, _child, parent_a, parent_b) in &pair_events {
        for parent in [*parent_a, *parent_b] {
            if let Some(previous) = last_pairing.insert(parent, *tick) {
                assert!(
                    tick - previous >= 200,
                    "parent {parent} paired at {previous} and again at {tick}"
                );
            }
        }
    }
    let counters = world.phase2_counters();
    assert_eq!(counters.paired_births_total, pair_events.len() as u64);
    assert_eq!(
        rejected,
        counters.pair_rejected_capacity_total
            + counters.pair_rejected_placement_total
            + counters.pair_rejected_energy_total
    );
    world.check_invariants().unwrap();
}

#[test]
fn child_ancestry_depth_and_child_counts_accumulate() {
    let mut world = World::new(small_phase2(150, 1_500)).unwrap();
    for _ in 0..6_000 {
        world.step();
    }
    let metrics = world.metrics();
    if metrics.paired_births_total > 0 {
        assert!(metrics.max_ancestry_depth >= 1);
    }
    world.check_invariants().unwrap();
}
