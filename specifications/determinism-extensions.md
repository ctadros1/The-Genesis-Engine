# Determinism Extensions Specification

## Purpose

Phases 5 through 18 add learning, signalling, artifacts, mutable terrain,
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

Three fixtures must remain reproducible at every future phase, from clean
processes, using the same seed and config:

| Fixture | Config hash | State checksum | Seed | Scale | Verified by |
|---|---|---|---|---|---|
| Phase 1 | `0x918a381c77559236` | `0x1e3158a26afd3b39` | `0x5eedcafef00dbeef` | 500 organisms, 500 ticks | `scripts/verify-phase1-determinism.sh` |
| Phase 2 | `0xf83d3981bf7dd189` | `0xff9dfcff5dffbf42` | `0x5eedcafef00dbeef` | 500 organisms, 500 ticks | `scripts/verify-phase2-determinism.sh` |
| Phase 9 | `0x9abc0cd47914127f` | `0x5f0c4e95e4f5170f` | `0x5eedcafef00dbeef` | 500 organisms, 8,000 ticks | `scripts/verify-phase9-determinism.sh` |

The first two are achieved by one mechanism, already proven by D-014:
**every new subsystem is a config section that is behaviorally inert when
disabled, is folded into the canonical config hash only when enabled, and
appends its state to the checksum only when present.** A world with all new
sections disabled executes the same code paths and produces the same bytes
as before.

**The Phase 9 fixture is a permanent obligation on stricter terms, and the
difference is the point of admitting it.** It is the only Rule 0 fixture
whose world runs an *enabled* post-Phase-2 section, so preserving it does
not follow from "a disabled section is inert". It requires that the schema-2
genome, meiosis, structural mutation, and controller v2 keep behaving
exactly as they do now, under a configuration pinned literally rather than
inherited from defaults. Two consequences follow, and neither is optional:

- **The fixture's config is written out field by field** in
  `apply_pinned_genome2_policy` (`crates/sim-cli/src/main.rs`) and in
  `phase9_fixture_config` (`crates/sim-core/tests/phase9_determinism.rs`),
  for the reason `experiments/phase9-c91-confirmatory.campaign` gives for
  its own caps block (D-078). `SimConfig::stable_hash` folds the entire
  genome2 section in when it is enabled - four policy strings, both registry
  versions, all seven `GenomeCaps` fields, the meiosis mode and crossover
  bound, and all eight `MutationConfig` fields. Revising any of those
  defaults must not move a fixture, and pinning is what stops it. The caps
  have already been restated once, by C9.8's measurement.
- **A later phase that reaches inside schema 2 will move it unless that
  reach is itself gated.** Phase 11 is the first case: `PlasticityGenes` is
  already a field on every edge locus and `EDGE_FLAG_PLASTIC` is already a
  flag bit, so a plasticity implementation that acts on them without its own
  enabled-by-config gate changes `0x5f0c4e95e4f5170f` while every section
  that was disabled stays disabled. That is a lineage break and needs an ADR,
  not a quiet re-baseline of the constant.

The fixture's horizon is 8,000 ticks rather than the 500 the other two use,
and the reason is a criterion, not a preference. `maturity_age_ticks` is 600
and founders spawn at age 0, so at 500 ticks a schema-2 world has had **zero
births**: it would pin meiosis, structural mutation, and the schema-2 birth
path by pinning none of them, while looking exactly as authoritative as a
fixture that pinned all three. **A fixture that is silently a control is
worse than no fixture**, so the counts that prove it is not one - births,
paired births, deaths, applied duplications, structural rejections - are
asserted next to the checksum in the script and in both test suites.

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
| 7 | Contest | 7 | Contest tie lottery, damage variance, retreat resolution |
| 8 | Meiosis | 9 | Crossover count and crossover positions per chromosome pair |
| 9 | StructuralMutation | 9 | Duplication, deletion, insertion, transposition site and extent |
| 10 | PlasticityInit | 11 | Initial plastic-state seeding at birth (zero by default policy; the stream exists so a nonzero policy does not renumber) |
| 11 | Signal | 13 | Emission noise and channel corruption |
| 12 | Perception | 13 | Selection among tied perception candidates |
| 13 | Artifact | 12 | Placement site selection, combination outcome variance, fracture products |
| 14 | MaterialYield | 12 | Extraction yield variance |
| 15 | Development | 14 | Ontogenetic variance at developmental checkpoints |
| 16 | Mortality | 8 | Age-dependent and extrinsic hazard draws |
| 17 | Morphogenesis | 10 | Developmental growth-program variance (unused under the default fully deterministic policy) |
| 18 | Abiogenesis | 15 | Protocell formation draws, keyed by cell |
| 19 | MicrobialField | 15 | Field-regime stochastic terms, keyed by cell, never by individual |
| 20 | Transition | 16 | Multicellularity handoff materialization draws |
| 21 | ClimateDrift | 6 | Optional bounded stochastic climate component (unused under the default deterministic policy) |
| 22 | FounderSeed | 6 | Archetype selection, deme centre choice, founder placement |

