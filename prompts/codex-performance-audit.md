# Codex Prompt: Performance Audit

## Role

You are implementing one narrowly scoped performance task for Artificial
Life Simulation. Since 2026-08-04 performance work is a standing discipline
rather than a phase of its own: it is carried by the active phase's
Benchmark Impact section plus `planning/backlog.md`.

## Required Reading
- README.md
- AGENTS.md
- the active phase plan in planning/ (see planning/backlog.md)
- planning/superseded/phase-5-performance-optimization.md (historical context only; do not execute it as written)
- docs/13-performance-strategy.md
- specifications/metrics-schema.md
- research/performance-notes.md
- infrastructure/gpu-evaluation.md

## Objective
Identify and improve one measured bottleneck without weakening correctness or operational safety.

## Scope Boundary
- Profile reproducible scenarios, make one bounded change, compare before/after, preserve baseline path.

## Explicit Exclusions
- Speculative rewrite.
- Distributed single-world execution.
- GPU adoption without end-to-end proof.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Full deterministic suite.
- Long-run stability where kernel changes.
- Tiered p50/p95/p99/RSS/bandwidth benchmark.
- Observer/save regression checks.

## Documentation Updates
- Performance notes/strategy
- metrics schema
- relevant ADR and decision log.

## Completion Report
Include baseline vs result, hardware/config/seed, correctness outcome, regression risk, and whether the change is retained.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
