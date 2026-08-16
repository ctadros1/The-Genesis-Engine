//! Phase 12 artifact half: what objects cost to store and to checkpoint.
//!
//! `#[ignore]`; run in release by `scripts/run-phase12-benchmarks.sh`.
//!
//! The number that settles `artifact.max_objects` (shipped at 4,096, ADR-0028)
//! is bytes per object in section 15 - not bytes per snapshot - and it is
//! measured here at three table sizes, in a world whose organisms are what
//! the base is (a 128 x 128 schema-2 world of 200 founders), so the object
//! share of the file is reported beside the whole. Simple objects are the
//! floor (`OBJECT_FIXED_BYTES` each, 100); composites add eight bytes per
//! constituent, and one arm has a composite for every fourth object so the
//! composition term is present, not assumed.
//!
//! Every arm restores all the way back into a world and checks the checksum,
//! so a snapshot that encodes fast by dropping the object section cannot be
//! reported as a cheap snapshot.

use sim_core::{
    CAUSE_COMBINED, CAUSE_EXTRACTED, MATERIAL_STONE, ObjectRecord, SimConfig, World, material,
};
use sim_persist::{decode_snapshot, encode_snapshot};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn base_config() -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = 200;
    config.max_entities = 4_000;
    config.genome2.enabled = true;
    config.worldmod.enabled = true;
    config.contest.enabled = true;
    config.artifact.enabled = true;
    config.artifact.max_objects = 8_192;
    config.artifact.max_objects_per_cell = 64;
    config.validate().expect("validates");
    config
}

/// A world with `count` free stones spread over the map, every fourth pair
/// combined into a depth-one composite when `with_composites` is set. Built
/// through the save path so the ledger and the table agree by construction
/// and the restore check proves it.
fn world_with_objects(count: usize, with_composites: bool) -> World {
    let world = World::new(base_config()).expect("world");
    let mut state = world.export_state();
    let cell_fp = state.config.cell_size_fp();
    let cells_x = state.config.cells_x as i32;
    let cells_y = state.config.cells_y as i32;
    let stone = material(MATERIAL_STONE).expect("stone");
    // Land cells, in index order, to scatter over.
    let probe = World::from_state(state.clone()).expect("probe");
    let land: Vec<(i32, i32)> = (0..cells_y)
        .flat_map(|cy| (0..cells_x).map(move |cx| (cx, cy)))
        .filter(|&(cx, cy)| probe.effective_traversable((cy * cells_x + cx) as usize))
        .collect();
    assert!(!land.is_empty());
    let table = state.objects.as_mut().expect("section on");
    let mut made = 0;
    let mut slot = 0;
    while made < count {
        let (cx, cy) = land[slot % land.len()];
        slot += 7;
        let x = cx * cell_fp + cell_fp / 2;
        let y = cy * cell_fp + cell_fp / 2;
        let id = state.next_entity_id;
        state.next_entity_id += 1;
        let record = ObjectRecord::simple(id, stone, 400, x, y, 0, CAUSE_EXTRACTED, 0);
        table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
        table.push(record);
        table.objects_allocated_total += 1;
        made += 1;
        if with_composites && made % 4 == 0 && made + 1 <= count {
            // Combine this stone with the previous one into a composite.
            let a = table.len() - 2;
            let b = table.len() - 1;
            let composite_id = state.next_entity_id;
            state.next_entity_id += 1;
            table.owner_id[a] = composite_id;
            table.owner_id[b] = composite_id;
            let (ida, idb) = (table.ids[a], table.ids[b]);
            let composite = ObjectRecord {
                id: composite_id,
                material_id: MATERIAL_STONE,
                x_fp: x,
                y_fp: y,
                integrity_q16: table.integrity_q16[a].min(table.integrity_q16[b]),
                mass_milli: table.mass_milli[a] + table.mass_milli[b],
                energy_milli: table.energy_milli[a] + table.energy_milli[b],
                hardness_q16: table.hardness_q16[a].max(table.hardness_q16[b]),
                durability_q16: table.durability_q16[a].min(table.durability_q16[b]),
                decay_q16: table.decay_q16[a].max(table.decay_q16[b]),
                holder_id: 0,
                owner_id: 0,
                depth: 1,
                created_tick: 0,
                creator_id: 0,
                cause: CAUSE_COMBINED,
                parent_id: 0,
                composition: vec![ida.min(idb), ida.max(idb)],
            };
            table.push(composite);
            table.objects_allocated_total += 1;
            made += 1;
        }
    }
    World::from_state(state).expect("the object world restores")
}

fn objects_section_bytes(world: &World) -> u64 {
    // Encode with and without the table and take the difference: the
    // section framing is included, which is what a file pays.
    let state = world.export_state();
    let full = encode_snapshot(&state, 1, 0, world.state_checksum(), sim_persist::BUILD_VERSION, 0, None)
        .expect("encode");
    let mut bare = state.clone();
    let table = bare.objects.as_mut().expect("section on");
    let empty = sim_core::ObjectTable::default();
    let saved = std::mem::replace(table, empty);
    // Keep the trailer population-long so the bare state still validates.
    let table = bare.objects.as_mut().unwrap();
    table.exposure_ticks = saved.exposure_ticks.clone();
    table.carry_ticks = saved.carry_ticks.clone();
    table.birth_band = saved.birth_band.clone();
    let bare_bytes = encode_snapshot(&bare, 1, 0, 0, sim_persist::BUILD_VERSION, 0, None).expect("encode bare");
    (full.len() as u64).saturating_sub(bare_bytes.len() as u64)
}

fn measure(label: &str, world: World) {
    let table = world.object_table().expect("section on");
    let objects = table.len();
    let composites = table.count_with_depth_at_least(1);
    let population = world.population();
    let state = world.export_state();
    let checksum = world.state_checksum();

    let started = Instant::now();
    let encoded = encode_snapshot(&state, 1, 0, checksum, sim_persist::BUILD_VERSION, 0, None).expect("encode");
    let encode_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    let started = Instant::now();
    let (_, decoded) = decode_snapshot(&encoded).expect("decode");
    let decode_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    let started = Instant::now();
    let restored = World::from_state(decoded).expect("restore");
    let restore_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    assert_eq!(restored.state_checksum(), checksum);
    let section_bytes = objects_section_bytes(&world);

    println!(
        "PHASE12-BENCH snapshot label={label} population={population} objects={objects} \
         composites={composites} snapshot_bytes={} objects_section_bytes={section_bytes} \
         bytes_per_object={} objects_share_milli={} \
         encode_us={encode_us:.1} decode_us={decode_us:.1} restore_us={restore_us:.1}",
        encoded.len(),
        section_bytes / (objects.max(1) as u64),
        section_bytes * 1_000 / (encoded.len() as u64).max(1),
    );
}

#[test]
#[ignore = "benchmark; run with --ignored"]
fn phase12_object_section_budget() {
    measure("none", world_with_objects(0, false));
    measure("simple-256", world_with_objects(256, false));
    measure("simple-4096", world_with_objects(4_096, false));
    measure("mixed-4096", world_with_objects(4_096, true));
}
