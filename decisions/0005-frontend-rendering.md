# ADR-0005: Frontend Rendering Technology

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
The observer needs pixel art, scientific overlays, mobile and wall support, and dense viewport performance.

## Options Considered
- TypeScript + PixiJS.
- Canvas only.
- Three.js.
- Phaser.
- Native desktop client.

## Proposed Decision
Propose TypeScript shell with PixiJS v8, WebGPU preferred and WebGL fallback.

## Consequences
Uses a purpose-built 2D renderer and culling/batching path; adds browser backend test matrix.

## Performance Implications
Benchmark draw calls/FPS/culling and stream data on target browsers.

## Operational Implications
Static browser assets served privately; no native client distribution initially.

## Revisit Conditions
Phase 0 browser spike finds compatibility/performance failure.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 0 Local Evidence

PixiJS 8.19.0 completed WebGL and WebGPU runs in local Chrome 150 with
per-entity culling and one instrumented draw call at 500/2,000 synthetic
markers. At 2,000 markers, desktop p95 update/cull plus render-submit CPU was
2.4 ms for WebGL and 2.2 ms for WebGPU; mobile-sized viewport values were 2.4
ms and 2.5 ms. These CPU-submission results do not establish GPU completion,
physical mobile/kiosk performance, or a WebGPU preference. Status remains
Proposed, and the spike defaults to WebGL while allowing an explicit WebGPU run.

## Phase 3 Local Evidence

The production observer (`apps/observer`) is a plain TypeScript shell with
PixiJS 8.19 (React deferred; decision log D-018): terrain texture layer,
pooled culled sprites, pan/zoom/pinch, selection, overlay, and charts.
Seven browser E2E tests pass on desktop and a 390x844 mobile viewport, and
live render sampling recorded frame interval p50 8.3 ms / p95 9.0 ms at
175 streamed organisms (WebGL, local Chrome). WebGPU preference and
physical mobile/kiosk devices remain unmeasured. Status remains Proposed.


## Amended By ADR-0024 (2026-08-04)

The 2D sprite decision is superseded in direction by ADR-0024, which adopts
3D voxel rendering with appearance derived from simulation state.

What this ADR's evidence still supports: TypeScript as the observer
language, the browser as the surface, viewport culling as the scaling
mechanism, and the server-authority boundary. What it no longer supports:
PixiJS as the renderer, and the measured Phase 0 and Phase 3 frame figures,
which were for sprite rendering and do not transfer to a voxel path.

Status remains Proposed. The physical desktop and mobile device gate that
was already open now covers a different rendering technique and must be
re-measured rather than inherited.
