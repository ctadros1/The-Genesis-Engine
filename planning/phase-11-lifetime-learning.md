# Phase 11: Lifetime Learning

Status: in progress from 2026-08-10. Policy version `lifesim-plasticity-v2`.
Specification: `specifications/plasticity-and-learning.md`.

**C11.1 and C11.2 were measured on 2026-08-11 and both are unmet, measured
nulls** (`experiments/results/phase11-c111-confirmatory-findings.txt`,
D-098 to D-105). They stood at NOT MEASURED until then; the four missing
pieces were built and the campaign was pre-registered in commit `4b160fe`
before it ran. C11.7 remains partial. The per-criterion status sits beside
each criterion below and never in place of it - the criterion text for
C11.1 through C11.8 is restored verbatim from before the 2026-08-10 status
pass, which had replaced several of them with their status. That is the
defect `170bce9` reverted in the Phase 12 plan, and it had already happened
here.

## Scope Correction, 2026-08-10: The Genes Are A Reserved Slot, Not A Mechanism

Recorded **before** any measurement, because it changes what this phase has
to build and because getting it wrong would have produced a confident null.

The backlog said `PlasticityGenes` had been "carried, inherited, and
validated since Phase 9 precisely so that enabling it is a flag rather than
a schema change." Carried and inherited, yes. **Reachable, no.** Verified
against the shipped code:

- `PlasticityGenes` is **discarded during diploid expression** - the gather
  destructures `LocusKind::Edge { source, target, weight, flags, .. }` and
  the `..` drops it. `ExpressedEdge` has no plasticity field.
- **No production path anywhere writes `EDGE_FLAG_PLASTIC`.** It is defined,
  masked, read during expression, and exported; it is written in exactly one
  test. `insert` writes `EDGE_FLAG_DELAYED`, `minimal_founder` writes
  `flags: 0`, `duplicate` copies its source. **No edge can become plastic.**
- `point_mutate`'s only `Edge` arm assigns `weight`. **`eta` can never leave
  zero.**
- `NodeRole` is never mutated, so `Modulatory` is unreachable by evolution
  and **rule forms 3 and 4 are dead on arrival**.

This is not a bookkeeping correction. The Risks table below names
"plasticity is selected to zero and the phase returns a null result" as
**the single most likely failure in this phase**. On the code as it stood
that null was mechanically guaranteed, and it would have been reported as a
finding about this world's structure. It is trap 16 in the evidence list:
inheritance and validation say nothing about whether anything can ever
*change* a field.

Therefore in scope, and not previously counted as such: expressing the genes
under a stated dominance policy, a point-mutation path that reaches every
plasticity field and the plastic flag, a node-role redraw so `Modulatory` is
reachable, and a `plasticity_enabled` gate that **consumes its draws either
way** so condition B is matched on total mutational input - the discipline
D-086 records for C10.3's control, which failed for the mirror-image reason.

## Numbering Corrections, 2026-08-10

- **Benchmark schema 7**, not the 5 stated below. Schema 5 was never emitted
  by any script; 6 is the highest in use (Phase 10). Recorded so nobody
  hunts for a 5 that does not exist.
- Event schema is already **4** as of C9.6, so a plasticity fault event is
  log tag **13** and event schema **5**.
- Snapshot section tag **12**; format version stays 3, since an absent
  optional section stays readable by every existing build.
- **Amended 2026-08-11.** The measurement substrate added a second section:
  the action census is snapshot tag **14**, and the format version by then
  reads **4** rather than 3, bumped by Phase 12 because the logical state
  gained a composed terrain checksum in its header - not because a section
  was appended. Tag 14 still does not move it, on tag 12's precedent.
- `RngSystem::PlasticityInit = 10` is reserved in
  `specifications/determinism-extensions.md` but **absent from the enum**;
  this phase adds it, unused.

## Problem

