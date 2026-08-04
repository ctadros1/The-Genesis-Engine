# Codex Prompt: Save-Format Migration

## Role

You are implementing one narrowly scoped task in Phase 4 onward for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- docs/12-data-storage-and-saves.md
- specifications/world-save-format.md
- specifications/event-schema.md
- infrastructure/backup-and-recovery.md

## Objective
Design and implement one explicit, reversible-or-safe-reject save-format migration.

## Scope Boundary
- Migration registry, old/new fixtures, validation, backup, isolated restore, compatibility report.

## Explicit Exclusions
- Silent field reinterpretation.
- Deleting old readers/migrators before retention policy allows.
- Live-world replacement before isolated proof.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Old fixture migration.
- Corrupt/unsupported rejection.
- Round-trip where semantically possible.
- Isolated restore/provenance comparison.

## Documentation Updates
- Save/event specs, migration register, backup runbook, ADR/decision log.

## Completion Report
State source/target version, semantic change, data loss policy, test results, rollback/reader-retention policy.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
