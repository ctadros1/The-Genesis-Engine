//! Phase 16: what the transition section costs to checkpoint.
//!
//! `#[ignore]`; run in release by `scripts/run-phase16-benchmarks.sh`.
//!
//! `TransitionState` is real, saved state (ADR-0032's recorded deviation
//! from "no new section"): one `u32` persistence counter per `(cell,
//! class)` slot plus a handful of run-total scalars. The measurement is
//! the byte delta between the same scratch ecology with the transition
//! off and on - an inert floor is fine, since the section's shape (and
//! so its encoded size) does not depend on whether any slot ever
//! triggers. Both arms restore all the way back into a world and check
//! the checksum, so a snapshot that encoded fast by dropping the
//! transition section cannot be reported as a cheap one.

use sim_core::{OriginMode, SimConfig, World};
use sim_persist::{decode_snapshot, encode_snapshot};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const CELLS: u32 = 64;
const CLASSES: u64 = 8;
const TICKS: u64 = 200;

fn config(transition: bool) -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
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
    if transition {
        config.transition.enabled = true;
        // Inert floor: the section's byte cost is fixed by its shape
        // (persistence counters plus scalar totals), not by whether any
        // slot ever crosses it - the same convention as the sim-core
        // transition-tick benchmark's `inert` arms.
        config.transition.density_floor_milli = i64::MAX / 4;
    }
    config.validate().expect("validates");
    config
}

fn measure(config: SimConfig) -> (usize, usize, u64) {
    let mut world = World::new(config).expect("world");
    for _ in 0..TICKS {
        world.step();
    }
    let state = world.export_state();
    let checksum = world.state_checksum();
    let plain = encode_snapshot(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encodes");
    let compressed = encode_snapshot(
        &state,
        1,
        0,
        checksum,
        sim_persist::BUILD_VERSION,
        0,
        Some(3),
    )
    .expect("encodes compressed");
    let (_, decoded) = decode_snapshot(&plain).expect("decodes");
    let restored = World::from_state(decoded).expect("restores");
    assert_eq!(
        restored.state_checksum(),
        checksum,
        "the restored world must hash identically or the size is a lie"
    );
    (plain.len(), compressed.len(), world.population() as u64)
}

#[test]
#[ignore = "sizing benchmark; run with --ignored"]
fn transition_snapshot_growth_off_vs_on() {
    let (off_bytes, off_zstd3, off_population) = measure(config(false));
    let (transition_bytes, transition_zstd3, transition_population) = measure(config(true));
    // Both arms are a scratch world with the transition either absent or
    // inert, so nothing ever materializes on either side: the delta
    // below is the section's shape alone, not a population divergence.
    assert_eq!(
        off_population, transition_population,
        "the arms diverged organism-side, so the delta is not the section's alone"
    );
    let delta_bytes = transition_bytes as i64 - off_bytes as i64;
    let slots = u64::from(CELLS) * u64::from(CELLS) * CLASSES;
    println!(
        "PHASE16-BENCH snapshot cells=64x64 classes=8 off_bytes={off_bytes} \
         off_zstd3_bytes={off_zstd3} transition_bytes={transition_bytes} \
         transition_zstd3_bytes={transition_zstd3} delta_bytes={delta_bytes} \
         delta_bytes_per_slot={:.1}",
        delta_bytes as f64 / slots as f64,
    );
}
