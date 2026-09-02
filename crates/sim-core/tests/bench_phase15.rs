//! Phase 15 benchmarks: what the field regime costs per tick, emitted as
//! `PHASE15-BENCH` markers collected by `scripts/run-phase15-benchmarks.sh`.
//!
//! The arms isolate the seam from the work, as every phase's benchmark
//! does. `disabled` is the same ecology with the chemistry gate off;
//! `chem` prices the substrate field alone (diffusion, reactions,
//! production); the `micro*` arms add the microbial passes over 8 and 32
//! classes and over 1 and 4 field steps per tick, with abiogenesis live
//! so the growth/death/mutation passes work over real density rather
//! than scanning zeros. The scale line prices the same 8-class arm on a
//! quarter-size map and across a 16x population spread: the field's cost
//! is per-cell by construction, so the population delta of the field arm
//! should match the population delta of the disabled arm - that is the
//! population-independence claim, measured rather than asserted.

use sim_core::{SimConfig, World};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const WARMUP_TICKS: u64 = 100;
const SAMPLE_TICKS: u64 = 500;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

fn field_config(
    cells: u32,
    organisms: u32,
    field: Option<(u32, u32, u32)>, // (replication_axis, aggregation_axis, steps)
) -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = cells;
    config.cells_y = cells;
    config.initial_organisms = organisms;
    config.max_entities = 4_000;
    if let Some((replication, aggregation, steps)) = field {
        config.chemistry.enabled = true;
        config.chemistry.field_steps_per_tick = steps;
        config.chemistry.microbial_enabled = true;
        config.chemistry.replication_axis = replication;
        config.chemistry.aggregation_axis = aggregation;
        config.chemistry.abiogenesis_enabled = true;
        // Above the truncation floor for seeded densities, as the
        // fixture and the campaign configs are.
        config.chemistry.mutation_q16 = 4_096;
    }
    config.validate().expect("validates");
    config
}

fn chem_only_config(cells: u32, organisms: u32) -> SimConfig {
    let mut config = field_config(cells, organisms, None);
    config.chemistry.enabled = true;
    config.chemistry.field_steps_per_tick = 1;
    config.validate().expect("validates");
    config
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
fn field_tick_cost_against_classes_and_steps() {
    let disabled_us = tick_cost_of(World::new(field_config(128, 200, None)).expect("w"));
    let chem_us = tick_cost_of(World::new(chem_only_config(128, 200)).expect("w"));
    let micro8_us = tick_cost_of(World::new(field_config(128, 200, Some((2, 2, 1)))).expect("w"));
    let micro8_steps4_us =
        tick_cost_of(World::new(field_config(128, 200, Some((2, 2, 4)))).expect("w"));
    let micro32_us = tick_cost_of(World::new(field_config(128, 200, Some((4, 4, 1)))).expect("w"));
    println!(
        "PHASE15-BENCH field-tick cells=128x128 organisms=200 disabled_us={disabled_us:.1} \
         chem_us={chem_us:.1} micro8_us={micro8_us:.1} \
         micro8_steps4_us={micro8_steps4_us:.1} micro32_us={micro32_us:.1}"
    );
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn field_cost_scales_with_cells_not_population() {
    let cells64_micro8_us =
        tick_cost_of(World::new(field_config(64, 200, Some((2, 2, 1)))).expect("w"));
    let organisms50_disabled_us = tick_cost_of(World::new(field_config(128, 50, None)).expect("w"));
    let organisms50_micro8_us =
        tick_cost_of(World::new(field_config(128, 50, Some((2, 2, 1)))).expect("w"));
    let organisms800_disabled_us =
        tick_cost_of(World::new(field_config(128, 800, None)).expect("w"));
    let organisms800_micro8_us =
        tick_cost_of(World::new(field_config(128, 800, Some((2, 2, 1)))).expect("w"));
    println!(
        "PHASE15-BENCH field-scale cells64_micro8_us={cells64_micro8_us:.1} \
         organisms50_disabled_us={organisms50_disabled_us:.1} \
         organisms50_micro8_us={organisms50_micro8_us:.1} \
         organisms800_disabled_us={organisms800_disabled_us:.1} \
         organisms800_micro8_us={organisms800_micro8_us:.1}"
    );
}
