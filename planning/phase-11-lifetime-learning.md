# Phase 11: Lifetime Learning

Status: in progress from 2026-08-10. Policy version `lifesim-plasticity-v2`.
Specification: `specifications/plasticity-and-learning.md`.

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

- [ ] **C11.1 Within-lifetime behavioral change. NOT MEASURED.** Deliberately
      distinguished from *unmet*: nothing was observed and no threshold was
      tested, so recording it unmet would claim a null nobody ran. Three
      pieces it needs do not exist - scripted-intervention machinery (there
      is no command queue, no intervention list, and no relocatable resource
      patch; resources are a per-cell scalar field and nothing moves),
      per-organism action counting (intents are transient `pub(crate)`
      scratch with no accessor), and the analysis that reduces a
      per-individual shift to a world-level rate. See
      `docs/21-open-questions.md`, which records the choice between building
      that machinery and substituting a climate schedule, and recommends
      building it.
- [ ] **C11.2 Learning is under selection. NOT MEASURED**, for the same
      reason, plus a fourth gap: there is **no neutral marker locus** anywhere
      in the genome, and the criterion's drift control is defined against one.

      Two measurements bear on it and are recorded now rather than saved for
      a campaign, because both are unfavourable and both were nearly missed.
      **Evolved plasticity is essentially absent**: after 30,000 ticks with
      the mutation gate open, 1 plastic edge across 500 organisms and 7
      across 1,999, with `mean_abs_learned_milli` zero in both. And over the
      10^6-tick ledger run the founders' 400 plastic edges reached **zero**
      by the end while 18.9 million updates accumulated along the way - so
      plasticity was active and then disappeared. That is the phase's named
      most-likely failure appearing, and it is now legible for a real reason
      rather than guaranteed by an unreachable flag.
- [x] **C11.3 Learned state is world state. Met.** Save, restore and continue
      is bit-identical with plastic edges carrying nonzero learned deltas, and
      the record is compared field by field rather than only by checksum - a
      checksum match also holds for a pair of cancelling defects (zeroing on
      restore *and* dropping the section from the hash). The corruption clause
      is discharged by injecting a **legal** value inside the clamp at the
      logical `SaveState` level and re-encoding, so the restore must accept it
      and then matter; a byte flip would only prove CRC32 works. Divergence is
      asserted positionally, not on the checksum, because learned state is
      hashed and a checksum difference is guaranteed either way.
- [x] **C11.4 No Lamarckian leakage. Met, by construction.**
      `LearnState::push_organism` takes no initial-state parameter and zeroes
      every row, so the reset is an invariant rather than a default. Asserted
      directly with parents carrying large deltas, against
      `sum_abs_learned_q16 == 0` per organism rather than a population mean,
      which two cancelling organisms could also produce.
- [x] **C11.5 Numeric safety. Met.** The 10^6-tick single-organism trace
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
- [x] **C11.6 Cost accounting. Met at the stated horizon.** Two measurements,
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
- [~] **C11.7 Snapshot and checkpoint budget. Snapshot half met; the
      checkpoint-stall half is not measured through the mechanism the plan
      names.** Sparse storage holds the budget comfortably. Framing is exact
      at **12 bytes per plastic edge + 8 per organism + 72 per section**:

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
- [x] **C11.8 Determinism and fixtures. Met.** A plasticity-disabled config
      reproduces the Phase 9 fixture (`0x5f0c4e95e4f5170f`) exactly, and the
      Phase 1 and Phase 2 fixtures are untouched. A flagged genome in a
      disabled world is completely inert. The permutation clause is
      discharged the way C9.7 established: rotating the learned rows changes
      the world, with an explicit assertion that the rows are not all
      identical, since a rotation of identical rows is the identity.

## What Is Not Claimed

The phase built the mechanism and measured its cost, safety and persistence.
It has **not** shown that anything learns anything useful, that plasticity is
selected for, or that behaviour changes within a lifetime in a way selection
did not produce - those are C11.1 and C11.2 and they are not measured.

The two numbers that exist point the other way: plasticity is barely present
after 30,000 ticks of mutation, and the founders' plastic edges are gone
after 10^6. Neither is a result yet, because neither had a control or a
pre-registered threshold. They are the reason the campaign is worth running
rather than a substitute for running it.

## Test Plan## Test Plan

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

## Risks

| Risk | Mitigation |
|---|---|
| **Plasticity is selected to zero and the phase returns a null result.** The single most likely failure in this phase | The `E-variable` sweep is the mitigation and is mandatory. If plasticity is still selected to zero under environmental variability, that is a real and reportable finding about this world's structure, and it is a strong signal that Phase 13 will also return null |
| Learned state doubles snapshot size and breaks the checkpoint budget | Sparse storage (plastic edges only); C11.7 measures it; asynchronous checkpointing from Phase 5 is a prerequisite for exactly this reason |
| Runaway plasticity destabilizes controllers into noise | Hard clamps on learned delta and effective weight; decay term; energy cost; fault counting |
| Float-to-fixed conversion introduces a subtle asymmetry that biases learning | The rounding rule is specified exactly and unit-tested at ties and sign boundaries |
| The modulatory design is too indirect for evolution to find | Recorded as an honest concern. A partial mitigation is that rules 1 and 2 are ungated and can produce unsupervised change without any modulator, so the search has a gradient toward useful plasticity before it has to discover modulation |

## Rollback

One config section. Disabled, `eta` is zero everywhere, the `learn` phase is
empty, no learned state is stored or checksummed, and the Phase 9 fixture
reproduces exactly. Plasticity genes remain inherited and inert, exactly as
they were between Phase 9 and Phase 11.
