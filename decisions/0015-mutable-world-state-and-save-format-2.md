# ADR-0015: Mutable World State And Save Format 2

Status: Proposed
Date: 2026-08-04
Author: Goal revision

Extends ADR-0007 (persistence format), which remains Proposed. The framed
binary snapshot plus zstd plus SQLite catalog choice is unchanged; this ADR
concerns what the payload must now contain.

## Context

Persistent structures require that organisms can change the world and that
the change outlives them. ALIF format 1 deliberately does not store terrain:
it regenerates from `(seed, config)` on load and verifies against the
recorded `terrain_checksum`, so a snapshot can never be silently
reinterpreted against different terrain.

That invariant is load-bearing. It is a large part of why the restore path
fails closed without trusting the payload, and a large part of why snapshots
are as small as they are. Organism-modified terrain breaks it: terrain is no
longer a pure function of `(seed, config)`.

## Options Considered

- **Keep terrain immutable.** Preserves the invariant and makes persistent
  structures impossible. Objects alone could sit on top of immutable terrain,
  which covers some of the goal but not digging, clearing, or blocking.
- **Store the full composed terrain.** Simple and correct. Discards the
  regeneration check entirely, so a corrupted or substituted terrain section
  would be accepted as long as its own checksum matched, and it grows every
  snapshot by the full field size regardless of how little was modified.
- **Reinterpret format 1 to mean "baseline" and add a delta section.**
  Rejected outright: format 1 files would silently acquire a new meaning,
  which is exactly the reinterpretation the project's rules forbid.
- **Baseline plus verified delta, as a new format 2 with a registered
  migration.**

## Proposed Decision

Adopt baseline-plus-verified-delta as ALIF format 2, specified in
`specifications/mutable-world-state.md` and
`specifications/world-save-format.md`.

The invariant is split and both halves are verified on every restore:

1. Regenerate the baseline from `(seed, config)` with the unchanged
   `lifesim-worldgen-v1` generator.
2. Verify it against `baseline_terrain_checksum`. This is byte-for-byte the
   format 1 check and it still fails closed. **A save still cannot be
   reinterpreted against a different generated world.**
3. Decode the modification section with every length capped before
   allocation.
4. Apply modifications in ascending `(layer_id, cell_index)` order.
5. Recompute `composed_terrain_checksum` and verify it.
6. Proceed to the existing full state validation and state-checksum
   comparison.

Supporting decisions:

- **Two representations, sparse and dense**, selected by a header flag, with
  the threshold as versioned config rather than a magic number. A sparse
  world and a heavily reworked world have very different cost profiles and a
  single representation would be wrong for one of them. The chosen
  representation is recorded so a reader never guesses.
- **Format 1 is never reinterpreted.** Format 1 saves load through a
  registered transform in `sim_persist::migration_for` that produces an
  empty modification set, and the result must be byte-identical to loading
  the same file under a format 1 reader. Format 1 readers and tests stay in
  the build permanently.
- **Elevation stays immutable in this slice.** It feeds coastline
  derivation, drainage, and the temperature lapse term, and the generator
  validates land fraction and connectivity against it. Making it mutable
  means either revalidating those invariants every tick or accepting that a
  world can be modified into an invalid configuration. Deferred and recorded
  as an open question, not permanently excluded.
- **Objects share the organism ID space.** One monotonic `next_object_id`
  with a type tag, so there is one total order over everything in the world
  and no cross-space ordering policy is needed. `next_entity_id` in
  save-state version 1 becomes `next_object_id` in version 2; the migration
  copies the value.
- **Baseline worldgen validation is not re-run against the composed world.**
  Organisms are permitted to make the world worse for themselves; that is a
  legitimate outcome, not a validation failure. What is checked each tick is
  narrower and is a safety property: positions in bounds and on land,
  biomass within capacity, occupancy within caps.

## Consequences

Positive: persistent structures become possible; the fail-closed restore
property is preserved rather than weakened; snapshot growth is proportional
to modification rather than to world size; format 1 history is intact.

Negative and accepted:

- A second checksum and a second failure mode on every restore.
- **Migration risk is the largest operational risk in the plan.** A subtle
  difference between the migrated and native paths would corrupt historical
  worlds silently. The guard is the byte-identity requirement in Phase 10
  criterion C10.5, plus keeping format 1 readers in the build so the
  comparison is always available.
- Snapshot growth from the object table and modification section stacks on
  top of schema 2 genome growth (ADR-0013) and learned state (ADR-0014).
  The checkpoint budget must be re-verified in Phase 10 rather than assumed
  to survive from Phase 8.
- Organisms can make a region uninhabitable and drive local extinction.
  Extinction is already a valid, savable, observable, latched state, so
  nothing special is needed, but it is worth stating that this is an
  accepted outcome rather than a bug.

## Performance Implications

Unmeasured. Phase 10 records: object count effect on spatial index build and
query; per-tick decay cost; snapshot size contribution of the object table
and modification section in both representations; incremental
composed-checksum cost; restore time with a large modification set.

The Phase 4 record shows snapshot size already dominated by per-organism
genomes; this ADR adds a third growth term and the interaction is the thing
to measure.

## Operational Implications

- Backup sets are unchanged in shape: snapshots, catalog, config, and now
  the event log, backed up together.
- The restore runbook gains the composed-terrain verification step.
- Protocol version change for terrain modification deltas to the observer.

## Revisit Conditions

- Modification density makes the sparse representation useless in practice,
  suggesting the threshold or the representation choice is wrong.
- Snapshot growth breaks the checkpoint budget at a supported tier.
- Elevation mutability becomes necessary for a research question, requiring
  its own ADR and a revalidation strategy.

## Evidence Required To Accept

- Phase 10 criteria C10.4 through C10.7, in particular the format 1
  migration byte-identity test and the sparse-dense equivalence test.
- Snapshot size, checkpoint stall, and restore time at both supported tiers
  with realistic object counts and modification density.
- A corruption sweep of at least 20,000 cases over the object table and
  modification section with zero panics.
- The Phase 4 restore-from-backup integration test extended to format 2.
- Explicit approval, since this changes durable data semantics.
