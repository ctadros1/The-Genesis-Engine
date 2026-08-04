# Pending Integration: Longevity And Rendering

Created 2026-08-04, alongside ADR-0023, ADR-0024,
`docs/27-time-scale-and-pacing.md`,
`specifications/long-horizon-soak.md`, and
`specifications/appearance-derivation.md`.

## Why This File Exists

These changes were written while a concurrent session was implementing
Phase 6. That session held `docs/22-decision-log.md`, `FILE_MANIFEST.md`,
`README.md`, `docs/19-implementation-roadmap.md`, `planning/backlog.md`,
and three specifications as uncommitted work.

Editing those files would have either clobbered in-progress work or forced a
merge in the middle of someone else's implementation. So the content that
belongs in them is staged here instead.

**This file is temporary.** Merge its contents into the target files once
Phase 6 is committed, then delete it.

## 1. Decision-log entries

Append to `docs/22-decision-log.md` using the **next available D-numbers**;
do not assume specific ones, because Phase 6 is claiming numbers
concurrently.

| Date | Status | Decision | Revisit Condition |
|---|---|---|---|
| 2026-08-04 | Accepted product direction | Two run modes (ADR-0023): **campaign worlds** (30 to 50 short independent worlds, maximum speed, disposable, the basis for every claim) and **flagship worlds** (one world, open-ended horizon, 1x default, preserved and backed up, watched). Neither changes kernel semantics. **A flagship world may never support a claim: n=1.** A flagship observation becomes a campaign proposal, with the flagship excluded from that campaign's seed set. Compute is an explicit allocation, not an emergent split | Soak criteria fail, making unattended operation unsupported; a flagship observation appears in a report as a claim; compute pressure makes the reservation untenable |
| 2026-08-04 | Proposed technical baseline | Time-scale position recorded (`docs/27-time-scale-and-pacing.md`): measured base rate is roughly 1,575 ticks per ancestry generation, so 1x gives about 23 generations/hour and 16,500 per 30 days. 1x is the flagship default. `dt` stays 100 ms and versioned. Speed multiplication never changes results, guaranteed by Phase 5's acceleration-neutrality criterion. Pacing is presentation and is never fixed by changing behavioral policy | Later phases move ticks-per-second materially, which each phase's Benchmark Impact section records; Phase 13 evolvable life history destabilizes the generational clock |
| 2026-08-04 | Proposed technical baseline | Long-horizon soak tiers (`specifications/long-horizon-soak.md`): Soak-7, Soak-30, Soak-90 against the current 864,000-tick maximum, which is 30 times short of a 30-day flagship run. Eight conjunctive criteria covering storage growth, tick-time drift, memory stationarity, invariants throughout, checkpoint restore at the horizon, clean-process determinism at the horizon, structural-quantity stationarity, and an explicit record of whether the world was still doing anything. **Soak-30 gates the flagship mode.** Re-run after any phase that adds a growth term | A tier fails, deferring flagship availability; a growth term is added by a later phase |
| 2026-08-04 | Proposed technical baseline | 3D voxel rendering with derived appearance (ADR-0024, amending ADR-0005). Governing rule: **author the visual primitives, not the appearances.** A palette of roughly fifteen primitives, one per module type and material and biome tint; every organism, artifact, and structure is an arrangement of those taken from simulation state. **No generative models in the render path** because they would invent state, cache at approximately zero hit rate, and break reproducibility; offline palette authoring and event-log narration remain permitted. Rendering dimensionality is not simulation dimensionality: the simulation stays 2D and the 3D view is a presentation with no kernel, determinism, or fixture impact | Voxel rendering cannot hold frame rate at the supported tier; the ADR-0022 D2 height subset lands, enabling stacked construction; sphere-world geometry is adopted |

## 2. FILE_MANIFEST.md additions

Documentation:

    - docs/27-time-scale-and-pacing.md

Planning:

    - planning/pending-integration-longevity-and-rendering.md   (delete after merge)

Specifications:

    - specifications/appearance-derivation.md
    - specifications/long-horizon-soak.md

Decisions:

    - decisions/0023-flagship-and-campaign-worlds.md
    - decisions/0024-voxel-rendering-and-derived-appearance.md

## 3. Roadmap note

`docs/19-implementation-roadmap.md` gains no new phase. Both additions are
cross-cutting:

- The **soak tiers** are a gate on the flagship mode and a standing
  obligation on every phase that adds a growth term (8, 9, 10, 11, 13).
  Each of those phases re-runs at least Soak-7 and restates its numbers.
- The **rendering change** is observer work, not a simulation phase. It has
  no kernel dependency and can land at any point after its benchmark
  evidence exists. It becomes materially more valuable after Phase 9, since
  morphology is what makes derived appearance structural rather than
  parametric.

## 4. Backlog additions

- Implement the voxel observer. Reuses protocol handling, selection,
  overlay, charts, and reconnect from the current observer; replaces the
  render layer. Gated on ADR-0024's benchmark evidence.
- Close the **composite geometry gap** in
  `specifications/artifact-and-material-ontology.md` (relative lattice
  offsets per constituent). Required before composites render as
  structures; the addition is specified at the end of that file.
- Build the **check-in report** for flagship worlds: what changed since the
  last observation, derived from the event log. Phase 16 machinery pointed
  at a different question.
- Define the **flagship retention and backup policy**, distinct from
  campaign pruning. A flagship world's checkpoint chain is irreplaceable
  once it has received any intervention.
- Raise or confirm the observer speed cap (currently clamped to 64) for
  flagship catch-up. Headless is uncapped, so this may not be needed.

## 5. Explicitly deferred

**Sphere-world geometry and off-planet environments.** Recorded in
ADR-0024's revisit conditions so that nothing in the rendering choice
forecloses them. The heightmap-over-raster approach does not: a sphere
changes the terrain parameterization and the camera, not the derivation
rule. Inter-world organism transfer additionally breaks Phase 5's world
isolation guarantee and would need its own ADR. Not designed here, by
instruction.
