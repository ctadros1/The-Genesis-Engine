# Codex Prompt: Phase 0 Bootstrap

## Role

You are implementing one narrowly scoped task in Phase 0 for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- CODEX.md
- planning/phase-0-discovery.md
- docs/01-user-requirements.md
- docs/03-system-architecture.md
- docs/22-decision-log.md
- research/

## Objective
Create or evaluate only the minimal local technical spikes and benchmark harness needed to validate proposed architecture choices.

## Scope Boundary
- Deterministic tick microbenchmark, snapshot encoding spike, renderer/browser spike, reproducible benchmark metadata.
- Read-only evidence gathering only where separately authorized.

## Explicit Exclusions
- No production deployment or Proxmox/VM/network/monitoring changes.
- No full application scaffold or unrelated dependency installation.
- No claims of scale without recorded results.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Run deterministic fixture twice from clean process.
- Record toolchain/config/seed/hardware provenance.
- Run renderer smoke at desktop and mobile viewport.

## Documentation Updates
- planning/backlog.md
- research/performance-notes.md
- docs/21-open-questions.md
- docs/22-decision-log.md
- relevant proposed ADRs.

## Completion Report
List spikes performed, exact measurements, decisions advanced/deferred, files changed, and the narrowest Phase 1 proposal.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