Three of these (17, 21, and to a lesser extent 19) exist specifically so that
adopting a stochastic policy later cannot renumber an existing stream. They
are allocated now and unused under the default deterministic policies.

`FounderSeed` (22) is deliberately separate from `GenomeInit` (5) so that
adding origin modes cannot shift the founder sequence of an existing
`random` world.

Values 23 and above are unallocated. An analysis module never draws from any
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

**That recipe cannot be executed literally, and pretending otherwise
produces a test that passes while proving nothing.** `World::from_state`
refuses a population that is not in ascending entity-ID order
(`RestoreError::EntityOrder`), which is a guarantee worth more than the
test - so a permuted save is not restorable, and a permutation applied
jointly to every per-organism array and then sorted back into ID order is
*exactly the identity* on the record. Three tests discharge the rule instead,
and the split is deliberate:

1. **The round trip** (`permuting_every_per_organism_array_together_and_
   sorting_back_is_a_round_trip`) is the positive control. It shows the
   transform is exact and that the fail-closed guard fires on the
   intermediate unsorted state. It cannot fail while the permutation is
   complete, and it is not the evidence.
2. **The negative sweep** (`scrambling_any_one_per_organism_array_changes_
   the_world`) is the evidence. Each per-organism array is rotated on its own
   and the world must notice - by refusing the restore, by checksumming
   differently, or by diverging within N ticks. Leaving one array out of a
   permutation is the desync a storage-permutation test exists to catch, and
   an array that is saved but never read fails here rather than passing
   silently. Rotate-by-one is used rather than reverse because its only fixed
   point is a constant array, so "the world did not notice" can be asserted
   to mean "there was nothing to notice".
3. **Compaction** is where storage order changes for real. A death shifts
   every later survivor down one index, so the same organism occupies a
   different slot in two runs that differ only in who died. Both the array
   level (`Schema2State::retain`) and the world level are covered; see C9.7.

The enumeration in tests 1 and 2 destructures `SaveState` with no `..`
(D-077), so a per-organism array cannot join the save without joining the
test.

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
| `lifesim-climate-state-v1` | Dynamic temperature and moisture fields | climate enabled |
| `lifesim-origin-state-v1` | Archetype and founder-file provenance | `origin.mode != random` |
| `lifesim-morphology-state-v1` | Developmental clock, non-viability counters | morphology enabled |
| `lifesim-chemistry-state-v1` | Per-cell substrate concentrations | field regime enabled |
| `lifesim-microbial-state-v1` | Per-cell, per-class microbial densities | field regime enabled |
| `lifesim-action-census-v1` | Per-organism cumulative action histograms and their counters | `probe.action_census_enabled` |

`lifesim-action-census-v1` is a **measurement** section and is hashed
anyway, which is worth stating because the alternative is tempting. A
lifetime's action counts have no source but the save: they accumulate from
intents computed from stored activations, so re-deriving them would need the
run replayed from tick zero. That is the same argument
`lifesim-learn-state-v1` makes. The cost is that **zeroing the counters
moves the checksum** - a world whose counters were reset at tick 5,000 is
not the world whose were not - and hiding that behind an unhashed field
would make a state change invisible to replay. The sampling path therefore
records cumulative rows and never resets; a before/after window is the
difference of two samples, which subsumes a reset and keeps sampling
provably read-only.

Nothing in the tick reads a census count: no controller input, no config
trigger, no selection term, and no RNG draw. That is what keeps the section
inside ADR-0016, and the five fixtures are the assertion of it rather than
the claim.

