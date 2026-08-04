# Codex Prompt: Phase 1 Minimum Simulation

## Role

You are implementing one narrowly scoped task in Phase 1 for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- planning/phase-1-minimum-simulation.md
- specifications/simulation-tick.md
- specifications/entity-component-model.md
- docs/04-simulation-model.md
- docs/05-world-model.md

## Objective
Implement one deterministic headless organism-environment vertical slice with energy, food, movement, death, metrics, and CLI.

## Scope Boundary
- Pure simulation kernel and focused runner.
- Versioned config/seed and invariant tests.
- Benchmark the supported prototype tier.

## Explicit Exclusions
- Neural evolution, browser UI, persistence catalog, GPU, public access, broad refactor.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Unit/property tests for energy/resource/lifecycle.
- Repeatable seed test.
- Long-run stability test.
- Record 500 and 2,000 organism benchmark.

## Documentation Updates
- docs/04-simulation-model.md
- docs/05-world-model.md
- docs/14-testing-strategy.md
- research/performance-notes.md
- docs/22-decision-log.md

## Completion Report
State supported tier, deterministic policy evidence, performance results, remaining instability, and next Phase 1 task.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
