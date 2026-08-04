# Phase 11: Mutable World And Artifacts

Status: planned, not started. Policy versions `lifesim-material-v1`,
`lifesim-artifact-v1`, `lifesim-worldmod-v1`. Introduces ALIF format 2.
Specifications: `specifications/artifact-and-material-ontology.md`,
`specifications/mutable-world-state.md`,
`specifications/world-save-format.md`.

## Problem

The world is immutable by organisms and there is no object ontology at all:
no materials, no items, no carrying, no combination. Tools and structures
require objects that exist independently of organisms, persist after their
makers die, and have physical properties an organism can exploit.

Terrain is currently regenerated from `(seed, config)` and checksum-verified
rather than stored, which is load-bearing for the snapshot format. This
phase breaks that invariant and replaces it.

## Why This Is After The Social Channel

Building artifacts before a transmission channel exists would produce
another inherited trait and teach us nothing. A world where organisms
construct things but cannot learn from each other tests whether construction
is genetically encodable, which is a much less interesting question than
whether construction accumulates. Every acceptance criterion below that
matters (C11.3 in particular) depends on transmission existing.

The order also means that if Phase 12 returns a clean null, this phase can be
entered knowingly, with its cumulative-dependency criterion understood in
advance to be unlikely, rather than being surprised by it.

## Scope

- A bounded material registry with physical properties and affordances.
- Objects as first-class entities in the shared object ID space.
- Actions: pick up, drop, place, strike, combine.
- Carrying with mass-proportional movement cost and body-scale capacity.
- Fracture: striking a target harder than its hardness produces fragments.
- Combination: composite properties derive from constituents by a stated
  function. **No recipe list anywhere in the system.**
- Object perception channels.
- Mutable terrain layers: traversability override, food capacity override,
  material yield.
- Carcasses become real objects, joining Phase 7's carcass work to the
  object system.
- ALIF format 2 with a registered format 1 migration.

## Non-Goals

- **No recipe table, crafting system, technology tree, or tool taxonomy.**
  There is no code path anywhere that says one combination is valid and
  another is not, beyond affordance bitsets and caps.
- No reward for making anything.
- No elevation mutability. It feeds coastline derivation, drainage, and the
  temperature lapse term, and the generator validates land fraction and
  connectivity against it. Deferred, recorded as an open question, not
  permanently excluded.
- No object-mediated signalling. Signals are Phase 12's transient field;
  objects are Phase 11's persistent state. Conflating them would blur both
  results.
- No claim that an observed composite is a "tool" in any sense beyond the
  measured definition used in the acceptance criteria.

## Prerequisites

- Phase 12 (transmission must exist for C11.3 to be meaningful).
- Phase 5's asynchronous checkpointing and event log.

## Determinism Notes

- New streams: `Artifact` (13), `MaterialYield` (14).
- Objects share the monotonic object ID counter with organisms (Rule 2), so
  there is one total order over everything in the world and no cross-space
  tie-break policy is needed.
- Object storage is struct-of-arrays sorted by object ID, compacted on
  removal, ordering verified by `check_invariants`.
- Per-cell object lists sorted by object ID before any order-sensitive use.
- Contested acquisition resolved by (priority, distance squared, object ID),
  then by a `lifesim-pairkey-v1` lottery if configured.
- Terrain modifications accumulate during `apply` and apply during
  `lifecycle` in ascending `(layer_id, cell_index)` order.
- Checksum sections `lifesim-object-state-v1` and
  `lifesim-terrainmod-state-v1`.
- Object integrity, disease-style accumulators, and modification values are
  fixed point (Rule 7).

## The Save Format Successor

The terrain regeneration invariant does not survive, and it is not silently
weakened. It is split and both halves are verified:

1. Regenerate the baseline from `(seed, config)` with the unchanged
   `lifesim-worldgen-v1` generator.
2. Verify the regenerated baseline against `baseline_terrain_checksum`.
   **This is byte-for-byte the format 1 check and still fails closed.**
3. Decode the modification section with all lengths capped before
   allocation.
4. Apply modifications in ascending `(layer_id, cell_index)` order.
5. Recompute and verify `composed_terrain_checksum`.
6. Proceed to existing full state validation and state-checksum comparison.

Format 1 is never reinterpreted in place. A format 1 save loads through a
registered migration in `sim_persist::migration_for` that produces an empty
modification set, and the result must be byte-identical to loading the same
file under a format 1 reader. Format 1 readers and tests stay in the build
permanently.

## Acceptance Criteria

Conditions, matched on seeds (30), config, and run length:

- **A**: artifacts persistent, combination enabled.
- **B**: artifacts ephemeral; a placed object decays to nothing within one
  tick. Actions exist and cost the same; only persistence is removed.
- **C**: actions permitted but physically inert; pick up, place, and combine
  succeed, cost energy, and confer no physical effect. This separates
  "actions fire" from "actions pay".
- **D**: combination disabled; simple objects only.

Criteria:

- [ ] **C11.1 Object actions are used, not just fired.** Under A, the rate
      of successful pick-up, place, and combine actions exceeds the rate
      under C by the stated effect size in at least 20 of 30 seeds. Condition
      C is the control that distinguishes evolved use from output channels
      firing at their baseline rate.
