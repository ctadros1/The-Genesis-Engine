# Phase 13: Physiology, Development, And Life History

Status: planned, not started. Policy version `lifesim-physiology-v1`.
Policy document: `docs/26-biological-realism-policy.md`.

## Problem

The organism model is energetically and developmentally thin. Metabolism is
a linear basal cost with a body-scale multiplier. Organisms are born
adult-shaped and mature by crossing an age threshold. Death by age is a hard
`max_age_ticks` cutoff rather than a hazard. There is no thermoregulation
despite thermal preference being an inherited but inert gene since Phase 2,
no sexual selection beyond a compatibility threshold, and no disease.

The standing biological realism policy says: where a mechanism has a
well-established biological form and an abstract shortcut, prefer the
biological form unless the shortcut is justified with the cost that made it
necessary. These are the remaining shortcuts with no such justification.

## Why This Phase Is Placed Here

Two arguments pull in opposite directions and the resolution is recorded
rather than glossed.

**For earlier:** physiology changes the entire selection landscape, so
introducing it late means every prior cross-condition result was measured in
a different world. Results do not transfer across it.

**For later:** every realism increment multiplies per-organism cost, which
is exactly what fights the scale and generation-count requirement that the
culture stack needs. Paying that cost through Phases 7 to 12 would reduce
the number of generations every one of those experiments could reach.

The resolution: genetics realism, which is unavoidable because the genome is
being rewritten anyway, happens in Phase 8. Doing it twice would be strictly
worse. Physiology realism, which is separable, happens here, after the
scale-sensitive culture experiments have run.

The cost is accepted and stated: **results from Phases 7 to 12 do not
transfer across this phase.** That is not a new problem. Every phase starts
a new replay lineage by design, and the versioned-policy discipline exists
precisely so that two incompatible rule sets are never called the same
experiment. What is new is the size of the change, so the campaigns that
matter most should be re-run under `lifesim-physiology-v1` before their
results are treated as the project's standing findings.

## Scope

- Allometric metabolism: basal cost scaling as a configured power of mass,
  replacing the linear multiplier.
- Thermoregulation: thermal preference becomes live, so the gene inherited
  and inert since Phase 2 finally does something, and `C_thermal` in
  `docs/04-simulation-model.md` stops being a documented placeholder.
- Ontogeny: a growth trajectory from birth size to adult size, with
  juveniles physically constrained (lower speed, lower carry capacity,
  smaller sensor range) and growth consuming energy.
- Senescence: age-dependent mortality hazard replacing the hard age cutoff,
  with lifespan an evolvable consequence rather than a config constant.
- Sexual selection: mate choice conditioned on perceived phenotype through
  the Phase 11 perception channels, replacing compatibility-threshold-only
  pairing.
- Disease as an optional slice: a transmissible load with contact-structured
  spread, run as its own condition and only if the earlier items land
  cleanly.

## Non-Goals

- No molecular biology, no chemistry, no explicit gene regulatory network
  dynamics.
- No continuous morphology or physical body simulation.
- No claim that any parameter value corresponds to a real organism.
- No real-world ecological prediction. `docs/02-scope-and-non-goals.md`
  keeps this permanently out of scope and this phase does not touch it.
- Disease is explicitly optional and is dropped without ceremony if the
  budget is tight.

## Prerequisites

- Phase 12, so the campaigns whose results this phase invalidates have
  already been run.
- Phase 11 perception channels for mate choice.

## Determinism Notes

- New streams: `Development` (15), `Mortality` (16).
- Developmental progress, accumulated hazard, and disease load are fixed
  point (Rule 7); all three accumulate over a lifetime.
- Disease transmission draws use `lifesim-pairkey-v1`, so an infection event
  does not depend on which of the pair is visited first.
- Checksum section `lifesim-physiology-state-v1`.

## Acceptance Criteria

The criteria here are unusual in this plan: several are checks that the
model reproduces a textbook result it was **not** tuned to produce. That is
the operational definition of realism used throughout
`docs/26-biological-realism-policy.md`. A mechanism that cannot be checked
against a known result is decoration.

Conditions, matched on seeds (12) and run length:

- **A**: physiology enabled.
- **B**: Phase 12 abstract energetics.
- **M-high / M-low**: an extrinsic-mortality sweep applied to A, used only
  for C13.3.

Criteria:

- [ ] **C13.1 Allometry is what it claims.** Measured basal metabolic rate
      against body mass across the living population fits a power law whose
      exponent matches the configured exponent within stated tolerance. This
      verifies the implementation does what the model says, which is the
      minimum bar before any of the following mean anything.
