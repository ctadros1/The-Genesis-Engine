# Phase 14: Ontogeny And Sexual Selection

**Executes after Phase 13 (social channel).**
`docs/19-implementation-roadmap.md` carries the authoritative execution
order.

Status: planned, not started. Policy version `lifesim-physiology-v2`.
Split from the former Phase 13 by ADR-0025; the demographic half executes
early as `planning/phase-8-demography-and-life-history.md`.

## Problem

Phase 8 delivered the demographic half of physiology: allometry,
thermoregulation, senescence, extrinsic mortality, and the life-history
tradeoff. What remains are the parts that genuinely cannot move earlier
because they depend on machinery built in Phases 10 through 13.

- **Ontogeny** as developmental growth is growth *of a module body*, which
  requires the morphology representation of Phase 10. Phase 8's juvenile
  penalty is a scalar constraint, not development.
- **Sexual selection** requires mate choice conditioned on *perceived*
  phenotype, which requires the perception channels of Phase 13.
- **Disease** is contact-structured and is more meaningful once social
  contact structure exists.

## Why This Executes Late

Unlike the demographic half, these three have hard upstream dependencies
rather than a scheduling preference. ADR-0025's reordering argument does not
apply to them: they are late because they cannot be early.

The cost recorded in ADR-0017 still holds for this slice. It changes the
selection landscape, so campaigns whose results are to become standing
findings are re-run under `lifesim-physiology-v2`. That cost is now much
smaller than it was, because the demographic half already landed and the
culture campaigns already ran in a regulated world.

## Scope

- Ontogeny: a growth trajectory from birth size to adult size, with
  juveniles physically constrained (lower speed, lower carry capacity,
  smaller sensor range) and growth consuming energy.
- Sexual selection: mate choice conditioned on perceived phenotype through
  the Phase 13 perception channels, replacing compatibility-threshold-only
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

- **Phase 8** (demography), which is the hard prerequisite: ontogeny
  extends a maturation model that 8 establishes.
- **Phase 10** (morphology), for developmental growth of a module body.
- **Phase 13** (social), for the perception channels mate choice reads.

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

**Primary endpoint: C14.2.** Acceptance is conjunctive. The world is the
replicate; per-organism quantities aggregate to a world-level statistic.
Seed floor 30, and 50 for C14.2 because mate-choice outcomes are
fixation-driven.

Conditions, matched on seeds and run length:

- **A**: ontogeny and sexual selection enabled.
- **B**: Phase 8 demography only, the baseline.
- **P-scramble**: mate choice enabled but perceived phenotype scrambled
  before it reaches the chooser, preserving the cost and the act while
  destroying the information. This is the control that separates "choice
  happens" from "choice is informed", and without it C14.2 is
  uninterpretable.

Criteria:

- [ ] **C14.1 Ontogeny is real.** Juveniles are measurably constrained:
      realized speed, carry capacity, and sensor range differ from adults by
      the configured amount, and juvenile mortality exceeds adult mortality.
      Growth energy flows through the ledger exactly. Distinct from Phase 8's scalar juvenile penalty: here the constraint is a consequence of
      an incompletely grown body.
- [ ] **C14.2 Mate choice is informed, not merely expressed (primary).**
      Under A, realized pairings are non-random with respect to perceived
      phenotype, and the assortment disappears under `P-scramble`, in at
      least 30 of 50 worlds. An A-versus-B difference without an
      A-versus-`P-scramble` difference is not sexual selection and is
      reported as a negative result.
- [ ] **C14.3 Costly display, or a measured null.** Report whether any
      trait under mate choice becomes exaggerated beyond its survival
      optimum, with the survival cost measured directly. **Expected to
      return null** and stated so in advance; a positive result here is the
      closest thing to ornament this project can produce and would be a
      notable finding requiring its own replication.
- [ ] **C14.4 Disease, if enabled.** Contact-structured spread produces an
      epidemic curve whose shape depends on contact rate in the direction
      the transmission model predicts, and disease load never produces
      energy from nothing.
- [ ] **C14.5 Exactness and determinism.** Ledger exact to the milli-unit
      over a 10^6-tick run with growth and, if enabled, disease;
      clean-process fixture replay; storage-permutation equality;
      disabled configs reproduce the Phase 13 fixture exactly.

## Test Plan

- Unit: allometric cost at mass boundaries; thermal cost at preference
  extremes; hazard function monotonicity; growth trajectory endpoints.
- Property: no state leaves bounds; hazard probability stays in [0, 1];
  disease load bounded.
- Statistical: C14.1 through C14.4 as automated tests with recorded
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
| The realism criteria fail and it is unclear whether the mechanism or the world is wrong | Each criterion names the specific mechanism it tests, and C14.1 gates the rest: if ontogeny is not measurably real, nothing downstream is interpretable, because mate choice on a developmental phenotype presupposes that development happened. Allometry itself is verified upstream by Phase 8's C8.4 |
| Disease destabilizes populations | Optional slice, own condition, dropped without ceremony if it does not land cleanly |
| Sexual selection creates runaway ornamentation that collapses populations | A real and interesting possible outcome rather than purely a risk; monitored through population and lifespan metrics, and reported either way |

## Rollback

Every item is an independently config-gated section: allometry,
thermoregulation, ontogeny, senescence, sexual selection, disease. Any
subset can be disabled. All disabled reproduces the Phase 12 fixture
exactly.
