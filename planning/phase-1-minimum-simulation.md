# Phase 1: Minimum Simulation

## Local Implementation Status

The Phase 1 slice was implemented on 2026-08-03 as `crates/sim-core` and
`crates/sim-cli` under behavior policy `phase1-behavior-v1`. All acceptance
criteria below are evidenced locally: deterministic fixtures and the
benchmark record are curated in `research/performance-notes.md` (benchmark ID
`phase1-local-20260804T034607Z`), and the test suites are described in
`docs/14-testing-strategy.md`. Deployment-shaped VM evidence remains a
Phase 0 gate and is explicitly not claimed here.

## Purpose
Prove a deterministic headless organism-environment loop before evolution, browser controls, or persistence complexity.

## Scope
- World raster, continuous organism positions, food growth, energy, movement, feeding, aging/death.
- Fixed tick, named RNG streams, spatial buckets, headless CLI, basic metrics.
- Simple asexual reproduction only if required to validate lifecycle; no neural controller yet.

## Non-Goals
- Sexual reproduction, species clustering, full observer, save catalog, GPU, disasters.
- Claims beyond the documented 500-2,000 prototype tier.

## Dependencies
- Accepted/revised Phase 0 baseline.
- Specifications for tick, ECS/data layout, world save header, metrics.

## Deliverables
- Pure sim-core crate/module and headless runner.
- Configurable seeded continent world and core metrics.
- Focused test suite and baseline benchmark.

## Technical Tasks
1. Implement versioned config/seed/world metadata.
2. Implement terrain/resource fields and bounded spatial index.
3. Implement organism lifecycle and energy ledger.
4. Implement stable ordered tick phases and metrics hooks.
5. Implement CLI to run, pause, inspect summary, and emit a deterministic checksum.

## Acceptance Criteria
- [x] Same compatible run produces documented deterministic result. Two clean
      processes match: state checksum `0x1e3158a26afd3b39` at 500 organisms /
      500 ticks, seed `0x5eedcafef00dbeef`, config hash `0x918a381c77559236`.
- [x] No NaN, out-of-bounds, energy-accounting, or entity-count invariant
      failure in long-run test. All-integer state cannot produce NaN; the
      864,000-tick (24-hour-equivalent) release run passed with exact ledger
      and bounds checks every 10,000 ticks.
- [x] Headless baseline benchmark reaches the declared supported population
      tier. Local fixed-population p95 tick: 228.6 us at 500 and 364.0 us at
      2,000 organisms against the 100 ms budget (local host only; VM tier
      claim still requires the Phase 0 infrastructure gate).
- [x] No UI/HTTP/persistence dependency leaks into sim-core. The crate has
      zero dependencies and no clock/filesystem/network use; timing hooks are
      host-implemented via `TickObserver`.

## Test Requirements
- Unit and property tests for formulas, movement, death, resource bounds.
- Seed fixture and 24-hour-equivalent long-run stability test.
- CLI integration test and metrics scrape-format test.

## Benchmark Requirements
- Phase timing, RSS, allocation count, entity count at 500 and 2,000.
- Spatial-query cost with representative density.

## Documentation Updates
- Update simulation/world/entity specs, test strategy, benchmark notes, decision log.

## Risks
- Premature neural/controller abstraction may obscure core invariants.
- Rule defaults can cause an uninformative extinction/explosion.

## Rollback Strategy
Keep config/version changes branchable. Revert code to the last deterministic fixture baseline; preserve failing seed/config evidence.

## Suggested Codex Prompt
Use prompts/codex-phase-1.md. Build only the minimum deterministic slice and do not add Phase 2/3 systems.
