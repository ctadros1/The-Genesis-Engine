//! Phase 16 benchmarks: what the field-to-individual transition
//! (ADR-0032) costs, emitted as `PHASE16-BENCH` markers collected by
//! `scripts/run-phase16-benchmarks.sh`.
//!
//! The plan's Benchmark Impact note says materialization is bursty:
//! cheap when nothing triggers, expensive in a tick where many cells do.
//! The two arms here price that shape separately. `transition-tick`
//! isolates the check scan itself - transition absent, versus enabled but
//! inert (`density_floor_milli` unreachable) at two check intervals, so
//! the per-tick cost of scanning every `(cell, class)` slot is visible
//! independent of whether anything ever triggers. `transition-burst`
//! prices the opposite extreme: the one tick in which everything planted
//! actually converts, at three trigger widths, plus the steady-state cost
//! once those organisms are alive and ordinary lifecycle has to carry
//! them - so the burst's *marginal* cost over ordinary population upkeep
//! is separable. The snapshot-growth marker lives in
//! `crates/sim-persist/tests/bench_phase16_snapshot.rs`.
//!
//! Modelled on `crates/sim-core/tests/phase16_transition.rs`'s
//! `scratch_config`, `frozen_field` and `eligible_class`, at this
//! benchmark's own 64x64 scale, and on `bench_phase15.rs`'s harness
//! conventions (`SEED`, warmup/sample tick counts, `median`).

use sim_core::{OriginMode, SimConfig, World, class_count, class_parameters};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const WARMUP_TICKS: u64 = 100;
const SAMPLE_TICKS: u64 = 500;
const CELLS: u32 = 64;
/// `transition.organism_energy_milli`'s value throughout this file -
/// the shipped default, and `phase16_transition.rs`'s `ENERGY`.
const ENERGY: i64 = 4_000;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

/// The field regime plus what the transition needs (phase 2, genome 2,
/// morphology), scratch-origin, at 64x64 - `phase16_transition.rs`'s
/// `scratch_config` shape at the benchmark's scale. Leaves
/// `config.transition` at its default (disabled); callers set what they
/// need.
fn base_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = CELLS;
    config.cells_y = CELLS;
    config.initial_organisms = 0;
    config.max_entities = 4_000;
    config.origin.mode = OriginMode::Scratch;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.chemistry.enabled = true;
    config.chemistry.field_steps_per_tick = 2;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    // Above the truncation floor for seeded densities, as every config
    // that wants the mutation term live is (Phase 15's convention).
    config.chemistry.mutation_q16 = 4_096;
    // Ten times the shipped abiotic input, so an eligible class reaches
    // an organism's worth of density inside a test horizon.
    config.chemistry.production_milli_per_step = 20;
    config
}

/// Freeze the field's own dynamics - no death, no mutation flow, no
/// growth - copied from `phase16_transition.rs`, so a density planted by
/// surgery is exactly the density the trigger reads at the next check.
fn frozen_field(config: &mut SimConfig) {
    config.chemistry.death_q16 = 0;
    config.chemistry.mutation_q16 = 0;
    config.chemistry.growth_rate_low_q16 = 0;
    config.chemistry.growth_rate_high_q16 = 0;
}

/// The lowest class at the top of the aggregation axis: eligible under
/// the default `aggregation_step_min` of 1. Copied from
/// `phase16_transition.rs`.
fn eligible_class(config: &SimConfig) -> usize {
    (0..class_count(&config.chemistry))
        .find(|&class| class_parameters(&config.chemistry, class).aggregation_step >= 1)
        .expect("an eligible class exists")
}

