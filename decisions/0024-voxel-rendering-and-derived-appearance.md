# ADR-0024: 3D Voxel Rendering And Derived Appearance

Status: Proposed
Date: 2026-08-04
Author: Rendering revision

**Amends ADR-0005** (Frontend Rendering Technology), which proposed
TypeScript with PixiJS v8 for a 2D pixel-art observer. ADR-0005 keeps its
status and its Phase 0 and Phase 3 evidence; this record states what
changes and what that evidence still supports.

## Context

Two questions arrived together: should the observer move to 3D voxels, and
how can novel organisms, artifacts, and structures be rendered without
pre-generating assets that would constrain what is allowed to exist.

The second question contains a false premise, and unpicking it answers both.

**Assets do not constrain what can exist.** The simulation defines what
exists; the renderer derives an appearance from it. A renderer that needs a
pre-made asset per organism is a symptom of a simulation that does not carry
enough structure to draw from. This project's simulation does carry it, and
after Phase 9 it carries it in a form that is already volumetric: an
organism is a set of typed modules at integer lattice positions with a
scale and inherited pigmentation. That is a voxel model sitting in the
genome.

## Options Considered

- **Keep 2D PixiJS with a sprite library.** Cheapest. Every organism,
  artifact, and structure must map onto a pre-authored sprite, so the
  visual vocabulary caps the expressible vocabulary. This is the outcome the
  question was worried about, and the worry is correct.
- **2D with procedurally composed sprites.** Better, and it still fights the
  representation: Phase 9 bodies are three-dimensional arrangements and a 2D
  projection discards the structure that makes them interpretable.
- **Generative models (image or 3D) producing assets at runtime.** Rejected;
  reasons below.
- **3D voxel rendering with appearance derived from simulation state, over a
  small authored primitive palette.**

## Proposed Decision

Adopt 3D voxel rendering with derived appearance, specified in
`specifications/appearance-derivation.md`.

### The governing rule

> **Author the visual primitives, not the appearances.**

This is the rendering form of ADR-0012's "author physics, not progress". The
palette is a bounded, versioned set of primitives, roughly one per module
type and one per material, on the order of fifteen entries. Every organism,
artifact, and structure in every world is an *arrangement* of those, and the
arrangement is simulation state rather than authored content.

Consequences: nothing is pre-generated, nothing is constrained by an asset
library, every appearance is traceable to the state that produced it, and a
body that evolved three motor modules visibly has three motor modules.

### Rendering dimensionality is not simulation dimensionality

The simulation is 2D: continuous fixed-point `(x, y)` over a raster, with
elevation as a field rather than a volume. That is unchanged by this ADR.

What is adopted is a **2.5D presentation**: heightmap terrain built from the
existing elevation field, with voxel organisms, artifacts, and structures
standing on it, under a free camera.

- This is a **presentation change only**. No kernel impact, no determinism
  impact, no fixture impact, no config-hash impact. The observer already may
  not own simulation truth, and rendering sits downstream of that boundary.
- What it does **not** buy: stacking, multi-storey structures, flight, or
  caves. Those require the simulation to have height, which is the 2.5D
  height-and-support subset deferred in ADR-0022 D2.
- A world rendered in 3D where objects visibly cannot stack will read as a
  limitation. That makes the D2 subset more attractive than it was, and it
  remains gated on the Phase 11 cost measurement rather than promoted here.
- Full volumetric terrain (caves, overhangs) would be a simulation change
  and is not proposed.

### No generative models in the render path

Not primarily because of the existing non-goal, which bans a language model
*deciding organism actions* and does not by itself cover a texture
generator. The reasons are separate and stronger:

1. **It would invent state.** An image or mesh model asked what a creature
   looks like answers from its prior about creatures, not from the organism.
   The picture stops being evidence about the simulation. This is ADR-0016's
   analysis-observes-never-instructs applied to rendering: the renderer may
   display state and may never invent it.
