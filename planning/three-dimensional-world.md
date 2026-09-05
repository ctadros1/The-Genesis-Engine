# A Three-Dimensional World, Staged

Status: proposed 2026-09-05, awaiting the owner's acceptance of a stage.
Decisions: ADR-0040 (this plan's authority), ADR-0024 (voxel
presentation), ADR-0022 D2 (height and support, deferred), ADR-0006 (the
stream), ADR-0007 (persistence).

## Problem

The owner wants the world simulated, not only drawn, in three
dimensions. The kernel is a two-dimensional cell grid and everything
above it - field, sensing, movement, artifacts, terrain, saves, stream,
twenty-three phases of pinned fixtures - is built on that. A single
conversion is not a task; it is a new project stage. This plan stages it
so each step delivers a working system and states its own cost.

## Stage 0: the voxel view in the console (Phase 24)

**Scope.** `apps/console` live screen gains a 3D presentation: the
spike's terrain mesh (quantised heightmap, biome tint, biomass surface,
chunked greedy meshing) from the server's terrain cells plus a new
elevation value per cell; the spike's parametric bodies from the entity
record's existing fields (position, heading, flags, pigment, body scale,
energy); a free camera (orbit, pan, zoom, keyboard); a 2D/3D toggle that
keeps selection, inspector, overlay and controls working in both. The
server adds elevation to the terrain tile block as an additive field at
`ALSP` 1.1-minor (the codec's bounded decode and corruption sweep extend
to it). The spike's code is consumed, not edited: copied into the console
with a provenance header, or imported read-only from
`apps/observer-voxel-spike/src`; coordinate with its owner on which.

**Acceptance criteria.**
- [ ] V0.1 The 3D view renders a live 64x64 world at the observer's
      budget (ADR-0024's frame-time evidence) on the developer VM's
      console; measured, not asserted.
- [ ] V0.2 Every 2D interaction (select, inspect, overlay, pause,
      resume, speed, world switch) works in 3D; Playwright.
- [ ] V0.3 The elevation layer round-trips the protocol's golden bytes
      and survives the corruption sweep; every existing stream test
      unchanged.
- [ ] V0.4 Objects and signals are absent from the 3D view and the view
      says so (ALSP 1.1's object records are not in scope here).

**Cost.** Days. No kernel change; no fixture moves.

## Stage 1: height and support (Phase 25)

**Scope.** A config section `space` (off by default, hashed only when
on, ADR-0014 style): slope cost on movement from the elevation
gradient; a standing height per body; artifacts with a height and a
support relation (an object rests on the ground or on a supporting
object; unsupported objects fall); bounded stacking. No flight, no
burrowing. Save format: the section's fields under a new format version
with retained readers. Stream: per-entity `z` and per-object height at a
minor version. Analysis: the census tools read `z` where present.

**Acceptance criteria.**
- [ ] V1.1 Every pinned fixture (verify 1-23) reproduces bit for bit
      with the section off.
- [ ] V1.2 A new fixture pins the section on; determinism over 10^6
      ticks and across the two architectures (D-109's terms).
- [ ] V1.3 Support is a fact of the tick pinned by test: an object with
      its support removed falls on the next tick, never floats.
- [ ] V1.4 A pre-registered campaign, flat control against height on
      matched seeds, on an endpoint the record already measures
      (territory contest rate, artifact placement, lineage count), with
      a SESOI and an equivalence reading, run on the VM.

**Cost.** Two to four weeks of kernel work and tests before the campaign.

## Stage 2: the volumetric world (Phase 26)

**Scope.** Cells indexed by `x, y, z` with a bounded depth; terrain as
a solid volume with air above and rock below the surface; movement in
three axes gated by morphology (a body without the module for it cannot
fly or dig); sensing as a sphere; the chemistry field per voxel; artifacts
occupying voxels; the origin modes generating a volume. `ALSP` 2.0 (a
major: records carry `z`, tiles become blocks, bounds re-derived); save
format re-versioned with a migration for depth-one worlds; every verify
script re-established, with the depth-one world required to reproduce
every earlier fixture.

**Acceptance criteria.** Written when the stage is accepted; the first is
fixed now: the depth-one volume reproduces every fixture of the
two-dimensional record.

**Cost.** Two to three months before the first campaign; every
measurement after it is a new baseline.

## Non-Goals

- Editing `apps/observer-voxel-spike`: it is another engineer's; this
  plan consumes it.
- A generative model anywhere in the render path.
- Changing any recorded result.

## Risks

| Risk | Mitigation |
|---|---|
| The 3D view is read as evidence about the world | Every derived appearance comes from simulation state through the palette (ADR-0024); the scientific overlay stays available in 3D |
| Stage 1's section leaks into flat worlds | Hashed only when on; verify 1-23 as the gate; a mutant that applies slope cost with the section off must fail a test |
| Stage 2 invalidates the record | The depth-one reproduction criterion makes the old world a case of the new one, not a casualty |
| The spike is edited by two sessions | Read-only consumption; its owner decides promotion |

## Rollback

Stage 0 is a client view and an additive stream field; Stage 1 a config
section off by default; Stage 2 is the only irreversible stage and is
accepted separately.
