# Phase 13a: Demography And Life History

**Executes after Phase 7 (contest), before Phase 8 (genome successor).**
Numbering is provisional: the file keeps a 13a label because a full
monotonic renumber is deferred; see ADR-0025 and `docs/19-implementation-roadmap.md`,
which carries the authoritative execution order.

Status: planned, not started. Policy version `lifesim-demography-v1`.
Split from the former Phase 13 by ADR-0025; the remainder is
`planning/phase-13b-ontogeny-and-sexual-selection.md`.

## Problem

The current world is demographically pathological, and the evidence is in
the Phase 2 long-run record:

    199,871 starvation deaths
        180 old-age deaths
      population pinned at the 5,000 `max_entities` ceiling

That is a world where food is the only brake and a memory guard is the only
ceiling. Reproduction is gated on energy alone, so every organism breeds
whenever it can, population grows until it starves, and any lineage that
spent energy on something other than survival was outcompeted instantly.

Three consequences, and the third is why this phase moved:

1. **No surplus can exist.** Every organism is one bad tick from death.
2. **`max_entities` is doing the ecology's job.** When population sits on a
   process-safety limit, that limit *is* the carrying capacity and every
   dynamic measured is an artifact of it.
3. **The culture stack cannot be measured in such a world.** Costly
   signalling, object manipulation, and social learning all cost energy that
   nobody has. Running Phases 10 to 12 here would produce nulls caused by
   starvation rather than by anything about culture, and an underpowered
   null is worse than a negative result because the two cannot be
   distinguished.

## Why This Executes Early

The former Phase 13 sat after the culture stack, on ADR-0017's argument that
physiology changes the selection landscape and should therefore land after
the scale-sensitive experiments. ADR-0025 reverses that for the demographic
half. The short form: **that argument assumed the culture results would
otherwise be valid, and in a world with 99.9 percent starvation mortality
they would not be.** Better to run those campaigns once, correctly, than
twice.

This slice needs nothing that does not already exist. Allometry uses the
body-scale gene, senescence uses age, the life-history tradeoff uses the
reproduction-investment gene, and thermal preference finally has the
temperature field the Phase 6 climate slice built. Its only prerequisite is
Phase 7.

## Scope

- **Allometric metabolism**: basal cost scaling as a configured power of
  mass, replacing the linear body-scale multiplier.
- **Thermoregulation**: thermal preference becomes live against the Phase 6
  temperature field, so the gene inherited-but-inert since Phase 2 finally
  does something and `C_thermal` stops being a documented placeholder.
- **Senescence**: an age-dependent mortality hazard replacing the hard
  `max_age_ticks` cutoff, with lifespan an evolvable consequence.
- **Extrinsic mortality**: a configured non-food hazard, so mortality is not
  dominated by starvation. This is the mechanism that lets a population sit
  *below* its food ceiling, which is the only way per-capita surplus exists.
- **Juvenile mortality and a maturation constraint**: a scalar
  pre-reproductive penalty. Not developmental growth, which needs morphology
  and is 13b.
- **Life-history tradeoff**: per-offspring investment and offspring number
  as an evolvable axis.
- **Death-cause accounting** as a first-class reported distribution.

## Non-Goals

- **No developmental ontogeny.** Growth of a module body is Phase 13b and
  needs morphology.
- **No sexual selection or mate choice.** Needs perception; Phase 13b.
- **No disease.** Optional slice, better with contact structure; Phase 13b.
- No claim that any parameter corresponds to a real organism.
- No population cap used as an ecological mechanism. `max_entities` remains
  a process-safety guard and this phase's whole point is that ecology should
  bind first.

## Prerequisites

- Phase 7 (contest), for damage as a non-food mortality source.
- Phase 6 climate (**done**), for the temperature field thermoregulation
  needs.

## Determinism Notes

- New stream `Mortality` (16), already reserved, for age-dependent and
  extrinsic hazard draws.
- Accumulated hazard and developmental progress are fixed point, per Rule 7:
  both integrate over a lifetime.
- Checksum section `lifesim-physiology-state-v1`, present only when enabled.
- Hazard draws are keyed on the organism, one-sided, so no pair key applies.

## Acceptance Criteria

**Primary endpoint: C13a.1.** Acceptance is conjunctive; secondary criteria
do not rescue a failed primary. The world is the replicate; per-organism
quantities aggregate to a world-level statistic before analysis. Seed floor
30 independent worlds, with pilot-driven power analysis setting the final
number.

Conditions, matched on seeds and run length:

- **A**: demography enabled.
- **B**: Phase 7 energetics, the current model. The baseline.
- **M-high / M-low**: an extrinsic-mortality sweep applied to A.
- **K-head**: `max_entities` raised well above the ecological equilibrium,
  applied to both A and B, so the guard cannot bind.

Criteria:

