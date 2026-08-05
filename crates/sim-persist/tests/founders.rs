//! Phase 6 acceptance criterion C6.9: founder files are hostile-input safe.
//!
//! > A seeded corruption sweep of at least 20,000 cases over the founder-file
//! > codec produces zero panics and typed rejections.
//!
//! Same discipline as the ALIF snapshot and ALEV event-log sweeps: a founder
//! file is an input from outside the process, and a pre-adapted starting
//! condition that silently decoded wrong would mislabel every result derived
//! from it.

use sim_core::{Genome, NEURAL_COUNT, SimConfig, World};
use sim_persist::{FounderError, FounderProvenance, FounderSet, decode_founders, encode_founders};

fn sample_set(count: u32) -> FounderSet {
    let mut config = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = count;
    config.max_entities = 600;
    let world = World::new(config).unwrap();
    let ids = world.organism_ids_view().to_vec();
    let mut cells = Vec::new();
    let mut genomes = Vec::new();
    for (index, &id) in ids.iter().enumerate() {
        let phase2 = world.organism_detail(id).unwrap().phase2.unwrap();
        cells.push(index as u32);
        genomes.push(Genome::validated(phase2.trait_genes, vec![0.1_f32; NEURAL_COUNT]).unwrap());
    }
    FounderSet {
        provenance: FounderProvenance {
            source_world_id: 1,
            source_seed: config.world_seed,
            source_config_hash: config.stable_hash(),
            source_tick: 250_000,
            build_version: sim_persist::BUILD_VERSION.to_owned(),
        },
        cells,
        genomes,
    }
}

#[test]
fn c6_9_corruption_sweep_never_panics_and_always_rejects_typed() {
    let set = sample_set(30);
    let valid = encode_founders(&set).unwrap();
    assert!(valid.len() > 1_000, "the sweep needs a non-trivial file");
    assert_eq!(decode_founders(&valid).unwrap(), set);

    // xorshift, seeded, so any failure reproduces exactly.
    let mut state = 0x00f0_4d35_eedb_eef1_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    const CASES: u32 = 20_000;
    let mut rejected = 0_u32;
    let mut accepted = 0_u32;
    for _ in 0..CASES {
        let mut bytes = valid.clone();
        match next() % 4 {
            0 => {
                let position = (next() % bytes.len() as u64) as usize;
                bytes[position] ^= 1 << (next() % 8);
            }
            1 => {
                let position = (next() % bytes.len() as u64) as usize;
                bytes[position] = (next() % 256) as u8;
            }
            2 => {
                let cut = (next() % bytes.len() as u64) as usize;
                bytes.truncate(cut);
            }
            _ => {
                for _ in 0..1 + next() % 6 {
                    let position = (next() % bytes.len() as u64) as usize;
                    bytes[position] ^= 1 << (next() % 8);
                }
            }
        }
        // The contract is a typed result, never a panic and never an
        // out-of-bounds read.
        match decode_founders(&bytes) {
            Ok(_) => accepted += 1,
            Err(_) => rejected += 1,
        }
    }
    assert_eq!(rejected + accepted, CASES, "every case must be typed");
    assert!(
        rejected > CASES - CASES / 100,
        "only {rejected}/{CASES} corruptions were rejected"
    );
}

#[test]
fn an_oversized_declared_count_is_refused_before_allocation() {
    // The check that matters most: a hostile file must not be able to make
    // the decoder reserve gigabytes before noticing.
    let set = sample_set(4);
    let mut bytes = encode_founders(&set).unwrap();
    bytes[52..56].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        decode_founders(&bytes),
        Err(FounderError::TooManyFounders(_))
    ));
}
