# Genome Schema 2: Diploid Variable-Topology Genome

Status: design specification, not implemented. Genome schema 1
(`lifesim-genome-v1`) remains the only implemented schema and stays readable
forever; its fixtures are load-bearing evidence.

Phase: 7. Policy versions: `lifesim-genome-v2`, `lifesim-meiosis-v1`,
`lifesim-structmut-v1`, `lifesim-controller-v2`.

## Problem With Schema 1

Schema 1 is a flat haploid vector of 14 trait genes plus exactly 696 neural
genes for topology 1. Two consequences block the current goal:

- Every new capability requires a human to add an input or output channel
  and bump the schema. Open-ended complexity cannot come from a structure
  only we can change.
- Inheritance is per-gene independent parent choice, which has no linkage,
  no dominance, no ploidy, and no structural mutation. It is a workable
  abstraction and an unrealistic model of genetics.

Schema 2 fixes both with one mechanism, because network growth by gene
duplication is simultaneously the realistic genetics answer and the
structural evolution answer. See `docs/26-biological-realism-policy.md` and
ADR-0013.

## Overview

A schema-2 genome is:

- **Diploid.** Two haplotypes. Haplotype slot 0 is inherited from the
  lower-ID parent, slot 1 from the higher-ID parent
  (`specifications/determinism-extensions.md` Rule 3).
- **Chromosomal.** Each haplotype holds `C` chromosomes, `C` fixed per
  world by config. Homologous chromosomes pair by index.
- **A sorted locus list per chromosome.** Loci are typed records sorted by
  ascending innovation ID. Sortedness is a decode-time invariant, not a
  convention.

Loci are the unit of mutation, crossover, and expression. Adjacency in
innovation order is linkage: a crossover between two loci separates them,
and duplication inserts copies adjacent to their source, so functionally
related material stays linked by default and can be unlinked by
transposition. That is the biological arrangement and it is also what makes
crossover positions meaningful.

## Structural Identity: Four Fields, Not One

An earlier draft carried a single `innovation_id` and used it for
alignment, ancestry, and event identity at once. `neuroevolution` section 1.6
identifies that as a false-equivalence hazard, and a global sequential
counter as order-fragile. ADR-0022 A8 adopts the correction: identity is
four separate fields.

| Field | Meaning | Used for |
|---|---|---|
| `gene_lineage_id` | Persistent identity of a heritable gene lineage | Ancestry and provenance analysis |
| `homology_id` | The structural slot two loci share | **Alignment during meiosis** |
| `structural_signature` | Canonical phenotype-relevant fields: source, target, role, activation, delay | Detecting genuinely equivalent structure |
| `mutation_event_id` | Identity of the reproduction/mutation event that created the locus | Audit and event-log joins |

Loci are sorted within a chromosome by `homology_id`, and it is
`homology_id` that makes crossover between structurally different homologues
meaningful.

### Derivation without a global counter

IDs are **derived, not allocated**: a domain-separated hash over a canonical
event key, rather than a shared mutable counter.

    homology_id = H("lifesim-homology-v1", policy_version, parent_homology,
                    operator, target_slot, attempt_ordinal)
    mutation_event_id = H("lifesim-mutevent-v1", world_seed, tick,
                          child_object_id, operator, attempt_ordinal)
    gene_lineage_id   = inherited from the source locus; fresh only on
                        duplication or insertion, derived as above

This removes the last piece of shared mutable state from reproduction, so
no allocation depends on scheduling or traversal, and it keeps the property
that the same structural mutation applied to the same parent under the same
policy yields the same identity.

**Independent identical mutations** therefore converge on the same
`homology_id` by construction. Two lineages that independently evolve the
same structural change align during meiosis instead of being treated as
disjoint, which is the problem NEAT's innovation record exists to patch and
which the original single-counter design would have reproduced.

Hash width and collision policy are versioned config; a collision between
different canonical keys is a fail-closed error, never a silent merge.

Allocation happens in the lifecycle phase when a child is materialized,
processing pending children in ascending child object-ID order, and within a
child in ascending (chromosome index, locus position) order. This is a pure
function of the tick's already-deterministic child ordering, so the counter
is reproducible.

Innovation IDs are what let two structurally different genomes align during
meiosis without a graph-matching heuristic: loci with equal innovation IDs
are homologous, and loci present in one gamete only are inherited as-is.
This is NEAT's historical-marking idea, obtained here as an ordinary
consequence of chromosomal inheritance rather than as a separate matching
algorithm.

