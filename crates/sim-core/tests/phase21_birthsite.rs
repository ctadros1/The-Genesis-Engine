//! Phase 21's record clauses (ADR-0036, C21.5): every admission carries
//! exactly one BirthSite record whose cell, maturity and masses equal the
//! world's own arrays at that tick, whose occupant count sees a same-tick
//! sibling, and which moves no checksum and survives a save round trip.

use sim_core::{EventKind, OriginMode, SimConfig, World};

const SEED: u64 = 0x0f21_b17e_0f21_b17e;
const ENERGY: i64 = 4_000;

fn scratch_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 24;
    config.cells_y = 24;
    config.initial_organisms = 0;
    config.max_entities = 400;
    config.origin.mode = OriginMode::Scratch;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.chemistry.enabled = true;
    config.chemistry.field_steps_per_tick = 2;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config.chemistry.mutation_q16 = 4_096;
    config.chemistry.production_milli_per_step = 20;
    config.chemistry.excretion_fraction_q16 = 32_768;
    config.chemistry.remains_fraction_q16 = 32_768;
    config.chemistry.consumption_fraction_q16 = 65_536;
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 25;
    config.transition.density_floor_milli = ENERGY;
    config.transition.persistence_checks = 2;
    config.transition.organism_energy_milli = ENERGY;
    config.transition.max_organisms_per_event = 4;
    config.validate().expect("validates");
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed");
}

#[test]
fn every_admission_carries_one_birth_site_record_equal_to_the_world_at_that_tick() {
    let config = scratch_config(SEED);
    let mut world = World::new(config).expect("world");
    let classes = sim_core::class_count(&config.chemistry);
    let mut admissions = 0_usize;
    let mut records = 0_usize;
    let mut same_tick_pairs_seen = 0_usize;
    for _ in 0..3_000 {
        world.step();
        let state = world.export_state();
        let chemistry = state.chemistry.as_ref().unwrap();
        let microbial = state.microbial.as_ref().unwrap();
        let mut admitted_this_tick: Vec<u64> = Vec::new();
        let mut sites: std::collections::BTreeMap<u64, (u32, u16)> = std::collections::BTreeMap::new();
        // A materialized organism's record shows the density it condensed
        // from: its own debit (and any later same-tick materialization in
        // the cell) lands after the record. Born organisms debit nothing.
        let mut materialized_energy: std::collections::BTreeMap<u64, i64> = std::collections::BTreeMap::new();
        let mut materialized_in_cell: std::collections::BTreeMap<u32, i64> = std::collections::BTreeMap::new();
        // Pass 1: the admissions (a BirthSite record is pushed before its
        // organism's Materialized record, so classify before checking).
        for event in world.events() {
            match event.kind {
                EventKind::Materialized { id, cell, energy_milli, .. } => {
                    admissions += 1;
                    admitted_this_tick.push(id);
                    materialized_energy.insert(id, energy_milli);
                    *materialized_in_cell.entry(cell).or_insert(0) += energy_milli;
                }
                EventKind::Birth { id, .. } | EventKind::PairedBirth { id, .. } => {
                    admissions += 1;
                    admitted_this_tick.push(id);
                }
                _ => {}
            }
        }
        // Pass 2: the records against the world's arrays.
        for event in world.events() {
            match event.kind {
                EventKind::BirthSite {
                    id,
                    cell,
                    occupants,
                    maturity_ticks,
                    substrate_milli,
                    microbial_milli,
                    biomass_milli,
                } => {
                    records += 1;
                    let cell = cell as usize;
                    // The record equals the world's arrays after this tick.
                    for slot in 0..sim_core::SUBSTRATE_COUNT {
                        assert_eq!(substrate_milli[slot], chemistry.concentrations[cell * sim_core::SUBSTRATE_COUNT + slot], "organism {id}: substrate {slot}");
                    }
                    let density: i64 = microbial.densities[cell * classes..(cell + 1) * classes].iter().sum();
                    match materialized_energy.get(&id) {
                        None => assert_eq!(microbial_milli, density, "organism {id}: microbial (born)"),
                        Some(&own) => {
                            let later = materialized_in_cell.get(&(cell as u32)).copied().unwrap_or(0);
                            assert!(microbial_milli >= density + own, "organism {id}: the record shows the density it condensed from ({microbial_milli} vs {density} + {own})");
                            assert!(microbial_milli <= density + later, "organism {id}: more than this tick's same-cell materializations lie between record and end ({microbial_milli} vs {density} + {later})");
                        }
                    }
                    assert_eq!(biomass_milli, state.biomass_milli[cell], "organism {id}: biomass");
                    if let Some(detail) = world.organism_detail(id) {
                        let phase2 = detail.phase2.expect("phase 2");
                        assert_eq!(u64::from(maturity_ticks), phase2.phenotype.maturity_ticks, "organism {id}: maturity");
                        let cell_fp = i64::from(config.cell_size_fp());
                        let actual = (i64::from(detail.y_fp) / cell_fp) as usize * config.cells_x as usize
                            + (i64::from(detail.x_fp) / cell_fp) as usize;
                        assert_eq!(cell, actual, "organism {id}: cell");
                    }
                    sites.insert(id, (cell as u32, occupants));
                }
                _ => {}
            }
        }
        assert_eq!(admitted_this_tick.len(), sites.len(), "one record per admission this tick");
        // Two organisms admitted into one cell in the same tick: the second
        // sees the first as an occupant (the count is incremented per
        // admission, not read from a stale index).
        let mut by_cell: std::collections::BTreeMap<u32, Vec<(u64, u16)>> = std::collections::BTreeMap::new();
        for (&id, &(cell, occupants)) in &sites {
            by_cell.entry(cell).or_default().push((id, occupants));
        }
        for group in by_cell.values_mut() {
            if group.len() >= 2 {
                group.sort();
                for pair in group.windows(2) {
                    assert!(pair[1].1 >= pair[0].1 + 1, "same-tick siblings: {:?}", pair);
                    same_tick_pairs_seen += 1;
                }
            }
        }
    }
    assert!(admissions > 100, "too few admissions: {admissions}");
    assert_eq!(records, admissions, "one record per admission");
    assert!(same_tick_pairs_seen > 0, "no same-tick same-cell pair to test the increment");
    world.check_invariants().expect("identities");
}

#[test]
fn the_birth_site_record_moves_no_checksum_and_survives_a_save_round_trip() {
    let config = scratch_config(SEED ^ 0x1);
    let mut a = World::new(config).expect("world");
    let mut b = World::new(config).expect("world");
    for _ in 0..1_500 {
        a.step();
        b.step();
        let _ = a.events().len();
    }
    assert_eq!(a.state_checksum(), b.state_checksum());
    let mut restored = World::from_state(a.export_state()).expect("restores");
    for tick in 1..=300 {
        a.step();
        restored.step();
        let ea: Vec<_> = a.events().iter().filter(|e| matches!(e.kind, EventKind::BirthSite { .. })).cloned().collect();
        let er: Vec<_> = restored.events().iter().filter(|e| matches!(e.kind, EventKind::BirthSite { .. })).cloned().collect();
        assert_eq!(ea, er, "tick {tick}: birth-site records differ after the round trip");
        assert_eq!(a.state_checksum(), restored.state_checksum(), "tick {tick}");
    }
}
