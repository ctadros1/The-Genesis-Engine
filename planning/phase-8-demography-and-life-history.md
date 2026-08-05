# Phase 8: Demography And Life History

**Executes after Phase 7 (contest), before Phase 9 (genome successor).**
`docs/19-implementation-roadmap.md` carries the authoritative execution
order.

Status: **implemented; primary endpoint met, three secondary criteria
unmet and recorded as unmet. 2026-08-05.** Policy version
`lifesim-demography-v1`. Decisions D-063, D-064, D-065.
Split from the former Phase 13 by ADR-0025; the remainder is
`planning/phase-14-ontogeny-and-sexual-selection.md`.

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
   nobody has. Running Phases 11 to 13 here would produce nulls caused by
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
  and is 14.
- **Life-history tradeoff**: per-offspring investment and offspring number
  as an evolvable axis.
- **Death-cause accounting** as a first-class reported distribution.

## Non-Goals

- **No developmental ontogeny.** Growth of a module body is Phase 14 and
  needs morphology.
- **No sexual selection or mate choice.** Needs perception; Phase 14.
- **No disease.** Optional slice, better with contact structure; Phase 14.
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

**Primary endpoint: C8.1.** Acceptance is conjunctive; secondary criteria
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

- [x] **C8.1 The population stops being starvation-dominated (primary). MET**, 26/30 against a bar of 20; baseline 0/30.
      Under A with `K-head`, the fraction of deaths attributable to
      starvation falls below a prespecified threshold, and the death-cause
      distribution is mixed across starvation, age, damage, and extrinsic
      hazard, in at least 20 of 30 worlds. Under B it remains
      starvation-dominated. This is the criterion that determines whether
      any later phase is measurable at all.
- [x] **C8.2 Population sits below carrying capacity. MET**: biomass saturation 0.990 under A against 0.741 under B. Under A with
      `K-head`, realized population divided by environmental carrying
      capacity is sustained below 1 by a prespecified margin, rather than
      tracking it exactly. Report per-capita energy above subsistence as the
      surplus measure. Under B the ratio pins at or near 1.
- [x] **C8.3 Ecology binds, not the guard. MET**: all 120 worlds below 90% of `max_entities`. Under `K-head`, population
      equilibrium is strictly below `max_entities` in at least 25 of 30
      worlds. A run whose population touches the ceiling is excluded from
      analysis and reported as excluded, because its dynamics are an
      artifact.
- [x] **C8.4 Allometry is what it claims. MET** at every configurable exponent (`phase8_demography.rs`). Measured basal metabolic rate
      against body mass fits a power law whose exponent matches the
      configured exponent within stated tolerance. Verifies the
      implementation before anything downstream is interpretable.
- [ ] **C8.5 Senescence responds to extrinsic mortality. UNMET**: 16/30 against a bar of 20. A measured null. Under `M-low`,
      evolved median lifespan is higher than under `M-high`, in at least 20
      of 30 worlds. A directional, falsifiable prediction from evolutionary
      theory that the model was not tuned to produce, and the strongest
      realism test available at this stage.
- [ ] **C8.6 Life-history tradeoff emerges. UNMET**: 13/30 under A. A negative correlation
      between per-offspring investment and offspring number appears across
      the evolved population in at least 20 of 30 worlds. Not authored
      anywhere; it must fall out of the energy budget.
- [ ] **C8.7 Thermal preference becomes load-bearing. UNMET**, and its control failed: the correlation is present under B too. The distribution
      of the thermal-preference gene correlates with the thermal
      distribution of the cells its carriers occupy, in at least 20 of 30
      worlds. Under B the gene is inert and the correlation is absent, which
      is the control confirming the measurement.
- [x] **C8.8 Exactness and determinism. MET** (`phase8_demography.rs`, both fixtures reproduce). Ledger exact to the milli-unit
      over a 10^6-tick run with thermoregulation, hazard, and allometry
      active; clean-process fixture replay; storage-permutation equality;
      demography-disabled configs reproduce the Phase 7 fixture exactly.

## Test Plan

- Unit: allometric cost at mass boundaries; thermal cost at preference
  extremes; hazard function monotonicity and its bound in [0, 1]; juvenile
  penalty at the maturation boundary.
- Property: no state leaves bounds; hazard probability stays in range under
  adversarial configs.
- Statistical: C8.4 through C8.7 as automated tests with recorded
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
| Senescence plus extrinsic mortality collapses populations | C8.2 and the sweep detect it; the hazard model is config and swept before any campaign |
| Prior Phase 7 results do not transfer across this phase | True and much cheaper than it would have been after Phase 13. Phase 7 is one phase of results, not four |

## Rollback

Every item is an independently config-gated section: allometry,
thermoregulation, senescence, extrinsic mortality, juvenile penalty, life
history. Any subset can be disabled. All disabled reproduces the Phase 7
fixture exactly.


## The Mortality Regime, Swept Before The Campaign

The plan's stated mitigation - "mortality regime is a swept environmental
condition with a stated range, reported as a curve, not a value chosen for
its result" - ran on seeds 1..8, **disjoint** from the confirmatory set.
`experiments/phase8-extrinsic-sweep.campaign`.