## Locus Types

All values are finite bounded f32 unless stated. Every bound is versioned
config; decode validates every one before construction.

| Type tag | Fields | Bounds |
|---:|---|---|
| 1 `Trait` | `trait_id: u16`, `value: f32`, `dominance: f32` | value in [0,1]; dominance in [0,1]; `trait_id` in the versioned trait registry |
| 2 `Node` | `innovation_id: u32`, `role: u8`, `activation_id: u8`, `bias: f32`, `time_constant: u16` | bias in [-8,8]; `role` in {Input, Hidden, Output, Modulatory}; `activation_id` in the versioned activation registry |
| 3 `Edge` | `innovation_id: u32`, `source: u32`, `target: u32`, `weight: f32`, `flags: u8`, `plasticity: PlasticityGenes` | weight in [-8,8]; `flags` bit 0 = plastic, bit 1 = disabled |
| 4 `IoBinding` | `innovation_id: u32`, `node: u32`, `channel_id: u16`, `gain: f32` | `channel_id` in the versioned input/output channel registry |

`PlasticityGenes` is specified in
`specifications/plasticity-and-learning.md`. It occupies space in every edge
locus whether or not the edge is plastic, so that enabling plasticity is a
flag flip rather than a schema change. When the plasticity section is
disabled, the fields are validated, inherited, and behaviorally inert:
exactly the pattern Phase 2 already uses for thermal preference and defense
tendency.

Type tag 5 `Regulatory` is reserved and unallocated. A coarse regulatory
locus is an open question, not a commitment
(`docs/21-open-questions.md`).

### Input and output channels are a registry, not a topology

Schema 1 hard-codes 20 inputs and 12 outputs. Schema 2 replaces this with a
versioned **channel registry**: a bounded enumeration of sensory channels the
world can supply and action channels the world can accept. An `IoBinding`
locus connects one network node to one registry channel.

Consequences:

- An organism may bind any subset of channels. Unbound input channels are
  simply not read; unbound action channels are never requested. There is no
  "documented neutral zero" placeholder any more, and no cost for channels
  an organism does not use.
- Adding a world capability (a signal channel, a `pick_up` action) adds a
  registry entry and bumps the registry version, which enters the config
  hash. It does not force a genome schema bump and does not invalidate
  existing schema-2 genomes, which simply have no binding to the new
  channel.
- A binding to an unknown channel ID fails decode, closed.

This is the mechanism that stops "every new capability is a schema bump".

## Structural Caps

Unbounded genomes make snapshot size, memory, and migration unprovable.
Every cap is versioned config; a mutation that would exceed one is rejected
deterministically and counted, with a typed event, exactly as births are
rejected at `max_entities`.

| Cap | Purpose |
|---|---|
| `max_chromosomes` | Fixed per world; meiosis pairs by index |
| `max_loci_per_chromosome` | Bounds decode allocation |
| `max_nodes`, `max_edges` | Bounds evaluation cost per organism per tick |
| `max_genome_bytes` | Bounds snapshot growth; the binding constraint in practice |
| `max_edges_per_node` | Bounds the per-node summation loop |

Snapshot budget is the real limit. The Phase 4 benchmark records that
snapshot size is already dominated by per-organism genome arrays at roughly
2.8 KB each. Caps must be chosen against a measured checkpoint budget, not
guessed, and Phase 9's benchmark requirements say so explicitly.

## Encoding

Magic `ALG2`, little-endian throughout, matching schema 1's conventions.

    header:
      magic            4 bytes  "ALG2"
      schema_version   u16      = 2
      registry_version u16      channel/activation registry version
      ploidy           u8       = 2
      chromosomes      u8       C
      flags            u16      reserved, must be zero
      total_len        u32      exact byte length of the whole record
    per haplotype (2), per chromosome (C):
      locus_count      u32
      per locus:
        type_tag       u8
        payload        fixed length per type tag
    trailer:
      checksum         u64      FNV-1a64 over everything above

Decode is fail-closed and bounded, following schema 1 exactly:

1. Verify magic, schema version, registry version, ploidy, chromosome count,
   and `total_len` against the actual buffer length **before any
   allocation**.
2. Verify the checksum before interpreting any payload.
3. Verify every locus count against `max_loci_per_chromosome` before
   allocating that chromosome.
