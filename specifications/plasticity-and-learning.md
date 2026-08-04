# Plasticity And Learning Specification

Status: design specification, not implemented. Phase 10. Policy version
`lifesim-plasticity-v1`. Depends on genome schema 2
(`specifications/genome-schema-2.md`).

## Problem

Controller weights are fixed at birth. Any behavioral improvement must
therefore wait for a mutation and a generation. Cumulative culture requires
change that propagates faster than genes, which requires change within a
lifetime. Without it, every "discovery" is just another inherited trait and
Phase 11's transmission question is unaskable.

## What Is Authored And What Is Evolved

Authored (physics):

- That an edge can be marked plastic.
- A small bounded registry of *rule forms* that a plastic edge may use.
- That a node may have the Modulatory role, and that modulatory activity
  gates plastic updates.
- The arithmetic, the clamps, and the cost.

Evolved (never authored):

- Which edges are plastic.
- Which rule form each plastic edge uses.
- The coefficients of that rule.
- Which nodes are modulatory, what drives them, and therefore **what the
  organism treats as reinforcing**.

That last point is the whole design. There is no reward function. There is
no fitness signal delivered to the network. The signal that gates learning
is an ordinary output of the organism's own evolved network, and what makes
it fire is a matter of evolutionary history, not of our specification. This
is what keeps `docs/02-scope-and-non-goals.md`'s prohibition on
"reinforcement learning against a hand-authored reward" intact while adding
lifetime learning. See ADR-0014.

## Genome Fields

Every `Edge` locus carries `PlasticityGenes`, present whether or not the
edge is plastic:

| Field | Type | Range | Meaning |
|---|---|---|---|
| `rule_id` | u8 | versioned rule registry | Which rule form |
| `eta` | f32 | [0, eta_max] | Learning rate |
| `a`, `b`, `c`, `d` | f32 | [-1, 1] each | Rule coefficients |
| `decay` | f32 | [0, decay_max] | Pull of the learned delta back toward zero per tick |
| `modulator_node` | u32 | node innovation ID or 0 | Which modulatory node gates this edge; 0 means ungated |

All are ordinary genes: inherited, dominance-expressed, point-mutable, and
under selection.

## Rule Registry

Bounded and versioned. Registry version enters the config hash. Rules are
*forms*; the coefficients that specialize them are genes.

| `rule_id` | Form | delta_w per tick, before scaling |
|---:|---|---|
| 0 | Static | 0 (edge is not plastic even if flagged) |
| 1 | Generalized Hebbian | `a*x*y + b*x + c*y + d` |
| 2 | Oja-normalized | `y*(x - y*w_eff)` |
| 3 | Modulated Hebbian | rule 1, multiplied by the modulator activation |
| 4 | Eligibility trace | rule 1 accumulated into a per-edge trace with decay, applied when the modulator fires |
| 5 | Observational | rule 1 with `x` replaced by the perceived-action input of the selected neighbour |

`x` is the presynaptic activation, `y` the postsynaptic activation, `w_eff`
the current effective weight.

Rules 0 through 4 are available from Phase 10. **Rule 5 requires the Phase 11
social channel and is not available before it.** Its inclusion is the
largest philosophical judgment call in the plan and is argued explicitly in
`planning/phase-11-social-channel.md`: it authors the *capacity* for a
synapse to be driven by an observed conspecific action, not the content of
what is learned. The alternative, requiring imitation to be discovered from
generic plasticity plus perception alone, is more philosophically pure and
substantially more likely to make Phase 11 unfalsifiable. Phase 11's design
runs both as conditions rather than picking one by assertion.

## Update Arithmetic

Learned state is world state that cannot be recomputed from the genome, so
it is saved, checksummed, and, per
`specifications/determinism-extensions.md` Rule 7, **fixed point**.

Per plastic edge:

    learned_q16: i32          // Q16, range +/- (8.0 << 16)
    trace_q16:   i32          // Q16, rules 4 and 5 only

Effective weight used in evaluation:

    w_eff = clamp(genome_weight + (learned_q16 as f32) / 65536.0, -8.0, 8.0)

Per tick, for each plastic edge, in ascending edge innovation-ID order:

1. Compute `delta` in f32 from the rule form, activations, and coefficients.
2. Scale: `delta *= eta`. If the rule is modulated, multiply by the
   modulator node's activation (clamped to [-1, 1]).
3. Convert to Q16 with round-half-away-from-zero:
   `delta_q16 = trunc(delta * 65536.0 + copysign(0.5, delta))`.
