//! Phase 19 benchmarks: what "coupling v2" (ADR-0034) costs, emitted as
//! `PHASE19-BENCH` markers collected by `scripts/run-phase19-benchmarks.sh`.
//!
//! C19.8 asks that the consumption pass be priced per organism and per
//! tick against the v1 world, and that the field's own per-cell cost stay
//! unchanged by the flag. The two arms here price those two halves
//! separately. `consumption-tick` isolates the organism-side cost: a
//! populated scratch world, consumption off (v1) versus on (v2), at two
//! population sizes sixteen times apart (well past the fourfold spread
//! C19.8 asks for - the per-organism effect turns out small enough that
//! a wide spread is what keeps it above the run-to-run scheduling noise
//! a single process on a shared machine picks up), so the marginal
//! per-organism-tick cost of the substrate loop is visible net of
//! whatever the field itself costs (which is identical in both arms -
//! same seed, same field config, same planted state, differing only in
//! `chemistry.consumption_fraction_q16`). `field-cell` isolates the
//! field-side cost: the same field step, no organisms at all, flag off
//! versus on, so the field's cost is shown to depend on the flag not at
//! all (there is nothing for the pass to iterate).
//!
//! Modelled on `bench_phase16.rs`'s harness conventions (`SEED`,
//! warmup/sample tick counts, `median`, `generable`, `eligible_class`,
//! `land_cells`/`plant_slots`-style surgery) and on
//! `phase19_consumption.rs`'s `coupled` and `planted` helpers, which
//! establish the fixture shape reused here: a scratch world, one-shot
//! transition materialization, and biomass zeroed in the occupied cell so
//! the organism's intake goes straight to substrate once consumption is
//! on ("one gut, one intake rate: biomass fills the capability first and
//! substrate only what it leaves").
//!
//! `consumption-tick`'s organisms materialize at three quarters of the
//! unicell energy capacity - room enough for the on arm to visibly fill
//! up on substrate, and (with biomass zeroed, in the off arm, which has
//! no coupling to fall back on) buffer enough against basal, move and
//! crowding costs to stay alive for `WARMUP_TICKS + SAMPLE_TICKS` ticks,
//! so the populations measured are the populations that were alive
//! throughout - the marker states the count materialized and the count
//! still alive at the end of sampling.

use sim_core::{
    OriginMode, S_MONOMER, SUBSTRATE_COUNT, SimConfig, World, class_count, class_parameters,
    unicell_derived,
};
use std::time::Instant;

const SEED: u64 = 0x19c0_5eed_f00d_beef;
const WARMUP_TICKS: u64 = 10;
const SAMPLE_TICKS: u64 = 200;
const CELLS: u32 = 64;
const SMALL_POP: usize = 16;
const LARGE_POP: usize = 256;
/// S_MONOMER planted per occupied cell - far more than any one organism
/// can draw across materialization plus `WARMUP_TICKS` plus
/// `SAMPLE_TICKS`, so the consumption-on arm never returns early for want
/// of anything to take.
const SUBSTRATE_PER_CELL: i64 = 4_000_000;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

/// The field regime plus what the transition needs (phase 2, genome 2,
/// morphology), scratch-origin, at 64x64 - `bench_phase16.rs`'s
/// `base_config` shape. `chemistry.consumption_fraction_q16` stays at its
/// default (0, off); callers set it. The transition stays disabled here;
/// `one_shot_transition` turns it on for the populated arm.
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
    config.chemistry.mutation_q16 = 4_096;
    config.chemistry.production_milli_per_step = 20;
    config
}

/// The lowest class at the top of the aggregation axis - copied from
/// `bench_phase16.rs` / `phase19_consumption.rs`.
fn eligible_class(config: &SimConfig) -> usize {
    (0..class_count(&config.chemistry))
        .find(|&class| class_parameters(&config.chemistry, class).aggregation_step >= 1)
        .expect("an eligible class exists")
}

