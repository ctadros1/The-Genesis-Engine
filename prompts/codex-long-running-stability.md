# Codex Prompt: Long-Running Stability Test

## Role

You are implementing one narrowly scoped task in Phase 2 onward for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- docs/04-simulation-model.md
- docs/14-testing-strategy.md
- docs/13-performance-strategy.md
- specifications/metrics-schema.md

## Objective
Run a bounded deterministic multi-generation stability scenario and produce evidence about correctness, memory, population behavior, and performance.

## Scope Boundary
- Representative fixed config/seed, telemetry, invariants, event sampling, and safe stop thresholds.

## Explicit Exclusions
- Changing rules mid-test without creating a new experiment.
- Claiming biological conclusions from a single run.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Non-finite/entity/energy invariants.
- RSS and allocation growth.
- Tick percentile and event-buffer bounds.
- Repeat same-seed control run.

## Documentation Updates
- research/performance-notes.md
- risk register
- decision log when a rule is changed.

## Completion Report
Provide config/seed/version/duration, outcome summaries, failures, resource curves, repeatability result, and next hypothesis.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
