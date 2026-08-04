# Phase 7: Territory, Contest, And Damage

Status: planned, not started. Policy version `contest-behavior-v1`.

## Why This Is Second, Not Sixth

The ordering suggested when the goal changed put territory and conflict
after the whole culture stack, on the grounds that it is the easiest of the
behavioral goals and does not depend on the others. Both are true. Three
further arguments move it earlier, and they are the reason this plan
deviates from the suggested order.

**It needs no schema change.** Topology 1 was designed with these channels
already reserved. Input 2 (health) reads a neutral 1.0, input 10 (local
threat estimate) and input 16 (recent damage fraction) are documented
neutral zeros, and output 4 (attack request) is a documented no-op. Phase 7
makes reserved channels live. It is the one behavioral goal implementable
inside the existing frozen topology, which means it can be delivered before
the expensive genome successor and can validate the Phase 5 experiment
harness on a real behavioral question.

**It creates the information that makes a signal worth having.** A signal
channel evolves into noise unless something worth signalling about exists.
Threat is spatially structured, time-varying, and fitness-relevant, and
alarm signalling is the best-attested natural signalling system precisely
because threat information has high value to the receiver. Running Phase 11
in a world with contest is running it in the condition most likely to
produce a result; running it in a world without contest tests it in a
weaker one.

**It de-risks the programme early.** It is the highest-prior-probability
positive result in the plan, and it exercises the multi-seed ablation
machinery on a question where a null result would itself be informative
about the harness rather than about the science.

The counterargument is real and recorded: it burns a lineage break and a
behavior-policy version before the genome successor lands, so Phase 8 will
break lineage again. Against that, every phase breaks lineage anyway, which
is what versioned policies are for.

## Problem

Organisms cannot harm each other, cannot perceive threat, and have no
mechanism by which local resource contention becomes structured conflict.
Health exists in the documented model and not in the kernel.

## Scope

- Health as real per-organism state, with damage, healing, and death by
  health depletion joining the existing death causes.
- The attack action wired to the existing reserved output channel, with
  validated intents resolved by the standard contention policy.
- Threat and recent-damage sensing wired to the existing reserved input
  channels.
- Carcasses as a resource, closing the gap `docs/06-organism-model.md` has
  documented since Phase 1.
- A kinship input derived from genomes, so grouping can become kin-biased
  without any authored group concept.
- Resource contention sharpened: configured local depletion that makes
  patches worth defending.

## Non-Goals

- No group, tribe, faction, alliance, or territory object. There is no
  authored notion of a group anywhere. Grouping, if it occurs, is a spatial
  and genetic statistic measured after the fact.
- No war, raid, or conflict mechanic.
- No genome schema change. Topology 1 is unchanged.
- No claim that observed violence is analogous to any human phenomenon.

## Prerequisites

- Phase 5, complete. The acceptance criteria below are multi-seed claims and
  are not measurable without it.

## Deliverables

- Health, damage, healing, and death-by-damage in the kernel, with the
  energy and health ledgers exact.
- Carcass objects with decay and consumption, energy-neutral against the
  source organism's recorded transferable energy.
- Contest resolution using `lifesim-pairkey-v1` for any stochastic element.
- Event schema extension: `Damage`, `DeathByDamage`, `CarcassCreated`,
  `CarcassConsumed`.
- Ablation conditions in the experiment harness.

## Determinism Notes

- New RNG stream: `Contest` (value 7). Draws for the contest tie lottery,
  damage variance, and retreat resolution, all keyed on the canonical pair
  key so the outcome does not depend on which combatant is visited first.
- Health and accumulated damage are fixed point, per Rule 7.
- Checksum section `lifesim-contest-state-v1`, present only when the contest
  section is enabled.
- Save-state version increments; the contest section is optional and absent
  from earlier saves.

## Acceptance Criteria

Conditions, all matched on seed set (12 seeds), config, and run length:

- **A**: contest enabled.
- **B**: contest disabled; attack remains a no-op. The Phase 2 baseline.
- **B'**: contest enabled but kinship-blind; the kinship input channel reads
  a neutral zero, and genetic compatibility gating for pairing is unchanged.
  This is the control that separates "violence happens" from "violence is
  kin-structured".

