# Codex Prompt: Security Review

## Role

You are implementing one narrowly scoped task in Any phase for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- docs/15-security-model.md
- docs/11-api-and-streaming-protocol.md
- specifications/world-save-format.md
- specifications/websocket-protocol.md
- infrastructure/network-plan.md

## Objective
Review code/config/design for private-access, input-validation, authorization, secret, and recovery risks.

## Scope Boundary
- Threat model, code-path review, negative tests, actionable findings ordered by severity.

## Explicit Exclusions
- Changing production security/network settings without approval.
- Treating UI visibility as authorization.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Auth/control negative tests.
- Malformed frame/save input tests.
- Secret scan/config review.
- Rate/bounds assertions.

## Documentation Updates
- Security model, risk register, ADR/decision log as applicable.

## Completion Report
Present findings first with file/line references, then residual risks and test gaps. State explicitly if no findings are discovered.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
