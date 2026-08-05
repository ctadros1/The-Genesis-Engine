# Phase 9: Evolvable Genome, Diploid Genetics And Variable Topology

Status: **in progress. 2026-08-05.** Landed: channel and activation
registries, the schema 2 genome model and derived identity, the ALG2
bounded fail-closed codec with every structural invariant, diploid
expression with evolvable dominance, meiosis with all four inheritance
modes, the five mutation operators with typed counted rejection, and
controller v2's hybrid evaluator, and world integration (config-gated
schema choice, founders, the controller seam, meiosis-based reproduction,
save/restore, snapshot codec, checksum, invariants, and campaign metrics).
**Not implemented**: the campaign criteria C9.1, C9.2, C9.5, C9.8, which
now have everything they need. Decisions D-066 to D-073. Policy versions
`lifesim-genome-v2`,
`lifesim-controller-v2`, `lifesim-meiosis-v1`, `lifesim-structmut-v1`.
Specification: `specifications/genome-schema-2.md`.

## Problem

Two problems with one solution.

**Frozen topology.** Every controller is 20 inputs, 16 and 12 hidden units,
12 outputs, 4 memory values, fixed at birth and fixed for the species.
Every new capability is a new channel and a schema bump performed by a
human. Open-ended complexity cannot come from a structure only we can
change.

**Unrealistic genetics.** Schema 1 is haploid, has no chromosomes, no
linkage, no dominance, and only point mutation. Inheritance is per-gene
independent parent choice, which is free recombination: co-adapted gene sets
cannot be held together and linkage disequilibrium decays instantly.

The solution to both is the same mechanism. Networks grow because genes
duplicate and diverge, and shrink because genes are deleted. Adopting
biological structural mutation gives structural evolution and genetic
realism at once, instead of bolting a graph-editing scheme onto a flat
vector. See ADR-0013 and `docs/26-biological-realism-policy.md`.

This is the most expensive phase in the plan and the one with the widest
blast radius. Everything after it depends on it.

## Scope

- Genome schema 2: diploid, chromosomal, sorted typed locus lists, per
  `specifications/genome-schema-2.md`.
- Meiosis with crossover and linkage, replacing per-gene independent choice.
- Dominance as an evolvable per-locus gene.
- Structural mutation: duplication, deletion, insertion, transposition, plus
  generalized point mutation.
- World-global innovation ID counter as saved world state.
- Controller v2: **hybrid** update over an arbitrary graph (ADR-0022 A9,
  D-043, D-066) - zero-delay edges in canonical topological order over the
  acyclic subgraph, delayed and recurrent edges from prior-state buffers -
  with activations and prior-state buffers as world state and edges summed
  in `homology_id` order.
- A versioned input/output channel registry replacing the hard-coded 20 and
  12, so future capabilities are registry entries rather than schema bumps.
- Structural caps with deterministic rejection and counting.

## Non-Goals

- No plasticity. `PlasticityGenes` are carried, inherited, validated, and
  behaviorally inert until Phase 11, following exactly the precedent set by
  thermal preference and defense tendency in Phase 2.
- No new sensory or action capability. The registry is populated with
  exactly the channels that exist after Phase 7, so the phase measures the
  effect of structural freedom and nothing else.
- No indirect or developmental encoding. A coarse regulatory locus type is
  reserved and unallocated, and is an open question, not a commitment.
- No schema 1 to schema 2 genome migration. Schema 1 worlds stay schema 1
  forever.
- No SIMD or batching rework. Variable topology makes the existing batching
  assumption invalid; the replacement strategy is a measurement question for
  a later performance slice, not a Phase 9 deliverable.

## Prerequisites

- Phase 5 (multi-seed harness) and Phase 7 (the channel set the registry
  will describe).

## Determinism Notes

- New streams: `Meiosis` (8), `StructuralMutation` (9).
- Innovation IDs come from a monotonic world counter, allocated in the
  lifecycle phase in ascending child object-ID order.
- Parent haplotype slots assigned by ID comparison (Rule 3).
- Per-node edge summation in ascending edge `homology_id` order (Rule 6).
  This is the single most easily overlooked determinism requirement in the
  phase: float addition is not associative, so a storage-order sum is a
  latent replay bug that only appears after a compaction changes layout.