Criteria:

- [ ] **C7.1 Kin-biased spatial aggregation.** Under A, mean genetic
      distance between organisms within radius R of each other is lower than
      between randomly paired organisms by at least the stated effect size,
      sustained across at least 50,000 ticks, in at least 8 of 12 seeds.
      Under B' the effect is absent or significantly smaller. Reported with
      the effect size, the seed-by-seed result, and the random-pairing
      baseline.
- [ ] **C7.2 Directed rather than indiscriminate violence.** Under A, the
      rate of damage events between organisms in different similarity
      clusters exceeds the rate within clusters by at least factor f in at
      least 8 of 12 seeds. Under B' the within and between rates do not
      differ beyond the stated tolerance. Cluster labels are used **only** as
      an offline analysis grouping for this statistic and never enter the
      simulation, per ADR-0016.
- [ ] **C7.3 Contest is not merely lethal noise.** Under A, median
      population and median lifespan across the seed set remain within the
      stated tolerance of B, or the difference is explained by a reported
      mechanism. A conflict model that simply collapses populations has not
      produced conflict; it has produced a mortality term.
- [ ] **C7.4 Energy and health accounting is exact.** The ledger stays exact
      to the milli-unit across damage, healing, death, carcass creation, and
      carcass consumption over a 10^6-tick run. Carcass energy never exceeds
      the source organism's recorded remaining transferable energy.
- [ ] **C7.5 Determinism.** Clean-process replay of the Phase 7 fixture;
      order-independence under storage permutation; contest-disabled configs
      reproduce `0xff9dfcff5dffbf42` exactly.

If C7.1 and C7.2 both fail across the full seed set, the honest conclusion
is that this world's scarcity structure does not produce kin-biased
conflict, and the reported result is that negative finding plus the measured
statistics. It is not a reason to weaken the thresholds after the fact;
thresholds are recorded before the runs.

## Test Plan

- Unit: damage formula bounds including zero and saturating cases; healing
  bounded by available energy; death by health depletion is terminal and
  idempotent; no action after death.
- Property: health never leaves bounds; carcass energy never exceeds source.
- Determinism: pair-key symmetry (swapping combatant visit order gives an
  identical outcome); storage permutation equality; clean-process fixture.
- Integration: contested carcass consumption between multiple organisms
  resolves deterministically; simultaneous mutual attack resolves once.
- Long run: 10^6 ticks with exact ledgers and bounded populations.
- Disabled-section equality against the Phase 2 fixture.

## Benchmark Impact

Contest adds work to `apply` (intent resolution against neighbours) and to
`sense` (threat estimation). Record the per-phase delta at both tiers
against the Phase 5 record. Carcasses add entities, so record the entity-count
effect separately from the per-organism cost. Benchmark schema 3.

## Documentation Updates

`docs/04-simulation-model.md` (combat and carcass sections move from
proposed to implemented), `docs/06-organism-model.md`,
`docs/07-neural-network-design.md` (reserved channels become live),
`specifications/event-schema.md`, `specifications/entity-component-model.md`
(Carcass component), `specifications/metrics-schema.md`,
`specifications/simulation-tick.md`, decision log, risk register.

## Risks

| Risk | Mitigation |
|---|---|
| Contest collapses populations and produces an uninformative extinction | C7.3 exists precisely to detect this; damage parameters are config and swept before the campaign |
| Attack evolves to always-on because it is cheap | Attack carries an explicit energy cost and a retaliation risk; if it still saturates, that is a reportable finding about the cost structure |
| Kin recognition is trivially solvable because the kinship input is handed over directly | Recorded as a real limitation: providing computed genetic distance as an input authors more than a real organism gets. An alternative condition where kinship must be inferred from visible phenotype only is a stated follow-up, not a Phase 7 deliverable |
| Cluster labels leak into behavior through the C7.2 statistic | Structural: the statistic is computed in the analysis crate over the event log, after the run |

## Rollback

The entire phase is one config section. Disabled, the world takes the Phase 2 code paths and reproduces its fixture. Health state is absent from
snapshots of disabled worlds.
