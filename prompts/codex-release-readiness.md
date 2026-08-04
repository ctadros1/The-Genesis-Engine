# Codex Prompt: Release Readiness

## Role

You are implementing one narrowly scoped task in After a phase or pre-deployment for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- active phase plan
- docs/14-testing-strategy.md
- docs/15-security-model.md
- docs/17-proxmox-deployment.md
- infrastructure/

## Objective
Assess a named version/phase against its documented acceptance, security, persistence, performance, observability, and rollback gates.

## Scope Boundary
- Evidence review, targeted validation, known-risk triage, go/no-go recommendation.

## Explicit Exclusions
- Deploying or changing infrastructure without explicit approval.
- Treating missing evidence as passing.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Required suite status.
- Benchmark provenance.
- Restore/backup result.
- Private-access/authorization check.
- Metrics/alert readiness.
- Rollback exercise/result.

## Documentation Updates
- Backlog, risk register, decision log, release record when introduced.

## Completion Report
Return a go/no-go recommendation, blockers, waivers with owners/expiry, and exact preflight steps.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
