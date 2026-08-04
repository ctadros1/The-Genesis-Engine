# ADR-0009: GPU Usage

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
servernode3 has a GTX 1660 Ti and servernode2 has an RTX 2060 Mobile, but compact neural inference may not benefit net of passthrough complexity.

## Options Considered
- CPU only initially.
- GPU passthrough to primary VM.
- Dedicated GPU experiment VM.
- Host-side GPU service.

## Proposed Decision
Propose CPU-only initial deployment and benchmark GPU only after neural inference is a measured bottleneck.

## Consequences
Reduces risk and preserves GPU resources; may delay a potential acceleration path.

## Performance Implications
Adopt only with end-to-end p95/RSS/VRAM/operational comparison.

## Operational Implications
Avoids guest drivers, passthrough, recovery and host scheduling complexity initially.

## Revisit Conditions
Phase 5 benchmark materially favors a safe GPU design.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.
