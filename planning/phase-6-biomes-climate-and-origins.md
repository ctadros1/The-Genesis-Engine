# Phase 6: Biomes, Climate Drift, And World Origin Modes

Status: **complete, 2026-08-04.** Benchmark record
`phase6-local-20260804T224418Z`. Decisions D-047, D-048, D-049. Policy versions `lifesim-biome-v1`, `lifesim-climate-v1`,
`lifesim-origin-v1`, generator `lifesim-worldgen-v2`. Specifications:
`specifications/biome-and-climate.md`,
`specifications/world-origin-modes.md`. Decision: ADR-0021.

## Implementation Status

All ten acceptance criteria are met. **Done and verified** (`crates/sim-core/src/climate.rs`, config section,
world integration, save/restore):

- Biome registry of seven biomes with permanent IDs, where the numeric order
  *is* the classification precedence, so "ties break by ascending biome ID"
  is the implementation rather than a comment. Classification is total by
  construction.
- Static fields: base temperature from latitude and lapse rate, coastal
  mask, land-normalized elevation, and a breadth-first sea-proximity field.
- Stateless climate drift: three incommensurate sinusoids evaluated in fixed
  point from the tick alone. C6.6's save-restore-continue equality holds.
- Moisture as an exactly-conserved field with a maintained gradient
  (see the finding below), stored under `lifesim-climate-state-v1`.
- Biome-dependent carrying capacity, reclassification on a configured
  cadence, and a ledgered `capacity_loss` sink so biomass conservation stays
  exact when a cell's biome becomes less productive.
- Degenerate-map rejection (C6.7), climate section in the ALIF codec as an
  optional section, and full fixture preservation (C6.1).

Origin modes (`crates/sim-core/src/origin.rs`, `crates/sim-persist/src/founders.rs`):

- `origin.mode` with `random` and `seeded`, the section excluded from the
  config hash while it holds the Phase 1/2 defaults, so both fixtures
  survive (C6.10).
- Founder demes with sorted centres, per-deme trait centres, and canonical
  `(group, draw_index)` ID allocation (C6.2, C6.3).
- Archetypes as trait distributions with biome affinity, biome-matched
  placement, and fail-closed refusal when no cell matches (C6.4).
- ALFP founder-population files with a self-checking header, record
  checksum, bounded decode, and a 20,000-case corruption sweep (C6.9).
- RNG streams `ClimateDrift` (21, unused under the default policy) and
  `FounderSeed` (22), appended and never renumbered.

## Three Findings From Building It

The first two were caught by the acceptance criteria doing their job, and
both would have quietly invalidated Phase 6 results.

**Biome classification collapsed onto elevation.** The first moisture model
made moisture a function of elevation alone. That makes the biome map a
relabelling of the elevation map: the driest cells are exactly the highest
cells, so `Highland` and `Arid` compete for the same cells and one ends up
empty. C6.7 rejected the world. The fix is a genuinely independent second
driver — breadth-first distance to water — blended with relief.

**Pure diffusion erases the world it models.** The conserving moisture
update was a straightforward diffusion, which satisfies C6.8 exactly and is
still wrong: diffusion has one fixed point, a uniform field. Worlds that
generated with all seven biomes had lost `Wetland` by tick 5,000. Every
Phase 6 result would have been a result about the model flattening its own
terrain. The update is now a pairwise exchange relative to a static per-cell
holding capacity, so the fixed point is a maintained gradient rather than
uniformity. Conservation is still exact, and a regression test asserts the
spread survives 20,000 ticks.

**Archetype IDs are absent, not merely inert.** The specification says the
kernel stores an archetype ID for provenance and nothing reads it. Storing an
authored label on every organism and trusting future code not to read it is a
standing invitation, so no ID is stored at all: an archetype influences the
genome it draws and then leaves no trace. C6.5 is therefore true by
construction. Which archetypes seeded a run is recorded where it is
legitimately needed — the config hash and the run manifest — so a report can
still state its starting condition (D-049).

The general lesson from the first two, recorded because it will recur: **a
conservation law is not a correctness proof.** Both defects conserved
everything they were supposed to conserve.

## Problem

