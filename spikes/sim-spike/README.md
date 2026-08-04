# Phase 0 Simulation Spike

This crate is disposable benchmark code, not the production `sim-core` API. It
tests three Phase 0 hypotheses with no network, database, wall-clock simulation,
or infrastructure dependency:

- stable-ID, fixed-point deterministic ticks over struct-of-arrays storage;
- named random draws derived from `(seed, tick, system, entity, draw)`;
- a little-endian, versioned, bounded, checksummed snapshot frame.

The snapshot payload is intentionally uncompressed. This isolates framing,
validation, checksum, and copy cost; ADR-0007's zstd choice remains proposed.

Use `scripts/verify-determinism.sh` for the two-clean-process fixture and
`scripts/run-phase0-benchmarks.sh` for the 500/2,000-organism local run.

