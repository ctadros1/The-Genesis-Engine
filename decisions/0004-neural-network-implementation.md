# ADR-0004: Neural Network Implementation

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
Controllers are small inherited networks evaluated for many organisms and must be serializable/inspectable.

## Options Considered
- Custom fixed matrix evaluator.
- ONNX Runtime.
- Candle/Burn/LibTorch.
- CUDA kernels.

## Proposed Decision
Propose custom compact CPU evaluator with bounded memory vector; defer SIMD/GPU.

## Consequences
Keeps topology/schema/control local and avoids ML runtime operational complexity.

## Performance Implications
Measure end-to-end neural share before framework/GPU adoption.

## Operational Implications
Avoids guest GPU drivers and external model artifacts initially.

## Revisit Conditions
Controller inference dominates measured ticks and another path wins end-to-end.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 2 Local Evidence

`sim-core` implements the custom compact CPU evaluator exactly as proposed
(topology 1, bounded memory vector, no ML framework, no GPU): scalar f32
evaluation with stack buffers, zero per-organism-per-tick heap allocation,
non-finite neutralization with fault events, and a rational activation
approximation so no libm transcendental is involved (ADR-0011). Fixture,
saturation, malformed-input, and determinism tests pass; the Phase 2
benchmark records the controller share of tick time at 500 and 2,000
entities (see `research/performance-notes.md`). SIMD/GPU remain deferred
pending profiling evidence. Status remains Proposed.
