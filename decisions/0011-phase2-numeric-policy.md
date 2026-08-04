# ADR-0011: Phase 2 Genome And Controller Numeric Policy

Status: Proposed
Date: 2026-08-04
Author: Phase 2 implementation

## Context

Phase 2 introduces f32 genome values and controller evaluation into a
kernel whose Phase 1 state is all-integer fixed point. The policy must keep
strict same-build replay, reject hostile input, and version every numeric
choice so experiments stay interpretable.

## Options Considered

- f32 values with libm tanh (platform-dependent transcendentals).
- f32 values with a rational activation approximation and no libm calls.
- Full fixed-point controller arithmetic.

## Proposed Decision

Propose bounded f32 genome/controller values evaluated with f32
add/multiply/divide and the rational activation `x*(27+x^2)/(27+9x^2)` on a
clamped domain (`lifesim-controller-v1`), while world positions, energy,
and biomass stay fixed-point integers. Variation uses the bounded uniform
distribution `uniform-bounded-v1` (probability 0.02, trait sigma 0.05,
neural sigma 0.4, clamped). Trait-to-attribute maps are linear with
deterministic rounding. Movement trigonometry uses integer Bhaskara
sine/cosine over binary angular measure. All constants live in the hashed
config/policy versions.

## Consequences

Same-build, same-platform replay is exact and clean-process verified. The
design avoids known cross-platform variance sources (no libm, no FMA
contraction in Rust defaults), but cross-platform byte equality is NOT
claimed without cross-platform evidence. Full fixed-point evaluation
remains the fallback if float evidence ever disagrees.

## Performance Implications

Scalar stack-buffer evaluation, no per-organism heap allocation; measured
in the Phase 2 benchmark record (controllers phase).

## Operational Implications

None beyond kernel testing; no new dependencies.

## Revisit Conditions

Cross-platform replay becomes a requirement; Phase 5 SIMD/batching work
changes evaluation order; or long-run evidence shows the variation
distribution or attribute ranges produce degenerate ecology.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 2 Local Evidence

Controller fixtures (zero, directional, saturated, extreme-weight,
non-finite), recombination bound/saturation tests, the malformed-input
harness, clean-process Phase 2 fixture replay, and multi-generation
long-run results all pass locally; see `research/performance-notes.md` for
the benchmark record. Status remains Proposed.
