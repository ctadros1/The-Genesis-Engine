# Phase 2: Evolution

## Local Implementation Status

The Phase 2 slice was implemented on 2026-08-04 as config-gated additions
to `crates/sim-core` and `crates/sim-cli` under policy `phase2-behavior-v1`
with genome schema `lifesim-genome-v1` (schema 1, topology 1), controller
policy `lifesim-controller-v1`, event schema 2, and similarity algorithm
`lifesim-similarity-v1`. Phase 1 fixtures are preserved exactly when
Phase 2 is disabled. Evidence: test suites in `docs/14-testing-strategy.md`,
benchmark and long-run records in `research/performance-notes.md`.
Deployment-shaped VM evidence remains a Phase 0 gate and is not claimed.

## Purpose
Introduce inherited compact neural behavior, sexual reproduction, lineage, and measured evolutionary dynamics without sacrificing stability.

## Scope
- Genome/phenotype schema, custom neural network, sensing/intents, mutation, crossover, sexual reproduction, lineage, species analysis.
- Long-run scenarios and invalid-genome protection.

## Non-Goals
- Structural topology evolution, language models, complex communication, rich morphology, browser-first controls.
- Biological claims beyond observed simulation results.

## Dependencies
- Phase 1 deterministic kernel and baseline metrics.
- Accepted neural/genome specifications and fixture format.

## Deliverables
- Versioned genome and controller schema.
- Bounded controller evaluator and intent resolver.
- Sexual reproduction/lineage event model.
- Species-cluster analysis job and long-run reports.

## Technical Tasks
1. Add normalized sensing and bounded output conversion.
2. Implement custom matrix evaluator with finite guards.
3. Add trait inheritance, crossover, mutation, and compatibility validation.
4. Implement lineage indexes/events and offline/periodic cluster analysis.
5. Create seeded ecology scenarios with assertions on health, not scripted outcomes.

## Acceptance Criteria
- [x] Malformed/non-finite genomes cannot crash or poison a world.
      Fail-closed typed decode; a validated-by-construction `Genome` is the
      only path into world state; 100,000-case seeded malformed-input
      harness ran with zero panics and zero invalid admissions; non-finite
      controller values are neutralized, counted, and evented.
- [x] Parentage, mutation, and birth energy are auditable. `PairedBirth`
      events carry both parent IDs, genome hash, per-parent investment, and
      mutated-gene counts; live ancestry state mirrors the event stream and
      is invariant-checked; the energy ledger stays exact to the milli-unit.
- [x] Strict replay covers controller and reproduction outcomes. In-process
      replay compares full event streams and checksums; two-clean-process
      fixture: config `0xf83d3981bf7dd189`, state `0xff9dfcff5dffbf42`
      (500 organisms / 500 ticks, seed `0x5eedcafef00dbeef`).
- [x] Long-run runs remain bounded and report meaningful diagnostics.
      200,000-tick release run: population bounded by the 5,000 ceiling,
      127 ancestry generations, 204,257 paired births, zero controller
      faults, exact ledgers, diversity and cluster diagnostics reported
      (see `research/performance-notes.md`).

## Test Requirements
- Neural fixture, fuzz, property, and saturation tests.
- Reproduction/lineage integration tests.
- Multi-generation long-run stability and replay tests.

## Benchmark Requirements
- Neural evaluation share of tick time and allocation count.
- Population/density scaling with representative controller topology.
- Species-analysis runtime outside the hot loop.

## Documentation Updates
- Update neural/genome/species docs, event schema, metrics, and proposed ADRs.

## Risks
- Neural signals may create opaque or degenerate behavior.
- Mutation/reproduction defaults may produce rapid extinction or runaway growth.

## Rollback Strategy
Retain Phase 1 worlds/configs. Disable new genome schema by feature/config version and preserve incompatible evolutionary saves as read-only.

## Suggested Codex Prompt
Use prompts/codex-phase-2.md. Do not introduce a UI workaround for an unproven kernel behavior.