- The hybrid update needs a **canonical topological order** over the
  zero-delay subgraph, canonicalized by `homology_id` with ties broken by it,
  so node evaluation order is a pure function of the genome. Delayed and
  recurrent edges read prior-state buffers, so no cycle handling is needed
  and a cycle among zero-delay edges is a decode-time error rather than a
  runtime condition. Activations and prior-state buffers are both world state
  and enter the checksum under `lifesim-activation-state-v1`.
- Checksum sections `lifesim-genome2-state-v1` and
  `lifesim-activation-state-v1`, present only under schema 2.

## Acceptance Criteria

Conditions, matched on seeds (30), config, and run length:

- **A**: structural mutation enabled (duplication, deletion, insertion,
  transposition at configured rates).
- **B**: structural mutation disabled; point mutation only, at a rate
  adjusted so total expected value-change per genome per generation matches
  A. Matching the total mutational input is what makes the comparison about
  *structure* rather than about *mutation load*.
- **C**: a neutral-marker control config with selection disabled, used only
  for the genetics-validation criteria.

Criteria:

- [ ] **C9.1 Structural evolution actually occurs.** Under A, the median
      expressed node count and edge count among living organisms at tick T
      differ from the founder median by at least the stated amount in at
      least 20 of 30 seeds, and the population contains more than one distinct
      (node count, edge count) pair in at least 25 of 30 seeds. Under B both
      are invariant by construction, which is the control confirming the
      measurement instrument.
- [ ] **C9.2 Structural freedom does not destabilize the ecology.** Median
      population and median lifespan under A are within the stated tolerance
      of B, or better, in at least 20 of 30 seeds. A topology system that
      simply kills worlds has not delivered evolvability.
- [x] **C9.3 Mendelian validation. Met** (`phase9_genetics.rs`), and it
      **found two real defects first** (D-068): transmission of a
      heterozygote's second allele measured 0.14 rather than 0.5. After the
      corrections, Hardy-Weinberg holds across 40 generations of random
      mating at n=600 with mean deviation under 0.06, and direct
      measurement puts allele transmission inside 0.47 to 0.53. Under condition C, at a marked neutral
      biallelic locus under random mating with selection disabled, observed
      genotype frequencies match Hardy-Weinberg expectation within sampling
      error across at least 30 generations, in at least 25 of 30 seeds. This
      is the check that meiosis is unbiased and that dominance expression is
      not silently distorting allele transmission.
- [x] **C9.4 Linkage validation. Met** (`phase9_genetics.rs`). The measured
      map function is 0.016, 0.031, 0.063, 0.117, 0.206, 0.360, 0.493 at
      distances 1 to 63: monotone, asymptotic to one half, and never above
      it. Recombination fractions exceeded 0.5 before the four-strand
      correction (D-068). Under condition C, the association
      between alleles at two marked loci decays with their map distance at
      the rate the configured crossover model predicts, within stated
      tolerance. This is the check that crossover does what the spec says
      and that linkage exists at all.
- [ ] **C9.5 Duplication versus explicit insertion.** Report the structural
      growth rate and the C9.2 stability outcome separately for a
      duplication-only variation policy and an insertion-enabled one. This
      is not pass or fail; it is the measurement that ADR-0013 commits to
      making rather than asserting. If duplication alone is too slow to
      produce C9.1 within the run budget, that is a finding and the
      insertion operator becomes the default with the reason recorded.
- [~] **C9.6 Bounded and fail-closed. Codec half met**: 100,000 seeded
      malformed cases produced zero panics and zero invalid admissions, with
      every accept re-validated and round-tripped (4,777 accepts, so the
      structural validation is genuinely exercised rather than everything
      dying at the checksum). The cap-rejection half needs the mutation
      operators, which are not implemented. A seeded malformed-input harness of
      at least 100,000 cases over the schema 2 codec produces zero panics
      and zero invalid admissions, every accept re-validated and
      round-tripped. Every structural cap rejects deterministically, counts,
      and events; no cap is ever silently exceeded.
