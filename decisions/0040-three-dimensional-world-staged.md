# ADR-0040: A Three-Dimensional World, Staged

Status: proposed 2026-09-05, for the owner's decision. Design
authority once accepted: `planning/three-dimensional-world.md`. Extends
ADR-0024 (the voxel presentation, still Proposed) and ADR-0022 D2 (height
and support, deferred). Where this record and the plan disagree, the
disagreement is a defect in this record.

## Context

The owner asked for the simulator "to not only display 3D voxels, but
also simulate 3D voxels". Today the simulation is two-dimensional by
construction: the kernel indexes cells on an `x, y` grid, organisms move
on it, the chemistry field is a per-cell scalar per class, sensing is a
radius on the plane, artifacts sit in cells, terrain is a per-cell
elevation used for capacity and biome and never for standing on, the
save format and the stream carry `x, y` records, and every one of the
twenty-three phases' fixtures pins a checksum of that world. ADR-0024
chose to *present* the 2D world in three dimensions (a heightmap from
the elevation field with voxel bodies on it) exactly so that the kernel
would not have to change; its measurement spike
(`apps/observer-voxel-spike`, 2,229 lines, terrain meshing and the
derived-appearance palette on synthetic fixtures) exists and is owned by
another engineer's session.

The owner chose, when asked: plan first, and build the voxel view into
the console rather than keep it separate. This record is the plan's
authority; it commits to nothing in the kernel until the owner accepts a
stage.

## Decision, in stages

Each stage is a phase of its own on the roadmap, with the house law
unchanged: an ADR before implementation, a plan with acceptance criteria,
a pre-registration before any campaign, and the standing rule that a
flag off reproduces every pinned fixture bit for bit.

**Stage 0 - the voxel view in the console (Phase 24).** The console's
live screen gains a 3D presentation of the world as it is: terrain as
the spike's quantised heightmap from the elevation field the server
already streams as `(land, food)` cells plus an elevation layer added to
the terrain tile block, organisms as the spike's parametric bodies from
the entity record's existing fields, a free camera, a 2D/3D toggle. The
spike's renderer is consumed as a dependency (copied with a provenance
header or imported read-only), never edited from this work; objects and
signals still do not travel (ALSP 1.1 is unbuilt) and the view says so.
Kernel untouched; one additive stream layer (terrain elevation), which
is a minor version, not a major. Cost: days.

**Stage 1 - height and support (Phase 25).** Elevation becomes physics
in a config section that is off by default and hashed only when on
(`space.enabled`): movement pays a slope cost, a body stands on the
surface, an artifact placed on a cell has a height and a support
relation (the ADR-0022 D2 subset), and structures may stack to a bounded
height. Movement stays on the surface (no flight, no burrowing). The
save format gains the height fields under the section; the stream gains
per-entity `z` and per-object height at a minor version. Every existing
fixture reproduces with the section off (verify script); a new fixture
pins it on. Measurement: a pre-registered campaign asking whether height
changes anything the record already measures (territory, artifacts,
lineages) against the flat control. Cost: two to four weeks of kernel
work plus the campaign.

**Stage 2 - the volumetric world (Phase 26).** Cells gain a `z` index
with a bounded depth; movement, sensing, the chemistry field, artifacts
and terrain become three-dimensional; burrowing and flight become
morphology-gated capabilities; the protocol increments its major
(`ALSP` 2.0) and the save format its version; determinism, the fixture
set and every verify script are re-established for the new substrate,
and the 2D world becomes the depth-one case that must still reproduce
every earlier fixture. Cost: two to three months of kernel work before
the first campaign, and a new baseline for every measurement after it.

## What this is not

- Not a change to any result already recorded: the twenty-three phases
  stay valid as the two-dimensional record.
- Not a generative-model render path (ADR-0024's prohibition stands).
- Not a decision to build Stage 1 or 2: each is accepted separately.

## Consequences

- The owner sees the world in three dimensions at Stage 0 without a
  kernel change, which is the part that can be delivered while Phase 23
  runs.
- The cost of "simulating 3D" is stated per stage rather than absorbed;
  the record can stop after any stage with a working system.
- Stage 2 is the only stage that makes the earlier fixtures a baseline
  rather than the standard; that boundary is explicit.

## Revisit

When Stage 0 is measured (frame time at the observer's dense-viewport
budget, per ADR-0024's evidence list); before Stage 1, when ADR-0022 D2's
support rules are specified; before Stage 2, when the other engineer's
voxel spike is promoted or retired, since the renderer it would need is
theirs.