/// The same config on the first seed at or after its own that generates a
/// world - `bench_phase16.rs`'s `generable`.
fn generable(mut config: SimConfig) -> SimConfig {
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed within 32 of {:#x}", config.world_seed);
}

/// Up to `n` distinct land cells, ascending - `bench_phase16.rs`'s
/// `land_cells`.
fn land_cells(world: &World, n: usize) -> Vec<usize> {
    let terrain = world.terrain();
    (0..terrain.cell_count())
        .filter(|&cell| terrain.capacity_milli[cell] > 0)
        .take(n)
        .collect()
}

fn median_tick_cost(world: &mut World, warmup: u64, sample: u64) -> f64 {
    for _ in 0..warmup {
        world.step();
    }
    let mut samples = Vec::with_capacity(sample as usize);
    for _ in 0..sample {
        let started = Instant::now();
        world.step();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    median(&mut samples)
}

/// Times `build()` twice and keeps the second reading: the very first
/// world timed in a fresh test process measures measurably slower than
/// an otherwise-identical later one (observed empirically - cold
/// allocator arenas and an unwarmed branch predictor/icache, not
/// anything about the configuration), so every arm gets an identical
/// throwaway warm-up world of its own shape before the kept measurement,
/// and which arm happens to run first in the process stops mattering.
fn warmed_median_tick_cost(build: impl Fn() -> World, warmup: u64, sample: u64) -> (f64, World) {
    let mut warm = build();
    median_tick_cost(&mut warm, warmup, sample);
    let mut world = build();
    let us = median_tick_cost(&mut world, warmup, sample);
    (us, world)
}

/// Freeze the class-density field's own dynamics - no death, no mutation
/// flow, no growth - `bench_phase16.rs`'s `frozen_field`, so a density
/// planted by surgery survives to the very next check exactly as
/// planted, undiminished by the field step that runs before the check.
fn frozen_field(config: &mut SimConfig) {
    config.chemistry.death_q16 = 0;
    config.chemistry.mutation_q16 = 0;
    config.chemistry.growth_rate_low_q16 = 0;
    config.chemistry.growth_rate_high_q16 = 0;
}

/// One organism's energy at materialization - three quarters of the
/// unicell's energy capacity, so there is both room enough for
/// consumption to matter and buffer enough (with biomass zeroed, in the
/// consumption-off arm) to stay alive for `WARMUP_TICKS + SAMPLE_TICKS`
/// ticks under basal, movement and crowding costs combined.
fn organism_energy(config: &SimConfig) -> i64 {
    unicell_derived(&config.morphology).energy_capacity_milli * 3 / 4
}

/// Enable one-shot materialization: a slot at or over the floor at the
/// very next (and only) check converts - `bench_phase16.rs`'s
/// `burst_config` shape, at `max_organisms_per_event` 1 so one planted
/// slot yields exactly one organism.
fn one_shot_transition(config: &mut SimConfig) {
    let energy = organism_energy(config);
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 1;
    config.transition.density_floor_milli = energy;
    config.transition.organism_energy_milli = energy;
    config.transition.max_organisms_per_event = 1;
    config.transition.max_materializations_per_tick = 1_024;
}

/// Plant one organism's worth of density (materializing on the next
/// tick) into each of `n` distinct land cells, plant `SUBSTRATE_PER_CELL`
/// of S_MONOMER into the same cells, and zero each cell's biomass so
/// intake goes straight to the substrate loop when consumption is on -
/// `phase19_consumption.rs`'s `planted()` shape, extended to `n` cells at
/// once the way `bench_phase16.rs`'s `plant_slots` extends single-slot
/// surgery to a burst. Conserving: every milli credited to microbial
/// density or field concentration is booked as `produced_milli`, and
/// every milli of biomass removed is booked against
/// `ledger.initial_biomass_milli`, so `World::from_state` accepts the
/// result. Steps once to materialize and returns the resulting world.
fn plant_population(mut config: SimConfig, n: usize) -> World {
    frozen_field(&mut config);
    one_shot_transition(&mut config);
    config.validate().expect("population config validates");
    let probe = World::new(config).expect("world");
    let cells = land_cells(&probe, n);
    assert!(
        cells.len() >= n,
        "need {n} distinct land cells, found {}",
        cells.len()
    );
    let classes = class_count(&config.chemistry);
    let class = eligible_class(&config);
    let energy = organism_energy(&config);
    let mut state = probe.export_state();
    for &cell in &cells[..n] {
        let slot = cell * classes + class;
        state.microbial.as_mut().unwrap().densities[slot] += energy;
        state.chemistry.as_mut().unwrap().produced_milli += i128::from(energy + SUBSTRATE_PER_CELL);
        let base = cell * SUBSTRATE_COUNT;
        state.chemistry.as_mut().unwrap().concentrations[base + S_MONOMER] += SUBSTRATE_PER_CELL;
        let biomass = state.biomass_milli[cell];
        state.biomass_milli[cell] = 0;
        state.ledger.initial_biomass_milli -= i128::from(biomass);
    }
    let mut world = World::from_state(state).expect("planted state restores");
    world.step(); // materialize
    let materialized = world.population();
    assert_eq!(
        materialized, n,
        "expected {n} organisms to materialize, got {materialized}"
    );
    world
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn consumption_tick_substrate_loop_cost() {
    // One generable seed shared by every arm, so the map itself cannot be
    // the source of a difference.
    let seed = generable({
        let config = base_config(SEED);
        config.validate().expect("base config validates");
        config
    })
    .world_seed;

    let mut off = base_config(seed);
    off.chemistry.consumption_fraction_q16 = 0;
    let mut on = base_config(seed);
    on.chemistry.consumption_fraction_q16 = 65_536; // Q16_ONE: coupling v2 fully on

    let (off_small_us, world_off_small) = warmed_median_tick_cost(
        || plant_population(off, SMALL_POP),
        WARMUP_TICKS,
        SAMPLE_TICKS,
    );
    let (on_small_us, world_on_small) = warmed_median_tick_cost(
        || plant_population(on, SMALL_POP),
        WARMUP_TICKS,
        SAMPLE_TICKS,
    );
    let (off_large_us, world_off_large) = warmed_median_tick_cost(
        || plant_population(off, LARGE_POP),
        WARMUP_TICKS,
        SAMPLE_TICKS,
    );
    let (on_large_us, world_on_large) = warmed_median_tick_cost(
        || plant_population(on, LARGE_POP),
        WARMUP_TICKS,
        SAMPLE_TICKS,
    );

    // The off arm never touches the field's consumed term; the on arm
    // must, or nothing here exercised the substrate loop it claims to
    // price.
    assert_eq!(world_off_small.metrics().chemistry_consumed_milli, 0);
    assert_eq!(world_off_large.metrics().chemistry_consumed_milli, 0);
    assert!(
        world_on_small.metrics().chemistry_consumed_milli > 0,
        "the {SMALL_POP}-organism on arm never ate"
    );
    assert!(
        world_on_large.metrics().chemistry_consumed_milli > 0,
        "the {LARGE_POP}-organism on arm never ate"
    );

    // Organisms alive at the end of sampling - the population the timing
    // above actually measured throughout, not merely at materialization.
    let alive_off_small = world_off_small.population();
    let alive_on_small = world_on_small.population();
    let alive_off_large = world_off_large.population();
    let alive_on_large = world_on_large.population();

    // Difference-in-differences: the on-vs-off delta at each population,
    // divided by the population difference, cancels whatever fixed
    // per-tick cost the two populations share (field mechanics, the
    // materialized bodies' ordinary upkeep) and isolates the marginal
    // per-organism-tick cost the substrate loop itself adds.
    let delta_small_us = on_small_us - off_small_us;
    let delta_large_us = on_large_us - off_large_us;
    let per_organism_us = (delta_large_us - delta_small_us) / (LARGE_POP as f64 - SMALL_POP as f64);

    println!(
        "PHASE19-BENCH consumption-tick cells=64x64 pop_small={SMALL_POP} pop_large={LARGE_POP} \
         off_small_us={off_small_us:.2} on_small_us={on_small_us:.2} \
         off_large_us={off_large_us:.2} on_large_us={on_large_us:.2} \
         alive_off_small={alive_off_small} alive_on_small={alive_on_small} \
         alive_off_large={alive_off_large} alive_on_large={alive_on_large} \
         per_organism_us={per_organism_us:.3}"
    );
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn field_cell_cost_is_unchanged_by_the_flag() {
    // No organisms ever appear: origin is Scratch, initial_organisms is
    // 0, and the transition stays disabled (base_config's default), so
    // the consumption pass - which only ever runs over living organisms
    // - has nothing to iterate regardless of the flag.
    let seed = generable({
        let config = base_config(SEED ^ 0x1);
        config.validate().expect("base config validates");
        config
    })
    .world_seed;

    let mut off = base_config(seed);
    off.chemistry.consumption_fraction_q16 = 0;
    off.validate().expect("off config validates");
    let mut on = base_config(seed);
    on.chemistry.consumption_fraction_q16 = 65_536;
    on.validate().expect("on config validates");

    let (off_us, world_off) = warmed_median_tick_cost(
        || World::new(off).expect("world"),
        WARMUP_TICKS,
        SAMPLE_TICKS,
    );
    let (on_us, world_on) = warmed_median_tick_cost(
        || World::new(on).expect("world"),
        WARMUP_TICKS,
        SAMPLE_TICKS,
    );
    assert_eq!(world_off.population(), 0, "an empty world stays empty");
    assert_eq!(world_on.population(), 0, "an empty world stays empty");
    assert_eq!(world_off.metrics().chemistry_consumed_milli, 0);
    assert_eq!(world_on.metrics().chemistry_consumed_milli, 0);

    let cells = f64::from(CELLS) * f64::from(CELLS);
    let per_cell_ns_off = off_us * 1_000.0 / cells;
    let per_cell_ns_on = on_us * 1_000.0 / cells;

    println!(
        "PHASE19-BENCH field-cell cells=64x64 organisms=0 off_us={off_us:.2} on_us={on_us:.2} \
         per_cell_ns_off={per_cell_ns_off:.2} per_cell_ns_on={per_cell_ns_on:.2}"
    );
}
