# Determinism Extensions Specification

## Purpose

Phases 5 through 12 add learning, signalling, artifacts, mutable terrain,
variable-topology genomes, and multi-world scheduling. Each is a fresh
opportunity to introduce nondeterminism. This document is the single
registry of the extended determinism contract: named RNG streams, ordering
rules, checksum composition, and the policy-version successors. It is
normative. A design that satisfies every other specification but violates
this one is rejected.

Nothing here loosens ADR-0010 or ADR-0011. The declared replay guarantee
remains clean-process equality on the same build, platform, and toolchain.
Cross-platform byte equality is still not claimed.

## Rule 0: The Fixture Preservation Guarantee

Phase 1 fixture `0x1e3158a26afd3b39` and Phase 2 fixture
`0xff9dfcff5dffbf42` must remain reproducible at every future phase, from
clean processes, using the same seed and config.

This is achieved by one mechanism, already proven by D-014: **every new
subsystem is a config section that is behaviorally inert when disabled, is
folded into the canonical config hash only when enabled, and appends its
state to the checksum only when present.** A world with all new sections
disabled executes the same code paths and produces the same bytes as
before.

Two corollaries that implementers get wrong:

- Adding a variant to `RngSystem` does not change any existing stream,
  because streams are keyed by the enum's numeric value and existing values
  are never renumbered. Appending is safe; reordering is a lineage break.
- Adding a `TickPhase` variant changes `TickPhase::ALL` and therefore the
  shape of per-phase benchmark records. It does not change simulation
  results as long as the new phase is empty when its section is disabled.
  Benchmark schema version increments; historical records stay valid but
  are only per-phase comparable within a schema version.

## Named RNG Streams

The derivation is unchanged:

    rng_key = named_random(world_seed, tick, system, subject_id, draw_index)

`RngSystem` values are permanent. Existing values (do not modify):

| Value | Name | Added |
|---:|---|---|
| 1 | WorldGen | Phase 1 |
| 2 | Spawn | Phase 1 |
| 3 | Movement | Phase 1 |
| 4 | Reproduction | Phase 1 |
| 5 | GenomeInit | Phase 2 |
| 6 | Recombination | Phase 2 |

Reserved successors, appended in phase order:

| Value | Name | Phase | Draws it owns |
|---:|---|---|---|
| 7 | Contest | 6 | Contest tie lottery, damage variance, retreat resolution |
| 8 | Meiosis | 7 | Crossover count and crossover positions per chromosome pair |
| 9 | StructuralMutation | 7 | Duplication, deletion, insertion, transposition site and extent |
| 10 | PlasticityInit | 8 | Initial plastic-state seeding at birth (zero by default policy; the stream exists so a nonzero policy does not renumber) |
| 11 | Signal | 9 | Emission noise and channel corruption |
| 12 | Perception | 9 | Selection among tied perception candidates |
| 13 | Artifact | 10 | Placement site selection, combination outcome variance, fracture products |
| 14 | MaterialYield | 10 | Extraction yield variance |
| 15 | Development | 11 | Ontogenetic variance at developmental checkpoints |
| 16 | Mortality | 11 | Age-dependent and extrinsic hazard draws |

Values 17 and above are unallocated. An analysis module never draws from any
stream; if an analysis needs sampling it uses a separate deterministic
sampler seeded from its report parameters, and its output cannot reach world
state (ADR-0016).

## Rule 1: Pairwise Draws Use A Canonical Pair Key

Any random draw whose outcome concerns two organisms, or an organism and an
object, must not depend on which of them the tick happened to visit first.
`subject_id` for such a draw is the canonical pair key, policy
`lifesim-pairkey-v1`:

    pair_key(p, q) = mix64(min(p, q)) ^ rotate_left(mix64(max(p, q)), 32)

where `mix64` is the existing splitmix-style finalizer in `sim-core::rng`.