- [ ] **C13a.1 The population stops being starvation-dominated (primary).**
      Under A with `K-head`, the fraction of deaths attributable to
      starvation falls below a prespecified threshold, and the death-cause
      distribution is mixed across starvation, age, damage, and extrinsic
      hazard, in at least 20 of 30 worlds. Under B it remains
      starvation-dominated. This is the criterion that determines whether
      any later phase is measurable at all.
- [ ] **C13a.2 Population sits below carrying capacity.** Under A with
      `K-head`, realized population divided by environmental carrying
      capacity is sustained below 1 by a prespecified margin, rather than
      tracking it exactly. Report per-capita energy above subsistence as the
      surplus measure. Under B the ratio pins at or near 1.
- [ ] **C13a.3 Ecology binds, not the guard.** Under `K-head`, population
      equilibrium is strictly below `max_entities` in at least 25 of 30
      worlds. A run whose population touches the ceiling is excluded from
      analysis and reported as excluded, because its dynamics are an
      artifact.
- [ ] **C13a.4 Allometry is what it claims.** Measured basal metabolic rate
      against body mass fits a power law whose exponent matches the
      configured exponent within stated tolerance. Verifies the
      implementation before anything downstream is interpretable.
- [ ] **C13a.5 Senescence responds to extrinsic mortality.** Under `M-low`,
      evolved median lifespan is higher than under `M-high`, in at least 20
      of 30 worlds. A directional, falsifiable prediction from evolutionary
      theory that the model was not tuned to produce, and the strongest
      realism test available at this stage.
- [ ] **C13a.6 Life-history tradeoff emerges.** A negative correlation
      between per-offspring investment and offspring number appears across
      the evolved population in at least 20 of 30 worlds. Not authored
      anywhere; it must fall out of the energy budget.
- [ ] **C13a.7 Thermal preference becomes load-bearing.** The distribution
      of the thermal-preference gene correlates with the thermal
      distribution of the cells its carriers occupy, in at least 20 of 30
      worlds. Under B the gene is inert and the correlation is absent, which
      is the control confirming the measurement.
- [ ] **C13a.8 Exactness and determinism.** Ledger exact to the milli-unit
      over a 10^6-tick run with thermoregulation, hazard, and allometry
      active; clean-process fixture replay; storage-permutation equality;
      demography-disabled configs reproduce the Phase 7 fixture exactly.

## Test Plan

- Unit: allometric cost at mass boundaries; thermal cost at preference
  extremes; hazard function monotonicity and its bound in [0, 1]; juvenile
  penalty at the maturation boundary.
- Property: no state leaves bounds; hazard probability stays in range under
  adversarial configs.
- Statistical: C13a.4 through C13a.7 as automated tests with recorded
  tolerances, seeds, and sample sizes. A statistical criterion checked by a
  human reading a chart is not a test.
- Determinism: clean-process fixture; storage permutation; disabled-section
  equality against Phase 7.
- Long run: 10^6 ticks with exact ledgers and a stable death-cause mix.

## Benchmark Impact

Adds per-organism per-tick work in `environment` (thermal field sampling),
`apply` (thermoregulation), and `lifecycle` (hazard draws). Record each
separately.

Record explicitly: the per-organism cost delta against Phase 7, and the
resulting change in ticks per second per world, since that number is the
honest price of moving this phase earlier and it now applies to every
campaign that follows rather than only to the last few.

Also record the population equilibrium reached under `K-head`, because that
number determines the achievable population for every later phase and is
currently unknown.

## Documentation Updates

`docs/04-simulation-model.md` (energy accounting, `C_thermal` becomes live),
`docs/06-organism-model.md` (lifecycle, senescence), `docs/26-biological-realism-policy.md`,
`specifications/entity-component-model.md`, `specifications/event-schema.md`,
`specifications/metrics-schema.md`, decision log, ADR-0017 amendment,
ADR-0025.

## Risks

| Risk | Mitigation |
|---|---|
| **Extrinsic mortality is tuned until surplus appears**, which would author the answer to the surplus question | Mortality regime is a swept environmental condition with a stated range, reported as a curve, not a value chosen for its result. ADR-0018's naming test applies: "a mortality regime" is an environment, "conditions producing surplus" is an outcome |
| Raising `max_entities` blows the memory or tick budget | The `K-head` value comes from the Phase 5 throughput measurement and this phase's own benchmark, not from optimism. If ecology cannot bind below the affordable ceiling, that is a reportable finding about the world size |
| Per-organism cost now applies to every later phase rather than only the last | Real and accepted. It is the price of measuring the culture stack in a world where measurement is possible, and ADR-0025 records it |
| Senescence plus extrinsic mortality collapses populations | C13a.2 and the sweep detect it; the hazard model is config and swept before any campaign |
| Prior Phase 7 results do not transfer across this phase | True and much cheaper than it would have been after Phase 12. Phase 7 is one phase of results, not four |

## Rollback

Every item is an independently config-gated section: allometry,
thermoregulation, senescence, extrinsic mortality, juvenile penalty, life
history. Any subset can be disabled. All disabled reproduces the Phase 7
fixture exactly.
