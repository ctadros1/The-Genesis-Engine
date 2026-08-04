# ADR-0008: Primary Deployment Node

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
The supplied homelab includes servernode1, servernode2, and servernode3. The world may run continuously without interfering with existing services.

## Options Considered
- servernode3 isolated VM.
- servernode1 VM.
- servernode2 GPU-focused VM.
- multi-node single world.

## Proposed Decision
Propose an isolated servernode3 VM with 16 vCPU and 16-24 GiB RAM pending live audit.

## Consequences
Uses stronger RAM/core headroom; actual contention/storage/backups must be verified.

## Performance Implications
Benchmark guest CPU/memory at tiered loads before making scale promises.

## Operational Implications
Requires explicit VM/storage/network/backup approval; no change is authorized by this ADR.

## Revisit Conditions
Live audit finds inadequate capacity/contended host or another node produces superior operational fit.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.