Two gaps, and they turn out to be the same gap.

`docs/05-world-model.md` has specified biomes, moisture, temperature, and
drainage since Phase 1 and none is implemented; the generator produces
elevation, a land mask, and an elevation-derived food field. So the
temperature field that Phase 13 thermoregulation needs does not exist, and
the thermal-preference gene has been inherited-but-inert since Phase 2 with
nothing to be preferent about.

Separately, how a world begins is hard-coded. There is no way to specify
founder genomes, seed separated groups, or start from anything but
fully-formed organisms. That blocks Phase 7 directly: kin-biased grouping
and inter-group conflict are far sharper from several separated demes than
from one well-mixed pool.

They are one phase because biome-matched seeding needs biomes.

## Scope

- Moisture and temperature fields; biome classification from elevation,
  temperature, and moisture; biome-dependent food capacity.
- Climate drift: a stateless deterministic quasi-periodic temperature and
  moisture term on timescales far longer than the season.
- `origin.mode` with `random` and `seeded`; founder demes; archetypes;
  biome-matched placement; founder-population files.
- Generator successor `lifesim-worldgen-v2`. Worlds generated by v1 keep
  using v1 forever.

## Non-Goals

- **No age, era, or world-phase state.** Climate drift is a temperature
  term. No code reads "the world is in an ice age", nothing is spawned or
  swapped because of one, and there is no age field anywhere. An observer
  may label a cold stretch afterwards; that is Phase 17.
- No archetype named after a real species, and no document describing a
  seeded run as containing real animals.
- No archetype ID readable by any rule, input channel, mating gate, or
  analysis grouping.
- No `scratch` mode; that is Phases 15 and 16.
- No weather forecasting. The moisture model stays the deliberately
  simplified one `docs/04` describes.
- No disasters.

## Prerequisites

- Phase 5, for the condition and campaign machinery the seeded-versus-random
  comparisons need.

## Determinism Notes

- New streams: `ClimateDrift` (21, unused under the default deterministic
  policy), `FounderSeed` (22). `FounderSeed` is separate from `GenomeInit`
  (5) so adding origin modes cannot shift an existing `random` world's
  founder sequence.
- `drift(tick)` is stateless: a fixed sum of sinusoids at incommensurate
  configured periods, evaluated in fixed point from tick alone. It cannot
  accumulate error and needs no storage, and it is exactly reproducible at
  an arbitrary tick offset after a restore.
- Classification and every field update iterate in ascending cell index;
  biome ties break by ascending biome ID.
- Deme centres are drawn, then sorted by cell index before founders are
  assigned. Founder entity IDs are allocated in ascending `(archetype_id,
  draw_index)` order.
- Biome is derived and excluded from the save. **Moisture** carries history
  and is stored under `lifesim-climate-state-v1`; temperature is not, because
  under the default deterministic policy it is a pure function of
  `(base, tick)` and carries no history at all. That is a deliberate
  departure from this document's original wording, recorded in D-047; a
  stochastic weather policy would move temperature into the section.
- The generator version is inside the config hash, so v1 worlds keep their
  terrain checksums and both fixtures forever.

## Acceptance Criteria

- [x] **C6.1 Fixture preservation.** `origin.mode = random` with default
      parameters and the climate section disabled reproduces
      `0x1e3158a26afd3b39` and `0xff9dfcff5dffbf42` from clean processes. A
      v1-generated world reproduces its recorded terrain checksum.
- [x] **C6.2 Founder generation is order-independent.** Permuting archetype
      iteration order and deme processing order produces an identical
      founder population, verified by state checksum at tick 0.
- [x] **C6.3 Demes are real.** With `deme_count = 4`, mean genetic distance
      within a deme is lower than between demes by the stated effect size at
      tick 0, in 30 of 30 seeds. This is deterministic setup, so anything
      less than 30 of 30 is a defect rather than a result.
- [x] **C6.4 Biome-matched placement or fail-closed.** Every founder is
      placed in a cell matching its archetype's affinity, or generation
      fails with a typed actionable error. No founder is ever silently
      placed in an unsuitable biome.
