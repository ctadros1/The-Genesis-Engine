# Neural Network Design

## Phase 2 Implementation Status (`lifesim-controller-v1`)

`sim-core` implements topology ID 1 exactly as proposed below: 20 inputs,
16 + 12 tanh hidden units, 12 outputs, 4 memory values, layer-major f32
weights/biases in [-8, 8]. Policy specifics recorded for replay:

- The activation is the rational approximation `x*(27+x^2)/(27+9x^2)` on
  the clamped domain [-3, 3], clamped to [-1, 1]. Evaluation uses only f32
  add/multiply/divide (no libm), so results are reproducible bit-for-bit on
  the recorded build/platform; cross-platform equality is not claimed.
- Inputs 1-9 and 13-15 plus memory are live; threat (10), temperature
  comfort (11), moisture comfort (12), and recent damage (16) are documented
  neutral zeros until their mechanics exist. Health (2) is neutral 1.0.
- Output mapping (config-thresholded, Q16-versioned): turn combines the
  turn channel with `(follow - avoid) * approach_tendency * relative
  heading`; throttle maps [-1, 1] onto [0, 1] baseline mobility with the
  rest channel as opt-out above its threshold; eat and mate are threshold
  gates; attack (4) is a documented no-op in Phase 2; channels 9-12 become
  next-tick memory, committed phase-separated after all evaluations.
- Non-finite inputs or sums are neutralized to zero, counted, and surfaced
  as `ControllerFault` events; with validated genomes the count stays zero.
- Evaluation allocates nothing per organism per tick (stack buffers only).

## Planned Successor: `lifesim-controller-v2` (Phase 8)

Topology 1 is frozen at 20 inputs, 16 and 12 hidden units, 12 outputs, and
4 memory values, so every new capability is a new channel and a schema bump
performed by a human. Open-ended complexity cannot come from a structure
only we can change. Phase 8 replaces it; the design is
`specifications/genome-schema-2.md` and the decision is ADR-0013.

What changes:

- Topology becomes variable. Nodes and edges are genome loci and change by
  gene duplication, deletion, insertion, and transposition.
- Hard-coded channel counts are replaced by a **versioned input/output
  channel registry**. An organism binds any subset of channels; unbound
  channels are never gathered or requested. Adding a world capability
  becomes a registry entry, not a genome schema bump.
- The four documented neutral placeholders below stop being placeholders.
  Health (2), threat (10), and recent damage (16) become live in Phase 7,
  and attack (4) stops being a no-op in the same phase. **Phase 7 needs no
  schema change precisely because topology 1 reserved these channels**,
  which is a large part of why it is scheduled ahead of the genome
  successor.
- Evaluation becomes synchronous: every node computes from the previous
  tick's activations. This removes topological sorting and cycle handling,
  makes node evaluation order irrelevant, and makes activations world state
  that is checksummed and saved. The 4-value memory vector becomes a special
  case of recurrent nodes rather than a separate concept.
- Per-node incoming edges are summed in ascending edge innovation-ID order.
  Float addition is not associative, so this is a policy requirement, not an
  implementation detail: a storage-order sum is a latent replay bug that
  only appears after a compaction changes layout.
- Plasticity (Phase 10) makes weights change within a lifetime under a
  genome-encoded rule. See `specifications/plasticity-and-learning.md`.

What does not change: the custom evaluator with no ML framework and no GPU
(ADR-0004), bounded finite values, non-finite neutralization with fault
events, and same-build determinism under ADR-0011.

Topology 1 evaluation, fixtures, and tests stay in the build permanently.
There is no schema 1 to schema 2 genome migration.

## Recommendation (Phase 2, Historical)

Implement a custom compact feed-forward f32 network with a fixed topology and a bounded internal memory vector. This keeps each controller genome serializable, inspectable, and cheap enough for batch evaluation. Do not begin with ONNX Runtime, Candle, Burn, LibTorch, or CUDA.

The "no evolving topology" part of this recommendation was correct for Phase 2 and is reversed for Phase 8 by ADR-0013, with the costs it identified
(batching, explainability, migration) accepted and measured rather than
dismissed.

## Proposed Initial Schema

    inputs: 20
    hidden layer 1: 16 tanh units
    hidden layer 2: 12 tanh units
    outputs: 12 bounded action channels
    memory: 4 values in [-1, 1]

The exact count is tunable per genome/schema version. Any topology change is a new genome version and requires migration/rejection behavior.

## Inputs

1. Energy fraction
2. Health fraction
3. Age fraction
4. Food gradient x
5. Food gradient y
6. Water/terrain suitability
7. Nearest organism proximity
8. Nearest organism relative heading
9. Local crowding
10. Local threat estimate
11. Local temperature comfort
12. Local moisture comfort
13. Current speed fraction
14. Current turn rate
15. Reproductive readiness
16. Recent damage fraction
17-20. Four memory values

Inputs are normalized, finite, and documented by schema version. Missing data uses explicit neutral values, never NaN.

## Outputs

1. Turn signal
2. Throttle signal
3. Eat request
4. Attack request
5. Rest request
6. Mate/reproduce request
7. Follow/approach bias
8. Avoid/flee bias
9-12. Next memory values

The action resolver converts output signals to finite intents and validates feasibility. A high output is a request, not a privileged command.

## Numerical Safety

- Clamp weights, biases, inputs, activations, memory, and outputs to versioned finite ranges.
- Replace non-finite intermediate values with a neutral value and emit a metric/event; repeated invalid genomes are quarantined from reproduction or rejected on load.
- Use stable activation implementations and test extremes.
- Bound all matrix lengths from schema metadata before allocation/deserialization.
- Never dynamically allocate per organism per tick.

## Genetics

Neural weights and biases are genome segments. Reproduction combines parent genes per specifications/organism-genome.md, then applies bounded mutation. Prototype uses cloning with mutation to validate infrastructure; Phase 2 adds sexual crossover.

Structural mutation was deferred through Phase 2 because it complicates
batching, migration, species distance, and explainability. Those costs are
real and are accepted, measured, and mitigated in Phase 8 rather than
denied. See ADR-0013 and `specifications/genome-schema-2.md`.

## Batch Strategy

Store topology-equal organisms in contiguous batches. Gather normalized inputs into SoA buffers, evaluate dense layers, scatter bounded outputs, and retain no per-tick activation history except selected debug samples. CPU scalar correctness precedes SIMD. GPU is viable only if measured compute savings exceed packing, transfer, synchronization, and operational complexity.

This strategy assumes shared topologies and does not survive Phase 8, where
topologies are per-organism. The replacement is a measurement question for a
later performance slice, not a Phase 8 deliverable. If the current
zero-per-organism-per-tick allocation property is lost, that loss is
recorded in the benchmark record rather than absorbed silently.

## Test Fixtures

Maintain fixed fixtures for all-zero, saturated, random-valid, malformed, non-finite, extreme-weight, and known-action genomes. Expected outputs must include tolerance policy and schema version.
