# Codex Prompt: Architecture Review

## Role

You are implementing one narrowly scoped task in Any pre-implementation or phase gate for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- docs/03-system-architecture.md
- docs/02-scope-and-non-goals.md
- docs/19-implementation-roadmap.md
- decisions/README.md
- research/

## Objective
Assess whether a proposed design keeps kernel/UI/storage/infrastructure boundaries, determinism, and scale claims coherent.

## Scope Boundary
- Compare credible alternatives, identify coupling/migration/operations/performance consequences, propose ADR updates.

## Explicit Exclusions
- Implementation work or broad refactor.
- Finalizing unbenchmarked choices.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Trace requirements to components.
- Identify data ownership and failure behavior.
- State benchmark and migration evidence needed.

## Documentation Updates
- Architecture docs, ADR candidates, decision log, open questions/risk register.

## Completion Report
Give a direct recommendation, invalidating assumptions, alternatives rejected, and exact next validation spike.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