This applies to contest resolution, contested pickup, imitation-candidate
tie-breaking, and any transfer between two entities. It does not apply to
draws that are genuinely one-sided (an organism's own movement jitter), which
keep the actor's ID as subject.

Meiosis is a deliberate exception and uses the prospective child's entity ID
as subject, matching the existing Phase 2 convention that all child-keyed
draws use the prospective child ID. Parent roles inside meiosis are assigned
by ID comparison (see Rule 3), not by traversal order, so the exception is
safe.

## Rule 2: One Shared Monotonic Object ID Space

Organisms and artifacts share a single monotonic `next_object_id` counter
with a type tag. IDs are strictly increasing, never reused within a world
lineage, and define one total order over everything in the world.

Rationale: a single order removes the need for any cross-space tie-break
policy, and the existing "index order equals ID order" invariant generalizes
unchanged. The cost is that organism IDs become sparse; nothing depends on
their density.

Consequences that must be honored:

- Artifact storage is struct-of-arrays sorted by object ID, compacted on
  removal in the same manner as organisms, and `check_invariants` verifies
  the ordering exactly as it does for organisms.
- Every per-cell or per-bucket membership list is sorted by object ID before
  it is iterated for any order-sensitive purpose. A bucket built by scan
  order is an implementation detail and may never be read in scan order.
- `next_entity_id` in save-state version 1 becomes `next_object_id` in
  version 2; the registered format 1 to format 2 migration copies the value
  and writes an empty artifact table.

## Rule 3: Roles Are Assigned By ID Comparison, Not Traversal

Wherever an interaction has asymmetric roles, the role assignment is a pure
function of the participants' IDs.

- Reproduction: haplotype slot 0 comes from the lower-ID parent, slot 1 from
  the higher-ID parent. Phase 2 already uses the same convention for the
  investment rounding remainder.
- Contest: initiator and responder are determined by the action intents, and
  when both are symmetric the lower ID is the initiator.
- Contested resource or object acquisition: resolved by (priority, distance
  squared, object ID), then by a pair-key lottery if configuration enables
  one.

## Rule 4: Social Learning Is Read-Prior, Commit-After

No organism may observe another organism's *current-tick* state. All
perception reads the previous tick's committed state. All learning updates
are computed into a shadow buffer during the controller phase and committed
in a dedicated phase after every organism has been evaluated.

This is the exact pattern the existing controller memory commit already
uses, and it is what makes "if A learns from B, the outcome does not depend
on array traversal order" true by construction rather than by inspection.
If A learns from B and B learns from A in the same tick, both read frozen
prior state, so the exchange is simultaneous and symmetric.

Required test, mirroring the existing insertion-order test: permuting the
internal storage order of a saved population and restoring it produces
identical checksums for the next N ticks.

## Rule 5: Candidate Sets Are Sorted Before Selection

Any "nearest neighbour", "visible conspecific", "reachable object", or
"imitation model" set is materialized, sorted by `(distance_squared,
object_id)`, truncated to a configured maximum K, and only then consumed.
Selection within the truncated set uses a named draw. Spatial bucket scan
order never reaches a decision.

## Rule 6: Float Summation Order Is Fixed By Identifier

Variable-topology evaluation sums an arbitrary number of incoming edges per
node. Float addition is not associative, so the summation order is part of
the policy: **incoming edges are summed in ascending edge innovation-ID
order**, independent of storage layout. Node activation itself is order-free
because evaluation is synchronous (see
`specifications/neural-network-schema.md`), but the per-node sum is not, and
must be pinned.

## Rule 7: Anything That Accumulates Over A Lifetime Is Fixed Point

Learned synaptic state accumulates across up to 10^5 or more ticks. Float
accumulation over that many steps amplifies precisely the reassociation and
contraction differences ADR-0011 is designed to avoid, and turns a policy
that is currently safe into one that is fragile.

Therefore learned weight state is stored and accumulated as Q16 fixed point
in `i32`, effective weight is `clamp(genome_weight + q16_to_f32(learned),
-8.0, 8.0)`, and the f32-to-Q16 conversion of the plasticity update uses
round-half-away-from-zero, specified exactly in
`specifications/plasticity-and-learning.md`.

The same rule applies to any future integrator: object integrity, disease
load, developmental progress, and accumulated damage are all fixed point.

## Rule 8: New State Enters The Checksum Under Its Own Tag

`World::state_checksum` keeps preamble tag `lifesim-state-v1` and its
existing field order unchanged. Each new subsystem appends a tagged section
only when that subsystem's state exists:

| Section tag | Contents | Present when |
|---|---|---|
| `lifesim-phase2-state-v1` | Existing Phase 2 per-organism state | phase2 enabled (unchanged) |
| `lifesim-contest-state-v1` | Health, damage counters | contest enabled |
| `lifesim-genome2-state-v1` | Variable-topology genomes, innovation counter | genome schema 2 |
| `lifesim-activation-state-v1` | Per-node activation vector | genome schema 2 |
| `lifesim-learn-state-v1` | Per-plastic-edge Q16 learned deltas, modulator state | plasticity enabled |
| `lifesim-signal-state-v1` | Committed signal field | social enabled |
| `lifesim-object-state-v1` | Artifact table, per-cell occupancy | artifacts enabled |
| `lifesim-terrainmod-state-v1` | Terrain override deltas, composed terrain checksum | mutable world enabled |
| `lifesim-physiology-state-v1` | Developmental stage, accumulated hazard, disease load | physiology enabled |

Sections are appended in the order listed. Adding a section to the end never
changes the checksum of a world that lacks it.

Non-finite values may not reach the checksum. Every f32 hashed by `to_bits`
is clamped and validated first; a non-finite value is neutralized, counted,
and evented, following the existing controller-fault policy.

## Rule 9: Every Behavior Change Gets A New Policy Version

Never redefine an existing policy string. Successors, all folded into the
canonical config hash only when their section is enabled:

| Existing | Successor | Scope |
|---|---|---|
| `phase1-behavior-v1` | unchanged | Phase 1 worlds |
| `phase2-behavior-v1` | unchanged | Phase 2 worlds |
| - | `contest-behavior-v1` | Damage, contest, territory (Phase 6) |
| `lifesim-genome-v1` | `lifesim-genome-v2` | Diploid variable-topology genome (Phase 7) |
| `lifesim-controller-v1` | `lifesim-controller-v2` | Synchronous variable-topology evaluation (Phase 7) |
| `uniform-bounded-v1` | `lifesim-meiosis-v1` | Recombination and crossover (Phase 7) |
| - | `lifesim-structmut-v1` | Structural mutation operators (Phase 7) |
| - | `lifesim-pairkey-v1` | Pairwise draw subject derivation (Phase 6) |
| - | `lifesim-plasticity-v1` | Learning rule registry and update arithmetic (Phase 8) |
| - | `lifesim-social-v1` | Perception and signalling (Phase 9) |
| - | `lifesim-material-v1` | Material registry and properties (Phase 10) |
| - | `lifesim-artifact-v1` | Object actions and combination physics (Phase 10) |
| - | `lifesim-worldmod-v1` | Terrain modification rules (Phase 10) |
| - | `lifesim-physiology-v1` | Allometry, ontogeny, senescence (Phase 11) |
| `lifesim-worldgen-v1` | unchanged | Baseline terrain generation stays identical |

Analysis policies are recorded in reports and are deliberately **not** in the
config hash, because an analysis version can never affect a world:
`lifesim-similarity-v1` (existing), `lifesim-era-v1`,
`lifesim-tradition-v1`.

Save framing versions are separate again: ALIF format 1 (existing), ALIF
format 2 (Phase 10). See `specifications/world-save-format.md`.

## Rule 10: Scheduling Never Reaches The Kernel

Multi-world execution runs independent worlds in separate schedulable units
that share no mutable state. The required proof is an equality test, not an
argument: for a fixed seed set and config, per-world final state checksums
must be identical at scheduler concurrency 1, 2, and C, and identical to
single-world execution of the same seed. Any work-stealing, thread-count,
or completion-order dependency shows up as a checksum difference.

Within a world, parallelism remains opt-in and gated on ADR-0010's existing
requirement of an ordering and reduction policy plus equality tests. Nothing
in Phases 5 through 12 authorizes intra-world parallelism.

## Test Obligations

Every phase that touches this document must add:

1. Disabled-section fixture equality against the previous phase's fixture.
2. Clean-process replay of the new phase's own fixture, two processes.
3. Storage-permutation equality over N ticks (Rule 4).
4. Analysis-neutrality equality with the analysis enabled and disabled.
5. Save, restore, continue: bit-identical trajectory with the new state
   present.
6. A seeded malformed-input harness over any new codec, zero panics, zero
   invalid admissions.
7. For Phase 5 specifically, the scheduler concurrency equality of Rule 10.
