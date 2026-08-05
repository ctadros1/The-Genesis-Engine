# ADR-0014: Lifetime Learning Model

Status: Proposed
Date: 2026-08-04
Author: Goal revision

## Context

Controller weights are fixed at birth, so behavior can change only across
generations. Cumulative culture is transmitted far faster than genes move.
Without within-lifetime plasticity, any "discovery" collapses into an
inherited trait and the transmission question cannot be posed.

The constraint that shapes the design is an existing non-goal:
`docs/02-scope-and-non-goals.md` puts "training neural networks with
reinforcement learning against a hand-authored reward" permanently out of
scope. That non-goal is correct and must survive, because an authored reward
is authored progress by another route: it names the outcome.

## Options Considered

- **No lifetime learning.** Keeps the non-goal trivially, and makes the
  project's central question unaskable.
- **Reinforcement learning against a fitness-derived reward.** Effective and
  directly prohibited. A reward signal derived from energy, survival, or
  reproduction is a hand-authored objective regardless of how it is
  packaged.
- **Fixed authored plasticity rule** applied uniformly (for example, plain
  Hebbian everywhere). Simple, and it authors what organisms learn by
  authoring the only rule available.
- **Genome-encoded plasticity with evolved neuromodulation.** The rule
  forms are a bounded authored registry; which edges are plastic, which rule
  each uses, its coefficients, and which nodes gate it are all genes.

## Proposed Decision

Adopt genome-encoded plasticity with evolved neuromodulation,
`lifesim-plasticity-v1`, specified in
`specifications/plasticity-and-learning.md`.

Authored: that an edge can be plastic; a small bounded registry of rule
forms; that a node may have the Modulatory role and that modulatory activity
gates plastic updates; the arithmetic, clamps, and energy cost.

Evolved: which edges are plastic, which rule form each uses, its
coefficients, which nodes are modulatory, and what drives them.

**There is no reward function.** No fitness signal is delivered to any
network. The signal that gates learning is an ordinary output of the
organism's own evolved network, so what counts as reinforcing is a matter of
evolutionary history rather than of our specification. This is what keeps
the RL non-goal intact while adding lifetime learning, and the non-goal text
is sharpened rather than relaxed.

Three supporting decisions:

**Fixed-point accumulation.** Learned weight state is Q16 `i32`, accumulated
with integer arithmetic and a specified rounding rule. Learned state
accumulates across 10^5 or more ticks, and float accumulation over that many
steps amplifies precisely the reassociation and contraction differences
ADR-0011 exists to exclude. Anything that integrates over a lifetime is
fixed point. The per-tick delta is computed in f32 and converted once, so
ADR-0011's existing guarantee covers the computation and integer arithmetic
covers the accumulation.

**No Lamarckian inheritance by default.** Learned state resets at birth.
This is an invariant, not an optimization: if learned weights were
inherited, a discovery would become a heritable trait and transmission would
be indistinguishable from inheritance, which would make Phase 13
unanswerable. A `lamarckian_fraction_q16` config field exists, defaults to
zero, and any nonzero value is an experimental condition that must be
reported.

**Plasticity costs energy.** Each plastic edge costs a configured increment
per tick through the existing ledger. Without a cost, everything becomes
plastic by drift and the trait carries no information. With a cost, the
amount of plasticity is itself under selection and "how much plasticity does
this environment pay for" becomes a measurable result.

## Consequences

Positive: within-lifetime change exists; learning rules are under selection;
the RL non-goal survives; determinism is stronger than a float design would
be.

Negative and accepted:

- **Plasticity may simply be selected to zero.** This is the most likely
  failure mode of Phase 11. Learning pays only when the environment varies
  within a lifetime and is predictable within it, so Phase 11 makes an
  environmental-variability sweep mandatory rather than optional. If
  plasticity is still selected down under variability, that is a real
  finding about this world and a strong predictor that Phase 13 will also
  return null.
- The modulatory design is indirect, and evolution may not find it. Partial
  mitigation: rule forms 1 and 2 are ungated and produce unsupervised change
  without any modulator, so there is a gradient toward useful plasticity
  before modulation has to be discovered.
- Learned state is world state that cannot be recomputed from the genome. It
  must be saved and checksummed, which grows snapshots. Sparse storage
  (plastic edges only) bounds the growth; Phase 11 measures it.

Compatibility: plasticity is one config section, inert when disabled.
`PlasticityGenes` are carried, inherited, and validated from Phase 9 onward
whether or not plasticity is enabled, following the precedent set by thermal
preference and defense tendency in Phase 2. Disabled configs reproduce the
Phase 9 fixture exactly.

## Performance Implications

A new `learn` tick phase whose cost scales with plastic edge count rather
than organism count. Snapshot growth proportional to evolved plasticity.
Both measured in Phase 11 against the Phase 9 record. No claim is made in
advance.

Asynchronous checkpointing (Phase 5) is a prerequisite specifically because
of the snapshot growth this decision causes.

## Operational Implications

Snapshot and checkpoint budget. Nothing else.

## Revisit Conditions

- Plasticity is selected to zero across the full environmental-variability
  sweep, suggesting the cost model or the rule registry is wrong.
- Sparse learned-state storage does not hold the checkpoint budget.
- Cross-platform replay becomes a requirement, at which point the f32
  delta computation would also need to move to fixed point; the accumulator
  already is.

## Evidence Required To Accept

- Phase 11 acceptance criteria, in particular C11.1 (per-individual
  within-lifetime change, not a population-level shift), C11.2 (learning
  under selection, measured against a neutral-marker drift control), and
  C11.5 (bit-identical 10^6-tick trace across clean processes).
- Snapshot size and checkpoint stall measured at both supported tiers.
- Ledger exactness with plasticity costs flowing through it.
- Compatibility and rollback impact: Phase 9 fixture reproduces exactly with
  plasticity disabled.
