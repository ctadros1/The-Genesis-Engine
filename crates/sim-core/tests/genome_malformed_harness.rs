//! Deterministic malformed-input harness for the genome codec boundary.
//!
//! This is NOT coverage-guided fuzzing: it is a seeded, reproducible
//! corruption sweep (bit flips, truncations, extensions, header scrambles,
//! and arbitrary buffers) against `Genome::decode`. Every case must return
//! a typed error or a fully valid genome; any panic fails the test and any
//! accepted output is re-validated and round-tripped. Set
//! `LIFESIM_MALFORMED_ITERS` to extend the sweep (default 4,000 cases).
//!
//! Minimized regression cases for every rejection path live in
//! `sim-core/src/genome.rs` unit tests; this harness adds breadth.

use sim_core::{GENOME_ENCODED_LEN, Genome, RngSystem, named_random};

const HARNESS_SEED: u64 = 0x00fa_57f0_0d5e_ed00;

fn draw(case: u64, salt: u32) -> u64 {
    named_random(HARNESS_SEED, case, RngSystem::WorldGen, case, salt)
}

fn corrupt(case: u64, base: &[u8]) -> Vec<u8> {
    match draw(case, 0) % 5 {
        // Bit flips at random positions.
        0 => {
            let mut bytes = base.to_vec();
            let flips = 1 + (draw(case, 1) % 8) as usize;
            for flip in 0..flips {
                let position = (draw(case, 2 + flip as u32) % bytes.len() as u64) as usize;
                let bit = (draw(case, 100 + flip as u32) % 8) as u32;
                bytes[position] ^= 1 << bit;
            }
            bytes
        }
        // Truncation.
        1 => {
            let length = (draw(case, 1) % (base.len() as u64 + 1)) as usize;
            base[..length].to_vec()
        }
        // Extension with random bytes.
        2 => {
            let mut bytes = base.to_vec();
            let extra = 1 + (draw(case, 1) % 64) as usize;
            for index in 0..extra {
                bytes.push((draw(case, 2 + index as u32) & 0xff) as u8);
            }
            bytes
        }
        // Header field scramble (magic, schema, topology, counts).
        3 => {
            let mut bytes = base.to_vec();
            let field = (draw(case, 1) % 4) as usize;
            let (offset, width) = [(0, 4), (4, 2), (6, 2), (8, 6)][field];
            for index in 0..width {
                bytes[offset + index] = (draw(case, 10 + index as u32) & 0xff) as u8;
            }
            bytes
        }
        // Arbitrary buffer up to twice the encoded length.
        _ => {
            let length = (draw(case, 1) % (2 * GENOME_ENCODED_LEN as u64 + 1)) as usize;
            (0..length)
                .map(|index| (draw(case, 2 + (index % 512) as u32) >> (index % 32) & 0xff) as u8)
                .collect()
        }
    }
}

#[test]
fn corrupted_inputs_never_panic_over_allocate_or_admit_invalid_records() {
    let iterations: u64 = std::env::var("LIFESIM_MALFORMED_ITERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4_000);
    let mut accepted = 0_u64;
    let mut rejected = 0_u64;
    for case in 0..iterations {
        let base = Genome::founder(HARNESS_SEED, case % 32).encode();
        let corrupted = corrupt(case, &base);
        match Genome::decode(&corrupted) {
            Ok(genome) => {
                accepted += 1;
                // Anything accepted must be fully valid and canonical.
                Genome::validated(*genome.traits(), genome.neural().to_vec())
                    .expect("accepted genome must satisfy validation");
                let reencoded = genome.encode();
                let decoded_again = Genome::decode(&reencoded).expect("round trip");
                assert_eq!(decoded_again, genome);
            }
            Err(_) => rejected += 1,
        }
    }
    // The sweep must actually exercise the rejection paths; a bit flip in
    // the payload without a checksum fix is essentially always rejected.
    assert!(rejected > iterations / 2, "rejected only {rejected}");
    // Sanity: untouched fixtures always decode.
    let valid = Genome::founder(HARNESS_SEED, 1).encode();
    assert!(Genome::decode(&valid).is_ok());
    eprintln!(
        "malformed-input harness: {iterations} cases, {accepted} accepted, {rejected} rejected"
    );
}

#[test]
fn zero_and_tiny_buffers_fail_closed() {
    for length in 0..GENOME_ENCODED_LEN.min(64) {
        let buffer = vec![0_u8; length];
        assert!(Genome::decode(&buffer).is_err());
    }
    assert!(Genome::decode(&[]).is_err());
}
