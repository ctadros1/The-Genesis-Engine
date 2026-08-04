# Phase 10: Lifetime Learning

Status: planned, not started. Policy version `lifesim-plasticity-v1`.
Specification: `specifications/plasticity-and-learning.md`.

## Problem

Controller weights are fixed at birth, so behavior can only change across
generations. Technology is cumulative culture, transmitted far faster than
genes can move. Without within-lifetime plasticity, any "discovery" collapses
into just another inherited trait, and Phase 11's transmission question
cannot even be posed: there would be nothing an organism could acquire that
it was not born with.

## Scope

- Per-edge plasticity genes, already carried inert since Phase 8, become
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
- No observational learning. Rule form 5 requires the Phase 11 social channel
  and is not in the registry yet.
- No backpropagation, no error signal, no supervised target.
- No claim that any observed change constitutes cognition.

## Prerequisites

- Phase 8 (schema 2 carries the plasticity genes; variable topology makes
  modulatory nodes expressible).
- Phase 5's asynchronous checkpointing, because learned state increases
  snapshot size.

## Determinism Notes

- New stream: `PlasticityInit` (10), reserved and unused under the default
  zero-initialization policy.
- Learned state is Q16 `i32`, accumulated with integer arithmetic and a
  specified rounding rule, per Rule 7. Float accumulation over 10^5 or more
  ticks is exactly the fragility ADR-0011 exists to avoid.
- Plastic edges are updated in ascending edge innovation-ID order.
- The `learn` phase reads only values committed earlier in the same tick and
  writes only learned state.
- Checksum section `lifesim-learn-state-v1`, present only when enabled.

## Acceptance Criteria

Conditions, matched on seeds (12), config, and run length:

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

- [ ] **C10.1 Within-lifetime behavioral change.** In a controlled reversal
      probe (a resource patch is relocated at tick t), individual organisms
      alive both before and after the relocation show a measurable shift in
      their own action distribution within their own lifetime under A, and
      do not under B. Measured **per individual**, not per population; a
      population-level shift is explicable by selection and proves nothing.
      Required in at least 8 of 12 seeds under `E-variable`.
- [ ] **C10.2 Learning is under selection.** The distribution of `eta` and
      plastic-edge-fraction at tick T differs from the founder distribution
      by more than the drift expectation, measured against a neutral marker
      locus in the same run as the drift control, in at least 8 of 12 seeds
      under `E-variable`. Under `E-stationary` the prediction is the
      opposite: plasticity should be selected *down* because it costs and
      does not pay. A result showing plasticity increasing in a stationary
      world would indicate the cost model is wrong, and would be reported as
      such.
- [ ] **C10.3 Learned state is world state.** Save, restore, and continue is
      bit-identical with plastic edges carrying nonzero learned deltas.
      Learned state cannot be recomputed from the genome and its presence in
      the snapshot is verified by a test that corrupts it and observes a
      trajectory divergence.
- [ ] **C10.4 No Lamarckian leakage.** Children of parents with large learned
      deltas start at exactly zero on every plastic edge. Asserted directly,
      not inferred.
- [ ] **C10.5 Numeric safety.** A 10^6-tick single-organism plasticity trace
      reproduces bit-identically across clean processes; `learned_q16` never
      leaves its clamp; effective weight never leaves [-8, 8]; injected
      non-finite activations are neutralized, counted, and evented with no
      panic.
- [ ] **C10.6 Cost accounting.** The energy ledger stays exact to the
      milli-unit with plasticity costs flowing through it over a 10^6-tick
      run.
- [ ] **C10.7 Snapshot and checkpoint budget.** Snapshot size and checkpoint
      stall are measured at both tiers with realistic evolved plasticity
      levels, against the Phase 8 record. If sparse learned-state storage
      does not hold the budget, the phase reports that and the plastic-edge
      cap is set from the measurement.
- [ ] **C10.8 Determinism and fixtures.** Plasticity-disabled configs
      reproduce the Phase 8 fixture exactly; storage-permutation equality
      holds.

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

Benchmark schema 5.

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
| **Plasticity is selected to zero and the phase returns a null result.** The single most likely failure in this phase | The `E-variable` sweep is the mitigation and is mandatory. If plasticity is still selected to zero under environmental variability, that is a real and reportable finding about this world's structure, and it is a strong signal that Phase 11 will also return null |
| Learned state doubles snapshot size and breaks the checkpoint budget | Sparse storage (plastic edges only); C10.7 measures it; asynchronous checkpointing from Phase 5 is a prerequisite for exactly this reason |
| Runaway plasticity destabilizes controllers into noise | Hard clamps on learned delta and effective weight; decay term; energy cost; fault counting |
| Float-to-fixed conversion introduces a subtle asymmetry that biases learning | The rounding rule is specified exactly and unit-tested at ties and sign boundaries |
| The modulatory design is too indirect for evolution to find | Recorded as an honest concern. A partial mitigation is that rules 1 and 2 are ungated and can produce unsupervised change without any modulator, so the search has a gradient toward useful plasticity before it has to discover modulation |

## Rollback

One config section. Disabled, `eta` is zero everywhere, the `learn` phase is
empty, no learned state is stored or checksummed, and the Phase 8 fixture
reproduces exactly. Plasticity genes remain inherited and inert, exactly as
they were between Phase 8 and Phase 10.
