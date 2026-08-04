# Observer Interface

## Phase 3 Implementation Status

`apps/observer` implements the first slice: world canvas (terrain texture
from quantized food/land cells, pooled organism sprites tinted by genome
pigment with body-scale sizing, pan/zoom/pinch, per-sprite culling),
selection with an inspector fed by the bounded HTTP detail endpoint
(energy, age, parents, generation, offspring, phenotype, genome hash),
a scientific overlay toggle (grid plus selected-organism sensor radius),
a population sparkline with a text alternative, pause/resume/speed
controls that are disabled without an admin token, and automatic
reconnect with keyframe resync. The shell is plain TypeScript with PixiJS
v8 (a React shell remains an option under ADR-0005; the deviation is
recorded in the decision log). Follow mode, replay browsing, lineage
trees, heatmaps, and wall/kiosk story mode remain future observer work.
Accessibility in this slice: live status region, real buttons, chart text
alternative, reduced-motion support (heading rotation disabled), and
selection indicated by ring plus inspector text rather than color alone.

## Design Direction

The observer is a clear scientific instrument with a lively pixel-art surface. It supports desktop, phone, and wall-display use without reducing the world to a game HUD. Pixel art conveys terrain and organisms; a switchable scientific layer reveals exact information, selection state, ranges, and heatmaps.

## Primary Views

| View | User Outcome | Required Data |
|---|---|---|
| World canvas | Pan, zoom, follow, inspect island in real time | Viewport tiles and organism deltas |
| Organism inspector | Understand one organism's state, traits, controller, parents, offspring | Entity metadata on demand |
| Population analytics | Compare population, species clusters, births/deaths, resource trends | Downsampled time series |
| Experiment/replay | Reopen a seed/config/save and compare branches | World catalog and provenance |
| Control console | Make logged sandbox interventions | Admin-authorized commands |
| Debug overlays | Diagnose simulation state and transport | Configurable diagnostic layers |

## Interaction Rules

- Observer access is read-only by default.
- Sandbox controls require administrator authorization and confirmation in the UI.
- Every accepted intervention creates an audit event and, if branching a saved world, a new world lineage.
- Pause, resume, step, and speed controls affect only the selected world and expose their effective state.
- Follow mode subscribes to a bounded region around an organism; it does not request full-world state.

## Rendering

Use PixiJS v8 for WebGPU-preferred/WebGL-fallback 2D rendering. Render terrain as cached/tiled layers, organisms as batched sprites, and scientific overlays as opt-in layers. Use culling, level of detail, capped trail history, and object reuse. Do not render text labels for every organism; labels appear at selection/zoom thresholds.

## Responsive Layout

Desktop: persistent world, inspector, control rail, and charts. Mobile: world-first canvas, bottom-sheet inspection, compact speed controls, and deferred detailed charts. Wall dashboard: read-only kiosk mode, high-contrast large labels, auto-follow/story mode optional, and no exposed destructive controls.

## Accessibility

The canvas must have text alternatives for selected-world status, simulation state, controls, and key alerts. Respect reduced motion. Color cannot be the only signal for species, health, threat, or selection. Chart values must be inspectable without relying only on hover.

## Acceptance Criteria

- A mobile user can pan/zoom, select, inspect, pause if authorized, and recover after reconnect.
- A scientific overlay can be toggled without changing simulation state.
- A viewport with dense organisms stays responsive through LOD/culling.
- Browser render cadence is decoupled from server tick cadence.
