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

## 6. Phase ordering change (ADR-0025)

The former Phase 13 is **split**, and its demographic half moves before the
culture stack. Reason: the Phase 2 record shows 199,871 starvation deaths
against 180 old-age deaths with population pinned on the `max_entities`
guard, so every energetically gated culture criterion would have returned a
null caused by starvation rather than by anything about transmission.

New execution order, with current file numbering preserved:

| Executes | Phase | File |
|---|---|---|
| after 7 | **13a Demography and life history** | `phase-13a-demography-and-life-history.md` |
| after 12 | **13b Ontogeny and sexual selection** | `phase-13b-ontogeny-and-sexual-selection.md` |

`planning/phase-13-physiology-and-life-history.md` no longer exists; it was
renamed to `phase-13b-...` and its demographic content moved to `13a`.

### Decision-log entry (next available D-number)

| Date | Status | Decision | Revisit Condition |
|---|---|---|---|
| 2026-08-04 | Proposed technical baseline | Former Phase 13 split by ADR-0025, amending ADR-0017's placement argument. **13a** (allometry, thermoregulation, senescence, non-food extrinsic mortality, juvenile mortality, life-history tradeoff, death-cause accounting) executes after Phase 7; it has no unmet prerequisites and delivers the non-food mortality that produces per-capita surplus. **13b** (developmental ontogeny, mate choice on perceived phenotype, optional disease) stays after Phase 12 because ontogeny needs the Phase 9 module body and mate choice needs Phase 12 perception. Accepted cost: per-organism cost arrives earlier and applies to every subsequent phase; Phase 7 results do not transfer across 13a | C13a.1 shows the death-cause distribution does not become mixed; the measured throughput cost outweighs running invalid campaigns; ecology cannot bind below an affordable `max_entities` |

### Roadmap changes (`docs/19-implementation-roadmap.md`)

- Phase table: replace the `13 Physiology, life history, and senescence`
  row with two rows, `13a` executing after 7 and `13b` after 12.
- Replace the "**13 after the culture stack**" ordering paragraph with the
  ADR-0025 argument: the old placement assumed the culture results would
  otherwise be valid, and at 99.9 percent starvation mortality they would
  not be. Note that only the demographic half moves, because ontogeny and
  mate choice have hard upstream dependencies.
- Decision gates table: split the `Realism` gate row into a `Demography`
  gate (C13a.1 to C13a.3: starvation ceases to dominate, population sits
  below carrying capacity, ecology binds rather than the guard) and a
  retained `Realism` gate (Hardy-Weinberg, linkage decay, allometric
  exponent, lifespan versus extrinsic mortality).

### Backlog changes (`planning/backlog.md`)

- Phase table: same two-row split.
- Ordered next work: after Phase 7, insert Phase 13a before Phase 8.
- Add: **raise `max_entities` above the ecological equilibrium** so ecology
  binds instead of a memory guard. Currently the guard is the carrying
  capacity and every measured dynamic is an artifact of it. This is a config
  change gated on the Phase 13a benchmark.

### FILE_MANIFEST.md

    - planning/phase-13a-demography-and-life-history.md
    - planning/phase-13b-ontogeny-and-sexual-selection.md
    - decisions/0025-demography-before-culture.md

(remove `planning/phase-13-physiology-and-life-history.md`)

## 7. Numbering repair, to be done in one pass

Phase numbering is no longer monotonic: `13a` executes between 7 and 8, and
`13b` after 12. This is deliberate and temporary. A full renumber touches
the roadmap, decision log, backlog, manifest, and every cross-reference in
roughly forty files, and those were held uncommitted by the Phase 6 session
when ADR-0025 was written.

**Do the renumber in one atomic pass once Phase 6 is committed**, not
incrementally. Two prior renumbers in this project each left defects that
took a sweep to find: stale multi-number lists where only the first number
was mapped ("Phases 8, 8, and 10"), and table rows whose bare phase numbers
stayed put while their meanings moved. Check both patterns explicitly
afterwards.

Suggested target sequence: 5 headless, 6 biomes/origins, 7 contest,
8 demography, 9 genome, 10 morphology, 11 learning, 12 artifacts,
13 social, 14 ontogeny/sexual selection, 15 abiogenesis,
16 multicellularity, 17 era detection.

## 8. Explicitly deferred

**Sphere-world geometry and off-planet environments.** Recorded in
ADR-0024's revisit conditions so that nothing in the rendering choice
forecloses them. The heightmap-over-raster approach does not: a sphere
changes the terrain parameterization and the camera, not the derivation
rule. Inter-world organism transfer additionally breaks Phase 5's world
isolation guarantee and would need its own ADR. Not designed here, by
instruction.