2. **Cache hit rate is approximately zero.** The natural cache key is the
   genome, and genomes are unique by construction.
3. **It breaks reproducibility.** Same seed and same config would produce
   the same world and different pictures.
4. **Cost and latency** at thousands of organisms changing every generation.

Two uses remain legitimate and are explicitly permitted:

- **Offline authoring of the primitive palette.** Using a generative tool to
  design the fifteen primitives once, committing the output as reviewed
  static assets, is asset creation with tooling assistance, not runtime
  generation. The palette is versioned like any other registry.
- **Event-log narration**, which `AGENTS.md` already permits as post-hoc
  text that consumes recorded events and can never influence a tick.

### On "relatively real"

Voxel is not photorealism, and that is deliberate. Photoreal rendering would
require inventing surface detail the simulation does not contain, which is
the generative-asset problem reintroduced by hand. Voxel maps one-to-one
onto state, so what is on screen is what the world actually contains, and
an observer can read structure directly off the image. Legibility is the
property this project needs from its renderer.

## Consequences

Positive: the asset-library ceiling disappears; Phase 9 morphology becomes
directly visible, which makes selection on body plan observable rather than
inferred; the primitive palette is small enough to author once.

Negative and accepted:

- **ADR-0005's Phase 0 and Phase 3 rendering evidence does not transfer.**
  The measured 2D figures (1 draw call, roughly 10 ms frames at 2,000
  entities, WebGL and WebGPU at desktop and mobile viewports) were for
  sprite rendering. A voxel path needs its own measurement before any
  performance claim, and the physical-device gate reopens.
- A renderer rewrite. The existing observer's protocol handling, selection,
  overlay, charts, and reconnect logic are reusable; the render layer is
  not.
- Instanced voxel rendering of thousands of multi-module organisms plus
  meshed terrain is well-trodden but is new work with a new failure mode
  (mesh rebuild cost on terrain modification).
- Mobile viability is unmeasured and may be worse than the sprite path.
- **Pre-Phase-9 organisms have no modules.** Schema 1 and 2 organisms carry
  only pigmentation, body scale, and a handful of scalars, so their
  appearance is a parametric derivation rather than a structural one until
  morphology lands. The specification covers both, and the pre-morphology
  form is deliberately plain.

Compatibility: no kernel, save, or determinism impact. Protocol gains
per-entity structural fields, which is a versioned protocol change under the
existing rules and not a silent field addition.

## Performance Implications

Unmeasured. Required before acceptance: frame time and draw calls against
entity count and mean module count; terrain mesh rebuild cost under
modification; memory; behavior at the dense viewport that the Phase 3 record
used, so the two are comparable in scenario if not in technique. No claim
about parity with the sprite path is made in advance.

## Operational Implications

Browser support matrix reopens: WebGL2 and WebGPU capability for instanced
voxel rendering is a different question from PixiJS sprite support, and the
physical-device gate that was already open now covers more.

## Revisit Conditions

- Voxel rendering cannot hold an acceptable frame rate at the supported
  entity tier, in which case the 2D path returns and the appearance
  derivation is projected rather than abandoned, since the derivation is
  representation-independent.
- The Phase 11 cost measurement leaves room for the height-and-support
  subset, at which point stacked construction becomes renderable and this
  ADR's main visible limitation lifts.
- Sphere-world geometry or off-planet environments are adopted, which would
  revisit the camera and terrain representation. **Explicitly out of scope
  here and deferred to a later project stage**; nothing in this ADR should
  foreclose it, and the heightmap-over-raster choice does not.

## Evidence Required To Accept

- Frame time, draw calls, and memory at the supported entity tier, with mean
  and distribution of module count.
- Terrain mesh rebuild cost under a realistic modification rate.
- Physical desktop and mobile device evidence, not viewport emulation.
- A rendered Phase 9 organism whose module structure is readable from the
  image, since that is the property motivating the change.