4. Validate every value's finiteness and bounds.
5. Validate structural invariants (below).
6. Construct. A `Genome2` in world state is valid by construction; there is
   no repair path and no partial acceptance.

### Structural invariants, all checked at decode

- Loci within a chromosome are strictly ascending by innovation ID; trait
  loci sort by `trait_id` within a reserved innovation range.
- Every `Edge.source` and `Edge.target` refers to a node innovation ID
  present in the same haplotype.
- Every `IoBinding.node` refers to a present node.
- Node roles are consistent: an Input-role node is never an edge target from
  a non-Input node in a way that violates the configured role rules; Output
  and Modulatory roles have no restriction beyond existence.
- Node and edge counts are within caps.
- The two haplotypes have the same chromosome count.

## Expression: Diploid To Phenotype And Network

Expression is a pure function of the genome, recomputed on load, never
persisted as truth. This preserves the existing rule that derived state is
recomputed rather than trusted from a save.

**Trait loci.** For a trait present on both haplotypes with values
`v0, v1` and dominances `d0, d1`:

    expressed = (d0 * v0 + d1 * v1) / (d0 + d1)          when d0 + d1 > 0
    expressed = 0.5 * (v0 + v1)                           when d0 + d1 == 0

This spans the biological range continuously: equal dominance gives additive
(codominant) inheritance, `d0 = 1, d1 = 0` gives complete dominance, and
intermediate values give incomplete dominance. Dominance is itself a gene,
so dominance relationships evolve. A trait present on one haplotype only is
hemizygous and expresses its single value.

The formula is order-free given the fixed haplotype slot assignment.

**Node and edge loci.** The expressed network is the union of both
haplotypes' nodes and edges, keyed by `homology_id`. For an innovation
present on both haplotypes, scalar parameters (bias, weight, gain, time
constant) combine by the same dominance formula; boolean flags combine by
a specified rule: `disabled` is expressed only when disabled on both
haplotypes (dominance of function), `plastic` is expressed when plastic on
either. Both rules are versioned policy and recorded, not incidental.

An innovation present on one haplotype only is expressed at its single
value. This makes a duplication immediately hemizygous and heterozygous,
which is exactly the biological situation after a duplication event and is
the source of the divergence that follows.

## Meiosis: `lifesim-meiosis-v1`

Each parent produces one gamete. For parent `P` producing a gamete for
prospective child `K`:

For each chromosome index `i` in `0..C`:

1. Draw the crossover count `n` from a bounded distribution using
   `named_random(seed, tick, Meiosis, K, draw_base + i*4 + 0)`. The
   distribution is versioned config: default is `n = 1 + (draw mod
   max_extra_crossovers + 1)` biased toward small `n`, with at least one
   crossover per chromosome per meiosis, which is the biological norm.
2. Draw `n` crossover positions in locus-index space from subsequent draws,
   deduplicate, and sort ascending. Positions are expressed in the merged
   `homology_id` ordering of the two homologues, so a crossover point is a
   position in innovation space, not an array index. This makes crossover
   meaningful between homologues of different lengths.
3. Walk the merged innovation ordering, alternating source haplotype at each
   crossover position, emitting each locus from the currently selected
   haplotype. A locus present only in the non-selected haplotype at that
   position is not emitted; a locus present only in the selected haplotype
   is emitted. This is ordinary segregation and it is what makes disjoint
   and excess structural material inherit sensibly without a special case.

The child's haplotype 0 is the gamete from the lower-ID parent and
haplotype 1 the gamete from the higher-ID parent. No traversal order enters
the result.

### Why this replaces `uniform-bounded-v1` rather than extending it

Per-gene independent parent choice is free recombination: linkage
disequilibrium decays instantly and a co-adapted set of loci cannot be held
together. That removes one of the main forces the model is meant to be able
to exhibit. Meiosis with a small number of crossovers per chromosome
preserves linkage, and linkage decay with map distance becomes a testable
prediction (Phase 9 acceptance criterion C9.5).

`uniform-bounded-v1` remains valid and unchanged for schema-1 worlds.

## Inheritance Modes

`genetics` section 1.2 records that crossover is not universally beneficial and can
destroy coadapted structure under strong epistasis. Paired reproduction
therefore does **not** imply mandatory crossover. The world genetics policy
selects a mode, and the alternatives exist as first-class controls rather
than as a future idea (ADR-0022 A10).