Three categories of state are deliberately **absent** from this table because
they are derived and recomputed on load rather than stored: phenotypes and
genome hashes (existing), biome classification (Phase 6), and module bodies
(Phase 10). Development is a pure function of `(genome, config)`, which is
what lets bodies stay out of both the checksum and the save. Field state is
the opposite case: it cannot be recomputed from anything, so it is stored.

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
| - | `contest-behavior-v1` | Damage, contest, territory (Phase 7) |
| `lifesim-genome-v1` | `lifesim-genome-v2` | Diploid variable-topology genome (Phase 9) |
| `lifesim-controller-v1` | `lifesim-controller-v2` | Synchronous variable-topology evaluation (Phase 9) |
| `uniform-bounded-v1` | `lifesim-meiosis-v1` | Recombination and crossover (Phase 9) |
| - | `lifesim-structmut-v1` | Structural mutation operators (Phase 9) |
| - | `lifesim-pairkey-v1` | Pairwise draw subject derivation (Phase 7) |
| - | `lifesim-plasticity-v2` | Learning rule registry and update arithmetic (Phase 11) |
| - | `lifesim-probe-v1` | Phase 11 measurement section: the action-class set, the partition/indicator split, `TURN_BAND_MILLI`, and the neutral marker locus. Hashed only when `probe.enabled`, and appended after Phase 12's section, so the five fixtures are unmoved. Enabling it starts a new replay lineage in both halves: the marker changes what point mutation can land on, and the census changes what is stored and checksummed |
| - | `lifesim-action-census-v1` | Action-class set and classification policy (Phase 11). A histogram recorded under one version is not comparable with one recorded under another, so the version is folded into the config hash and into the `.alac` header's `policy_hash` |
| - | `lifesim-social-v1` | Perception and signalling (Phase 13) |
| - | `lifesim-material-v1` | Material registry and properties (Phase 12) |
| - | `lifesim-artifact-v1` | Object actions and combination physics (Phase 12) |
| - | `lifesim-worldmod-v1` | Terrain modification rules (Phase 12) |
| - | `lifesim-physiology-v1` | Allometry, ontogeny, senescence (Phase 13) |
| `lifesim-worldgen-v1` | `lifesim-worldgen-v2` | Adds moisture, temperature, and biome fields (Phase 6). v1 worlds keep v1 forever, so their terrain checksums and both fixtures are unaffected |
| - | `lifesim-biome-v1` | Biome classification thresholds (Phase 6) |
| - | `lifesim-climate-v1` | Climate drift periods and amplitudes (Phase 6) |
| - | `lifesim-origin-v1` | Origin modes, archetypes, deme placement (Phase 6) |
| - | `lifesim-morphology-v1` | Module registry and phenotype derivation (Phase 10) |
| - | `lifesim-develop-v1` | Developmental growth program (Phase 10) |
| - | `lifesim-chemistry-v1` | Substrate registry, reactions, diffusion (Phase 15) |
| - | `lifesim-microbial-v1` | Genotype class registry and field dynamics (Phase 15) |
| - | `lifesim-transition-v1` | Materialization threshold and class-to-genome map (Phase 16) |

Genome schema versions are a separate axis from policy versions: schema 1
(existing), schema 2 (Phase 9, diploid variable topology), schema 3 (Phase 10,
adds regulatory loci for development). All three decoders and their fixtures
stay in the build permanently and **there is no migration between them**, for
the reason ADR-0013 records: a converted genome would be a record that never
existed.

Analysis policies are recorded in reports and are deliberately **not** in the
config hash, because an analysis version can never affect a world:
`lifesim-similarity-v1` (existing), `lifesim-era-v1`,
`lifesim-tradition-v1`, `lifesim-spatial-index-v1` (Phase 7),
`lifesim-plasticity-analysis-v1` (Phase 11, C11.1 and C11.2),
`lifesim-conjunction-census-v1` (Phase 11, descriptive).

Save framing versions are separate again: ALIF format 1 (existing), ALIF
format 2 (Phase 12) - **in practice ALIF format 4**, since the Phase 9 and
Phase 11 sections landed first; see `specifications/world-save-format.md`.
Artifact framing versions are separate from both: ALEV format 1 (event log,
Phase 5), ALSS format 1 (spatial samples, Phase 7), ALAC format 1
(per-individual action samples, Phase 11).

## Rule 10: Scheduling Never Reaches The Kernel

