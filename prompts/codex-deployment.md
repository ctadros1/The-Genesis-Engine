# Codex Prompt: Deployment Planning Or Approved Rollout

## Role

You are implementing one narrowly scoped task in Phase 0/4/5 deployment gate for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- docs/17-proxmox-deployment.md
- infrastructure/README.md
- infrastructure/deployment-plan.md
- infrastructure/network-plan.md
- infrastructure/backup-and-recovery.md
- infrastructure/monitoring-plan.md

## Objective
Perform only approved read-only audit or approved explicitly enumerated deployment steps with rollback and verification.

## Scope Boundary
- Document actual target values, application health, metrics, backup/restore evidence.
- Use least privilege and preserve existing services.

## Explicit Exclusions
- Any unapproved Proxmox/VM/firewall/DNS/WireGuard/GPU/monitoring change.
- Credential creation/transmission without explicit authorization.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Pre/post health check.
- Service tick and private observer check.
- Metrics scrape verification.
- Save/restore or backup evidence as applicable.
- Rollback readiness confirmation.

## Documentation Updates
- Actual live values only in approved deployment records
- decision log, monitoring/backup runbooks, risk register.

## Completion Report
List exact actions, approvals, changed infrastructure, verification, rollback status, and no-op items. If not approved, provide only the proposed change plan.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
