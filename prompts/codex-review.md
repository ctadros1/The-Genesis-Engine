# Codex Prompt: Code Review

## Role

You are implementing one narrowly scoped task in Any implementation phase for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- active phase plan
- relevant specifications
- docs/22-decision-log.md
- changed code/tests

## Objective
Review the change primarily for defects, behavioral regressions, determinism, security, persistence/protocol compatibility, and missing tests.

## Scope Boundary
- Review only the requested diff and necessary dependencies.
- Run or inspect targeted tests as appropriate.

## Explicit Exclusions
- Unrequested rewrites.
- Implementing fixes unless explicitly requested.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Check test coverage and stale docs.
- Check benchmark claims against evidence.
- Check user-visible and operational failure cases.

## Documentation Updates
- Inline comments or review report only unless asked to edit.

## Completion Report
List findings first ordered by severity with file/line reference; then questions, residual risks, and a brief summary.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