Controller weights are fixed at birth, so behavior can only change across
generations. Technology is cumulative culture, transmitted far faster than
genes can move. Without within-lifetime plasticity, any "discovery" collapses
into just another inherited trait, and Phase 13's transmission question
cannot even be posed: there would be nothing an organism could acquire that
it was not born with.

## Scope

- Per-edge plasticity genes, already carried inert since Phase 9, become
  live.
- A bounded versioned registry of plasticity rule forms; coefficients are
  genes under selection.
- Modulatory nodes: plasticity gated by an evolved network output, so what
  counts as reinforcing is itself evolved.
- Fixed-point learned-weight accumulation, saved and checksummed.
- Per-plastic-edge energy cost, so the amount of plasticity is under
  selection.
- A `learn` tick phase after `apply`.

## Non-Goals

- **No reinforcement learning against a hand-authored reward.** This is the
  central constraint of the phase. There is no fitness signal delivered to
  any network, no authored reward function, and no gradient computed against
  an objective we chose. The modulatory signal is an ordinary output of the
  organism's own evolved network. The existing non-goal in
  `docs/02-scope-and-non-goals.md` survives intact and is sharpened rather
  than relaxed. See ADR-0014.
- No Lamarckian inheritance by default. Learned state resets at birth.
- No observational learning. Rule form 5 requires the Phase 13 social channel
  and is not in the registry yet.
- No backpropagation, no error signal, no supervised target.
- No claim that any observed change constitutes cognition.

## Prerequisites

- Phase 9 (schema 2 carries the plasticity genes; variable topology makes
  modulatory nodes expressible).
- Phase 5's asynchronous checkpointing, because learned state increases
  snapshot size.

## Determinism Notes

- New stream: `PlasticityInit` (10), reserved and unused under the default
  zero-initialization policy.
- Learned state is Q16 `i32`, accumulated with integer arithmetic and a
  specified rounding rule, per Rule 7. Float accumulation over 10^5 or more
  ticks is exactly the fragility ADR-0011 exists to avoid.
- Plastic edges are updated in ascending edge `homology_id` order.
- The `learn` phase reads only values committed earlier in the same tick and
  writes only learned state.
- Checksum section `lifesim-learn-state-v1`, present only when enabled.
- **Added by the measurement substrate**: checksum section
  `lifesim-action-census-v1`, present only when
  `probe.action_census_enabled`, appended after Phase 12's section. It is
  hashed for `learnstate.rs`'s reason - a lifetime's counts have no source
  but the save - and the consequence is that `reset` moves the checksum, so
  the sampling path records cumulative rows and never resets. Nothing in the
  tick reads a count (ADR-0016), and the five fixtures are the assertion of
  that rather than the claim.

## Acceptance Criteria

Conditions, matched on seeds (30), config, and run length:

- **A**: plasticity enabled.
- **B**: plasticity disabled; `eta` forced to zero for every edge, genomes
  otherwise identical in distribution. Behavior can change only across
  generations.
- **E-stationary / E-variable**: an environmental-variability sweep applied
  across both A and B. In `E-stationary`, resource patch locations are fixed.
  In `E-variable`, patch locations shift on a configured schedule. This
  sweep is not optional: learning pays for itself only when the environment
  varies within a lifetime but is predictable within it, and testing
  plasticity only in a stationary world is a design that guarantees a null
  result for an uninteresting reason.

Criteria:

