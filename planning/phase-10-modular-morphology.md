# Phase 10: Modular Morphology And Development

Status: **complete, with one criterion not met. 2026-08-06.** Landed: the
module registry and discrete lattice, derived phenotype, genome schema 3's
regulatory loci with the bounded growth program, world integration through a
narrow phenotype seam, bodies derived rather than stored, and the C10.3 and
C10.6 campaigns. **C10.1, C10.2, C10.4, C10.5, C10.6, C10.7, C10.8, C10.9,
C10.10 and C10.11 are met; C10.3 is not.** Decisions D-080 to D-087. Policy
versions `lifesim-morphology-v1`, `lifesim-develop-v1`,
`lifesim-morph-analysis-v1`; genome schema 3. Specification:
`specifications/morphology-and-development.md`. Decision: ADR-0019.

**The one sentence worth carrying: morphology matters, and it does not
spread.** Among the morphological variation that exists, body size predicts
reproductive success in 26 of 30 worlds against a within-world permutation
null - the strongest positive result in the phase. But the median body is
the founder's three modules in all 30 worlds while the mean is 4.65 and the
median world carries 33 distinct bodies, so C10.3's divergence clause fails
0 of 30 and the conjunctive criterion is **not met**. Eighty-nine percent of
growth-rule mutations are silent (D-080), so morphology diverges in the tail
and never in the median. This is the fixation-scale problem D-079 recorded
for C9.1, recurring one phase later on a different mechanism.

**The developmental encoding survived its own gate**, which was not a
foregone conclusion: ADR-0022 D1 made C10.4 pass/fail precisely so that a
discontinuous genotype-phenotype map would take the parameterized fallback
instead. It passes, narrowly, and the margin is recorded rather than
smoothed.

## Problem

Body plan is a small fixed parameter set. Every organism has the same
structure and only its numbers differ. Nothing about shape can evolve.

This blocks the goal in two distinct ways. Morphological radiation is not
reachable when structure is fixed. And there is no representation in which
"one cell" and "many differentiated cells" are the same kind of object,
which is exactly what Phase 16's transition requires.

## Scope

- An organism as typed modules on a discrete integer lattice.
- A bounded versioned module type registry: structural, sensory, motor,
  digestive, storage, reproductive, neural.
- Derived phenotype: mass, speed, sensor range, intake, storage capacity,
  basal cost, and controller node budget all become consequences of the
  module set.
- Genome schema 3: regulatory loci carrying a developmental growth program,
  allocating the locus type ADR-0013 reserved.
- Deterministic bounded development from a single origin module.
- Non-viable bodies rejected at birth with typed reasons and counters.

## Non-Goals

- **No physical body simulation.** No rigid body dynamics, no joints, no
  torque, no soft tissue, no module-to-module collision. Modules confer
  capability and cost; they do not move relative to each other. This
  boundary is what keeps determinism exact and cost tractable.
- **No multicellularity mechanic.** A one-module body is a unicell and many
  modules is multicellular; the difference is a region of the same
  morphospace reached by ordinary structural mutation. Nothing detects,
  flags, or rewards the transition.
- No float geometry anywhere in morphology.
- No storage of bodies. They are derived from the genome, like phenotypes.
- No claim that any evolved body resembles any real organism.

## Prerequisites

- Phase 9 (genome schema 2 supplies the locus machinery, innovation IDs, and
  meiosis that schema 3 extends).

## Determinism Notes

- New stream `Morphogenesis` (17), unused under the default fully
  deterministic development policy; it exists so adopting developmental
  noise later cannot renumber.
- Regulatory loci evaluate in ascending `homology_id` order; actions apply
  in ascending `(locus_homology_id, lattice_index)` order.
- Every module sum iterates in ascending lattice index, pinning float
  summation order exactly as Rule 6 does for controller edges.
- Lattice positions are integers; morphology is exactly hashable.
- Development is a pure function of `(genome, config)`, which is why bodies
  are excluded from the save and recomputed on load.
- Checksum section `lifesim-morphology-state-v1` carries only the
  developmental clock and non-viability counters.

## Acceptance Criteria

- [x] **C10.1 Development is pure and order-independent. Met.** Same genome
      and config gives the same body across 200 genomes on both lattices; a
      body recomputed after save and restore is identical, which is what lets
      bodies be excluded from the save at all; and permuting regulatory locus
      storage order gives an identical body, because `Body::from_modules`
      sorts into canonical lattice order and `rules_of` sorts by
      `(homology_id, haplotype)`.