- [~] **C9.7 Determinism and fixtures. Fixture half met**: a schema-1
      configured world still reproduces `0xff9dfcff5dffbf42` with schema 2 in
      the build. Replay, storage permutation, and the compaction test need
      world integration. Clean-process replay of the Phase 9
      fixture; storage-permutation equality; edge-summation order
      independence from storage layout proven by a compaction test; schema 1
      configured worlds still reproduce `0xff9dfcff5dffbf42` exactly.
- [ ] **C9.8 Snapshot budget.** Snapshot size per organism and checkpoint
      cost are measured at both tiers under a realistic evolved topology
      distribution, and the structural caps are set from that measurement.
      Caps chosen before the measurement are provisional and must be
      restated afterward.

## Test Plan

- Codec: bounded fail-closed decode of every header field, per-chromosome
  locus counts, value bounds, sortedness, dangling references; the 100,000
  case harness.
- Expression: pure function; identical phenotype and network after save and
  restore; dominance formula at the boundary cases (both dominances zero,
  one zero, hemizygous locus).
- Meiosis: order independence (swap parent visit order, identical child);
  crossover position determinism; homologues of different lengths segregate
  correctly.
- Structural operators: duplication produces valid fresh innovation IDs;
  deletion guards reject orphaning; transposition preserves content; every
  operator output re-validates.
- Evaluation: hybrid update equality under node storage permutation;
  zero-delay propagation crosses more than one edge in a tick; a zero-delay
  cycle is rejected at decode;
  edge summation order pinned; recurrent topologies evaluate without special
  handling; non-finite neutralization counted and evented.
- Genetics validation: C9.3 and C9.4 as automated statistical tests with
  recorded tolerances and seeds, not as manual analyses.
- Long run: multi-generation stability with structural churn, exact ledgers,
  bounded genome sizes.

## Benchmark Impact

This phase changes the cost model fundamentally. Phase 2 measured controller
evaluation at roughly 0.32 to 0.33 microseconds per organism per tick with a
fixed 20-16-12-12 topology and stack buffers with zero heap allocation.
Variable topology means per-organism cost now varies with evolved size and
the existing batching assumption (group organisms by topology ID) no longer
holds, because topologies are no longer shared.

Required measurements: controller phase cost as a function of node and edge
count; allocation behavior (the zero-per-tick-allocation property must be
preserved or its loss explicitly recorded); snapshot size per organism
versus topology size; memory per organism. Record the distribution, not just
the mean, because evolved topology sizes will be skewed.

Benchmark schema 4.

## Documentation Updates

`docs/07-neural-network-design.md`, `docs/08-genetics-and-evolution.md`,
`docs/09-species-and-lineage.md` (genetic distance across variable
structures), `specifications/organism-genome.md`,
`specifications/neural-network-schema.md`,
`specifications/entity-component-model.md`,
`specifications/world-save-format.md`,
`research/neural-network-options.md`, decision log, risk register, ADR-0013.

## Risks

| Risk | Mitigation |
|---|---|
| Genome bloat: duplication grows genomes until snapshots and memory become unmanageable | Hard caps with deterministic rejection; caps set from the C9.8 measurement; deletion rate configured to be non-negligible; genome size is a reported metric with an alert threshold |
| Per-organism cost variance makes tick time unpredictable | Measure the distribution; cap per-organism edge count; a bounded evaluation budget per organism per tick is available as a fallback policy and would itself be a selection pressure toward small networks, which must be reported if used |
| Loss of batching regresses performance badly | Measured, not assumed. If severe, the fallback is grouping by structural signature rather than exact topology, which is a later performance slice |
| Duplication-driven growth is too slow to show C9.1 in the run budget | C9.5 makes this an explicit measured comparison with a defined fallback rather than a discovered failure |
| Diploidy doubles genome storage | Real and accepted. It is the direct cost of the realism policy and is recorded in ADR-0017 |
| Structural distance for mating compatibility creates instant reproductive isolation and fragments the population | Compatibility weights are config; sweep before the campaign; monitor population fragmentation as a reported metric |

## Rollback

Genome schema is a config choice. A schema 1 world under a schema 2 build
runs the schema 1 paths unchanged. Schema 1 decode, evaluation, fixtures,
and tests stay in the build permanently as historical evidence. There is no
migration to reverse because there is no migration.