Multi-world execution runs independent worlds in separate schedulable units
that share no mutable state. The required proof is an equality test, not an
argument: for a fixed seed set and config, per-world final state checksums
must be identical at scheduler concurrency 1, 2, and C, and identical to
single-world execution of the same seed. Any work-stealing, thread-count,
or completion-order dependency shows up as a checksum difference.

### Intra-world parallelism (amended by ADR-0026)

This rule previously stated that nothing authorizes intra-world
parallelism. ADR-0026 amends that: intra-world parallelism is authorized
**under the conditions below, and only under them**. ADR-0010's requirement
of an ordering and reduction policy plus equality tests is satisfied here,
not waived.

Population is the binding constraint on the project's central question, and
one world on one core caps a flagship world at an estimated 10,000 to 30,000
organisms. That is the motivation; the conditions are what make it safe.

**The required shape.** This is Rules 4 and 5 generalized from perception
and learning to the whole tick, not a new pattern:

1. Freeze read state at the tick boundary.
2. Partition by `f(object_id, P)`, a pure function of stable ID. **Never by
   thread count and never by array slice.**
3. Workers produce intents into per-partition buffers. **No worker writes
   world state.**
4. Merge canonically, in ascending `(partition_index, object_id)`. **Never
   in completion order.**
5. Resolve cross-partition conflicts under the existing complete policies:
   priority key, then stable ID, then `lifesim-pairkey-v1` where a lottery
   is configured.
6. Commit once, single-threaded.
7. Hash and compare against the single-threaded reference.

**Partition count `P` is a config constant independent of thread count**,
folded into the config hash. Changing `P` changes reduction tree topology
and is therefore a new replay lineage; a different `P` must produce a
different config hash.

**Reduction discipline.** Cross-partition reductions must be integer, or use
a reduction tree whose topology is fixed independent of worker count.
Integer addition is associative, so fixed-point accumulators are
order-independent by construction. This project's state is entirely fixed
point and its ledger is `i128`, which is what makes thread-count invariance
reachable at all.

**Standing obligation on every future phase.** Any new cross-organism
reduction is integer, or has a fixed-topology tree. A float sum introduced
across organisms silently degrades thread-count invariance to per-thread-
count determinism, and the degradation will not announce itself.

**Rule 6 is unchanged and is where this most easily goes wrong.**
Per-organism float summation stays pinned to ascending `homology_id` order.
It is safe under partitioning only because it never crosses organisms: all
per-organism float work lives inside a single partition. Any design that
moves float summation across a partition boundary violates this rule.

**The guarantee, in preference order.** ADR-0026 proposes Tier 1. A phase
claiming a lower tier records the degradation in the decision log rather
than absorbing it.

| Tier | Guarantee |
|---|---|
| **1 (proposed)** | Same seed, **any** thread count, bit-identical. Thread count is a scheduling detail and stays out of the config hash |
| **2 (fallback)** | Same seed **and same thread count**, bit-identical. Thread count enters the config hash; a different thread count is a different replay lineage |
| **3** | **Not available.** True non-determinism would remove fail-closed checkpoint verification, degrading restore from "reproduces exactly or errors" to "looks statistically similar"  -  on the one world whose checkpoint chain is irreplaceable. Abandoning intra-world parallelism is preferable |

**Campaign worlds stay single-threaded** and remain the basis for every
claim (ADR-0023). Parallelism is a flagship capability. Under Tier 1 that
distinction costs nothing because results are identical; under Tier 2 it
matters, and it is acceptable only because a flagship world is already
barred from supporting a claim on n=1 grounds.

Nothing here authorizes speculative execution, rollback, or optimistic
parallel discrete-event simulation. The model is synchronous and phased.

## Test Obligations

Every phase that touches this document must add:

1. Disabled-section fixture equality against the previous phase's fixture.
2. Clean-process replay of the new phase's own fixture, two processes. The
   fixture's horizon must be long enough that the mechanisms the phase added
   actually run, and the counts that prove they ran are asserted beside the
   checksum. A fixture whose horizon is too short is a control with a
   fixture's authority.
3. Storage-permutation equality over N ticks (Rule 4), in the three-part form
   that rule now specifies, not the literal recipe.
4. Analysis-neutrality equality with the analysis enabled and disabled.
5. Save, restore, continue: bit-identical trajectory with the new state
   present.
6. A seeded malformed-input harness over any new codec, zero panics, zero
   invalid admissions.
7. For Phase 5 specifically, the scheduler concurrency equality of Rule 10.
