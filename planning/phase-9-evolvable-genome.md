# Phase 9: Evolvable Genome, Diploid Genetics And Variable Topology

Status: **complete, with one criterion partial. 2026-08-05.** Landed:
channel and activation registries, the schema 2 genome model and derived
identity, the ALG2 bounded fail-closed codec with every structural
invariant, diploid expression with evolvable dominance, meiosis with all
four inheritance modes, the five mutation operators with typed counted
rejection, controller v2's hybrid evaluator, world integration, and the
C9.1/C9.2/C9.5/C9.8 campaigns. **C9.1 to C9.5 and C9.8 are met; C9.6 and
C9.7 are partial** - C9.6 because structural-cap rejections are counted and
checksummed but emit no event, C9.7 because a Phase 9 fixture,
storage-permutation equality, and the compaction test are unwritten. Decisions D-066 to D-079. Policy versions
`lifesim-genome-v2`, `lifesim-controller-v2`, `lifesim-meiosis-v1`,
`lifesim-structmut-v1`, `lifesim-structure-analysis-v1`. Specification:
`specifications/genome-schema-2.md`.

**The two results worth carrying forward.** First, **at the documented
default mutation rates structural evolution does not reach the population
median at all** - 0 of 30 worlds - because duplication and deletion ship at
the same rate, which puts genome size at mutation-deletion equilibrium with
no expected growth. It takes ten times the default duplication rate, or
insertion enabled at the default rate, to move the median. Standing
structural variation is present and substantial at every rate. Second,
**the structural caps were mutually inconsistent**: three of the four could
never bind, because the byte cap ran out at ~341 loci while the node and
edge caps needed 58 KB. Both were found by measurement rather than by
inspection, and neither would have announced itself.

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

**Provenance note, stated once because it applies to every number below.**
The archived manifest (`experiments/results/phase9-c91-confirmatory-manifest.txt`)
was produced at commit `0dc7794`, *before* C9.8 restated the structural caps.
The caps are hashed into the config, so re-running the campaign file today
reproduces the design and the results but not the config hashes. This is
harmless and it is checkable: the caps demonstrably never bound - measured
`rejected_cap` is 0 and the largest genome reached a tenth of the byte cap -
so no run outcome can have depended on them. The campaign file has since been
amended to **pin the caps explicitly**, so this coupling cannot recur; that
amendment is why the file on disk differs from the copy embedded in the
manifest, which is preserved verbatim as what actually ran.

Conditions, matched on seeds (30), config, and run length. As run:
`experiments/phase9-c91-confirmatory.campaign`, 60,000 ticks, seeds
3001..3030 (disjoint from the ecology sweep's 1..8 and the climate probe's
2002..2011), 128x128 at `cell_capacity_milli` 240,000 with extrinsic hazard
13 and climate off - the regime the ecology sweep selected by a rule fixed
before it ran.

- **A**: structural mutation enabled. Split into five arms, because the
  duplication rate turned out to be the variable that decides the result:
  **A1** (shipped defaults, duplication:deletion 1:1), **A3** (3:1),
  **A10** (10:1), and **A1i** / **A10i**, which add insertion at the same
  rate as duplication. Transposition is 0 everywhere: with single-chromosome
  founders it is provably inert (D-074).
- **B**: structural mutation disabled; point mutation only, at a rate
  adjusted so total expected value-change per genome per generation matches
  A. Matching the total mutational input is what makes the comparison about
  *structure* rather than about *mutation load*.

  That phrase has two defensible readings and both were run rather than one
  being picked silently. **B** holds the point rate identical to the A arms
  (same value mutation, no structural mutation); **Bload** raises it to
  A10's total operator rate (same total mutational input). The two agree on
  every criterion - both structurally invariant in 30 of 30, both within
  tolerance of each other - so the choice turned out not to matter, which is
  worth knowing rather than assuming.
- **C**: a neutral-marker control config with selection disabled, used only
  for the genetics-validation criteria. Met in `phase9_genetics.rs` as
  automated statistical tests rather than as a campaign.

Criteria:

