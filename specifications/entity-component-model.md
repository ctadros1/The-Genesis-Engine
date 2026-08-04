# Entity And Component Model

## Phase 1 Implementation Notes

`sim-core` stores organisms as parallel dense arrays (`ids`, `x_fp`, `y_fp`,
`energy_milli`, `age_ticks`, `cooldown_ticks`) kept permanently sorted by
stable 64-bit entity ID: births append strictly increasing IDs and removal
compacts in order, so index order always equals ID order and
`check_invariants` verifies it. IDs never recycle.

Phase 2 (config-gated) adds lockstep parallel arrays for GenomeRef
(validated genome, canonical hash), Phenotype (derived fixed-point
attributes, recomputed and cross-checked by invariants), Controller (four
bounded memory values, heading, speed, last turn), and Reproduction/
ancestry (two immutable parent IDs, ancestry depth, child count, creation
tick). These arrays exist only when Phase 2 is enabled and compact with the
same removal flags as the primary arrays. Carcass components remain
unimplemented.

## Planned Successors (Phases 7 To 11)

**One shared object ID space.** Organisms and artifacts draw from a single
monotonic `next_object_id` with a type tag, so there is one total order over
everything in the world and no cross-space tie-break policy is needed. The
existing "index order equals ID order, IDs never recycle" invariant
generalizes unchanged and is verified for artifacts exactly as it is for
organisms. Organism IDs become sparse; nothing depends on their density.

Every per-cell or per-bucket membership list is sorted by object ID before
any order-sensitive iteration. A bucket built by scan order is an
implementation detail and may never be read in scan order.

New components, each present only when its config section is enabled and
compacting with the same removal flags as the primary arrays:

| Component | Fields | Phase |
|---|---|---|
| Health | health, accumulated damage (fixed point) | 6 |
| Carcass | remaining energy, decay state, source entity ID (the entry below, finally implemented) | 6 |
| Genome2 | diploid chromosomal genome, canonical hash; expression recomputed on load | 7 |
| Activation | per-node activation vector (world state under synchronous evaluation) | 7 |
| Learned | sparse `(edge_innovation_id, learned_q16, trace_q16)` for plastic edges only, sorted by innovation ID | 8 |
| Inventory | held object IDs, carried mass | 10 |
| Object | material, position, integrity, holder, composition list, depth, creator, created tick | 10 |
| Physiology | developmental stage, accumulated hazard, disease load (fixed point) | 11 |

Learned state is stored sparsely rather than densely because the Phase 4
record already shows snapshot size dominated by per-organism genome arrays;
a dense learned copy of every weight would roughly double it.

The existing Serialization rule holds and gains force: expression, phenotype,
genome hashes, and spatial buckets are all recomputed on load and never
trusted from a save. Learned state is the exception that proves it: it
**cannot** be recomputed from the genome, which is exactly why it must be
saved and checksummed.

## Recommended Storage

Use a stable EntityId and data-oriented dense component arrays. Each component store has a dense value array, parallel EntityId array, and ID-to-index map. Iterate a canonical sorted dense view or a maintained stable allocation order that is proven deterministic. Avoid a generic dynamic ECS runtime until benchmarked.

## Core Components

| Component | Required Fields |
|---|---|
| Identity | entity ID, world ID, birth tick, parent IDs |
| Transform | x/y, heading, velocity |
| Life | energy, health, age, life state, death cause |
| GenomeRef | schema/version/hash and immutable genome data/ref |
| Phenotype | derived bounds, senses, metabolism, visual traits |
| Controller | fixed memory/output buffers |
| Reproduction | maturity, readiness, cooldown, offspring count |
| Spatial | current cell/bucket index |
| Carcass | remaining energy, decay state, source entity ID |

## Lifecycle Rules

Allocation occurs only in a defined birth/spawn phase after capacity validation. Removal is logical first, physical cleanup later. Entity IDs never recycle in a world lineage. Components never retain references that outlive an entity; relationship lookup uses IDs and validated indexes.

## Serialization

Snapshot logical components in schema-declared stable field order. Rebuild derived indexes after load; do not persist pointer/index artifacts as source of truth.
