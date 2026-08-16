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


## Planned Successor: 3D Voxel Observer (ADR-0024)

The observer moves from 2D pixel-art sprites to a 3D voxel presentation:
heightmap terrain built from the existing elevation field, with voxel
organisms, artifacts, and structures standing on it under a free camera.

**Appearance is derived, never authored per entity.** The renderer holds a
bounded palette of roughly fifteen primitives (one per module type, one per
material, one per biome tint) and every organism, artifact, and structure is
an arrangement of those, taken from simulation state. Nothing is
pre-generated, so nothing about the asset pipeline caps what the simulation
is allowed to contain. See `specifications/appearance-derivation.md`.

*Status 2026-08-16: the kernel now has artifacts (Phase 12, ADR-0028: four
registry materials, simple and composite objects with a per-object
material, integrity and depth) but no frame carries them yet - `ALSP` 1.1
is unbuilt (`specifications/websocket-protocol.md`), so the observer, 2D
or voxel, cannot show an object. The "one primitive per material" palette
above has four materials to draw from when it does.*

Consequences for this document's design direction:

- **Legibility is preserved and strengthened.** The scientific-overlay
  principle is unchanged, and structure becomes directly readable: after
  Phase 10 an organism with three motor modules visibly has three motor
  modules, so selection on body plan is observed rather than inferred.
- **Rendering dimensionality is not simulation dimensionality.** The
  simulation stays 2D. The 3D view is a presentation of a 2D world with an
  elevation field, so it carries no kernel, determinism, or fixture impact.
  Stacking, multi-storey structures, and flight are **not** available and
  require the height-and-support subset deferred in ADR-0022 D2.
- **No generative models in the render path.** An image or mesh model asked
  what a creature looks like answers from its prior, not from the organism,
  which would make the view stop being evidence about the world. Offline
  authoring of the primitive palette is permitted; runtime generation is
  not.
- **Reusable from the current observer**: protocol handling, selection,
  overlay toggling, charts, controls, reconnect and resync. **Not reusable**:
  the render layer.
- Pre-Phase-9 organisms have no modules and render through a plain
  parametric derivation from the pigmentation, body-scale, and heading
  fields the render record already carries.

Sphere-world geometry and off-planet environments are explicitly out of
scope and deferred to a later project stage.
