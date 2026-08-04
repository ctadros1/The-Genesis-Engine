# World Save Format

## Phase 4 Implementation Notes (ALIF format 1)

Implemented in `crates/sim-persist` over `sim_core::SaveState` (logical
state version 1). Header: magic `ALIF`, format version, header length,
flags (bit 0 = zstd), world ID, parent world ID, tick, seed, config hash,
save-state and genome schema versions, build-version string reference,
event-log offset (zero until the event-log file exists), uncompressed and
stored lengths, payload CRC32, state checksum, and terrain checksum —
little-endian throughout, matching the kernel's canonical hashing.
Payload sections (tagged, length-prefixed, per-section CRC32): config,
world metadata, organism table, biomass field, ledger/counters, and the
Phase 2 table (genomes, controller memory, heading/speed, ancestry).
Static terrain is deliberately not stored: it regenerates from
`(seed, config)` on load and must match the recorded terrain checksum, so
a snapshot can never be silently reinterpreted against different terrain.
All lengths are capped before allocation or decompression; decoded state
passes full kernel validation (genome validity, ordering, bounds, exact
ledger conservation) and the recorded state checksum before a world
exists. Unknown format versions fail closed through the explicit
migration registry (`sim_persist::migration_for`); no transforms are
registered yet because only format 1 exists. Atomic write, catalog
ordering, recovery, and restore-verification behavior follow the Write
Contract and Restore Test sections below and are covered by the
`sim-persist` test suite.

## Planned Successor: ALIF Format 2 (Phase 10)

Design: `specifications/mutable-world-state.md`. Decision: ADR-0015.

The format 1 property that terrain is not stored, regenerates from
`(seed, config)`, and is checksum-verified is load-bearing and cannot
survive organism-modified terrain. It is **split, not abandoned**:

- `baseline_terrain_checksum`: the regenerated baseline is still verified
  against `(seed, config)` using the unchanged `lifesim-worldgen-v1`
  generator. This is byte-for-byte the format 1 check and still fails
  closed. A save still cannot be reinterpreted against a different generated
  world.
- `composed_terrain_checksum`: verified after the stored modification delta
  is applied in ascending `(layer_id, cell_index)` order.

New payload sections, each tagged, length-prefixed, and per-section
checksummed like the existing ones:

| Section | Contents | Present when |
|---|---|---|
| Contest | Health, damage counters | contest enabled |
| Genome 2 | Diploid variable-topology genomes, `next_innovation_id` | schema 2 |
| Activations | Per-node activation vector | schema 2 |
| Learned state | Per-plastic-edge Q16 deltas and traces, sparse | plasticity enabled |
| Signal field | Committed signal field | social enabled |
| Objects | Artifact table, composition lists, per-cell occupancy | artifacts enabled |
| Terrain modification | Sparse or dense delta, flagged in the header | mutable world enabled |
| Physiology | Developmental stage, hazard, disease load | physiology enabled |

Header changes: `next_entity_id` becomes `next_object_id` (organisms and
artifacts share one monotonic ID space); the baseline and composed terrain
checksums replace the single `terrain_checksum`; a representation flag
selects sparse or dense modification encoding; the event-log offset stops
being zero once Phase 5 lands the log file.

Save-state version increments to 2.

### Migration, and what it must not do

**Format 1 is never reinterpreted in place.** A format 1 file loads through
a registered transform in `sim_persist::migration_for` that produces an
empty modification set, an empty object table, and absent optional sections.
The acceptance requirement is byte identity: the migrated result must equal
the world produced by loading the same file under a format 1 reader. Format
1 readers and their tests stay in the build permanently so that comparison
is always available.

Unknown format versions continue to fail closed through the registry.

## File Layout

Header: magic ALIF, format version, header length, flags, world ID, parent world ID, tick, world seed, simulation/build version string reference, config hash, generator/genome schema versions, uncompressed length, compressed length, payload checksum.

Payload sections: world metadata; terrain/static fields; dynamic environmental fields; entity component tables; genome table; event-log checkpoint reference; deterministic RNG/config state; optional analytics summary. Sections have tagged IDs, lengths, and per-section checksums when added.

## Write Contract

Write temporary file in destination filesystem, flush, checksum, atomically rename, then commit catalog metadata. A catalog record never claims a successful save until the final file validates. Save format must be endian-defined; all decoded lengths are capped before allocation/decompression.

## Migration

A migration declares source/target format, supported semantic versions, transform, tests, expected loss if any, and rollback. Unknown save versions fail closed with an actionable error. Never deserialize raw Rust layout or rely on compiler struct order.

## Restore Test

Load a save in an isolated destination, validate checksum/header/schema, rebuild derived indexes, pause at recorded tick, compare documented state checksum, and only then make it eligible for a world branch or replacement.