- [ ] **C11.1 Within-lifetime behavioral change.** In a controlled reversal
      probe (a resource patch is relocated at tick t), individual organisms
      alive both before and after the relocation show a measurable shift in
      their own action distribution within their own lifetime under A, and
      do not under B. The shift is computed **per individual**, because a
      population-level shift is explicable by selection and proves nothing;
      it is then **aggregated to a world-level rate, and the world is the
      replicate** (ADR-0022 A5). Individuals are nested observations, not
      sample size. Required in at least 20 of 30 worlds under `E-variable`.
      This is the phase's designated primary endpoint.

      **Status 2026-08-11: UNMET, and it is a measured null with a control.**
      Not the same as the NOT MEASURED it stood at until the confirmatory
      campaign ran, and the difference is recorded rather than blurred: a
      threshold was fixed in advance, the machinery the criterion needs was
      built, and the measurement was taken.

      | | Avar | Astat | Bvar | Bstat |
      |---|---|---|---|---|
      | worlds | 30 | 30 | 30 | 30 |
      | **directed count (the criterion)** | **0** | **0** | **0** | **0** |
      | two-sided count (reported only) | 29 | 30 | 30 | 30 |
      | median rho, milli | -55 | -58 | -58 | -64 |
      | null p95 range, milli | [6, 54] | [8, 13] | [7, 11] | [7, 14] |
      | individuals, median per world | 19,139 | 11,632 | 19,758 | 12,036 |

      Bar **20 of 30** under Avar, with the Bvar control required to stay
      **strictly below 20**. Avar reaches 0. The seed-paired contrast of rho
      between the arms is null as well: Avar minus Bvar is +3 milli, 95
      percent interval [-1, +8], p = 0.707.

      **The two-sided column is an artefact and must not be read as
      evidence.** It is 29 to 30 of 30 in every arm, and it is *strongest*
      in Bstat, where plasticity is disabled and the relocation has zero
      magnitude - nothing whatever happens at the event tick. An association
      that survives removal of the event was not caused by the event.

      **The directed statistic is not identified** (D-100). The matched
      control boundary sits at `event + relocate / 2`, which is 1,000 ticks
      later in every organism's life than the event boundary it is paired
      with. Measured in a stationary rolling cohort where nothing happens at
      the event tick and behaviour is a pure function of age, the offset
      alone gives rho = +158 against a null of 30 and **passes this
      criterion**, with a dose-response of 76 / 158 / 334 / 700 milli at
      offsets of 500 / 1,000 / 2,000 / 4,000 ticks. The sign follows the
      substrate's age trend, so this campaign's 0 of 30 depends on which way
      that trend happens to run. Nothing was changed to accommodate this -
      no threshold moved and no rule was touched - and the correction a
      re-run needs is filed in `docs/21-open-questions.md`. **C11.1 must not
      be re-measured with the current pairing.**

      **What the instrument could ever have seen** (D-101), reproduced
      independently on fresh worlds with a from-scratch `.alac` reader: over
      1,175,285 Avar records, `eat == age` and `mate == age` in 100.0000
      percent and `rest == 0` and `attack == 0` in 100.0000 percent. Only
      the three heading bands vary; they are a partition, so two degrees of
      freedom, and `turn_left` carries 0.59 percent of locomotion. C11.1's
      detectable space was effectively **one number** - the fraction of
      ticks spent turning right rather than heading straight - driven by the
      founder's single evolved `energy_fraction -> turn` path.

      Recorded before the campaign and preserved, because it is why the
      criterion stood at NOT MEASURED: three pieces it needs did not exist -
      scripted-intervention machinery (there was no command queue, no
      intervention list, and no relocatable resource patch; resources are a
      per-cell scalar field and nothing moved), per-organism action counting
      (intents were transient `pub(crate)` scratch with no accessor), and
      the analysis that reduces a per-individual shift to a world-level
      rate. All three now exist: `worldmod`'s relocating capacity patch,
      `sim_core::actioncensus` with the `.alac` artifact, and
      `sim_analysis::plasticity`.
