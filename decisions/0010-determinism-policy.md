# ADR-0010: Determinism Policy

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
Users require fixed seeds and replayable runs while the system may evolve rules and optimize concurrency.

## Options Considered
- Best-effort random runs.
- Strict deterministic mode plus optional relaxed mode.
- Byte-perfect cross-platform guarantee.

## Proposed Decision
Propose strict deterministic mode with named RNG streams, fixed tick/order, versioned config/build metadata, and declared per-version replay guarantee.

## Consequences
Adds discipline and test cost but makes experiments interpretable. Cross-platform byte equality is not promised without evidence.

## Performance Implications
Parallelism must prove deterministic ordering/reduction or be disabled in strict mode.

## Operational Implications
Save/event metadata must retain provenance; rule changes branch compatibility.

## Revisit Conditions
Evidence requires a stronger/weaker guarantee or supported platforms expand.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 0 Local Evidence

The fixed-point spike uses stable-ID ordering and named draws keyed by seed,
tick, system, entity, and draw index. Two clean processes produced identical
500-organism/500-tick output with state checksum `0xf263056bcafcfdc0` and
snapshot CRC32 `0xfdf3f46f`; insertion-order and unrelated-draw tests also
passed. Cross-platform, parallel, intervention-log, and version-transition
policies remain untested. Status remains Proposed.

## Phase 1 Local Evidence

The production-oriented `sim-core` kernel carries the policy forward with
all-integer fixed-point state, named streams (`lifesim-rng-v1`) keyed by
seed/tick/system/subject/draw, stable-ID iteration everywhere ordering
matters, on-demand full-state checksums, and behavior/RNG/worldgen version
strings folded into the config hash. Evidence: two clean `lifesim fixture`
processes matched exactly (see `research/performance-notes.md` for the
recorded checksums), pause/resume proved trajectory-neutral, host event-read
patterns cannot perturb state, and seed/config changes diverge. Cross-platform
replay, parallel execution, and intervention logs remain out of scope and
untested. Status remains Proposed.

## Phase 2 Local Evidence (floating-point policy)

Phase 2 introduces bounded f32 genome/controller values under the recorded
floating-point determinism policy (ADR-0011): f32 add/multiply/divide only,
rational activation approximation, no libm transcendentals, no reliance on
FMA contraction; world state stays fixed-point. The declared guarantee is
clean-process replay on the same build, platform, and toolchain, verified
by `scripts/verify-phase2-determinism.sh` and in-process event-equality
replay tests. The design avoids known cross-platform variance sources, but
cross-platform byte equality is explicitly NOT claimed without evidence.
Enabling Phase 2 changes the config hash and starts a new replay lineage;
disabled configs replay bit-identically to Phase 1, preserving its
fixtures. Status remains Proposed.
