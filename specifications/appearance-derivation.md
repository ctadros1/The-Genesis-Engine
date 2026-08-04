# Appearance Derivation Specification

Status: design specification, not implemented. Decision: ADR-0024. Policy
version `lifesim-appearance-v1` (observer-side; see Determinism below for
why it is not in the config hash).

## The Rule

> **Author the visual primitives, not the appearances.**

Appearance is a **pure function of simulation state and the primitive
palette**. No appearance is stored, inherited, transmitted, or authored per
entity. Adding a new kind of thing to the world never requires adding an
asset for it; it requires the simulation to carry enough structure for the
renderer to draw from.

This is ADR-0012's "author physics, not progress" applied to rendering, and
it has the same test: if you find yourself authoring an appearance for a
specific outcome, that outcome has been scripted.

## The Primitive Palette

A bounded, versioned registry, on the order of fifteen entries. Palette
version is recorded in observer reports and screenshots.

| Primitive class | Entries | Source of the entry |
|---|---|---|
| Module primitives | One per module type: structural, sensory, motor, digestive, storage, reproductive, neural | `specifications/morphology-and-development.md` registry |
| Material primitives | One per material: stone, wood, fiber, carcass | `specifications/artifact-and-material-ontology.md` registry |
| Terrain primitives | One per biome tint plus water | `specifications/biome-and-climate.md` registry |

A primitive is a unit cube with a base colour, a surface treatment, and
nothing else. It carries no identity, no semantics, and no association with
any particular organism or artifact.

**The palette is authored once.** Designing those fifteen primitives with a
generative tool and committing the reviewed output is asset creation with
tooling assistance and is permitted (ADR-0024). Generating anything at
runtime is not.

## Layer 1: Terrain

Derived from fields that already exist. Nothing new is required from the
kernel.

    height(cell)  = elevation field, quantised to integer height steps
    tint(cell)    = biome primitive, modulated by moisture and temperature
    surface(cell) = biomass density selects a surface treatment
    overrides     = terrain modification deltas (Phase 11) replace the above

Rendered as a greedy-meshed heightmap. Terrain modification invalidates only
the affected chunk, so the mesh rebuild is bounded by modification rate
rather than world size. That rebuild cost is one of the measurements ADR-0024
requires before acceptance.

## Layer 2: Organisms

### With morphology (Phase 9 onward)

The body **is** the model. For each module:

    voxel_position = module.lattice_position
    voxel_scale    = module.scale
    voxel_class    = module.module_type          -> primitive
    voxel_colour   = primitive base colour, shifted by expressed
                     pigmentation genes (hue, pattern)
    orientation    = organism heading

Modules are emitted in ascending lattice index, so the same organism always
produces the same geometry in the same order.

Two properties follow, and they are the reason for the whole design:

- **Every organism is visually unique without any asset being unique**,
  because the arrangement is its genome.
- **Structure is legible.** An organism with three motor modules looks like
  it has three motor modules, so selection acting on body plan is directly
  observable rather than inferred from a chart.

### Before morphology (schema 1 and 2 organisms)

Current organisms have no modules, only pigmentation, body scale, and
scalar traits. Their appearance is a **parametric derivation** and is
deliberately plain:

    a single scaled voxel body, sized by body_scale_q8,
    coloured by pigment_hue_q8 and pigment_pattern_q8,
    oriented by heading_bam,
    with a maturity indicator

These are exactly the fields the existing render record already carries, so
this layer needs no protocol change. When morphology lands the parametric
form is retained for schema 1 and 2 worlds, which continue to exist and
continue to be renderable forever.

## Layer 3: Artifacts And Structures

An object renders as its material primitive, scaled by mass, tinted by
integrity so that decay is visible.

### The composite geometry gap

`specifications/artifact-and-material-ontology.md` defines a composite as a
bounded **list** of constituent object IDs with derived scalar properties
(mass sums, hardness maxima, durability minima). It defines **no spatial
arrangement**, so a depth-2 composite currently has no shape to render.

This is a genuine specification gap, surfaced by the rendering work rather
than created by it, and it needs closing in the artifact spec rather than in
the renderer. The renderer must not invent an arrangement, because an
invented arrangement is authored appearance.

The required addition, stated here as the constraint the artifact spec must
satisfy:

- Combination records a **relative lattice offset and orientation** per
  constituent, chosen deterministically at combination time from the
  combining organism's state and the `Artifact` stream.
- The offset set is validated as connected and non-overlapping, exactly as
  a body is; an invalid arrangement fails the combination with a typed
  reason rather than producing an invalid object.
- Fracture restores constituents to independent objects at their offset
  positions, preserving the existing mass and energy conservation.

Until that lands, composites render as an aggregate blob sized by total mass
and coloured by dominant material, and **the observer must not present that
as the object's structure**.

## Determinism And Boundaries

Appearance derivation is **observer-side and downstream of everything**.

- It reads render records and read-only detail views. It never writes.
- It is **not** in the config hash. A palette change alters what a world
  looks like and never what it does, which is the inverse of the rule for
  behaviour policies and the same asymmetry ADR-0016 applies to analysis
  versions.
- It is **not** in the state checksum. No appearance is stored.
- Palette version is recorded in reports and screenshots so an image can be
  reproduced.
- Derivation is a pure function, so the same state and palette always give
  the same image. This matters for regression testing the renderer and for
  comparing two runs visually.
- The kernel gains nothing. No new field exists for rendering alone; every
  input to derivation is state the simulation already needs.

## Protocol Implications

Render records gain per-entity structural data once morphology exists: a
bounded module list of `(lattice_position, type, scale)`, capped by
`max_modules`, plus the existing pigmentation and heading fields.

This is a **versioned protocol change** (`ALSP` minor version), not a silent
field addition, and it is bounded before allocation like every other count.
Deep morphology and full composition trees stay on the HTTP detail path;
they do not belong in a per-frame state stream.

The existing prohibition holds unchanged: **genome and controller matrices
never appear in state frames.** A module list is a phenotype summary, not a
genome.

## Test Requirements

- Derivation is a pure function: same state and palette give byte-identical
  geometry.
- Emission order is canonical (ascending lattice index) and independent of
  storage layout.
- A one-module body renders correctly, since that is the unicellular case
  and Phase 15 depends on it.
- A schema 1 organism renders through the parametric path with no module
  data present.
- Palette version mismatch between a report and a renderer is detected and
  reported, never silently rendered with the wrong palette.
- Terrain mesh rebuild is bounded by modification rate, verified under a
  high modification load.
- No render path writes to any world state, asserted structurally by the
  observer's existing read-only boundary.