- [ ] **C11.2 Learning is under selection.** The distribution of `eta` and
      plastic-edge-fraction at tick T differs from the founder distribution
      by more than the drift expectation, measured against a neutral marker
      locus in the same run as the drift control, in at least 20 of 30 seeds
      under `E-variable`. Under `E-stationary` the prediction is the
      opposite: plasticity should be selected *down* because it costs and
      does not pay. A result showing plasticity increasing in a stationary
      world would indicate the cost model is wrong, and would be reported as
      such.

      **Status 2026-08-11: UNMET, and it is a measured null with a live
      control.** Avar 8 of 30, Bvar 0 of 30, against a bar of 20 and a
      control ceiling of 20. All eight are on the plastic-flag scale; the
      `eta` scale is 0 of 30 in every arm. One Avar world went extinct and
      is reported `drift_no_variance` rather than as a failure, so the
      treatment count is 8 of 29 defined worlds.

      | | Avar | Astat | Bvar | Bstat |
      |---|---|---|---|---|
      | worlds counted | 8 | 8 | 0 | 0 |
      | on the eta scale | 0 | 0 | 0 | 0 |
      | on the plastic-flag scale | 8 | 8 | 0 | 0 |
      | median plastic fraction, milli | 75 | 68 | 0 | 0 |
      | median marker set fraction, milli | 61 | 61 | 72 | 68 |
      | median plastic excess, milli | +1 | -2 | -76 | -68 |
      | median eta excess, milli | 0 | 0 | -1 | 0 |
      | moved eta alleles | 26,654 | 15,699 | 0 | 0 |
      | moved marker alleles | 13,751 | 6,975 | 20,967 | 6,580 |

      The drift margin is 25 milli, anchored to one expected mutational step
      at the pinned `point_delta_q16 = 3277`. Avar's plastic-allele fraction
      exceeds its own marker's by **+1 milli**. Plasticity spread at the
      drift rate and not above it.

      **The within-arm marker is the control that decides this, and it is
      live.** In Bvar, `eta` and `EDGE_FLAG_PLASTIC` are frozen by the
      mutation gate - 0 of 684,525 edge alleles moved - while 20,967 marker
      alleles moved, so the marker carries the real population size, the
      real variance in reproductive success, the real linkage and the real
      mutation regime. **The criterion's *arm* control is not live**, and
      that is recorded rather than left implicit: with `eta` and the flag
      pinned at zero, both B excesses are always at most zero and
      `selected_over_drift` is unreachable by construction, so "the control
      stays strictly below the ceiling" is satisfied mechanically. It did
      not matter here, because the criterion failed on its treatment count
      alone; it would have mattered had Avar reached 20. A control bound to
      a constant is not a control, one rung up from where the findings file
      caught the same shape in `plastic_excess_milli`.

      **The E-stationary prediction is not observed either.** Astat clears
      the bar in the same 8 of 30 worlds with a median excess of -2 milli.
      Plasticity was not selected down in the stationary arm. Under the
      census reading below that is expected rather than a refutation of the
      cost model: the trait is very nearly neutral because almost every
      flagged edge carries rule 0 and does nothing.

      **The census changes what this null is about** (D-099). Plasticity was
      not selected down; **it was never assembled**. A nonzero learned delta
      needs four conditions on one edge locus, each behind a different one
      of seven point-mutation targets, plus a fifth for the two modulated
      rules. 9 of 684,370 Avar edge alleles satisfy all four - 13 per
      million, in 4 of 30 worlds, and at most 4 independent assembly events
      after the pseudoreplication correction. Diploid expression is more
      permissive, so 59 of the 48,119 expressed plastic edges clear it, and
      25 rows held a nonzero learned weight at tick 60,000 in 14 worlds.
      Every incomplete state on the path computes bit-identically to the
      founder, and `learn_phase` charges every flagged edge whatever its
      rule, so the interior of the path is a plateau and the flagged half of
      it is deleterious. There is no monotone non-negative path from the
      founder to the phenotype.

      Recorded before the campaign and preserved. The criterion stood at
      NOT MEASURED for C11.1's three reasons plus a fourth: there was **no
      neutral marker locus** anywhere in the genome, and the drift control
      is defined against one. Two measurements bore on it and were
      unfavourable. **Evolved plasticity is essentially absent**: after
      30,000 ticks with the mutation gate open, 1 plastic edge across 500
      organisms and 7 across 1,999, with `mean_abs_learned_milli` zero in
      both. And over the 10^6-tick ledger run the founders' 400 plastic
      edges reached **zero** by the end while 18.9 million updates
      accumulated along the way. Both readings survive the campaign; the
      census supplies the mechanism they lacked.
