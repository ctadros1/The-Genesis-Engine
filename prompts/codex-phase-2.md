# Codex Prompt: Phase 2 Neural Evolution

## Role

You are implementing one narrowly scoped task in Phase 2 for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- planning/phase-2-evolution.md
- docs/06-organism-model.md
- docs/07-neural-network-design.md
- docs/08-genetics-and-evolution.md
- specifications/organism-genome.md
- specifications/neural-network-schema.md

## Objective
Add one bounded, versioned neural/genetic/reproduction slice while keeping the kernel deterministic and safe under hostile genome input.

## Scope Boundary
- Custom controller evaluator, normalized senses/intents, genome validation, mutation/crossover, lineage events.
- Long-run scenario evidence.

## Explicit Exclusions
- LLM creature control, unbounded topology evolution, UI workaround, untested GPU framework.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Neural fixtures and fuzz/property tests.
- Reproduction/lineage integration tests.
- Deterministic replay.
- Long-run stability/phase timing benchmark.

## Documentation Updates
- Neural/genetics/species docs
- event and genome specs
- testing/performance docs
- ADR and decision log.

## Completion Report
Explain new schema/version, invalid-input behavior, replay effect, measured neural cost, and next task.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
