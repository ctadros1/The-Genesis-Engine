# Neural Network Schema

## Phase 2 Implementation Notes

Topology 1 is implemented as `lifesim-controller-v1` (see
`docs/07-neural-network-design.md` for the full policy record). Deviations
and clarifications versioned there: the activation is a rational tanh
approximation evaluated with f32 add/multiply/divide only; pre-activation
sums clamp at +/-8; the per-stage output clamp to [-1, 1] applies via the
activation's own bound. Batch evaluation is scalar per organism with stack
buffers (no per-tick heap allocation); SoA batching remains a Phase 5
optimization option. Cross-version decode is fail-closed (unknown topology
IDs are rejected).

## Planned Successor: Variable Topology (Phase 9)

Full design in `specifications/genome-schema-2.md`; decision in ADR-0013.
Topology 1 stays registered, evaluable, and fixture-covered forever.

Changes that matter for this specification:

- **Canonical layout.** Layer-major weight matrices are replaced by sorted
  typed locus lists. Nodes and edges carry the four identity fields of
  ADR-0022 A8, derived by domain-separated hash rather than allocated from a
  counter, and loci within a chromosome are strictly ascending by
  `homology_id` as a decode-time invariant.
- **Evaluation.** Hybrid, per ADR-0022 A9. Edges are typed zero-delay or
  delayed. Zero-delay edges are evaluated in a **canonical topological
  order** over the acyclic subgraph, so information can cross several edges
  within one tick. Delayed and recurrent edges read from **prior-state
  buffers**, which breaks every cycle by construction and needs no cycle
  special case.

  An earlier draft made every edge delayed (fully synchronous update). That
  was chosen to avoid topological sorting entirely, at the cost of one edge
  of propagation per tick, which makes a deep network unable to respond
  within a tick. The hybrid is strictly more capable and remains
  deterministic, because the topological order is canonicalized by
  `homology_id` and ties are broken by it.

  A cycle among zero-delay edges is a **decode-time error**, not a runtime
  condition: the genome is rejected, counted, and evented, exactly as a cap
  violation is. Activations and prior-state buffers are both world state,
  saved and checksummed under `lifesim-activation-state-v1`.
- **Summation order is policy.** Per-node incoming edges are summed in
  ascending edge `homology_id` order, never in storage order. Float addition
  is not associative, so a storage-order sum is a replay bug that stays
  invisible until a compaction changes layout.
- **Channels.** The fixed 20 inputs and 12 outputs become a versioned
  registry. An organism binds any subset through `IoBinding` loci; unbound
  channels are never gathered or requested and cost nothing. A binding to an
  unknown channel ID fails decode.
- **Bounds.** `max_nodes`, `max_edges`, `max_edges_per_node`, and
  `max_genome_bytes` cap allocation and per-tick work. Every cap rejects
  deterministically, counts, and events.
- **Batching.** Grouping organisms by topology ID no longer works, because
  topologies are per-organism. The replacement is a measurement question for
  a later performance slice.
- **Plasticity.** From Phase 11, edges may be plastic and their effective
  weight includes a Q16 learned delta. See
  `specifications/plasticity-and-learning.md`.

Unchanged: bounded finite weights in [-8, 8], activation clamping, the
rational activation approximation with no libm, non-finite neutralization
with bounded diagnostics, fail-closed cross-version decode, and the custom
evaluator with no ML framework and no GPU.

## Initial Topology

Topology ID 1: 20 normalized inputs, 16 tanh hidden units, 12 tanh hidden units, 12 bounded outputs, 4 memory values. Weights/biases are f32 in [-8, 8]; activations/memory/output are clamped to [-1, 1] after each configured stage.

## Canonical Layout

Store layer-major arrays: weight[output_index][input_index], then bias[output_index]. The encoded topology specifies exact counts; implicit matrix shapes are forbidden. Batch layout groups organisms by topology ID and maintains contiguous input/output buffers.

## Evaluation

For each layer: z_j = bias_j + sum_i(weight_ji * x_i); a_j = tanh(clamp(z_j, -activation_limit, activation_limit)). Before use, every input is finite/clamped. If a non-finite value occurs, replace with zero, emit a bounded diagnostic, and apply quarantine/rejection policy at lifecycle boundary.

## Output Mapping

Outputs map to intents after finite clamping; action thresholds and scaling are config-versioned. Memory outputs become next-tick memory only after all controller evaluation completes. Controllers cannot mutate world state directly.

## Fixtures

Fixtures cover zero, known directional, saturated, max/min weight, malformed length, invalid topology, NaN/infinity encoded values, and cross-version decode. Record expected tolerance policy.