- [x] **C11.3 Learned state is world state.** Save, restore, and continue is
      bit-identical with plastic edges carrying nonzero learned deltas.
      Learned state cannot be recomputed from the genome and its presence in
      the snapshot is verified by a test that corrupts it and observes a
      trajectory divergence.

      **Met.** Save, restore and continue
      is bit-identical with plastic edges carrying nonzero learned deltas, and
      the record is compared field by field rather than only by checksum - a
      checksum match also holds for a pair of cancelling defects (zeroing on
      restore *and* dropping the section from the hash). The corruption clause
      is discharged by injecting a **legal** value inside the clamp at the
      logical `SaveState` level and re-encoding, so the restore must accept it
      and then matter; a byte flip would only prove CRC32 works. Divergence is
      asserted positionally, not on the checksum, because learned state is
      hashed and a checksum difference is guaranteed either way.
- [x] **C11.4 No Lamarckian leakage.** Children of parents with large learned
      deltas start at exactly zero on every plastic edge. Asserted directly,
      not inferred.

      **Met, by construction.**
      `LearnState::push_organism` takes no initial-state parameter and zeroes
      every row, so the reset is an invariant rather than a default. Asserted
      directly with parents carrying large deltas, against
      `sum_abs_learned_q16 == 0` per organism rather than a population mean,
      which two cancelling organisms could also produce.
- [x] **C11.5 Numeric safety.** A 10^6-tick single-organism plasticity trace
      reproduces bit-identically across clean processes; `learned_q16` never
      leaves its clamp; effective weight never leaves [-8, 8]; injected
      non-finite activations are neutralized, counted, and evented with no
      panic.

      **Met.** The 10^6-tick single-organism trace
      reproduces bit-identically across two clean processes (fixture schema 5,
      config `0xae34cd2b6f7a3e13`, state `0x53b354bd94e82bcf`, 2,000,000
      updates, 0 faults). `learned_q16` stays inside its clamp and effective
      weight inside [-8, 8] under an adversarial sweep of 2,592,000 `step`
      calls and in a running population. Non-finite deltas are neutralized,
      counted and evented with no panic, exercised by injection.

      **The first cut of the trace was silently a control.** With the input
      bound to `energy_fraction`, which is constant in a zero-cost world, the
      network reached a fixed point and the learned value reached an
      equilibrium where decay cancels the delta: `mean_abs_learned_milli` read
      **964 at 10^4 ticks and 964 at 10^6**. It would have reproduced
      perfectly forever while measuring nothing. Rebound to a monotone input,
      it reads 100 and 171, and the verify script now **refuses the run if
      the two horizons agree**.

      One honest gap: a non-finite delta is unreachable through validated
      genes, so the fault path has no running-world coverage and is defended
      at the unit and record level only.
- [x] **C11.6 Cost accounting.** The energy ledger stays exact to the
      milli-unit with plasticity costs flowing through it over a 10^6-tick
      run.

      **Met at the stated horizon.** Two measurements,
      because they answer different questions. Exactness *at a moment*: on
      tick 1, against a matched control, the debit is exactly one edge per
      organism per tick - an approximate version would pass on a cost off by
      a factor, and a long run cannot make this comparison because the two
      populations diverge. Exactness *over duration*, which is what the
      criterion states: **10^6 ticks, 100 invariant checks, population 400,
      40,328 births, 18,971,594 plasticity updates, 18,970,462 milli-EU
      charged, ledger exact throughout**, with the plasticity debit inside
      `spent_milli` rather than beside it.

      The world for that run **mirrors C10.9's** (128x128, cell capacity
      240,000, physiology on). The first cut invented a thinner one and it
      went extinct with *and* without plasticity, so the debit was not the
      cause - the population guard is what said so, and without it a million
      ticks of an empty world would have reported as a pass (trap 1).