| `extrinsic_hazard_q16_per_s` | Median population | Starvation | Senescence | Extrinsic | Causes > 5% | Extinct | Biomass saturation |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 13 (default) | 2,436 | 88.1% | 5.4% | 6.5% | 3 | 0/8 | 0.864 |
| 65 | 290 | 55.2% | 6.1% | 38.8% | 3 | 0/8 | 0.988 |
| 200 | 0 | 26.6% | 1.2% | 72.3% | 2 | **8/8** | 1.000 |
| 655 | 0 | 12.3% | 0% | 87.7% | 2 | **8/8** | 1.000 |
| 2000 | 0 | 3.3% | 0% | 96.7% | 1 | **8/8** | 1.000 |

**The usable window is narrow and the curve is the finding.** Below about
65 the world stays starvation-dominated; at 200 and above every world dies.
The confirmatory campaign takes 65 for condition A and brackets it at 30
and 100 for the M-low / M-high contrast C8.5 needs.

A separate carrying-capacity sweep came first, because at the 30,000
default with climate enabled three of eight worlds went extinct **before any
mortality was added**. A world that cannot hold a population is not one in
which demography can be measured, so the campaign runs at 120,000.

### Prespecified thresholds

- **C8.1**: starvation below **700 of 1000** deaths, **and** at least
  **three causes each above 5%**, in at least 20 of 30 worlds. Set against
  the Phase 2 pathology of 999/1000 rather than against the sweep: the bar
  is "starvation is no longer the overwhelming majority", and the full
  curve is reported so the sensitivity is visible.
- **C8.3**: population below **90%** of `max_entities`.
- Seeds: the first 30 from 2001 that world generation accepts. Twenty of
  2001..2120 are refused for having no `Arid` cells. The rule is
  outcome-blind - it reads terrain only, before any condition applies - and
  preflight proves the declared design is the one that ran.

## Results

`experiments/phase8-c81-confirmatory.campaign`, 4 conditions x 30 seeds,
20,000 ticks, 120 worlds, campaign hash `0x1965d347a52d52a8`. Report:
`experiments/results/phase8-c81-confirmatory-demography.txt`.

### The phase's purpose is achieved

| | B (Phase 7 energetics) | A (demography) |
|---|---:|---:|
| Median starvation share | **1000/1000** | **494/1000** |
| Median causes above 5% | 1 | 3 |
| Worlds meeting C8.1 | **0/30** | **26/30** |
| Median biomass saturation | 0.741 | 0.990 |
| Median per-capita energy | 9,920 | 10,774 |
| Median population | 4,784 | 304 |

**C8.1, the primary endpoint, is met**: 26 of 30 worlds against a bar of 20,
while the baseline scores 0 of 30 and stays at a starvation share of exactly
1000/1000. **C8.2 is met**: under A the food field sits at 99% of capacity
while B strips it to 74%, which is what a population held below its food
ceiling looks like, and per-capita energy is higher. **C8.3 is met**: every
one of the 120 worlds is below 90% of the guard, the largest reaching 6,712
against a 40,000 ceiling. The `max_entities` artifact the whole phase
existed to remove is gone.

### Three secondary criteria are unmet, and are recorded as unmet

- **C8.5 (senescence responds to extrinsic mortality): not met.** Median
  completed lifespan is 1,084 ticks under M-low against 1,052 under M-high -
  the predicted direction, but only **16 of 30 worlds**, against a bar of 20.
  Two things weaken the test and both are reported rather than corrected
  after the fact. Maximum observed age *does* order as predicted across all
  three regimes (15,677 / 14,170 / 12,192 ticks for M-low / A / M-high),
  so the tail moves even though the median does not; and M-high sits near
  the edge of viability, with six of its thirty worlds below ten organisms,
  so its lifespan estimate rests on a median of 237 completed lifespans
  against M-low's 1,979. **This is a measured null on a directional
  prediction from evolutionary theory that the model was not tuned to
  produce**, which is what makes it worth having.
- **C8.6 (life-history tradeoff): not met.** Under A the correlation between
  a parent's mean per-offspring investment and its lifetime offspring count
  is **13 of 30 worlds negative**, median rho 0. Under B it is 20 of 30,
  median rho -6 - at the bar, but B is the control, not the treatment. The
  tradeoff does not fall out of the energy budget here.
- **C8.7 (thermal preference becomes load-bearing): not met, and the
  control is why.** Under A the correlation between an organism's preferred
  temperature and the temperature it stands in has median rho 164 in 18 of
  30 worlds. But **the criterion's control fails**: under B, where the gene
  is inert, the correlation is *not* absent - median rho 38, positive in 21
  of 30 worlds - because biome capacity correlates with temperature, so
  organisms follow food into thermally non-random cells whether or not they
  can feel temperature. Paired against its own control A exceeds B by a
  median of +134 milli in **19 of 30** worlds, one short of the bar. The
  honest reading is that thermoregulation probably is doing something and
  this design cannot demonstrate it.

Acceptance is conjunctive, so **Phase 8 is implemented but not fully
accepted**. What it was scheduled early to deliver - a world where
mortality is mixed, population sits below carrying capacity, and ecology
rather than a memory guard sets the ceiling - is delivered and measured.
What it additionally hoped to show about biological realism is not.
