# Phase 12: Mutable World And Artifacts

Status: planned, not started. Policy versions `lifesim-material-v1`,
`lifesim-artifact-v1`, `lifesim-worldmod-v1`. Introduces **ALIF format 4**
(see the version correction below).
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

## Why This Is *Before* The Social Channel

**Corrected 2026-08-10.** This section previously argued the opposite, and
was stale text left over from before the reordering. `docs/19-implementation-
roadmap.md` records the reversal and ADR-0022 A2 triple-sources it: **11
before 12, artifacts precede signalling.**

The superseded argument was that building artifacts before a transmission
channel exists produces another inherited trait. That assumed transmission
means *signalling*, and it does not: **an artifact left behind is a
transmission channel that requires no perception of conspecifics at all.**
`cumulative_culture` 1.2 puts persistent generic artifacts inside the
*minimum viable* transmission system; `artifacts` 1.7 puts social
transmission at step 10 of 12, after carrying, reuse, caching, structures and
stigmergy; `social_organization` 1.1 puts stigmergic cooperation before
communication.

So this phase delivers objects and the **first** transmission mechanism,
stigmergy, and Phase 13 then asks whether a second, faster channel adds
anything on top of it - a sharper question, because it has a baseline to
beat. C12.3 is therefore measurable here: composite objects accumulating is
an artifact-mediated question, not a signalling one.

Two consequences of the correction, recorded rather than silently applied:

- **Phase 13 is not a prerequisite** and has been removed from the list
  below. Nothing in this phase may be deferred on the grounds that Phase 13
  does not exist yet.
- **C12.8 says "the Phase 13 fixture", which does not exist.** Under the
  corrected order the fixture this phase must preserve is the **Phase 11**
  one (config `0xae34cd2b6f7a3e13`, state `0x53b354bd94e82bcf`), along with
  Phase 1, Phase 2 and Phase 9. The criterion is not weakened by this - it is
  strengthened, because four fixtures must survive rather than one.

## Version Correction, 2026-08-10

Recorded before any code, because getting it wrong would write a migration
registry against a version that already exists.

Every sentence in this plan, in `specifications/mutable-world-state.md`, in
`specifications/world-save-format.md` and in ADR-0015 that says **"ALIF
format 2"** predates formats 2 and 3, both of which shipped for reasons
unrelated to terrain: format 2 added the Phase 6/7/8 config sections, format
3 split the Phase 2 section's two counts (D-076). The shipped version is
**3**, so the mutable-world successor is **format 4**, and
`SAVE_STATE_VERSION` goes 1 to 2.

**The "format 1 to format 2 migration" this plan demands is not
implementable and never was.** A format-1 file cannot say what its climate
settings were, and a format-2 schema-2 file physically does not contain the
per-organism state format 3 added - so neither can be transformed into a
later format without inventing data. Formats 1 and 2 therefore have **no
registered migration, by design**. The real obligation is **3 to 4**, and
C12.5's byte-identity target is the **format-3 reader**, which stays in the
build permanently.

Also found: `migration_for` returns `Result<(), String>` and has no
transform type at all, so a "registered migration" is not currently
expressible. Building one is part of this phase rather than an assumed
prerequisite.

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
- ALIF format 4 with a registered format 3 migration.

## Non-Goals

- **No recipe table, crafting system, technology tree, or tool taxonomy.**
  There is no code path anywhere that says one combination is valid and
  another is not, beyond affordance bitsets and caps.
- No reward for making anything.
- No elevation mutability. It feeds coastline derivation, drainage, and the
  temperature lapse term, and the generator validates land fraction and
  connectivity against it. Deferred, recorded as an open question, not
  permanently excluded.
- No object-mediated signalling. Signals are Phase 13's transient field;
  objects are Phase 12's persistent state. Conflating them would blur both
  results.
- No claim that an observed composite is a "tool" in any sense beyond the
  measured definition used in the acceptance criteria.

## Prerequisites

