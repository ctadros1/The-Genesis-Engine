# Phase 14: Abiogenesis And The Unicellular Regime

Status: planned, not started. Policy versions `lifesim-chemistry-v1`,
`lifesim-microbial-v1`. Specification:
`specifications/unicellular-regime.md`. Decisions: ADR-0020, ADR-0018.

## Problem

The `scratch` origin mode begins with no organisms. Something must happen
between an empty world and the first individual-based organism, and it
cannot happen in the existing engine: per-individual microbes would need
several orders of magnitude more entities and ticks than the kernel has ever
been benchmarked at.

## Why This Phase Is Late

This is the most counterintuitive ordering decision in the roadmap, because
"from scratch" sounds like it should be first. Narrative order is not
dependency order.

- **Nothing depends on it.** Phases 6 through 13 all work from existing
  organisms. If this phase never lands, everything else still stands.
- **It depends on Phase 9.** The transition materializes one-module
  organisms, which requires the morphology representation to exist.
- **It is the least tractable work in the programme** and the most likely to
  return null. Placing the riskiest, least-depended-on work last is what
  keeps a null here from consuming the budget of everything else.

## Scope

- A field regime over the existing raster: per-cell chemistry
  concentrations and per-cell, per-genotype-class microbial densities.
- Diffusion, abiotic reaction, growth, death, and class-to-class mutation
  flow, all fixed point and exactly conserving.
- Abiogenesis: protocell formation as a rate function of local conditions.
- Reverse coupling: organisms consume from and excrete into the field.
- `origin.mode = scratch` producing a chemistry-only world.
- Scaffold configs under ADR-0018, each with a written description passing
  the naming test, and each with an unscaffolded control.

## Non-Goals

- No individual-based microbes.
- No real chemistry. A small bounded set of interconvertible abstract
  substrates with stoichiometric rules and an energy currency.
- No open-ended microbial genome evolution. Genotype classes are a bounded
  discretization, and ADR-0020 records this as a deliberate realism loss.
  Only the individual regime can demonstrate open-ended evolution.
- No multicellularity. That is Phase 15.
- No claim that `field_steps_per_tick` relates two real timescales
  correctly. It is the knob that makes the phase reachable.

## Prerequisites

- Phase 9 (morphology, for the one-module bodies Phase 15 will materialize).
- Phase 6 (origin modes framework and biome/climate fields, which the
  chemistry and abiogenesis rate functions read).
- Phase 5's throughput measurements, because this phase's affordability is
  the open question.

## Determinism Notes

- New streams: `Abiogenesis` (18), `MicrobialField` (19).
- **No per-individual randomness exists in the field regime.** Field cost
  and field determinism are both independent of population size, which is
  what makes the regime affordable.
- All field iteration in ascending cell index; class flows in ascending
  `(cell_index, source_class, target_class)`.
- Everything fixed point. The field integrates over horizons far longer than
  an organism lifetime, so Rule 7 applies here with more force than
  anywhere else in the project.
- Diffusion and reaction stencils conserve totals **exactly by
  construction**, not approximately.
- Field state is stored, unlike derived bodies and biomes, because it cannot
  be recomputed. Checksum sections `lifesim-chemistry-state-v1` and
  `lifesim-microbial-state-v1`.

## Acceptance Criteria

Conditions per ADR-0018, matched on seeds (30) and run length. Every
scaffolded condition runs its unscaffolded control on the same seeds.

- **N** (neutral): chemistry configured without regard to abiogenesis.
- **S1..Sk** (scaffolded): environmental structure shaped toward protocell
  formation and persistence, at a swept range of intensities. Each carries a
  written description that names no target.

Criteria:

- [ ] **C14.1 Exact conservation.** Total mass and energy across chemistry,
      field populations, and individuals is invariant to the milli-unit over
      a 10^6-tick run. Diffusion conserves exactly under adversarial
      gradients. This is the most important criterion in the phase: a
      conservation defect here corrupts every result downstream and is the
      classic failure mode of a two-representation design.
- [ ] **C14.2 Field cost is independent of individual population.** Measured
      across population tiers with the field regime active. If field cost
      scales with population, the regime separation has leaked and the
      design is wrong.