- [x] **C9.1 Structural evolution actually occurs. Met, and the rate at
      which it is met is the result.** The stated amount was never stated, so
      it was fixed before the run at **founder + 1** - median node count >= 4
      or median edge count >= 3, the smallest shift a median of integers can
      express - in at least 20 of 30 seeds. The duplication rate was swept at
      1x, 3x, and 10x its documented default with deletion held at default,
      and **every rate is reported**, so there is no primary arm to select
      within after the fact.

      | condition | dup:del | median shift | diversity | median nodes | median edges | distinct |
      |---|---|---|---|---|---|---|
      | A1 (default) | 1:1 | **0/30** | 30/30 | 3 | 2 | 9 |
      | A3 | 3:1 | **0/30** | 30/30 | 3 | 2 | 13 |
      | A10 | 10:1 | **29/30** | 30/30 | 4 | 2 | 21 |
      | A1i (+insertion) | 1:1 | **30/30** | 30/30 | 3 | 3 | 18 |
      | A10i (+insertion) | 10:1 | **30/30** | 30/30 | 4 | 5 | 58 |
      | B (control) | - | 0/30 | 0/30 | 3 | 2 | 1 |
      | Bload (control) | - | 0/30 | 0/30 | 3 | 2 | 1 |

      **At the documented default rates, structural evolution does not reach
      the population median at all - 0 of 30 - and this is not a null
      result about the mechanism.** Duplication and deletion ship at the same
      rate, so genome size is at mutation-deletion equilibrium and has no
      expected growth; the standing variation is real and substantial (nine
      distinct structures, mean node count 3.094 against a founder's 3.000)
      and simply never fixes. The median is a fixation-scale statistic and the
      criterion asks a fixation-scale question.

      The diversity clause is met at **30/30 under every treatment condition**
      and **0/30 under both controls**, which is the cleanest separation in
      the phase. Both controls are structurally invariant in 30 of 30 - median,
      mean, and distinct count all exactly at the founding values - so the
      instrument is confirmed.

      What this establishes is narrower than "structure evolves": given a
      per-birth rate and a generation count, *some* structural change is
      arithmetic. What is not arithmetic is that the changes are viable and
      spread, and that is what the diversity and median columns measure.
- [x] **C9.2 Structural freedom does not destabilize the ecology. Met at
      every rate, on both quantities, and by the stronger test as well.** The
      tolerance was fixed before the run at 25 percent **or better** - one
      sided by design, because a treatment that raises population has not
      destabilized anything - with a bar of 20 of 30 seed-paired worlds.

      | condition | population | median lifespan | population TOST |
      |---|---|---|---|
      | A1 | 29/30 | 30/30 | equivalent |
      | A3 | 27/30 | 30/30 | equivalent |
      | A10 | 26/30 | 30/30 | equivalent |
      | A1i | 25/30 | 30/30 | equivalent |
      | A10i | 21/30 | 30/30 | equivalent |
      | Bload | 28/30 | 30/30 | equivalent |

      Every contrast also passes **TOST equivalence** on the relative scale
      against the same bound, which is a stronger statement than the count:
      the whole bootstrap interval sits inside +/-25 percent rather than
      merely most worlds doing so. **Zero extinctions in 210 worlds.**

      The cost is monotone in structural freedom and worth stating plainly:
      population falls a median 4.7 percent under A10 and 5.2 percent under
      A10i, and A10i is the only condition that comes near the bar at 21 of
      30. Structural freedom is not free; it is affordable.
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
- [x] **C9.5 Duplication versus explicit insertion. Measured, and the
      answer is that duplication alone is too slow at its documented rate.**
      Reported, not pass/fail, as the criterion specifies.

      **Insertion at the default rate does what duplication needs ten times
      its default rate to do.** A1i reaches the median shift in 30 of 30 at a
      1:1 duplication-deletion balance; A1, identical but for insertion being
      off, reaches it in 0 of 30. The two operators also grow different
      things - duplication moves node count (A10: mean 3.000 to 4.084, edges
      barely at 2.067) and insertion moves edge count (A1i: edges 2.000 to
      3.197, nodes barely at 3.120) - so they are complements rather than
      substitutes, and A10i moves both (nodes 4.307, edges 5.227, 58 distinct
      structures against A10's 21).

      **Insertion's price is inviable recombinants, and it is real but
      small.** Insertion adds edges between nodes drawn at random, and
      crossover can then separate an edge from the node it references, so the
      zygote has a dangling reference and no network. Per world: A10i wastes
      about 2,091 matings against roughly 84,500 births (2.5 percent), A1i
      about 26 (0.03 percent), A10 four in total across all thirty worlds.
      Rejection rates follow the same order - 19 percent of attempted
      mutations under A10i against 3.3 percent under A10 - and much of the
      remainder is the self-loop draw, which a three-node founder produces
      about a third of the time.

      **Consequence, recorded as the criterion requires:** the ADR-0013
      duplication-only baseline is retained as the *default*, because it is
      the biologically motivated mechanism and it does work - at 10x the
      shipped rate. Insertion is not made the default; it is recorded as the
      cheaper route to structural change and as the operator a phase should
      enable when it needs edge growth specifically. What is changed is the
      documentation of the shipped rates, which are now known to sit below
      the threshold at which duplication produces population-level structural
      change in 60,000 ticks.
- [x] **C9.6 Bounded and fail-closed. Met 2026-08-10; the event half was
      the last piece.** 100,000 seeded malformed cases produced zero
      panics and zero invalid admissions, with every accept re-validated and
      round-tripped (4,777 accepts, so structural validation is genuinely
      exercised rather than everything dying at the checksum).

      Every structural cap now rejects deterministically and is **counted**,
      by typed reason, in world state that enters the checksum -
      `every_rejection_is_counted_rather_than_silent` drives a cap until it
      binds and checks the count. Two reasons were added because the
      counters were not readable otherwise: `Inapplicable` for a precondition
      that does not hold, and `Cycle` for an insertion that would close one
      (D-074).

      **The event now exists.** `EventKind::StructuralMutationRejected`
      (event log tag 12, event schema version 4) carries the child, the
      operator code, and the typed reason. `mutate` returns a fixed-size
      allocation-free `MutationReport` and the caller events it, so the
      kernel keeps its existing shape: `structmut` still knows nothing about
      the world or the log.

      **Every typed rejection is evented, not only `RejectReason::Cap`.**
      The criterion's wording is cap-specific ("no cap is ever silently
      exceeded") but the counters it names are per-class, and a log that
      carried a strict subset of the counters could not be reconciled
      against them. The reason field makes the cap subset filterable, and
      the classes that are expected rather than alarming - `Inapplicable`,
      `Cycle` - are exactly the ones whose *rate* is worth watching.

      That reconciliation is the test.
      `every_structural_rejection_is_evented_as_well_as_counted` runs 10,000
      ticks with every operator at a rate that makes rejections happen, and
      asserts the evented count equals the counter **class by class** - a
      total would agree while two classes were swapped - with
      `dropped_events_total == 0` so the comparison is against a complete
      stream, and a guard that at least two classes were exercised so the
      equalities are not all `0 == 0`.
- [x] **C9.7 Determinism and fixtures. Met 2026-08-10, with one clause
      discharged by a stated substitute because the clause as written is
      unfalsifiable.** A schema-1 configured world still reproduces
      `0xff9dfcff5dffbf42` and a Phase 1 world still reproduces
      `0x1e3158a26afd3b39`; all four determinism scripts pass.

      **The Phase 9 fixture**: config `0x9abc0cd47914127f`, state
      `0x5f0c4e95e4f5170f`, 500 organisms, seed `0x5eedcafef00dbeef`,
      **8,000 ticks**, replayed across two clean processes by
      `scripts/verify-phase9-determinism.sh` - which also `grep`-pins both
      literals, closing a gap `verify-phase1` and `verify-phase2` leave open
      (they compare two runs to each other and never check the constant).
      The genome2 caps, meiosis mode and all eight mutation rates are pinned
      literally, so a default revision cannot move the fixture silently.

      **The horizon is the load-bearing choice.** At the Phase 1/2 horizon of
      500 ticks this fixture would pin nothing about Phase 9: `maturity_age_
      ticks` is 600 and founders spawn at age 0, so 500 ticks gives **zero
      births**, zero structural mutations, one distinct structure. Measured
      at the fixture's own seed: 1,500 ticks gives 15 births and no
      duplications; 3,000 gives 36 births and one point mutation, still no
      duplications; 8,000 gives 124 births, 8 structural mutations applied,
      2 duplications, 3 distinct structures. The fixture test asserts every
      one of those counts alongside the checksum, so it cannot silently
      become a control - trap 1 in the evidence list, arriving through a
      horizon rather than through an extinction.

      **Storage-permutation equality could not be tested as specified.**
      Rule 4 asks that permuting a saved population's storage order and
      restoring it produce identical checksums - but `World::from_state`
      fails closed on ids that are not ascending, and a *complete joint*
      permutation followed by the id-sort needed to satisfy that check is the
      **identity on the record**. The clause is therefore unfalsifiable as
      written. Discharged by three tests that each assert something: a
      positive control proving the permutation harness is exact, whose own
      comment says it is not the evidence; the evidence, which rotates **each
      per-organism array on its own** and requires the world to refuse the
      restore, checksum differently, or diverge within 200 ticks - 23 arrays
      inventoried by exhaustive destructuring, 18 detected and 5 asserted
      *constant*, so "not noticed" can never quietly mean "carried but never
      read"; and the fail-closed control. `determinism-extensions.md` Rule 4
      is amended to say this rather than leaving the original requirement
      standing (D-089).

      **The compaction tests** exist at both levels and were each verified by
      breaking the mechanism: deleting the plan swap or the activation swap
      from `Schema2State::retain` fails them. They are a regression guard
      against a *future* layout refactor, not a bug hunt - nothing today
      reorders loci within an organism, and the tests say so.

      One thing that is **not** evidence here, recorded because it was nearly
      claimed as such: the confirmatory campaign was run twice, and the two
      runs' per-world state checksums cannot be compared, because
      `World::state_checksum` hashes the config hash into its preamble and
      the two runs used different structural caps. Cross-run agreement was
      instead checked on behavioural quantities - population, births,
      generations, and every structure metric - which is what shows the cap
      restatement changed nothing.
- [x] **C9.8 Snapshot budget. Measured; caps restated.** Measured under the
      confirmatory campaign's own mutation regime after 30,000 ticks, at both
      documented tiers, with the entity guard deliberately binding so the
      tiers are populations rather than labels on one measurement.

      | tier | population | snapshot | bytes/organism | genome share | encode | decode | restore |
      |---|---|---|---|---|---|---|---|
      | 500 | 500 | 937 KB | 1,875 | 77.9% | 17.2 ms | 16.0 ms | 13.9 ms |
      | 2,000 | 1,999 | 3.35 MB | 1,676 | 87.2% | 52.8 ms | 54.0 ms | 62.1 ms |

      Evolved topology distribution: nodes p50 3, p90 4, p99 5-6, **max 7**;
      edges p50 2, p90 2, p99 4, **max 4**; genome bytes p50 1,450, p99
      1,605, **max 1,692**. Marginal cost of a structural locus is **44.4
      bytes**, against a **1,229-byte** fixed cost for a founder's header and
      trait block.

      **Schema 2 snapshots are smaller than schema 1's, not larger.** Phase 4
      recorded roughly 2.8 KB per organism for the flat genome; an evolved
      schema-2 genome is 1.68 to 1.88 KB, because a minimal evolved topology
      is smaller than a fixed 20-16-12-12 one. Diploidy doubles the genome and
      variable topology more than pays it back at the sizes that evolved.

      **The provisional caps were mutually inconsistent, and that is what
      forced a real restatement rather than a confirmation.**
      `max_genome_bytes` of 16,384 admits about 341 structural loci; the node
      cap (256, checked across both haplotypes, so 512 node loci) and the edge
      cap (1,024, so 2,048) would have needed 58 KB, and
      `max_loci_per_chromosome` of 512 exceeded the byte budget on its own.
      **Three of the four caps could never bind.** The rule adopted is that
      every cap must be individually reachable within `max_genome_bytes`,
      which stays the joint budget:

      | cap | was | now | basis |
      |---|---|---|---|
      | `max_genome_bytes` | 16,384 | **16,384** | 32.8 MB worst case at tier 2,000 against 3.35 MB actual; affordable, unchanged |
      | `max_nodes` | 256 | **160** | reachable inside the byte budget; 23x the observed maximum of 7 |
      | `max_edges` | 1,024 | **160** | reachable; 40x the observed maximum of 4 |
      | `max_edges_per_node` | 64 | **32** | reachable given `max_edges`; 32x observed |
      | `max_loci_per_chromosome` | 512 | **160** | reachable inside the byte budget |
      | `max_chromosomes`, `min_nodes` | 4, 2 | **4, 2** | unchanged |

      **The restated caps are jointly reachable, not merely individually
      so**, which is stronger than the rule required: the node and edge caps
      together need 15,426 bytes against the 16,384 budget. And they do not
      bind - `rejected_cap` is **0** over 30,000 ticks at the campaign's most
      aggressive duplication rate, with the largest genome reaching 1,668
      bytes and 6 nodes. The campaign now pins the caps explicitly, so its
      effective config cannot move when a default is revised.

      **Not validated for flagship scale, and that is a stated limit.**
      Soak-30 is roughly 16,500 generations against the 61 measured here, and
      duplication above the deletion rate is a growth process with that much
      longer to act. Genome size is now a structural quantity that Soak-30's
      stationarity criterion (D-055) has to watch.
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

Benchmark schema 4. Run by `scripts/run-phase9-benchmarks.sh`, split across
two crates because the snapshot half needs the codec and `sim-core` is
dependency-free.

**Measured, and the direction is the opposite of what this section
predicted.** Schema 2 is *faster* than schema 1 and its snapshots are
*smaller*:

| tier | schema 1 | schema 2 | ratio |
|---|---|---|---|
| 500 | 274.7 us/tick (3,640 t/s) | 190.3 us/tick (5,255 t/s) | **1.44x faster** |
| 2,000 | 1,692.5 us/tick (591 t/s) | 1,056.6 us/tick (946 t/s) | **1.60x faster** |

Snapshot: 1.68 to 1.88 KB per organism against Phase 4's roughly 2.8 KB for
the flat genome.

**This is a statement about how little structure evolved, not about the
evaluator being efficient, and it must not be read as the latter.** A
schema-1 controller is a fixed 20-16-12-12 network on every organism; an
evolved schema-2 controller has a median of three nodes and two edges. The
comparison is between a large fixed network and a small evolved one, and the
small one wins on both cost and size for exactly that reason. The batching
loss the section anticipated is real and is simply dominated by the size
difference at the topologies that evolved. At the caps - 160 nodes and 160
edges - the ordering would reverse, which is why the caps are set from a
budget rather than from the observed distribution alone.

The zero-per-tick-allocation property is preserved, asserted by a
capacity-watching test rather than by inspection (D-071).

## Documentation Updates

`docs/07-neural-network-design.md`, `docs/08-genetics-and-evolution.md`,
`docs/09-species-and-lineage.md` (genetic distance across variable
structures), `specifications/organism-genome.md`,
`specifications/neural-network-schema.md`,
`specifications/entity-component-model.md`,
`specifications/world-save-format.md`,
`research/neural-network-options.md`, decision log, risk register, ADR-0013.

## Risks

Every risk below now carries its measured outcome. Four of the six did not
materialize; one materialized exactly as written; one is untested.

| Risk | Mitigation | Outcome |
|---|---|---|
| Genome bloat: duplication grows genomes until snapshots and memory become unmanageable | Hard caps with deterministic rejection; caps set from the C9.8 measurement; deletion rate configured to be non-negligible; genome size is a reported metric with an alert threshold | **Did not materialize at campaign scale, and is untested at flagship scale.** Largest evolved genome 1,692 bytes against a 16,384 cap; `rejected_cap` zero in 210 worlds. But 61 generations is not 16,500, and duplication above the deletion rate is a growth process - carried to Soak-30 (D-055, D-078) |
| Per-organism cost variance makes tick time unpredictable | Measure the distribution; cap per-organism edge count; a bounded evaluation budget per organism per tick is available as a fallback policy and would itself be a selection pressure toward small networks, which must be reported if used | **Did not materialize.** The distribution is tight - nodes p50 3, p99 6, max 7 - so the bounded-evaluation fallback was not needed and no selection pressure toward small networks was introduced |
| Loss of batching regresses performance badly | Measured, not assumed. If severe, the fallback is grouping by structural signature rather than exact topology, which is a later performance slice | **Did not materialize, and the sign is inverted:** schema 2 runs 1.4x to 1.6x *faster* than schema 1. The batching loss is real and is dominated by evolved topologies being far smaller than the fixed 20-16-12-12 one. This would reverse near the caps |
| Duplication-driven growth is too slow to show C9.1 in the run budget | C9.5 makes this an explicit measured comparison with a defined fallback rather than a discovered failure | **Materialized exactly as written, at the shipped rates.** Duplication at its default rate moves the median in 0 of 30 worlds; it needs 10x, or insertion at 1x. Because C9.5 existed, this arrived as a measurement with a stated fallback rather than as a failed campaign |
| Diploidy doubles genome storage | Real and accepted. It is the direct cost of the realism policy and is recorded in ADR-0017 | **Repaid.** Per-organism snapshot cost is 1.68 to 1.88 KB against schema 1's roughly 2.8 KB: the doubling is smaller than what variable topology saves |
| Structural distance for mating compatibility creates instant reproductive isolation and fragments the population | Compatibility weights are config; sweep before the campaign; monitor population fragmentation as a reported metric | **Untested.** No fragmentation appeared - zero extinctions, populations within 25 percent of control at every rate - but compatibility weights were not swept, and at a median of 3 nodes there is little structural distance for the metric to act on. Revisit when evolved structures diverge further |

## Rollback

Genome schema is a config choice. A schema 1 world under a schema 2 build
runs the schema 1 paths unchanged. Schema 1 decode, evaluation, fixtures,
and tests stay in the build permanently as historical evidence. There is no
migration to reverse because there is no migration.