- Phase 11 (this phase's fixtures must preserve it, and lifetime learning is
  what makes an artifact something an organism can come to use rather than
  only inherit a disposition toward).
- Phase 5's asynchronous checkpointing and event log.
- **Not** Phase 13; see the corrected ordering above.

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

**Status 2026-08-10: the mutable-world half is built and verified; the
artifact half is NOT STARTED.** The two halves are separable and were built
in that order because Phase 11's C11.1 needs the terrain-override layer and
nothing else from this phase.

**Status 2026-08-16: the artifact half is IN PROGRESS under ADR-0028**,
which records the design and every departure from the commissioned review
and from the 2026-08-04 spec. Two things found on the way that this plan
must carry: (1) D-114 - no schema-2 lineage could ever bind a channel its
founder did not, so the phase adds a `bind` mutation operator, off by
default and common-mode across the four conditions, without which C12.1's
null would be guaranteed by mechanism; (2) the four conditions are realised
as settable fields of the `artifact` section (`inert` for C, `ephemeral`
for B, `max_composition_depth = 0` for D). Nothing below has changed except
where marked; the criteria's thresholds are as they were.

- [ ] **C12.1 Object actions are used, not just fired.** Under A, the rate
      of successful pick-up, place, and combine actions exceeds the rate
      under C by the stated effect size in at least 20 of 30 seeds. Condition
      C is the control that distinguishes evolved use from output channels
      firing at their baseline rate.
      **NOT BUILT.** Requires the object system, the five actions, and the
      four-condition campaign. No object exists yet. Distinct from unmet:
      nothing was measured, and no threshold above has been changed.
- [ ] **C12.2 Structures persist and matter.** Under A, the median lifetime
      of a placed object exceeds the run's median organism lifespan in at
      least 15 of 30 seeds, **and** organisms occupying cells containing
      placed objects show a measurable fitness difference (reproductive
      output or survival) against matched cells without them. Both halves
      are required: persistence without a fitness effect is litter, not
      structure.
      **NOT BUILT.** Same reason as C12.1.
- [ ] **C12.3 Cumulative dependency.** Under A, the frequency of composite
      objects of combination depth two or more increases over time in at
      least N of 30 seeds, with N stated before the campaign. Under D this
      is zero by construction. **This is the criterion most likely to return
      null and it is stated that way in advance.** A null here is a real
      result about this world's physics and is reported as such, not
      quietly dropped or replaced with a weaker measure after the fact.
      **NOT BUILT.** Same reason as C12.1. Note the ordering correction
      above: this *is* measurable in this phase once objects exist, because
      stigmergy is the transmission channel it needs, and it does not wait
      on Phase 13. `N` is still unstated and must be fixed in the
      pre-registration, before the campaign runs.
- [~] **C12.4 Determinism and identity.** Object IDs strictly increase and
      never repeat; storage index order equals ID order; contested
      acquisition is order-independent under storage permutation;
      clean-process fixture replay.
      **Status 2026-08-16: met for the artifact half as built (ADR-0028).**
      Objects and organisms draw one `next_entity_id`; `check_invariants`
      proves every tick that object ids are strictly ascending, that no
      object id is an organism id, and that `objects_allocated_total`
      equals what the counter handed out; the table is struct-of-arrays in
      id order and `retain` compacts it in lockstep. Contested acquisition
      is resolved by (priority, distance², organism id) and
      `a_contested_pick_up_is_decided_by_distance_before_id_and_never_by_visit_order`
      shows the nearer, higher-id claimant winning; the "storage
      permutation" clause is discharged the way Rule 4's is - a permuted
      table is not restorable (`RestoreError`), so the round trip
      `a_world_with_held_objects_and_composites_round_trips_and_steps_identically`
      is the test. **This phase now has its own fixture and script**:
      `scripts/verify-phase12-determinism.sh` pins the `--artifact` trace at
      8,000 ticks (config `0xc64259e739b525d4`, state `0x853d257398a2718c`),
      refuses a vacuous fixture mechanism by mechanism, and shows conditions
      B and C replaying as distinct lineages. The mutable-world half's
      modification-set clauses stand as written above.
