# Mutable World State Specification

Status: design specification, not implemented. Phase 12. Policy version
`lifesim-worldmod-v1`. Introduces ALIF format 2; see
`specifications/world-save-format.md`.

## The Invariant That Breaks

ALIF format 1 deliberately does not store terrain. Terrain regenerates from
`(seed, config)` on load and must match the recorded `terrain_checksum`, so
a snapshot can never be silently reinterpreted against different terrain.
This is load-bearing: it is why `sim-persist`'s restore path is fail-closed
in a way that does not depend on trusting the payload, and it is a large
part of why snapshots are as small as they are.

Organisms modifying terrain breaks it. Terrain is no longer a pure function
of `(seed, config)`.

## The Successor: Baseline Plus Verified Delta

The invariant is not abandoned. It is split in two, and both halves are
verified.

    baseline terrain = worldgen(seed, config)          // still regenerated
    world terrain    = compose(baseline, modifications) // deltas are stored

On restore:

1. Regenerate the baseline from `(seed, config)` using the unchanged
   `lifesim-worldgen-v1` generator.
2. Verify the regenerated baseline against the recorded
   `baseline_terrain_checksum`. **This is byte-for-byte the format 1 check
   and it still fails closed.** A save can still never be reinterpreted
   against a different generated world.
3. Decode the modification section, with all lengths bounds-checked before
   allocation.
4. Apply modifications in ascending `(layer_id, cell_index)` order.
5. Recompute `composed_terrain_checksum` over the result and verify it
   against the recorded value.
6. Proceed to the existing full state validation and state-checksum
   comparison.

Both checksums are recorded in the header. Corrupting either the baseline
identity or the modification set is detected before a world exists. The
property "a restore either reproduces the exact recorded world or fails with
a typed error" is preserved unchanged.

## Modification Representation

Two representations, selected by a header flag, because a sparse world and a
heavily reworked world have very different cost profiles.

**Sparse (default).** A sorted list of `(layer_id: u8, cell_index: u32,
value: i64)` overrides. Sorted ascending by `(layer_id, cell_index)`;
sortedness and uniqueness are decode-time invariants. Application is a
simple ordered scan, so it is trivially deterministic.

**Dense.** A full field per modified layer. Selected when the modified cell
count exceeds `dense_threshold_q16` of the cell count for that layer. The
threshold is versioned config, not a magic number, and the chosen
representation is recorded in the header so a reader never guesses.

Both representations carry their own section checksum. Encoding is
deterministic: the same logical modification set always produces the same
bytes, which is required for the golden-snapshot tests to mean anything.

## Mutable Layers

Kept deliberately small in Phase 12. Each added layer is a new lineage and a
new set of interactions to validate.

| Layer | Mutable | Effect | Notes |
|---|---|---|---|
| Food biomass | Already mutable | Consumption and regrowth | Unchanged; already in the snapshot |
| Traversability override | New | Blocks or permits movement through a cell | Set by placed blocking objects and by digging |
| Food capacity override | New | Raises or lowers a cell's carrying capacity `K` | Set by clearing or by deposited organic material |
| Material yield | New | Remaining extractable material in a cell | Depleted by `strike`, regenerates on a configured schedule |

**Elevation is deliberately not mutable in Phase 12.** Elevation feeds
coastline derivation, drainage, and the temperature lapse term, and the
generator validates land fraction and connectivity against it. Making it
mutable means either revalidating those invariants every tick or accepting
that a world can be modified into an invalid configuration. Neither is worth
it for the first slice. It is recorded as an open question, not a permanent
exclusion.

## Ordering And Determinism

- Modifications are applied to the live world during the `lifecycle` phase,
  in ascending `(layer_id, cell_index)` order, from a per-tick buffer
  accumulated during `apply`. Two organisms modifying the same cell in the
  same tick therefore compose in a fixed order regardless of which was
  visited first.
- Conflicting modifications to the same cell in the same tick resolve by
  (modification priority, actor object ID). Priority is a property of the
  modification kind, not of the actor.
- The modification set is world state: hashed under
  `lifesim-terrainmod-state-v1` and included in the state checksum only when
  the section is enabled, so all earlier fixtures are unaffected.
- `composed_terrain_checksum` is recomputed incrementally as modifications
  are applied, not recomputed over the full field every tick. The
  incremental and full computations must agree; a test asserts it at
  intervals, as the existing invariant checks do.

## Interaction With Worldgen Validation

The generator's invariants (land fraction, connected habitable region, water
boundary) are validated against the **baseline**, at generation time, as
today. They are not revalidated against the composed world, because
organisms are permitted to make the world worse for themselves; that is a
legitimate outcome, not a validation failure.

What is validated on the composed world every tick is narrower and is a
safety property rather than an ecological one: no organism position may be
outside bounds or on a non-land cell, no biomass may be negative or exceed
capacity, and no cell may exceed its occupancy cap. These are already
`check_invariants` obligations and extend naturally.

One consequence worth stating: organisms can, in principle, make a region
uninhabitable and drive themselves extinct there. Extinction is already a
valid, savable, observable world state with a latched event. Nothing
special is needed.

## Rendering And Streaming

The observer needs modified cells. The protocol gains a terrain-modification
delta in the keyframe and delta frames, bounded by viewport as all other
terrain data already is. This is a protocol version change
(`specifications/websocket-protocol.md`), not a silent field addition.

## Test Requirements

- Baseline check still fails closed: a save whose `(seed, config)` produces
  a different baseline is rejected with the existing typed error.
- Composed check fails closed: a tampered modification section is rejected.
- Representation equivalence: a modification set encoded sparse and dense
  restores to identical worlds with identical composed checksums.
- Threshold crossing: a world that crosses `dense_threshold_q16` mid-run
  saves, restores, and continues bit-identically.
- Format 1 to format 2 migration: a format 1 save loads through the
  registered migration, produces an empty modification set, and yields a
  world byte-identical to loading it under a format 1 reader.
- Ordering: two organisms modifying the same cell in the same tick produce
  the same result under storage permutation.
- Incremental versus full composed-checksum agreement over a long run.
- Disabled-section equality: mutable world disabled reproduces the Phase 11
  fixture exactly.