- [x] **C10.2 The unicellular case works. Met.** A one-module body is
      legal, viable, and produces bounded derived attributes on both
      lattices, verified directly rather than left for Phase 16 to discover.
      It has mass, upkeep and intake; with no motor it has zero thrust and
      sits at the speed floor, which is a sessile organism rather than an
      invalid one.

      Worth recording for Phase 16: a one-module body is legal but is **not**
      what this phase's founders are. The founder grows gut, motor and sensor
      because a lone gut is immobile and blind and dies (D-085), so the
      unicellular case is a reachable region of the morphospace rather than
      the starting point.
- [ ] **C10.3 Morphological change has consequence, not just variance. NOT
      MET on the pre-registered rule, and the clause that fails is not the
      one that matters most.** Conjunctive over three clauses, because A13
      says novelty is not progress. Across 30 worlds under M (evolvable
      morphology) against F (growth program present, inherited, expressed,
      and frozen):

      | clause | rule | result |
      |---|---|---|
      | a. divergence | median module count above the founder's 3, and more than one distinct body | **0/30** |
      | b. consequence | body size correlates with offspring beyond a 199-fold within-world permutation null | **26/30** |
      | c. persistence | divergence present at both the halfway and final samples | **0/30** |
      | **all three** | | **0/30**, bar 20 |

      **The divergence clause fails on the median while the mean moves a
      long way.** Median module count is 3 - the founder body - in all 30
      worlds, while the *mean* is 4.65 and the median world carries **33
      distinct bodies**. More than half the population keeps the founder
      shape and a minority grows much larger. That is the same fixation-scale
      problem D-079 recorded for C9.1 one phase earlier: a median moves only
      when a variant reaches half the population, and at 89 percent silent
      morphological mutations (D-080) nothing gets close.

      **The consequence clause is the phase's strongest positive result and
      it is worth stating on its own.** Among the morphological variation
      that does exist, body size predicts reproductive success: median
      Spearman correlation +132 milli against a permutation null of +47, in
      26 of 30 worlds, with the observed correlation ranging +5 to +285. The
      null is a within-world shuffle of bodies against reproductive records,
      so it preserves both marginals and the entire age structure and removes
      only the pairing - the confound an unpaired comparison cannot address.
      Under F the same measurement is undefined in 30 of 30 worlds, because
      a frozen program produces exactly one body and there is nothing to
      correlate. **Morphology matters; it just does not spread.**

      The criterion stands as **not met**. The rule was fixed before the run
      and is not being revised after seeing the data. What is recorded for
      the next phase to use is that "divergence from the founder
      distribution" was operationalized here as a median shift, and a
      distributional statistic - 33 distinct bodies against the founder's
      one - would have answered a different and arguably better-posed
      question.
- [x] **C10.4 Discontinuity gate. PASSES, narrowly.** ADR-0022 D1 promoted
      this from a metric to a gate: a failure takes the parameterized
      body-plan fallback and abandons the developmental encoding. Threshold
      fixed before measuring - a single-locus mutation must leave more than
      half the body intact, median phenotypic distance below 500 milli.

      Result over 60,000 trials, 11,019 of which reached a growth rule:
      **median 0 against a bar of 500 - the gate passes.** It passes by one
      percentage point of a vacuity guard set in the same breath: **89.0
      percent of morphological mutations change nothing at all**, against a
      pre-registered ceiling of 90. On bodies of four or more modules, where
      the distance metric has resolution at all, the median *effective*
      mutation moves 351 milli and is comfortably continuous; the all-sizes
      effective median of exactly 500 is the metric's quantization on
      one-module bodies rather than a discontinuity. Zero mutations were
      lethal.

      **The silence is the finding, not the pass**, and C10.3's result is
      what it predicted: nine in ten morphological mutations do nothing, so
      morphology diverges in the tail and never in the median (D-080).
- [x] **C10.5 Non-viability rate is bounded and reported. Met, and the
      rate is zero.** No body was refused as non-viable in any of the 90
      campaign worlds or either benchmark lattice. That is a consequence of
      D-081: non-viability is reserved for bodies that are structurally
      impossible - empty, disconnected, overlapping, out of bounds - rather
      than merely doomed, and the growth grammar cannot produce those from a
      connected origin. Every rejection class is typed and counted in world
      state and enters the checksum, so a campaign cannot run against one
      silently.

      The refusals that **do** occur are node-budget refusals (C10.7), which
      are a coupling rather than a viability failure and are counted
      separately.
- [x] **C10.6 Structural freedom does not destabilize the ecology. Met.**
      M against F, seed-paired, within 25 percent **or better**:

      | quantity | within or better | mean relative | TOST |
      |---|---|---|---|
      | median population | **22/30** | -10.6% | equivalent |
      | median lifespan | **28/30** | -11.2% | equivalent |

      Bar is 20 of 30 and both clear it, and both are additionally
      TOST-equivalent against the same bound, which is stronger than the
      count. **Zero extinctions in 90 worlds.** Evolvable morphology costs
      about a tenth of the population and a tenth of the lifespan against a
      frozen body plan - a real cost, reported rather than explained away,
      and comfortably inside the tolerance.