- [~] **C11.7 Snapshot and checkpoint budget.** Snapshot size and checkpoint
      stall are measured at both tiers with realistic evolved plasticity
      levels, against the Phase 9 record. If sparse learned-state storage
      does not hold the budget, the phase reports that and the plastic-edge
      cap is set from the measurement.

      **Snapshot half met; the checkpoint-stall half is not measured through
      the mechanism the plan names.** Sparse storage holds the budget
      comfortably. Framing is exact at **12 bytes per plastic edge + 8 per
      organism + 72 per section**:

      | tier | condition | bytes/organism | learn share |
      |---|---|---|---|
      | 500 | off | 1875 | 0 |
      | 500 | evolved | 1890 | 0.4% |
      | 500 | seeded (2.05 edges/organism) | 1906 | 1.7% |
      | 2000 | off | 1676 | 0 |
      | 2000 | evolved | 1685 | 0.4% |
      | 2000 | seeded (2.07 edges/organism) | 1707 | 1.9% |

      At the provisional `max_plastic_edges = 32` the worst case is **392
      bytes per organism, 21 percent of a tier-500 organism** - so the cap,
      not the representation, is what decides whether the budget holds at the
      ceiling, and that is the number a later revision should restate from.

      `learn` phase cost, which had no benchmark target at all: p50 3.3 to
      8.3 microseconds and p95 4.9 to 27.4 across 0, 1 and 2 plastic edges
      per organism at both tiers, 3 to 65 milli of whole-tick time. Note no
      prior benchmark in this repo computes a p95; the plan asks for one and
      a percentile over timing samples was added for it.

      **Not measured: checkpoint stall through `AsyncCheckpointer`.** What is
      reported is synchronous encode/decode/restore time on the tick thread.
      The plan calls asynchronous checkpointing a Phase 5 prerequisite for
      exactly this measurement, so this is a gap rather than a substitution.
- [x] **C11.8 Determinism and fixtures.** Plasticity-disabled configs
      reproduce the Phase 9 fixture exactly; storage-permutation equality
      holds.

      **Met.** A plasticity-disabled config
      reproduces the Phase 9 fixture (`0x5f0c4e95e4f5170f`) exactly, and the
      Phase 1 and Phase 2 fixtures are untouched. A flagged genome in a
      disabled world is completely inert. The permutation clause is
      discharged the way C9.7 established: rotating the learned rows changes
      the world, with an explicit assertion that the rows are not all
      identical, since a rotation of identical rows is the identity.

## The Conditions As Run, And What Bounds The Null

`experiments/phase11-c111-confirmatory.campaign`, campaign hash
`0x96ed767dbd9060e6`, pre-registered in commit `4b160fe` before the run.
120 worlds - 4 arms x 30 seeds, seeds 6001..6030 with no exclusions -
60,000 ticks each, 8 workers, 4,512.9 s wall, 0 failed, 1.8 GB of
artifacts. Analysis version `lifesim-plasticity-analysis-v1`, census policy
`lifesim-action-census-v1`, analysis seed `0x9e3779b97f4a7c15`, 199
permutations.

`E-variable` is realized by **building the machinery** rather than by
substituting a climate schedule, which is what
`docs/21-open-questions.md` recorded as the default: Phase 12's
mutable-world half supplies a relocating capacity patch that is a pure
function of `(seed, config, tick)`. Its control is a **zero-magnitude
schedule**, not a schedule-free world, because relocating a patch trims
biomass into a ledgered loss sink and a schedule-free arm would differ in
standing biomass for a reason unrelated to learning.

**Two calibrations move this campaign off the shipped defaults, and both
bound how far the null generalizes.** Each was ruled by a control rather
than by either criterion's outcome, and each was fixed on pilot seeds
9001..9004, disjoint from the confirmatory set.

- **`genome2.mutation.point_q16 = 65535`, ten times the shipped 6554.**
  The rule was that the neutral marker's alleles must actually move, or the
  drift control cannot bound drift and C11.2 is undefined everywhere for a
  reason about the mutation rate. At 6554, **0** marker alleles had left the
  founder value in the pilot at 20,000 ticks; at 65535, **126**.
  `point_delta_q16` stays pinned at its documented 3,277, because C11.2's
  bar is computed from it.