- [x] **C6.5 Archetype IDs are inert.** A run with archetype IDs permuted,
      founder genomes held identical, produces a bit-identical trajectory.
      This is the test that keeps an authored label from explaining an
      outcome, and it is not optional.
- [x] **C6.6 Climate drift is stateless and exact.** `drift(tick)` computed
      at tick T directly equals `drift(tick)` computed after saving at an
      arbitrary earlier tick and restoring. Temperature and moisture stay
      within configured bounds for every cell across a 10^6-tick run
      covering full drift cycles.
- [x] **C6.7 Biome distribution is validated.** Generation rejects a world
      in which any biome is empty or occupies the whole map, with an
      actionable error, exactly as land-fraction validation does today.
- [x] **C6.8 Moisture conserves.** Total moisture is exactly conserved by
      the diffusion-like update over a 10^6-tick run.
- [x] **C6.9 Founder files are hostile-input safe.** A seeded corruption
      sweep of at least 20,000 cases over the founder-file codec produces
      zero panics and typed rejections.
- [x] **C6.10 Seeded and random are distinguishable experiments.** The two
      modes under otherwise identical config produce different config
      hashes, and the comparison report refuses to aggregate them.

Every criterion is an automated test:
C6.1/C6.6/C6.8 in `crates/sim-core/tests/determinism.rs`, C6.2 to C6.5 and
C6.10 in `crates/sim-core/tests/phase6_origin.rs`, C6.7 in the generation
validator exercised by both, C6.9 in `crates/sim-persist/tests/founders.rs`,
and the climate unit tests in `crates/sim-core/src/climate.rs`.

## Test Plan

- Unit: classification at threshold boundaries and ties; drift evaluation at
  extreme tick values; lapse and latitude terms at bounds.
- Property: temperature and moisture stay in bounds under adversarial
  configs; biome classification is total (every land cell classifies).
- Determinism: C6.2, C6.5, C6.6 as automated tests; clean-process fixture.
- Integration: founder file round trip; deme placement honoring minimum
  separation; fail-closed placement when a biome is absent.
- Long run: 10^6 ticks with full drift cycles, exact moisture conservation,
  bounded fields.
- Disabled-section equality against the Phase 5 fixture.

## Benchmark Impact

New per-tick cost in `environment`: moisture diffusion, temperature update,
and biome reclassification over the full cell grid. The Phase 1 record
already identifies the environment phase as the dominant fixed cost at about
200 microseconds for the 65,536-cell logistic regrowth scan, so this phase
lands directly on the known hot path and the measurement matters.

Record: environment phase p50/p95 with each field enabled independently;
reclassification cost and whether it needs a dirty-cell strategy; founder
generation one-time cost at tick 0 by mode.

Benchmark schema 4.

## Documentation Updates

`docs/05-world-model.md` (fields become live), `docs/04-simulation-model.md`
(climate section), `docs/06-organism-model.md`, `docs/21-open-questions.md`,
`specifications/experiment-config-schema.md`,
`specifications/simulation-tick.md`, `specifications/metrics-schema.md`,
`specifications/world-save-format.md`, decision log, ADR-0021.

## Risks

| Risk | Mitigation |
|---|---|
| Environment phase cost grows past the tick budget on the already-dominant hot path | Measured per field; dirty-cell or region tracking is the known first optimization and was already the Phase 5 hypothesis in the superseded plan |
| Archetype authoring quietly encodes expectations about what should evolve | C6.5 inertness test; archetypes are distributions not organisms; no real-species names; every seeded result reports its archetype set |
| A seeded result is read as a reachability claim | Recorded in ADR-0021: a seeded run is a weaker basis for "behavior X evolved" than a random one, and reports must say which mode produced them |
| Climate drift is described as an era somewhere downstream | No age state exists to describe. `docs/25` governs the vocabulary and Phase 17 owns post-hoc labelling |
| Biome thresholds produce a degenerate map | C6.7 fails generation closed rather than producing a one-biome world |

## Rollback

Climate and origin are separate config sections. Both disabled with
`origin.mode = random` at defaults gives the Phase 5 execution path and
fixture. `lifesim-worldgen-v1` remains in the build permanently; v1 worlds
never see v2.