- [x] **C10.7 Brain costs body. Met.** Controller node budget is the config
      floor plus what neural modules confer, and a child whose expressed
      network needs more nodes than its body supports is **refused and
      counted**, never trimmed - a trimmed network is one no genome encoded.
      Neural is the only module type whose upkeep is superlinear in scale, so
      doubling neural scale multiplies upkeep by sixteen while a motor's goes
      up by eight: cognition is expensive structurally rather than by
      stipulation.

      Measured binding: 282 refusals on the square lattice and 352 on the
      hex over 30,000 ticks. Verified in both directions - with the floor
      below the founder network's node count the budget binds, and with a
      generous floor the same world refuses nothing, so the refusal is a
      coupling rather than an unconditional rejection.
- [x] **C10.8 Bounds and caps. Measured; caps confirmed with the evidence.**
      Module-count distribution after 30,000 ticks at the campaign's own
      mutation regime, at the 2,000-organism tier:

      | lattice | p50 | p90 | p99 | max | distinct | non-viable |
      |---|---|---|---|---|---|---|
      | square | 3 | 3 | 3 | **18** | 11 | 0 |
      | hex | 3 | 3 | 3 | **6** | 10 | 0 |

      **The distribution is extraordinarily concentrated**: p99 is the
      founder's 3 on both lattices, with a thin tail reaching 18 on the
      square lattice and 6 on the hex. Whatever else is true of this
      encoding, it does not produce runaway bodies.

      Caps are **confirmed rather than changed**, and unlike Phase 9's they
      are mutually consistent to begin with - the C9.8 rule that every cap
      must be individually reachable holds here without adjustment:

      | cap | value | reachable? | against observed |
      |---|---|---|---|
      | `max_modules` | 64 | yes: the radius-8 lattice has 289 cells | 3.6x the maximum of 18 |
      | `lattice_radius` | 8 | yes: needs 9 modules in a line, well inside `max_modules` | never approached |
      | `max_growth_steps` | 16 | yes: doubling reaches 64 in six steps | never binds |
      | `required_types_mask` | 0 | n/a | nothing is structurally required (D-081) |

      Zero non-viable bodies and zero module-cap refusals at either lattice.
      The cap that **does** bind is the controller node budget - 282 and 352
      refusals on square and hex - which is C10.7 working rather than a limit
      being hit.

      **Not validated for flagship scale**, the same caveat D-078 records for
      the structural caps: 30,000 ticks is roughly 30 generations against
      Soak-30's ~16,500, and a tail that reaches 18 in 30 generations is not
      evidence about where it reaches in 16,500. Module count joins genome
      size as a quantity Soak-30's stationarity criterion must watch.
- [x] **C10.9 Ledger exactness** with growth energy flowing through it over
      a 10^6-tick run. Met: `bench_phase10::phase10_ledger_exact_over_a_million_ticks`
      reconstructs the energy and biomass ledgers from first principles every
      10,000 ticks and at the end, and asserts the world neither went extinct
      nor stopped growing bodies - a million ticks of exactness over an empty
      world would prove nothing.
- [x] **C10.10 Snapshot size is unaffected. Met by construction and
      verified.** No body is stored: the morphology save section carries only
      the developmental counters, and a restored world regrows every body
      from its genome and reproduces the state checksum exactly. Per-organism
      snapshot bytes are therefore identical to Phase 9's. The verification
      that matters is not the byte count but
      `bodies_are_derived_so_a_restored_world_regrows_them_identically`,
      which steps both worlds 400 ticks after restore - a checksum match at
      rest would not catch a field that only matters once the world moves.
- [x] **C10.11 Fixtures preserved. Met.** Morphology disabled reproduces
      the Phase 2 fixture `0xff9dfcff5dffbf42` and the Phase 1 fixture
      `0x1e3158a26afd3b39` exactly, and the config section is hashed only
      when enabled (D-014), so a morphology-off world hashes as it did before
      Phase 10 existed. Checked behaviourally as well:
      `the_flat_world_is_untouched_by_morphology_existing` perturbs
      morphology caps on a disabled world and asserts both the config hash
      and the state checksum are unmoved.
## Test Plan

- Unit: derivation formulas at module-count and scale boundaries;
  connectivity validation; growth-step termination; cap enforcement.
- Property: every body reachable within caps produces in-bounds derived
  attributes; development always terminates.
- Determinism: C10.1 as automated tests; clean-process fixture; lattice
  summation order independence from storage layout under compaction.
- Integration: retired trait loci are genuinely unused; sensory and motor
  modules bind channels correctly; a one-module organism completes a full
  lifecycle.
