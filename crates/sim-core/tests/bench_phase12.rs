//! Phase 12 mutable world: the two costs this half of the phase adds.
//!
//! `#[ignore]`; run in release with
//! `cargo test --release -p sim-core --test bench_phase12 -- --ignored --nocapture`.
//!
//! Both numbers exist because a decision was made against the
//! specification's stated design and the specification asks for the cost
//! instead.
//!
//! 1. **The composed terrain checksum is a full recompute.**
//!    `specifications/mutable-world-state.md` asks for an incremental
//!    computation cross-checked against a periodic full one. FNV-1a is a
//!    multiply-and-xor chain over a byte stream, so changing a byte in the
//!    middle changes every subsequent state: there is no update that folds
//!    one cell's new value into a finished hash, and the specification's
//!    clause describes an algorithm the chosen primitive cannot provide. The
//!    honest answer is a full recompute on demand, and the number that makes
//!    it honest is how long one costs.
//!
//! 2. **The composed capacity accessor is a binary search per cell.**
//!    `grow_food` and the biomass invariant walk every cell, so enabling the
//!    section adds a `log2(overrides)` probe to each. Measured rather than
//!    argued: if it were expensive, the fix would be a cursor walk like the
//!    one `composed_checksum` already uses.

use sim_core::{LAYER_CAPACITY_SCALE, SimConfig, World};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const ONE_Q16: u32 = 65_536;
const WARMUP_TICKS: u64 = 100;
const SAMPLE_TICKS: u64 = 500;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn patch_config(scale_q16: u32, radius: u32) -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.worldmod.enabled = true;
    config.worldmod.patch_enabled = true;
    config.worldmod.relocate_interval_ticks = 500;
    config.worldmod.patch_radius_cells = radius;
    config.worldmod.patch_capacity_scale_q16 = scale_q16;
    // The shipped cap of 4,096 refuses a patch whose footprint exceeds it,
    // which is the point of the cap; the large arm here is deliberately past
    // it because the independent variable is the override count.
    config.worldmod.max_capacity_overrides = 65_536;
    config
}

/// Microseconds per `step`, median over `SAMPLE_TICKS` after a warmup.
fn tick_cost_us(config: SimConfig) -> (f64, usize) {
    let mut world = World::new(config).expect("world");
    for _ in 0..WARMUP_TICKS {
        world.step();
    }
    let mut samples = Vec::with_capacity(SAMPLE_TICKS as usize);
    for _ in 0..SAMPLE_TICKS {
        let started = Instant::now();
        world.step();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let overrides = world.worldmod_state().map_or(0, |state| state.len());
    (median(&mut samples), overrides)
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn composed_terrain_checksum_full_recompute_cost() {
    // The default map: 256 x 256 = 65,536 cells, each contributing 13 bytes
    // to the hash (one land byte, a u32 elevation, an i64 capacity).
    let mut world = World::new(patch_config(2 * ONE_Q16, 15)).expect("world");
    let cells = world.terrain().cell_count();

    let mut empty = Vec::new();
    for _ in 0..50 {
        let started = Instant::now();
        let checksum = world.composed_terrain_checksum();
        empty.push(started.elapsed().as_secs_f64() * 1_000_000.0);
        assert_eq!(checksum, world.terrain().terrain_checksum);
    }

    // Now with a live patch, which is the case that actually walks the
    // override cursors.
    for _ in 0..600 {
        world.step();
    }
    let overrides = world.worldmod_state().expect("section").len();
    assert!(
        overrides > 0,
        "no patch was written, so this measures nothing"
    );
    let mut populated = Vec::new();
    for _ in 0..50 {
        let started = Instant::now();
        let checksum = world.composed_terrain_checksum();
        populated.push(started.elapsed().as_secs_f64() * 1_000_000.0);
        assert_ne!(checksum, world.terrain().terrain_checksum);
    }

    println!(
        "composed_terrain_checksum cells={cells} empty_median_us={:.1} \
         overrides={overrides} populated_median_us={:.1}",
        median(&mut empty),
        median(&mut populated)
    );
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn tick_cost_with_the_section_disabled_quiet_and_patched() {
    // Three arms, and the middle one is the one that isolates the accessor:
    // enabled with an empty set pays the `Option` match and nothing else,
    // while the patched arm pays the binary search on every one of the
    // 65,536 cells `grow_food` walks.
    let disabled = SimConfig::phase1_default(SEED);
    let mut quiet = disabled;
    quiet.worldmod.enabled = true;
    quiet.worldmod.patch_enabled = false;

    let (disabled_us, _) = tick_cost_us(disabled);
    let (quiet_us, quiet_overrides) = tick_cost_us(quiet);
    let (small_us, small_overrides) = tick_cost_us(patch_config(2 * ONE_Q16, 8));
    let (large_us, large_overrides) = tick_cost_us(patch_config(2 * ONE_Q16, 45));

    println!(
        "tick_cost_us disabled={disabled_us:.1} \
         quiet={quiet_us:.1}(overrides={quiet_overrides}) \
         patch_r8={small_us:.1}(overrides={small_overrides}) \
         patch_r45={large_us:.1}(overrides={large_overrides})"
    );
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn modification_write_cost_as_a_function_of_set_size() {
    // A sorted-array insert is O(n) in the set size, which is fine at the
    // patch's scale and is the number the artifact half needs before it
    // starts writing overrides per organism per tick.
    let mut config = SimConfig::phase1_default(SEED);
    config.worldmod.enabled = true;
    config.worldmod.max_capacity_overrides = 65_536;
    let mut world = World::new(config).expect("world");
    let habitable: Vec<usize> = (0..world.terrain().cell_count())
        .filter(|cell| world.terrain().capacity_milli[*cell] > 0)
        .collect();

    let mut report = Vec::new();
    let mut written = 0_usize;
    for target in [1_000_usize, 4_000, 16_000] {
        let started = Instant::now();
        while written < target.min(habitable.len()) {
            world.apply_terrain_modification(
                LAYER_CAPACITY_SCALE,
                habitable[written],
                Some(i64::from(ONE_Q16)),
            );
            written += 1;
        }
        let elapsed = started.elapsed().as_secs_f64() * 1_000_000.0;
        report.push((written, elapsed));
    }
    for (size, elapsed_us) in report {
        println!("modification_writes reached_size={size} elapsed_us={elapsed_us:.1}");
    }
    assert_eq!(world.worldmod_state().expect("section").len(), written);
}
