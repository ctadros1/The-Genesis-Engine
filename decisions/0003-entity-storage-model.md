# ADR-0003: Entity Storage Model

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
Thousands of organisms require efficient iteration but clear serialization/debuggability.

## Options Considered
- General ECS framework.
- Custom SoA dense component stores.
- Object-oriented per-organism graph.

## Proposed Decision
Propose custom SoA component stores with stable IDs and ECS-style systems.

## Consequences
Improves locality and snapshot clarity; costs bespoke storage/index code.

## Performance Implications
Validate sensor/neural/lifecycle hot-loop locality and deterministic iteration.

## Operational Implications
No deployment effect; schema requires explicit component versions.

## Revisit Conditions
A measured ECS framework offers material benefit without obscuring determinism/serialization.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 1 Local Evidence

`sim-core` uses custom SoA component arrays kept sorted by stable 64-bit ID
with append-only births, ordered compaction on death, and no ID recycling;
`check_invariants` proves ordering and exact ledger conservation each run.
The local Phase 1 benchmark records ~5 allocations per tick at both tiers
(dominated by two per-tick scratch vectors). No generic ECS framework was
needed. Serialization and Phase 2 component growth remain unevaluated.
Status remains Proposed.

## Phase 2 Local Evidence

Phase 2 component growth landed without an ECS framework: genome,
phenotype, controller, and ancestry arrays live in a config-gated parallel
SoA block that appends and compacts in lockstep with the primary arrays;
invariants verify synchronization, genome validity, and phenotype
consistency every check. Controller evaluation stays allocation-free per
tick. Save-format serialization remains a Phase 4 question. Status remains
Proposed.
