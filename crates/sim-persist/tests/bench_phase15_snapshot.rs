//! Phase 15: what the field regime costs to checkpoint (C15.8).
//!
//! `#[ignore]`; run in release by `scripts/run-phase15-benchmarks.sh`.
//!
//! Field state is the fifth snapshot growth term, after schema-3 genomes,
//! learned state, objects, and terrain deltas - and unlike bodies and
//! biomes it is stored, never recomputed (ADR-0020). The measurement is
//! the byte delta between the same ecology with the field stack off and
//! on: the coupling fractions are zero and nothing organism-side reads
//! the field, so the organism state is identical between the arms and the
//! delta is exactly the chemistry + microbial sections plus their config
//! bytes. Both arms restore all the way back into a world and check the
//! checksum, so a snapshot that encoded fast by dropping the field
//! sections cannot be reported as a cheap snapshot.

use sim_core::{SimConfig, World};
use sim_persist::{decode_snapshot, encode_snapshot};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const CELLS: u32 = 64;
const TICKS: u64 = 4_000;

fn config(field: bool) -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = CELLS;
    config.cells_y = CELLS;
    config.initial_organisms = 40;
    config.max_entities = 4_000;
    if field {
        config.chemistry.enabled = true;
        config.chemistry.field_steps_per_tick = 1;
        config.chemistry.microbial_enabled = true;
        config.chemistry.abiogenesis_enabled = true;
        config.chemistry.mutation_q16 = 4_096;
    }
    config.validate().expect("validates");
    config
}

fn measure(field: bool) -> (usize, usize, u64) {
    let mut world = World::new(config(field)).expect("world");
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
fn field_snapshot_growth_off_vs_on() {
    let (off_bytes, off_zstd3, off_population) = measure(false);
    let (field_bytes, field_zstd3, field_population) = measure(true);
    // The decoupling claim the delta interpretation rests on: identical
    // organism dynamics with the field on or off.
    assert_eq!(
        off_population, field_population,
        "the arms diverged organism-side, so the delta is not the field's alone"
    );
    let cells = u64::from(CELLS) * u64::from(CELLS);
    println!(
        "PHASE15-BENCH snapshot cells=64x64 ticks={TICKS} population={off_population} \
         off_bytes={off_bytes} off_zstd3_bytes={off_zstd3} \
         field_bytes={field_bytes} field_zstd3_bytes={field_zstd3} \
         field_delta_bytes={} field_delta_zstd3_bytes={} delta_bytes_per_cell={:.1}",
        field_bytes - off_bytes,
        field_zstd3 as i64 - off_zstd3 as i64,
        (field_bytes - off_bytes) as f64 / cells as f64,
    );
}