- **`worldmod.patch_radius_cells = 32` and
  `patch_capacity_scale_q16 = 262144` (4.0), against defaults of 15 and
  2.0.** The rule was that the schedule must measurably change the world, or
  the environmental factor is decoration. It does: median final population
  is 5,964 in Avar against 2,846 in Astat and 5,653 against 2,716 in the B
  arms, so the E-variable arms carry roughly twice the population of their
  zero-magnitude controls at the same seeds.

So **the null is a null at ten times the shipped point-mutation rate**, with
a patch of radius 32 against the default 15 - about 4.5 times the cell
footprint, 4,225 cells of a 16,384-cell map before the habitable filter - at
twice the default capacity scale, over 64 generations of ancestry. It says nothing about the shipped rates except
that they are further from assembling the phenotype, not closer: the
waiting-time arithmetic in the census scales inversely with `point_q16`, so
at 6554 the shortfall would be a factor of roughly 170 rather than 17. That
scaling holds the locus count fixed and is first-order only - a lower point
rate also means fewer duplications and so a slightly smaller denominator -
which moves the number in the direction that makes the shortfall smaller,
not larger, and not by an order of magnitude.

## What Is Not Claimed

The phase built the mechanism, measured its cost, safety and persistence,
and has now measured both behavioural criteria. It has **not** shown that
anything learns anything useful, that plasticity is selected for, or that
behaviour changes within a lifetime in a way selection did not produce.
C11.1 and C11.2 are unmet, and they are unmet as measured nulls with
pre-registered thresholds and stated controls.

Four things are specifically **not** established by that null.

- **Not** that lifetime learning is worthless in this world. The census
  measures whether the phenotype was *reachable*, and separately whether the
  states on the way to it were visible to selection. It cannot say what
  learning would have been worth had it existed.
- **Not** that plasticity was selected away. The flag is as common as its
  neutral twin. What the census shows is that the conjunction was assembled
  at most four independent times per arm in 30 worlds.
- **Not** a behavioural finding about relocation. C11.1's two-sided
  association reproduces in the arm where nothing happens at the event tick,
  and its directed statistic is confounded by an age offset whose sign
  decides the verdict (D-100).
- **Not** a general statement about this world's behavioural repertoire.
  The instrument could resolve one heading statistic (D-101).

## Test Plan

- Unit: each rule form's update at boundary activations; decay arithmetic;
  the f32-to-Q16 rounding rule at ties and at sign boundaries; clamp
  behavior.
- Property: learned state stays in bounds under adversarial coefficient and
  activation combinations.
- Determinism: the 10^6-tick trace; storage permutation; clean-process
  fixture.
- Integration: birth reset; save round trip with nonzero learned state;
  structural mutation interacting with learned state (a duplicated edge
  starts at zero, a deleted edge takes its state with it).
- Behavioral: the reversal probe as a scripted deterministic scenario with
  recorded per-individual action histograms.
- Disabled-section equality.

## Benchmark Impact

The `learn` phase is new and its cost scales with plastic edge count, not
organism count. Record: `learn` phase p50/p95 at both tiers across a range
of plastic-edge fractions; the additional per-organism snapshot bytes; the
checkpoint stall delta; allocation behavior in the learn path (must remain
zero per organism per tick).

Benchmark schema 7 (see Numbering Corrections above; 5 was never used).

## Documentation Updates

`docs/07-neural-network-design.md`, `docs/06-organism-model.md` (learned
state joins the state table), `docs/02-scope-and-non-goals.md` (the RL
non-goal is sharpened, not removed), `specifications/simulation-tick.md`,
`specifications/entity-component-model.md`,
`specifications/world-save-format.md`, `specifications/metrics-schema.md`,
decision log, ADR-0014.