- [x] **C12.5 Save format correctness. Met, every clause, each
      mutation-verified.** ALIF **format 4** (not 2; see the version
      correction above), `SAVE_STATE_VERSION` 2, with the format-3 reader
      retained permanently and a real `Migration` type where `migration_for`
      previously had no way to express a transform at all.
      - baseline check still fails closed on a `(seed, config)` mismatch;
      - composed check fails closed on a tampered modification section -
        verified by disabling the check and watching two tests fail;
      - sparse and dense restore to identical worlds;
      - a world crossing `dense_threshold_q16` mid-run saves, restores and
        continues bit-identically;
      - the 3-to-4 migration yields a world byte-identical to a format-3
        load, compared as `SaveState` equality, then as world equality, then
        over 200 further ticks.
      A 20,000-case corruption sweep of the modification section passes, and
      every declared per-layer count is bounded adversarially per standing
      rule 2. Two migration assertions are tautologies by construction and
      are recorded as such in D-097 rather than left to look load-bearing.
- [~] **C12.6 Mass and energy exactness.** The ledger stays exact to the
      milli-unit across creation, combination, fracture, decay, carrying,
      and consumption over a 10^6-tick run. Combining then fracturing
      restores constituent mass and energy exactly. Rounding remainders go
      deterministically to the lowest constituent object ID.
      **Status 2026-08-16: built and exact per tick; the 10^6-tick horizon
      is running.** Mass and energy are conserved quantities of the object
      table under a ten-term ledger (`ObjectLedger`), and
      `check_invariants` asserts `table = extracted + carcass - decayed -
      consumed - dust` for both, every tick, in every artifact test and in
      the trace; `combining_then_fracturing_restores_constituent_ids_mass_and_energy_exactly`
      is the restoration clause by id. Rounding remainders go where the
      criterion says: a simple object's fracture puts the division
      remainder on the first (lowest new id) fragment, a composite comes
      apart by id without rounding, and consumption's mass share leaves its
      remainder on the object; only a fragment under
      `min_fragment_mass_milli` is booked out, to dust, by name. The 10^6-tick single-world soak (`phase12-c126-ledger-soak`, seed
      12201, the confirmatory base under condition A, invariants checked
      every 5,000 ticks) was started on 2026-08-16 and its result belongs
      here when it finishes; until then this criterion is **partial by the
      horizon alone**.
- [~] **C12.7 Caps enforced visibly.** Composition depth, composition
      breadth, cell occupancy, carry capacity, and object count all reject
      deterministically, count, and event. A run that is silently pressed
      against a cap must be visible in its report.
      **Status 2026-08-16: met, with one cap named as unbindable.** Every
      refusal is typed (`RefuseReason`, 13 reasons), counted per reason,
      hashed, and evented (`ObjectActionRefused`, tag 22), and the manifest
      carries every count as `artifact_refused_*` so a run pressed against
      a cap is visible in its report; `every_cap_rejects_counts_and_events_when_driven`
      drives the world object cap, the held cap, the occupancy cap and the
      carry capacity until each binds, and
      `the_depth_cap_refuses_the_composite_that_would_exceed_it` the depth
      cap. **`max_composition_breadth` cannot bind** (D-116): a combine
      joins exactly two objects and validation refuses a breadth below two,
      so the refusal path is reachable by no valid configuration; it is
      recorded as vestigial rather than counted among the enforced caps.
      The mutable-world half's clauses stand as written above.
- [x] **C12.8 Fixtures preserved. Met, and broader than written.** With the
      section disabled, all four world fixtures reproduce exactly - Phase 1
      `0x1e3158a26afd3b39`, Phase 2 `0xff9dfcff5dffbf42`, Phase 9
      `0x5f0c4e95e4f5170f`, Phase 11 `0x53b354bd94e82bcf` - and a
      disabled-section world encodes the payload format 3 wrote.