- Statistical: C10.3 and C10.4 as automated tests with recorded tolerances,
  seeds, and sample sizes.
- Long run: multi-generation stability with morphological churn, exact
  ledgers, bounded module counts.

## Benchmark Impact

The second largest cost change in the programme after Phase 9.

Record: development cost per birth against genome regulatory-locus count;
per-organism per-tick cost against module count **as a distribution**, since
evolved sizes will be skewed; the interaction with controller cost, because
neural modules drive node budget and the two skews multiply; non-viability
rate; and confirmation that snapshot size per organism is unchanged.

Caps are set from this measurement, not before it.

Benchmark schema 6, run by `scripts/run-phase10-benchmarks.sh`.

**Measured, and as in Phase 9 the direction is the opposite of what this
section expected.** Morphology makes the tick *faster*:

| tier | morphology off | morphology on | ratio |
|---|---|---|---|
| 500 | 184.4 us (5,422 t/s) | 157.7 us (6,341 t/s) | **1.17x faster** |
| 2,000 | 953.6 us (1,049 t/s) | 827.0 us (1,209 t/s) | **1.15x faster** |

**This is not evidence that development is cheap, and it must not be read as
such.** Development runs once per birth and is bounded; the tick-rate
difference comes from somewhere else entirely. A founder body derives a
sensor range of 6,000 against the trait-derived organism's 7,898, and the
sense phase is a spatial query whose cost scales with that radius. Morphology
is winning on a smaller perception radius, not on evaluation. At a larger
evolved body - which the distribution shows is rare - the ordering would
reverse.

Snapshot size per organism is **unchanged**, because no body is stored
(C10.10). The module-count distribution is p50 3, p99 3, max 18 on the square
lattice and 6 on the hex - concentrated enough that the skew this section
anticipated, and its multiplication with controller skew, did not
materialize at campaign scale.

## Documentation Updates

`docs/06-organism-model.md` (body plan section is substantially rewritten),
`docs/07-neural-network-design.md` (node budget), `docs/08-genetics-and-evolution.md`
(schema 3), `docs/02-scope-and-non-goals.md` (rich morphology moves out of
deferred), `docs/09-species-and-lineage.md` (morphological distance),
`specifications/genome-schema-2.md` (locus type 5 allocated),
`specifications/entity-component-model.md`,
`specifications/neural-network-schema.md`,
`specifications/websocket-protocol.md` (module summary in render records),
`docs/10-observer-interface.md`, decision log, ADR-0019.

## Risks

Every risk now carries its measured outcome.

| Risk | Mitigation | Outcome |
|---|---|---|
| Genotype-phenotype discontinuity is severe enough that selection cannot act | C10.4 measures it directly. If severe, a more constrained growth grammar or the parameterized fallback is reconsidered, and that is a real possible outcome of this phase  | **Did not materialize, narrowly.** C10.4's gate passes at a median of 0 against a bar of 500, and 351 milli on bodies large enough for the metric to resolve. The fallback was not taken |
| High non-viability rate collapses effective fecundity | C10.5 with a stated ceiling; growth grammar constrained so most programs produce connected bodies  | **Did not materialize; the rate is zero.** Non-viability is reserved for structurally impossible bodies (D-081), which a connected growth grammar cannot produce |
| Development cost per birth dominates the tick | Bounded `max_growth_steps` and `max_modules`; measured before caps are fixed; development runs at birth, not per tick, unless ontogeny is enabled  | **Did not materialize.** The tick is *faster* with morphology on, for reasons unrelated to development cost - see Benchmark Impact |
| Module-count skew multiplied by topology skew makes tick time unpredictable | Both distributions measured; a per-organism evaluation budget is the fallback policy and would itself be a selection pressure toward small bodies, which must be reported if used  | **Did not materialize.** The module distribution is p50 3, p99 3: there is almost no skew to multiply. The per-organism evaluation budget fallback was not needed and no selection pressure toward small bodies was introduced |
| The lattice proves too restrictive for a research question | That is an argument for physical morphology and therefore a different project. Recorded in ADR-0019 rather than solved here  | **Untested.** Nothing in this phase pressed against the lattice: the maximum evolved body is 18 modules on a 289-cell lattice |
| Retired trait genes leave dead code paths that quietly still apply | Retired trait IDs are never reused; an integration test asserts they are unread  | **Held.** Three trait IDs are retired, never reused, still inherited, and unread; `from_body` overwrites exactly those three and nothing else |

## Rollback

One config section selecting genome schema. Disabled, schema 2 organisms use
the Phase 9 body-plan code path and reproduce its fixture. Schema 1, 2, and 3
decoders and fixtures all stay in the build permanently, and there is no
migration between them.