/// The same config on the first seed at or after its own that generates a
/// world: some seeds produce sub-minimum land at small maps (a Phase 15
/// trap), and nothing here depends on which seed that is.
fn generable(mut config: SimConfig) -> SimConfig {
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed within 32 of {:#x}", config.world_seed);
}

fn tick_cost_of(mut world: World) -> f64 {
    for _ in 0..WARMUP_TICKS {
        world.step();
    }
    let mut samples = Vec::with_capacity(SAMPLE_TICKS as usize);
    for _ in 0..SAMPLE_TICKS {
        let started = Instant::now();
        world.step();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    median(&mut samples)
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn transition_tick_check_scan_cost() {
    // One generable seed shared by all three arms, so the map itself
    // cannot be the source of a difference.
    let seed = generable({
        let config = base_config(SEED);
        config.validate().expect("base config validates");
        config
    })
    .world_seed;

    let mut disabled = base_config(seed);
    disabled.transition.enabled = false;
    disabled.validate().expect("disabled config validates");

    let mut inert_check100 = base_config(seed);
    inert_check100.transition.enabled = true;
    // Unreachable: this arm prices the scan, never a trigger.
    inert_check100.transition.density_floor_milli = i64::MAX / 4;
    inert_check100.transition.check_interval_ticks = 100;
    inert_check100
        .validate()
        .expect("inert check100 config validates");

    let mut inert_check1 = inert_check100;
    inert_check1.transition.check_interval_ticks = 1;
    inert_check1
        .validate()
        .expect("inert check1 config validates");

    assert_eq!(
        class_count(&disabled.chemistry),
        8,
        "the marker line's classes=8 label assumes the default axis sizes"
    );

    let disabled_us = tick_cost_of(World::new(disabled).expect("world"));
    let inert_check100_us = tick_cost_of(World::new(inert_check100).expect("world"));
    let inert_check1_us = tick_cost_of(World::new(inert_check1).expect("world"));

    println!(
        "PHASE16-BENCH transition-tick cells=64x64 classes=8 disabled_us={disabled_us:.1} \
         inert_check100_us={inert_check100_us:.1} inert_check1_us={inert_check1_us:.1}"
    );
}

/// A scratch world tuned so a slot planted by surgery materializes on the
/// very next (and only) check: `check_interval_ticks` 1 and
/// `persistence_checks` 1 mean the first check a restored world ever
/// makes both counts and completes the window, and `density_floor_milli`
/// set to one organism's energy is comfortably below the four organisms'
/// worth this benchmark plants.
fn burst_config(seed: u64) -> SimConfig {
    let mut config = base_config(seed);
    frozen_field(&mut config);
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 1;
    config.transition.density_floor_milli = ENERGY;
    config.transition.organism_energy_milli = ENERGY;
    config.transition.max_organisms_per_event = 4;
    config.transition.max_materializations_per_tick = 1_024;
    config.max_entities = 4_000;
    config.validate().expect("burst config validates");
    generable(config)
}

/// Up to `n` distinct land cells, ascending - the slots the burst plants
/// into.
fn land_cells(world: &World, n: usize) -> Vec<usize> {
    let terrain = world.terrain();
    (0..terrain.cell_count())
        .filter(|&cell| terrain.capacity_milli[cell] > 0)
        .take(n)
        .collect()
}

/// Conserving surgery: plant four organisms' worth of density into each
/// of `cells` (in the first eligible class), booking the same total as
/// production so the restore's identity check accepts it.
fn plant_slots(config: SimConfig, cells: &[usize]) -> World {
    let world = World::new(config).expect("world");
    let classes = class_count(&config.chemistry);
    let class = eligible_class(&config);
    let mut state = world.export_state();
    let mass = 4 * ENERGY;
    for &cell in cells {
        let slot = cell * classes + class;
        state.microbial.as_mut().unwrap().densities[slot] += mass;
        state.chemistry.as_mut().unwrap().produced_milli += i128::from(mass);
    }
    World::from_state(state).expect("planted state restores")
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn transition_burst_tick_cost() {
    let config = burst_config(SEED ^ 0x1);
    let probe = World::new(config).expect("world");
    let cells = land_cells(&probe, 64);
    assert!(
        cells.len() >= 64,
        "need 64 distinct land cells for the widest burst, found {}",
        cells.len()
    );

    let mut burst_us = [0.0_f64; 3];
    let mut slots64_after_us = 0.0_f64;
    for (index, &n) in [1_usize, 16, 64].iter().enumerate() {
        let mut world = plant_slots(config, &cells[..n]);
        let started = Instant::now();
        world.step();
        burst_us[index] = started.elapsed().as_secs_f64() * 1_000_000.0;
        assert_eq!(
            world.metrics().materialized_total,
            4 * n as u64,
            "the {n}-slot burst did not materialize 4 organisms per slot - \
             a burst that did not happen is not a burst benchmark"
        );
        if n == 64 {
            let mut samples = Vec::with_capacity(20);
            for _ in 0..20 {
                let started = Instant::now();
                world.step();
                samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
            }
            slots64_after_us = median(&mut samples);
        }
    }
    let (slots1_burst_us, slots16_burst_us, slots64_burst_us) =
        (burst_us[0], burst_us[1], burst_us[2]);
    let per_organism_us = (slots64_burst_us - slots64_after_us) / 256.0;

    println!(
        "PHASE16-BENCH transition-burst cells=64x64 slots1_burst_us={slots1_burst_us:.1} \
         slots16_burst_us={slots16_burst_us:.1} slots64_burst_us={slots64_burst_us:.1} \
         slots64_after_us={slots64_after_us:.1} per_organism_us={per_organism_us:.2}"
    );
}