- [ ] **C11.2 Structures persist and matter.** Under A, the median lifetime
      of a placed object exceeds the run's median organism lifespan in at
      least 15 of 30 seeds, **and** organisms occupying cells containing
      placed objects show a measurable fitness difference (reproductive
      output or survival) against matched cells without them. Both halves
      are required: persistence without a fitness effect is litter, not
      structure.
- [ ] **C11.3 Cumulative dependency.** Under A, the frequency of composite
      objects of combination depth two or more increases over time in at
      least N of 30 seeds, with N stated before the campaign. Under D this
      is zero by construction. **This is the criterion most likely to return
      null and it is stated that way in advance.** A null here is a real
      result about this world's physics and is reported as such, not
      quietly dropped or replaced with a weaker measure after the fact.
- [ ] **C11.4 Determinism and identity.** Object IDs strictly increase and
      never repeat; storage index order equals ID order; contested
      acquisition is order-independent under storage permutation;
      clean-process fixture replay.
- [ ] **C11.5 Save format correctness.** Baseline check still fails closed
      on a `(seed, config)` mismatch; composed check fails closed on a
      tampered modification section; sparse and dense representations
      restore to identical worlds; a world crossing the density threshold
      mid-run saves, restores, and continues bit-identically; the format 1
      migration produces a world byte-identical to a format 1 load.
- [ ] **C11.6 Mass and energy exactness.** The ledger stays exact to the
      milli-unit across creation, combination, fracture, decay, carrying,
      and consumption over a 10^6-tick run. Combining then fracturing
      restores constituent mass and energy exactly. Rounding remainders go
      deterministically to the lowest constituent object ID.
- [ ] **C11.7 Caps enforced visibly.** Composition depth, composition
      breadth, cell occupancy, carry capacity, and object count all reject
      deterministically, count, and event. A run that is silently pressed
      against a cap must be visible in its report.
- [ ] **C11.8 Fixtures preserved.** Artifacts and mutable world disabled
      reproduces the Phase 12 fixture exactly.

## Test Plan

- Codec: bounded fail-closed decode of the object table and modification
  section; seeded corruption sweep of at least 20,000 cases, zero panics.
- Migration: format 1 to format 2 equality test; unknown format versions
  still fail closed through the registry.
- Ledger: combine-fracture round trip; carcass energy never exceeds source;
  long-run exactness.
- Determinism: storage permutation; contested pickup symmetry; incremental
  versus full composed-checksum agreement at intervals.
- Integration: held objects dropped on death in ascending object-ID order;
  blocked-cell movement rejection; occupancy cap rejection.
- Behavioral: the C11.1 and C11.2 probes as scripted deterministic
  scenarios across all four conditions.
- Restore-from-backup: extend the existing Phase 4 isolated restore test to
  format 2 with a nonempty modification set and composite objects.

## Benchmark Impact

This phase adds entities. Objects participate in spatial indexing,
perception, and snapshots, so the cost model changes at every level.

Record: object count effect on spatial index build and query; per-tick
object decay cost as a function of object count; snapshot size contribution
of the object table and the modification section, sparse and dense;
composed-terrain checksum incremental cost; the density threshold crossing
cost; restore time with a large modification set.

Note the specific risk to measure: the Phase 4 record shows snapshot size
already dominated by per-organism genomes, and Phases 8 and 10 add to that. A
world with many persistent objects and a heavily modified terrain adds a
third growth term. The checkpoint budget must be re-verified here, not
assumed to survive from Phase 10.

Benchmark schema 7.

## Documentation Updates

`docs/05-world-model.md` (world editing by organisms, mutable layers),
`docs/04-simulation-model.md`, `docs/06-organism-model.md` (carrying,
carcasses), `docs/12-data-storage-and-saves.md`,
`docs/02-scope-and-non-goals.md` (construction moves from deferred),
`specifications/world-save-format.md`,
`specifications/entity-component-model.md`,
`specifications/event-schema.md`, `specifications/websocket-protocol.md`
(terrain modification deltas, protocol version change),
`specifications/simulation-tick.md`, `specifications/metrics-schema.md`,
`docs/10-observer-interface.md`, decision log, ADR-0015.

## Risks

| Risk | Mitigation |
|---|---|
| Snapshot growth from objects plus modified terrain plus schema 2 genomes plus learned state breaks the checkpoint budget | Measured here explicitly rather than assumed; sparse representations; asynchronous checkpointing; object count caps |
| Save format 2 migration risk: a subtle difference between the migrated and native paths corrupts historical worlds | C11.5's byte-identity requirement is the guard; format 1 readers stay in the build; migration is registered and fail-closed, never inferred |
| Object churn dominates the tick | Decay is a bounded per-cell sweep, not a per-object scan, wherever possible; measured before adoption |
| C11.3 returns null and the phase looks like a failure | It is stated in advance as the likely outcome. The phase's value is C11.1 and C11.2 plus a measured negative on C11.3 |
| Organisms make regions uninhabitable and drive local extinction | Not a bug. Extinction is already a valid, savable, observable, latched state. Worth reporting, not preventing |
| Terrain modification interacts with worldgen validation invariants | Baseline invariants validate at generation only; the composed world is checked against narrower safety invariants each tick. Stated explicitly in `specifications/mutable-world-state.md` |

## Rollback

Objects and mutable world are separate config sections and can be disabled
independently. Disabled, the Phase 12 fixture reproduces exactly. ALIF format
1 remains readable forever; format 2 saves of worlds with both sections
disabled carry empty object and modification sections and restore
identically to a format 1 save of the same world.
