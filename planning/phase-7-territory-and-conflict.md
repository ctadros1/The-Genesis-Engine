# Phase 7: Territory, Contest, And Damage

Status: planned, not started. Policy version `contest-behavior-v1`.

## Why This Is Second, And What It Can And Cannot Establish

Contest is scheduled early for two reasons that survive review, and its
scope was cut for a third that did not.

**It needs no schema change.** Topology 1 already reserves its channels:
input 2 (health) reads a neutral 1.0, input 10 (threat) and input 16 (recent
damage) are documented neutral zeros, and output 4 (attack) is a documented
no-op. Contest is the one behavioral mechanism implementable inside the
frozen topology, so it validates the Phase 5 harness on a real question
before the expensive genome work.

**It makes threat information real** before a signal channel exists, which
is the condition under which a costly signal is most likely to pay.

**What was withdrawn.** The original justification claimed organized
violence "tends to fall out of scarcity plus kin-biased grouping". The
social-organization review explicitly rejects that shortcut
(`social_organization` section 1.3) and lists roughly eleven further dependencies:
persistent interaction communities, recognition, target discrimination,
coalition recruitment, numerical-advantage assessment, spatial memory,
free-rider suppression, and value that can be captured rather than merely
destroyed. Almost all of them land in Phases 10 to 12.

So Phase 7 delivers the **physics of damage and contest** and measures only
what that physics can establish on its own. The organized-conflict claims
move to Phase 12, where recognition, memory, and transmission exist. See
ADR-0022 A1.

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
- Perceptible phenotype cues on the existing sensory channels. **No
  genotype-distance input**: ADR-0022 A3 forbids direct access to genetic
  distance, pedigree, or observer labels, so kin recognition must be
  solvable from perceptible cues or not solvable at all.
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

**Primary endpoint: C7.1.** Secondary criteria do not rescue a failed
primary (ADR-0022 A7). The world is the replicate throughout; per-organism
quantities are aggregated to a world-level statistic before analysis
(ADR-0022 A5).

Conditions, matched on seed set (30 independent worlds), config, and run
length. Final seed count is set by simulation-based power analysis from a
pilot; 30 is the floor.

- **A**: contest enabled.
- **B**: contest disabled; attack remains a no-op. The Phase 2 baseline.
- **C**: contest enabled but damage set to zero, so the action fires and
  costs energy without consequence. This separates "attack is expressed"
  from "attack does anything".

Criteria:

- [ ] **C7.1 Contest changes spatial structure (primary).** Under A, a
      world-level index of spatial aggregation and of encounter avoidance
      differs from B by at least the prespecified smallest effect of
      interest, in at least 20 of 30 worlds. Condition C distinguishes the
      energetic cost of attacking from its consequences. Reported with an
      interval and an equivalence result, so a null is interpretable.
- [ ] **C7.2 Contest is consequential, not lethal noise.** Median population
      and median lifespan under A remain within the stated tolerance of B,
      or the difference is explained by a reported mechanism. A model that
      simply collapses populations has produced a mortality term, not
      conflict.
- [ ] **C7.3 Attack is used selectively rather than saturating.** The
      per-organism attack rate under A is bounded away from both zero and
      the ceiling, and correlates with local resource contention at the
      world level. Saturation is a reportable finding about the cost
      structure, not a failure to be tuned away.
- [ ] **C7.4 Energy and health accounting is exact.** The ledger stays exact
      to the milli-unit across damage, healing, death, carcass creation, and
      carcass consumption over a 10^6-tick run. Carcass energy never exceeds
      the source organism's recorded remaining transferable energy.
- [ ] **C7.5 Determinism.** Clean-process replay of the Phase 7 fixture;
      order-independence under storage permutation; pair-key symmetry;
      contest-disabled configs reproduce `0xff9dfcff5dffbf42` exactly.

### Explicitly not claimed here

Kin-biased grouping, directed inter-group violence, territoriality, and
coalition formation are **not** Phase 7 criteria. They require recognition
from perceptible cues, associative memory, and spatial memory, none of which
exist until Phases 10 to 12. Measuring them here would produce either a
null that says nothing or an artifact of the demes the world was seeded
with. They are Phase 12 criteria.

Kin structure that arises purely from limited dispersal is measured and
reported as a **descriptive observation** of the seeded population
structure, never as evidence of recognition.

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
| Kin structure is read as kin *recognition* | Phase 7 has no recognition mechanism and claims none. Limited-dispersal kin structure is reported descriptively; recognition is a Phase 12 question with perceptible cues only |
| Cluster labels leak into behavior through the C7.2 statistic | Structural: the statistic is computed in the analysis crate over the event log, after the run |

## Rollback

The entire phase is one config section. Disabled, the world takes the Phase 2 code paths and reproduces its fixture. Health state is absent from
snapshots of disabled worlds.
