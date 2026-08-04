# ADR-0002: World Coordinate Model

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
The user wants a WorldBox-like big bounded continent and smooth organism movement with environmental realism.

## Options Considered
- Continuous organisms over raster environment.
- Fully grid-based organisms/world.
- Fully continuous fields and organisms.

## Proposed Decision
Propose continuous x/y organisms with raster terrain/climate/resource fields and bounded coast.

## Consequences
Gives visual fluidity and efficient environmental lookup while requiring spatial buckets and collision rules.

## Performance Implications
Benchmark cell resolution/bucket density and viewport aggregation; avoid all-pairs neighbor checks.

## Operational Implications
Snapshots store fields plus logical entity state; no special infrastructure.

## Revisit Conditions
Phase 1 proves spatial/index cost or visual quality unacceptable.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 1 Local Evidence

`sim-core` implements continuous fixed-point organism coordinates (1/1024 m)
over raster terrain/food cells with bounded-coast movement rejection, spatial
buckets sized to the interaction radius, and worldgen validation. Phase 1
tests cover bounds, water rejection, and deterministic replay; the local
Phase 1 benchmark records spatial-index and sense phase costs at 500 and
2,000 organisms (see `research/performance-notes.md`). Visual quality and
deployment-shaped VM cost remain unevaluated. Status remains Proposed.