### Measured

Benchmarks, 256x256 (65,536 cells), release:

| Quantity | Value |
|---|---|
| Tick, section disabled | 253.6 us |
| Tick, section enabled, **zero** overrides | 305.4 us |
| Tick, patch radius 8 (289 overrides) | 312.0 us |
| Tick, patch radius 45 (6,187 overrides) | 315.3 us |
| Composed checksum, full recompute | ~1,000 us (empty and at 955 overrides alike) |
| Modification writes, 1,000 / 4,000 / 16,000 entries | 48.5 / 202.2 / 1,044.9 us |

**The cost is the seam, not the data.** Enabling the section costs about
20 percent of tick time with *no* overrides at all, because every terrain
read goes through a composed accessor; going from zero to 6,187 overrides
then costs another 3 percent. That is the opposite of the shape the plan's
risk table anticipated, and it means the thing to optimise later - if
anything - is the accessor, not the representation.

**The composed checksum is a full recompute, not incremental.** FNV-1a
cannot be updated for a value changed in the middle of a stream, and a
recompute costs about a millisecond at 65,536 cells - roughly four ticks'
worth - so it is taken on a cadence rather than every tick. The
specification asks for an incremental computation cross-checked against a
full one; what is implemented is the full one, and the honest reason is that
the incremental version is not expressible under this hash. Recorded rather
than faked.

## Test Plan

- Codec: bounded fail-closed decode of the object table and modification
  section; seeded corruption sweep of at least 20,000 cases, zero panics.
- Migration: format 3 to format 4 equality test; unknown format versions
  still fail closed through the registry.
- Ledger: combine-fracture round trip; carcass energy never exceeds source;
  long-run exactness.
- Determinism: storage permutation; contested pickup symmetry; incremental
  versus full composed-checksum agreement at intervals.
- Integration: held objects dropped on death in ascending object-ID order;
  blocked-cell movement rejection; occupancy cap rejection.
- Behavioral: the C12.1 and C12.2 probes as scripted deterministic
  scenarios across all four conditions.
- Restore-from-backup: extend the existing Phase 4 isolated restore test to
  format 4 with a nonempty modification set and composite objects.

## Benchmark Impact

This phase adds entities. Objects participate in spatial indexing,
perception, and snapshots, so the cost model changes at every level.

Record: object count effect on spatial index build and query; per-tick
object decay cost as a function of object count; snapshot size contribution
of the object table and the modification section, sparse and dense;
composed-terrain checksum incremental cost; the density threshold crossing
cost; restore time with a large modification set.

Note the specific risk to measure: the Phase 4 record shows snapshot size
already dominated by per-organism genomes, and Phases 9 and 11 add to that. A
world with many persistent objects and a heavily modified terrain adds a
third growth term. The checkpoint budget must be re-verified here, not
assumed to survive from Phase 11.

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
| Save format 4 migration risk: a subtle difference between the migrated and native paths corrupts historical worlds | C12.5's byte-identity requirement is the guard; format 1 readers stay in the build; migration is registered and fail-closed, never inferred |
| Object churn dominates the tick | Decay is a bounded per-cell sweep, not a per-object scan, wherever possible; measured before adoption |
| C12.3 returns null and the phase looks like a failure | It is stated in advance as the likely outcome. The phase's value is C12.1 and C12.2 plus a measured negative on C12.3 |
| Organisms make regions uninhabitable and drive local extinction | Not a bug. Extinction is already a valid, savable, observable, latched state. Worth reporting, not preventing |
| Terrain modification interacts with worldgen validation invariants | Baseline invariants validate at generation only; the composed world is checked against narrower safety invariants each tick. Stated explicitly in `specifications/mutable-world-state.md` |

## Rollback

Objects and mutable world are separate config sections and can be disabled
independently. Disabled, every existing fixture reproduces exactly. The format
3 reader remains in the build forever; format 4 saves of worlds with both sections
disabled carry empty object and modification sections and restore
identically to a format 1 save of the same world.
