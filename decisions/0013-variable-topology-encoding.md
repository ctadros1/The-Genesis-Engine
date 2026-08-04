# ADR-0013: Variable-Topology Genome Encoding

Status: Proposed
Date: 2026-08-04
Author: Goal revision

Supersedes nothing. Extends ADR-0004 (custom compact CPU evaluator), which
remains Proposed and whose core choice of a custom evaluator with no ML
framework is unchanged.

## Context

Controller topology is frozen at 20 inputs, 16 and 12 hidden units, 12
outputs, and 4 memory values. Every new capability requires a human to add a
channel and bump the genome schema. Open-ended complexity cannot come from a
structure only we can change.

Separately, `docs/26-biological-realism-policy.md` requires genetics to be
modelled realistically where determinism and budget allow. Schema 1 is
haploid, unlinked, dominance-free, and point-mutation-only.

These look like two problems. They have one solution.

## Options Considered

- **Keep the fixed topology and widen it.** Adds channels but not
  evolvability, and every future capability is still a schema bump.
- **NEAT-style graph-editing operators**: explicit `add node` and
  `add connection` mutations with historical markings. Well understood and
  effective at small scale. It is an authored graph-editing scheme layered
  on a flat vector, and it does nothing for genetic realism.
- **Indirect or developmental encoding** (a genome that specifies a growth
  process rather than a network). Highest ceiling in principle; hardest to
  make deterministic, hardest to analyze, and the ALife literature does not
  establish that it wins at this scale.
- **Chromosomal diploid genome with variable-length typed locus lists, where
  structural change comes from gene duplication, deletion, insertion, and
  transposition.**

## Proposed Decision

Adopt the fourth option as genome schema 2 (`lifesim-genome-v2`), specified
in `specifications/genome-schema-2.md`.

Key elements:

- Diploid, chromosomal, with typed loci sorted by innovation ID. Adjacency
  in innovation order is linkage.
- Structural identity as **four fields** (gene lineage, homology class,
  structural signature, mutation event), each derived by domain-separated
  hash over a canonical event key. **Amended by ADR-0022 A8**: the original
  draft used a single innovation ID from a world-global monotonic counter,
  which conflated four meanings and made allocation order-fragile. Alignment
  during meiosis is by homology class, which gives NEAT's historical-marking
  behavior as a consequence of ordinary chromosomal inheritance, and makes
  independent identical mutations converge on the same class rather than
  being treated as disjoint.
- Meiosis with crossover replaces per-gene independent parent choice, so
  linkage exists and co-adapted gene sets can be held together.
- Dominance is a per-locus evolvable gene, spanning codominant to complete
  dominance continuously.
- Structural mutation is genomic: duplication, deletion, insertion,
  transposition. Gene duplication followed by divergence is the principal
  mechanism by which real regulatory and neural complexity increased, and it
  gives structural evolution and genetic realism from one mechanism.
- A versioned input/output channel registry replaces hard-coded channel
  counts, so a new world capability is a registry entry rather than a genome
  schema bump.
- Evaluation is synchronous: every node computes from the previous tick's
  activations. This removes topological sorting and cycle handling entirely
  and makes node evaluation order irrelevant. Activations become world
  state.
- Hard structural caps with deterministic rejection and counting.

## Consequences

Positive:

- New world capabilities no longer require a genome schema change.
- Genetics realism and structural evolvability arrive together.
- Recurrent topologies are free, and the existing 4-value memory vector
  becomes a special case of recurrent nodes rather than a separate concept.
- Synchronous evaluation is the simplest determinism story available for an
  arbitrary graph.

Negative and accepted:

- **Duplication-driven growth is slower and less directed than explicit
  add-node mutation.** The ALife literature does not establish that
  duplication-based or indirect encodings outperform direct ones at this
  scale, and asserting otherwise would be unevidenced. The mitigation is
  that both operator sets act on the same locus list: the explicit insertion
  operator remains available as a configured variation policy, and Phase 8
  criterion C8.5 measures the comparison rather than assuming an answer. If
  duplication alone cannot produce structural change within the run budget,
  insertion becomes the default with the measurement recorded.
- Diploidy roughly doubles genome storage. This is the direct cost of the
  realism policy and is accepted under ADR-0017.
- Batching by topology ID no longer works, because topologies are no longer
  shared. The replacement strategy is a measurement question for a later
  performance slice, and the loss of the current zero-per-tick-allocation
  property, if it occurs, must be recorded rather than absorbed.
- Per-organism evaluation cost becomes variable and skewed, making tick time
  less predictable.
- Synchronous update means information propagates one edge per tick. A deep
  network needs many ticks to respond. `propagation_passes_per_tick` is a
  config knob with an evolutionary consequence, and its default of 1 is a
  recorded policy choice rather than an obvious one.

Compatibility: schema 1 decode, evaluation, fixtures, and tests stay in the
build permanently. A world is schema 1 or schema 2 by config; there are no
mixed-schema worlds and no schema 1 to schema 2 genome migration. Converting
a schema 1 genome would produce a record that never existed and a lineage
that cannot be replayed.

## Performance Implications

Substantial and unmeasured. Phase 2 measured controller evaluation at
roughly 0.32 to 0.33 microseconds per organism per tick with a fixed
topology and stack buffers. Variable topology changes the cost model at
every level: evaluation, allocation, snapshot size, and memory.

No performance claim is made here. Phase 8's benchmark requirements demand
the distribution rather than the mean, because evolved topology sizes will
be skewed, and the structural caps are to be set from that measurement
rather than guessed in advance.

## Operational Implications

Snapshot growth is the operational risk. The Phase 4 record already shows
snapshot size dominated by per-organism genome arrays at roughly 2.8 KB
each, and the server's checkpoint is synchronous on the tick thread.
Asynchronous checkpointing is a Phase 5 prerequisite, and
`max_genome_bytes` is expected to be the binding structural cap in practice.

## Revisit Conditions

- C8.5 shows duplication-only growth cannot reach the structural change the
  phase requires within budget.
- Evaluation cost or snapshot growth breaks the tick or checkpoint budget at
  a supported tier.
- A regulatory or developmental locus type becomes necessary; type tag 5 is
  reserved for it and remains an open question, not a commitment.

## Evidence Required To Accept

- Phase 8 acceptance criteria C8.1 through C8.8, in particular the Mendelian
  and linkage validations (C8.3, C8.4) and the duplication-versus-insertion
  comparison (C8.5).
- Benchmark evidence for evaluation cost distribution, snapshot size, and
  memory at both supported tiers.
- Malformed-input harness of at least 100,000 cases with zero panics and
  zero invalid admissions.
- Compatibility and rollback impact: schema 1 fixtures reproduce exactly.
