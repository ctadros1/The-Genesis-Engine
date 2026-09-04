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

## The Console (ADR-0039)

`apps/observer` is unchanged and still connects to world 1 behind a token
paste. `apps/console` is the second client: the same instrument, entered
the way a game is entered. Plain TypeScript, Vite and PixiJS 8, no
framework; `protocol.ts` and `render.ts` are copied from `apps/observer`
with a provenance header naming the source and the copy date, and
`apps/observer` stays the source of truth for both.

The console is a screen stack. One screen is mounted at a time; every
screen is reachable by keyboard alone (arrow keys move, Enter activates,
Escape pops) and by pointer alone.

| Screen | What it is for |
|---|---|
| Title | The name, a live pixel background, a status strip naming the active profile, its role and the connection, and the menu: Continue (the last world viewed), Worlds, New World, Load Save, Server, About |
| Server | Connection profiles (name, REST base, WS base, observer token, admin token) kept in `localStorage`, a Test button that proves the tokens before committing to them, and the derived role shown as a badge; the last profile auto-connects on launch |
| Worlds | One card per hosted world - name, status, tick, population, a sparkline from polled history, seed, config hash, measured tick cost - refreshed every two seconds, with View, Pause/Resume, Save, Stop, Branch and Delete |
| New World | The builder: preset picker with descriptions, shipped recipes (named settings sets, e.g. Phase 22's chemistry-field base) applied on top of a preset, the settings grouped by prefix with search and typed inputs (toggle, number bounded by the field's type, choice select), a seed field with Randomise, the server's preview showing `config_hash` and validation errors live, and a summary of the settings that differ from the preset before Create |
| Live | The Observer's canvas, HUD, inspector, chart, overlay and pause/resume/speed, plus Back and a world switcher; reconnect and keyframe resync as before |
| Saves | Per world: the list, Save now, Verify, and Branch into a new world |

Three properties are load-bearing rather than incidental:

- **The builder is generated from the server's schema.** It hardcodes no
  field name and no validation rule; it knows only how to render each
  field type and how to diff a value against the active preset's default.
  The schema is the campaign field registry, so the console cannot name a
  setting a campaign could not, and it cannot drift from the vocabulary
  the experiments use.
- **Settings are editable before a world starts and never after.** The
  builder is the only place settings are typed, and there is no route
  behind which a running world's config could change (ADR-0039, docs/11).
- **Role is a capability, not a decoration.** A profile with no admin
  token derives the observer role, and every admin control is absent or
  disabled with an explanation rather than present and failing at the
  server. Admin actions confirm in a dialog that names the world and the
  action before they send, and the server audits each one with its world
  id, as this document's interaction rules require.

Accessibility carries over unchanged, and `tests/c4-accessibility.spec.ts`
is the check: a live status region announcing each screen and each
accepted action, focus order through every screen, real buttons, no
colour-only status (the status badge carries its word), and the title
background animation off under `prefers-reduced-motion`.

Tokens live in `localStorage` on the private LAN, exactly as the Observer
already keeps them. They are sent only as an `Authorization` header:
never in a URL, never in a log line, and the token inputs are password
fields.

The console runs first as a developer instance on the VM
(`scripts/run-console-dev.sh`, recipe in `docs/28`). Production at
`genesisengine.local` still serves `apps/observer` and changes only
through the root-only installer.

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