- [ ] **C14.3 Abiogenesis occurs and persists, or is reported as not
      occurring.** Report the rate of protocell formation and the fraction
      of formations producing a population that persists beyond a stated
      window, for **N and every S condition**, as a curve against scaffold
      intensity. The reportable result is the difference between conditions,
      never the scaffolded number alone.
- [ ] **C14.4 The scaffold is described without naming its target.** Every
      scaffold config carries a plain-language description that passes
      ADR-0018's naming test under review. A config describable only as
      "conditions that favor life" is withdrawn.
- [ ] **C14.5 Abiogenesis disabled produces a valid empty world** that
      remains savable, observable, and invariant-clean, exactly as an
      extinct world does today.
- [ ] **C14.6 Reverse coupling balances.** Organism consumption and death
      return exactly what they took, verified by ledger over a long run with
      heavy field-organism exchange.
- [ ] **C14.7 Determinism.** Clean-process fixture replay; field update
      order independence from storage layout; save round trip with a
      populated field.
- [ ] **C14.8 Snapshot growth is measured.** Per-cell field state adds a new
      growth term to a snapshot already carrying schema-3 genomes, learned
      state, objects, and terrain deltas. Measured against the Phase 13
      record, with the checkpoint budget re-verified rather than assumed.
- [ ] **C14.9 Fixtures preserved.** Field regime disabled reproduces the
      Phase 13 fixture exactly.

**Recorded in advance:** under condition N, abiogenesis is expected either
never to fire at a useful rate or to produce populations that do not
persist. That is the predicted outcome, not a failure, and C14.3 is written
so the negative is a measurement.

## Test Plan

- Unit: diffusion stencil conservation; reaction stoichiometry; growth and
  death rate laws at bounds; abiogenesis rate function at extremes.
- Property: no concentration or density leaves bounds under adversarial
  configs; totals conserved under every operation independently.
- Determinism: clean-process fixture; storage permutation; save round trip
  mid-population.
- Integration: organism-field exchange in both directions; empty-world
  validity; field regime with zero organisms and organisms with zero field.
- Long run: 10^6 ticks with exact conservation checked at intervals.
- Disabled-section equality against the Phase 13 fixture.

## Benchmark Impact

Field cost is proportional to cells times classes. At the default
65,536-cell world this lands on the environment phase, which the Phase 1
record already identifies as the dominant fixed cost.

Record: field phase p50/p95 against cell count and class count; the
`field_steps_per_tick` multiplier's effect; independence from organism
population across tiers; snapshot growth per cell; restore time with a
populated field.

Benchmark schema 9.

## Documentation Updates

`docs/04-simulation-model.md`, `docs/05-world-model.md`,
`docs/12-data-storage-and-saves.md`, `specifications/world-save-format.md`,
`specifications/simulation-tick.md`, `specifications/event-schema.md`,
`specifications/metrics-schema.md`, `docs/20-risk-register.md`, decision
log, ADR-0020.

## Risks

| Risk | Mitigation |
|---|---|
| **Conservation defect across the two representations.** The classic failure of this design shape: passes casual testing, corrupts long runs | C14.1 is the phase's weightiest criterion; conservation is by construction rather than by tolerance; checked at intervals over 10^6 ticks |
| Field cost dominates the tick | C14.2 plus the same dirty-cell strategy the environment phase already needs |
| Compute cost makes the campaign infeasible, worsened by ADR-0018's mandatory controls doubling every claim | Real and unresolved. Phase 5 measures the ceiling; if the campaign does not fit, the phase reports as underpowered rather than negative |
| Genotype-class discretization is too coarse for anything interesting to happen | Class count is config and swept; recorded in ADR-0020 as a deliberate realism loss |
| Scaffolding does more work than intended | ADR-0018's naming test plus review; a config granting an effective advantage to the target is a defect and is withdrawn |
| Snapshot growth breaks the checkpoint budget | C14.8 measures it; field state is the fifth growth term and the budget has been re-verified at each of the previous four |

## Rollback

One config section. Disabled, no field state exists, `scratch` is
unavailable, and the Phase 13 fixture reproduces exactly.