Updated by the measurement substrate and the campaign, 2026-08-11:
`specifications/world-save-format.md` (snapshot section 14 and the `.alac`
artifact), `specifications/experiment-config-schema.md` (`output actions`,
the `action_samples` column, and why `genome2.mutation.point_delta_q16` is
settable), `specifications/metrics-schema.md` (the `action_samples` column
and both Phase 11 report formats), `specifications/determinism-extensions.md`
(Rule 8's section table gains `lifesim-action-census-v1`; Rule 9 gains
`lifesim-probe-v1` and the artifact framing versions),
`docs/21-open-questions.md` (the `E-variable` question is discharged; four
opened), `docs/22-decision-log.md` (D-098 to D-105), `planning/backlog.md`,
`FILE_MANIFEST.md`.

## Risks

| Risk | Mitigation |
|---|---|
| **Plasticity is selected to zero and the phase returns a null result.** The single most likely failure in this phase | The `E-variable` sweep is the mitigation and is mandatory. If plasticity is still selected to zero under environmental variability, that is a real and reportable finding about this world's structure, and it is a strong signal that Phase 13 will also return null |
| Learned state doubles snapshot size and breaks the checkpoint budget | Sparse storage (plastic edges only); C11.7 measures it; asynchronous checkpointing from Phase 5 is a prerequisite for exactly this reason |
| Runaway plasticity destabilizes controllers into noise | Hard clamps on learned delta and effective weight; decay term; energy cost; fault counting |
| Float-to-fixed conversion introduces a subtle asymmetry that biases learning | The rounding rule is specified exactly and unit-tested at ties and sign boundaries |
| The modulatory design is too indirect for evolution to find | Recorded as an honest concern. A partial mitigation is that rules 1 and 2 are ungated and can produce unsupervised change without any modulator, so the search has a gradient toward useful plasticity before it has to discover modulation |

### Risk Outcome, 2026-08-11

**Row 1 happened.** The phase returned a null result on both behavioural
criteria, which the table names as the single most likely failure. The
mitigation was applied in full - the `E-variable` sweep ran, it was crossed
with a matched zero-magnitude control, and the environmental factor
demonstrably bit ecologically - and the null survived it.

**The census changes the interpretation, and the change matters for the
revisit conditions on D-023, D-025 and D-035.** The row predicts "plasticity is
**selected to zero**", and that is not what happened. Plasticity was never
assembled. The phenotype needs four conditions on one edge locus, each
behind a different one of seven point-mutation targets; 9 of 684,370 Avar
edge alleles carry all four, from at most four independent assembly events.
Every incomplete state on the path computes bit-identically to the founder,
so selection has nothing to act on, and the flagged half of the path is
deleterious because the learn phase charges per flagged edge with no
reference to the rule. That is a **plateau with a moat**, not a gradient
running downhill.

Two consequences follow, and both are reachability statements rather than
statements about the value of learning.

- The revisit conditions on D-023, D-025 and D-035 - "Phase 11 shows
  plasticity is selected to zero everywhere", "plasticity selected to zero
  across the environmental-variability sweep" - are **not** triggered by
  this result, and reading them as triggered would draw the wrong
  conclusion about Phase 13.
  What Phase 11 shows is that a four-condition conjunction at one locus is
  out of reach in 64 generations at ten times the shipped mutation rate.
- The last row of the table - "the modulatory design is too indirect for
  evolution to find" - has its stated partial mitigation removed by
  measurement. Rules 1 and 2 being ungated does not give the search a
  gradient, because an edge carrying rule 1 with `eta = 0` and an edge
  carrying rule 0 execute the same instructions and write the same bytes.
  There is no gradient toward plasticity at all until all four conditions
  are present at once.

## Rollback

One config section. Disabled, `eta` is zero everywhere, the `learn` phase is
empty, no learned state is stored or checksummed, and the Phase 9 fixture
reproduces exactly. Plasticity genes remain inherited and inert, exactly as
they were between Phase 9 and Phase 11.