- [ ] **C13.2 Life-history tradeoff emerges.** Under A, a negative
      correlation emerges between per-offspring investment and offspring
      number across the evolved population, in at least 8 of 12 seeds. This
      is the classic tradeoff axis and it is not authored anywhere: it must
      fall out of the energy budget. Under B, the correlation is absent or
      weaker.
- [ ] **C13.3 Senescence responds to extrinsic mortality.** Under `M-low`,
      evolved median lifespan is higher than under `M-high`, in at least 8
      of 12 seeds. This is a specific, directional, falsifiable prediction
      from evolutionary theory that the model was not tuned to produce, and
      it is the strongest realism test in the entire plan. A failure here is
      genuinely informative: it means either the hazard model, the energy
      budget, or the mutational access to lifespan genes is wrong.
- [ ] **C13.4 Thermal preference becomes load-bearing.** Under A, the
      distribution of the thermal preference gene correlates with the
      thermal distribution of the cells its carriers occupy, in at least 8
      of 12 seeds. Under B the gene is inert and the correlation is absent,
      which is the control confirming the measurement.
- [ ] **C13.5 Ontogeny is real.** Juveniles are measurably constrained:
      their realized speed, carry capacity, and sensor range differ from
      adults by the configured amount, and juvenile mortality exceeds adult
      mortality. Growth energy flows through the ledger exactly.
- [ ] **C13.6 Exactness and determinism.** Ledger exact to the milli-unit
      over a 10^6-tick run with growth, thermoregulation, hazard, and (if
      enabled) disease; clean-process fixture replay; storage-permutation
      equality; physiology-disabled configs reproduce the Phase 12 fixture
      exactly.
- [ ] **C13.7 Disease, if enabled.** Contact-structured spread produces an
      epidemic curve whose shape depends on contact rate in the direction
      the transmission model predicts, and disease load never produces
      energy from nothing.

## Test Plan

- Unit: allometric cost at mass boundaries; thermal cost at preference
  extremes; hazard function monotonicity; growth trajectory endpoints.
- Property: no state leaves bounds; hazard probability stays in [0, 1];
  disease load bounded.
- Statistical: C13.1 through C13.4 as automated tests with recorded
  tolerances, seeds, and sample sizes, not as manual analyses. A statistical
  acceptance criterion that is checked by a human reading a chart is not a
  test.
- Determinism: clean-process fixture; storage permutation; pair-key symmetry
  for disease transmission.
- Long run: exact ledgers with all physiology terms active.
- Disabled-section equality against the Phase 12 fixture.

## Benchmark Impact

Physiology adds per-organism per-tick work in `environment` (thermal field
sampling), `apply` (growth, thermoregulation), and `lifecycle` (hazard
draws). Record each separately, because the phase's whole tension is that
realism costs throughput.

Record explicitly: the per-organism cost delta against Phase 12, and the
resulting change in ticks per second per world and therefore in generations
reachable per unit of compute. That number is the honest price of this phase
and it belongs in the record, not in a footnote.

Benchmark schema 8.

## Documentation Updates

`docs/04-simulation-model.md` (energy accounting, `C_thermal`),
`docs/05-world-model.md` (temperature field becomes live),
`docs/06-organism-model.md` (lifecycle, ontogeny, senescence),
`docs/08-genetics-and-evolution.md` (sexual selection),
`docs/26-biological-realism-policy.md` (table status),
`specifications/entity-component-model.md`,
`specifications/event-schema.md`, `specifications/metrics-schema.md`,
decision log, ADR-0017.

## Risks

| Risk | Mitigation |
|---|---|
| Throughput regression reduces reachable generations below what the earlier phases needed | Measured explicitly as a headline number; if severe, individual physiology items are independently config-gated and can be disabled per campaign |
| Prior results do not transfer and the project loses its accumulated findings | Stated in advance; the campaigns that matter are re-run under the new policy before their results become standing findings |
| The realism criteria fail and it is unclear whether the mechanism or the world is wrong | Each criterion names the specific mechanism it tests, and C13.1 gates the rest: if allometry does not verify, nothing downstream is interpretable |
| Disease destabilizes populations | Optional slice, own condition, dropped without ceremony if it does not land cleanly |
| Sexual selection creates runaway ornamentation that collapses populations | A real and interesting possible outcome rather than purely a risk; monitored through population and lifespan metrics, and reported either way |

## Rollback

Every item is an independently config-gated section: allometry,
thermoregulation, ontogeny, senescence, sexual selection, disease. Any
subset can be disabled. All disabled reproduces the Phase 12 fixture
exactly.