| Mode | Behavior |
|---|---|
| `clonal` | Single parent, no recombination. The baseline control |
| `paired_whole_genome` | Two parents, offspring takes one parent's genome intact. Isolates mate choice from recombination |
| `biparental_assort` | Independent assortment of whole chromosomes, no within-chromosome crossover |
| `meiotic` | Full meiosis with crossover, as specified below. The default |
| `rearranging` | Meiotic plus rare non-homologous rearrangement. Experimental |

Any claim about the benefit of recombination is reported against at least
`clonal` and `paired_whole_genome`. Inheritance-mode probabilities are not
heritable; making them so would be a separately versioned experiment.

## Mutation

Applied to the child genome after meiosis, in a fixed operator order, each
gated by its own versioned rate. All draws use
`named_random(seed, tick, system, child_id, draw_index)` with `system =
Recombination` for value mutation (preserving the existing convention) and
`StructuralMutation` for structural operators.

| Operator | Effect | Notes |
|---|---|---|
| Point | Perturb one locus value by a bounded delta, clamp | Generalizes `uniform-bounded-v1` to typed loci |
| Duplication | Copy a contiguous run of `1..max_dup_run` loci; copies receive fresh innovation IDs and are inserted immediately after the source run | The growth mechanism. Copies are exact, so divergence is a later event, as in biology |
| Deletion | Remove a contiguous run | Guarded: a deletion that would orphan an edge, remove the last `IoBinding` for a required channel, or drop the node count below `min_nodes` is rejected and counted |
| Insertion | Add one new `Edge` locus between two existing nodes, or one new `Node` with one in-edge and one out-edge | Available as an explicitly configured alternative to duplication-only growth, so the two can be compared (ADR-0013) |
| Transposition | Move a contiguous run to another position on the same or another chromosome | Changes linkage without changing content |

Rejected operations are counted per class in the state counters, hashed, and
surfaced as bounded events. Silent rejection is not permitted; an experiment
that is quietly running against a cap must be visible in the report.

After all operators, the genome is re-validated. A genome that fails
validation is a bug, not a runtime condition: the operators are written to
produce valid records by construction, and the validation is an assertion.
If it ever fails, the birth is rejected with a typed reason rather than
admitting an invalid record.

## Compatibility Gating

Reproduction may be gated on compatibility computed from the two genomes:

    distance = w_t * trait_distance + w_s * structural_distance

where `structural_distance` is the fraction of `homology_id` values not shared
between the two expressed networks, and `w_t`, `w_s` are Q16 config weights.

This is physics, not an analysis label: it is computed from the records
themselves and would still function if the analysis modules were deleted.
See `docs/25-emergence-and-epistemic-position.md` for why that distinction
matters and where the line is.

## Schema 1 Coexistence

- Schema 1 decode, evaluation, and fixtures remain in the build unchanged.
  They are historical evidence and are not migrated away.
- A world is schema 1 or schema 2 by config; there is no mixed-schema world.
- Cross-schema mating does not exist, because cross-schema worlds do not
  exist. `specifications/organism-genome.md`'s existing rule (reject or use
  a declared migration) stands and is satisfied by rejection.
- There is **no** schema 1 to schema 2 genome migration. A schema-1 world
  loaded under a schema-2 build stays schema 1 and continues identically.
  Converting one would produce a genome that never existed and a lineage
  that cannot be replayed; the project's rule against reinterpreting old
  worlds forbids it.

## Test Requirements

- Bounded fail-closed decode: magic, schema, registry version, ploidy,
  chromosome count, length, checksum, per-chromosome locus count, value
  finiteness and bounds, sortedness, dangling edge or binding reference.
- Seeded malformed-input harness of at least 100,000 cases: zero panics,
  zero invalid admissions, every accept re-validated and round-tripped.
  This mirrors the existing Phase 2 harness and its reporting format.
- Expression is a pure function: same genome gives same phenotype and same
  network, and recomputation after save and restore is identical.
- Meiosis determinism and order-independence: swapping which parent is
  visited first produces an identical child.
- Mendelian validation: at a marked neutral biallelic locus under random
  mating with selection disabled, genotype frequencies match Hardy-Weinberg
  expectation within sampling error over at least 30 generations.
- Linkage validation: allele association between loci decays with map
  distance at the rate the crossover model predicts.
- Cap enforcement: every cap rejects deterministically, counts, and events;
  no cap is ever silently exceeded.
- Fixture preservation: a schema-1 configured world reproduces
  `0xff9dfcff5dffbf42` exactly.