4. Apply decay: `learned_q16 -= (learned_q16 * decay_q16) >> 16`, integer
   arithmetic, truncating toward zero.
5. Accumulate and clamp: `learned_q16 = clamp(learned_q16 + delta_q16,
   -LEARN_LIMIT_Q16, LEARN_LIMIT_Q16)`.

Steps 1 through 3 are f32 and therefore same-build deterministic under
ADR-0011. Step 4 and 5 are integer, so nothing accumulates float error
across a lifetime. A non-finite `delta` is neutralized to zero, counted, and
evented as a plasticity fault, following the existing controller-fault
policy exactly.

The clamp in step 5 is what bounds the learned contribution and keeps
`w_eff` inside the range every downstream bound already assumes.

## Cost

Plasticity is not free. Each plastic edge costs a configured energy
increment per tick, and the per-organism total is deducted through the
existing energy ledger as an action cost. Without a cost, everything becomes
plastic by drift and the trait is uninformative. With a cost, the number of
plastic edges is itself under selection, and "how much plasticity does this
environment pay for" becomes a measurable result.

## Reset At Birth: No Lamarckian Inheritance

`learned_q16` and `trace_q16` are zero at birth. This is an invariant, not a
default, and it is the property that keeps Phase 11's question meaningful: if
learned state were inherited, a discovery would become a heritable trait and
transmission would be indistinguishable from inheritance.

A `lamarckian_fraction_q16` config field exists and defaults to zero.
Nonzero values are an explicit experimental condition that must be reported
in every result derived from such a run. It is never a default and never
silently enabled.

The `PlasticityInit` RNG stream is reserved for a future nonzero
initialization policy so that adopting one does not renumber streams.

## State, Checksum, And Save

Per organism, the learned state is a list of `(edge_innovation_id,
learned_q16, trace_q16)` for plastic edges only, sorted by innovation ID.
Non-plastic edges store nothing.

This sparsity matters. The Phase 4 benchmark records that snapshot size is
already dominated by per-organism genome parameter arrays at roughly 2.8 KB
each, and the server's checkpoint is synchronous on the tick thread. Storing
a dense learned copy of every weight would roughly double snapshot size and
put the checkpoint stall at risk of exceeding the tick interval at the upper
supported tier. Storing only plastic edges keeps the cost proportional to
the plasticity that actually evolved.

Two obligations follow, and both are Phase 10 acceptance criteria:

- Snapshot size and checkpoint stall are measured at both supported tiers
  with plasticity active, against the Phase 4 record.
- Asynchronous or double-buffered checkpointing (the deferred item behind
  D-019) is a Phase 5 prerequisite, not a Phase 10 discovery.

Checksum section tag: `lifesim-learn-state-v1`, appended only when the
plasticity section is enabled, so all earlier fixtures are unaffected.

## Tick Integration

Learning adds one phase. The tick order becomes:

    commands, environment, spatial_index, sense, controllers, apply,
    learn, lifecycle, finalize

`learn` runs after `apply` and reads only values already committed for the
tick: presynaptic and postsynaptic activations from the synchronous network
update, and modulator activations from the same update. It writes only into
learned state. No organism's learning reads another organism's current-tick
state (`specifications/determinism-extensions.md` Rule 4).

The phase is empty when the plasticity section is disabled, preserving
per-phase timing comparability at the cost of a benchmark schema increment.

## Interaction With Variable Topology

A duplicated edge inherits its source's `PlasticityGenes` but starts with
zero learned state, because learned state is per-organism and reset at
birth. Structural mutation therefore never carries learned content, which is
consistent with the no-Lamarckian-inheritance invariant.

An edge that is deleted takes its learned state with it. An organism whose
genome changes structurally between generations has no learned state to
reconcile, because it starts empty.

## Test Requirements

- Fixed-point exactness: a 10^6-tick plasticity trace on one organism
  reproduces bit-identically across clean processes.
- Bounds: `learned_q16` never leaves its clamp; `w_eff` never leaves
  [-8, 8]; no non-finite value reaches the checksum.
- Fault handling: injected non-finite activations produce neutralization,
  a counted fault, and a bounded event; no panic.
- Save round trip: save, restore, continue is bit-identical with plastic
  edges carrying nonzero learned state.
- Birth reset: a child of two parents with large learned deltas starts at
  zero on every plastic edge.
- Disabled-section equality: plasticity disabled reproduces the Phase 8
  fixture exactly.
- Cost accounting: the energy ledger stays exact to the milli-unit with
  plasticity costs flowing through it.
- Order independence: permuting stored organism order leaves checksums
  unchanged over N ticks.
