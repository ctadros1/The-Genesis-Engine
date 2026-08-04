# Codex Prompt: Phase 4 Persistence

## Role

You are implementing one narrowly scoped task in Phase 4 for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- planning/phase-4-persistence-and-analytics.md
- docs/12-data-storage-and-saves.md
- specifications/world-save-format.md
- specifications/event-schema.md
- infrastructure/backup-and-recovery.md

## Objective
Implement one save/recovery/export slice that is versioned, bounded, atomic, and proven by restore tests.

## Scope Boundary
- Logical state encoding, header/version/checksum validation, temporary write/atomic rename, catalog/event linkage, isolated restore.

## Explicit Exclusions
- Raw memory serialization.
- Best-effort unknown-version loading.
- Production backup changes without approval.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Golden/corrupt/truncated/oversized save tests.
- Interrupted-write simulation.
- Isolated restore and provenance comparison.
- Save duration/size benchmark.

## Documentation Updates
- Save/event/metrics specs
- backup runbook
- decision log/ADR
- risk register.

## Completion Report
State format version, migration policy, crash safety evidence, restore result, and rollback plan.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
