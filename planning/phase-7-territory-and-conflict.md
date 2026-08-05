# Phase 7: Territory, Contest, And Damage

Status: **complete. 2026-08-05.** Primary endpoint C7.1 measured and met;
C7.2's tolerance clause and C7.3 recorded unmet rather than adjusted. Policy
version `contest-behavior-v1`, unchanged. Decisions D-051, D-052, D-060.

## Implementation Status

**Delivered and verified** (`crates/sim-core/src/contest.rs`, world
integration, save/restore, event schema 3):

- Health as real per-organism state with damage, healing, and death by
  health depletion joining the existing causes; both accumulators fixed
  point per determinism rule 7.
- The attack action wired to the reserved `OUT_ATTACK` channel, and health
  (input 2), threat (input 10), and recent damage (input 16) wired to their
  reserved input channels. No genome schema change: topology 1 already
  reserved all four.
- Canonical pair key `lifesim-pairkey-v1` and RNG stream `Contest` (7), so
  damage does not depend on which combatant the tick visited first.
- Carcasses with decay and consumption, their own exact energy ledger, and a
  table kept sorted by ID.
- Event schema 3: `Damage`, `DeathByDamage`, `CarcassCreated`,
  `CarcassConsumed`, all additive over version 2.
- Threat is a **perceptible phenotype cue** (relative body size and
  closeness). There is deliberately no genotype-distance channel: ADR-0022
  A3 forbids direct access to genetic distance, pedigree, or observer
  labels, so kin recognition must be solvable from perceptible cues or not
  at all.

**C7.4 and C7.5 are met** with automated evidence
(`crates/sim-core/tests/phase7_contest.rs`): exact energy, biomass,
population, and carcass ledgers across a contest-heavy run; carcass energy
never exceeding its source; clean replay; save-restore-continue equality;
and the contest-disabled world reproducing `0xff9dfcff5dffbf42` exactly.

**C7.1 is met.** The world-level index it required now exists
(`crates/sim-analysis`, `lifesim-spatial-index-v1`) and the confirmatory
campaign measured it: 52 of 60 worlds on aggregation and 44 of 60 on
encounter, against a prespecified bar of 40. See the analysis plan and
results below, including which half of that result survives its confound.

## The First Campaign (2026-08-04), At A Configuration That Was Not Recorded

Kept as the historical record. Its absolute numbers are **not comparable**
with the confirmatory campaign below: its median populations were 15 to 48,
against 87 to 454 for the same conditions at the project's standard
500-organism 256x256 tier, so it was run on a much smaller world whose
campaign file was never saved. That is the reason campaign files now live in
`experiments/`. Its qualitative decomposition does reproduce.

A pre-campaign parameter sweep (the plan's stated mitigation for
"contest collapses populations") swept damage over {100, 300, 600, 1200} and
attack cost over {0, 15, 60, 120}. Attack cost turned out to be nearly
irrelevant at realistic attack rates; damage dominates.

The confirmatory campaign then ran 30 seeds x 4 conditions at 20,000 ticks
with the documented conservative defaults. A fourth condition **D** was added
beyond the three the plan specifies, because conditions A and C both change
*two* things at once — they make the attack action live **and** the reserved
sensing channels live — so neither isolates the action. D enables contest
with an attack threshold above the controller's output range: perception
changes, no attack can ever fire.

| Condition | Median population | Median births | Median attacks | Median deaths by damage |
|---|---:|---:|---:|---:|
| **B** contest disabled | 48 | 102 | 0 | 0 |
| **D** perception only | 48 | 75 | 0 | 0 |
| **C** attacks fire, zero damage | 30 | 56 | 2,797 | 0 |
| **A** full contest | 15 | 12 | 206 | 18 |

The decomposition is clean and it is the finding:

- **Making the reserved channels live costs nothing.** D's median population
  is identical to B's. Threat and health perception, on their own, change no
  measurable outcome.
- **The attack action alone costs about 37 percent of the population**, with
  damage set to exactly zero. That is an energy-cost effect at a saturating
  attack rate, not a mortality effect.
- **Damage costs a further 31 percent.**

Against the criteria as written:

- **C7.2 is not satisfied on its tolerance clause** — a median population of
  15 against 48 is far outside any reasonable tolerance — but it is
  satisfied on its alternative clause, "or the difference is explained by a
  reported mechanism", and the mechanism is decomposed above against three
  controls rather than asserted.
- **C7.3 is not satisfied.** Under condition C the per-organism attack rate
  saturates (up to 14,456 attacks per world). Under A it looks lower only
  because attackers die sooner. The plan anticipated exactly this:
  "Saturation is a reportable finding about the cost structure, not a
  failure to be tuned away." It is reported as one.

No criterion was weakened after seeing the data. C7.2's tolerance and C7.3's
bound are recorded as unmet rather than adjusted.

## The C7.1 Analysis Plan, Prespecified

Fixed before the confirmatory campaign ran. Recorded here in the order the
decisions were actually made, because for two of them *when* they were
fixed is the whole point.

### Fixed before the pilot

- **The index** (`lifesim-spatial-index-v1`, `crates/sim-analysis`). Two
  nested quadrat scales over habitable area, each scored with Morisita's
  index of dispersion, pooled over samples:
  - **Aggregation** = Morisita at 64 m (16 cells). `1.0` is placement at
    random over the land available.
  - **Encounter** = Morisita at 8 m (2 cells) divided by the aggregation
    index. `1.0` is fine-scale spacing that is random *given* coarse-scale
    position; below 1.0 is encounter avoidance. The fine scale sits inside
    the band over which organisms actually interact: attack range 3 m,
    pairing range 4 m, crowding radius 6 m, sensor range at most 12 m.

  Morisita rather than a variance-to-mean ratio or a nearest-neighbour
  distance because the conditions differ several-fold in population, and
  Morisita's expectation is 1 under complete spatial randomness for any
  population size. The two-scale ratio because aggregation and encounter
  are not independent, and dividing conditions the second on the first.
  Everything is integer counts in milli-units, so a report is exact.
- **Sampling**: every 50 ticks over 20,000 ticks (400 samples per world).
- **Burn-in**: samples at or before tick 5,000 are discarded, because the
  opening layout still reflects founder placement.
- **SESOI**: a **10 percent relative change** in an index. Expressed
  relative rather than absolute because the same absolute step means
  different things to an index at 1.8 and one at 6.3.
- **Unit and pairing**: the world is the replicate (ADR-0022 A5), and
  conditions are seed-matched, so the analysis is paired by seed.

### Fixed from the pilot, before the confirmatory campaign

The pilot is `experiments/phase7-c71-pilot.campaign`, seeds 1..16,
**disjoint** from the confirmatory set, so the data that set these is not
also the evidence.

- **The decision rule is directed.** The criterion as literally written -
  worlds whose index "differs by at least the SESOI" - turns out not to
  discriminate: condition D, whose mean effect on aggregation is +1.4
  percent, still crossed a 10 percent SESOI in **12 of 16 worlds**, because
  per-world seed variation is itself larger than the SESOI. Counting only
  worlds that cross **in a direction fixed in advance** is strictly
  stronger than the criterion as written. Under the directed rule D scores
  7 of 16 and 2 of 16 and correctly fails.
- **The direction is `decrease`** on both indices, taken from the pilot.
- **The null rate is 0.5**, which makes the 20-of-30 bar an exact one-sided
  sign test at alpha 0.049. It is conservative by construction: under any
  symmetric null at most half the worlds can move in the fixed direction.
  The pilot's near-null contrasts came in at or below it (D: 7/16 and
  2/16), which is the check that it is not optimistic.
- **60 worlds per condition.** Simulation-based power (nonparametric
  bootstrap over the pilot's own paired differences, `lifesim-power-v1`):
  the binding index is `encounter`, at 0.90 power at the 30-world floor and
  0.94 at 60. Aggregation is at 1.00 throughout. 60 buys margin against the
  pilot's own rate uncertainty - 12 of 16 has a wide interval - at
  negligible compute.
- **Acceptance is conjunctive** (ADR-0022 A7): C7.1 passes only if **both**
  indices clear the bar. Each is also reported separately.

### What the pilot already showed, and why condition E is in the design

The pilot found that condition **E** - contest disabled, carrying capacity
reduced to 18,000 milli - reproduces most of A's **aggregation** drop (-32
percent against A's -42) while moving the **encounter** index in the
*opposite* direction (+48 percent, 0 of 16 worlds in A's direction). So an
ecological change with no contest in it at all can move the aggregation
index about as far as contest does, and only the encounter index responds
to contest specifically. E is a supporting comparison and not a criterion:
it lowers density by lowering food everywhere, so it is not a clean density
control, and it is reported as what it is.

**Correction, made after the confirmatory campaign ran.** The sentence that
stood here claimed that within condition B neither index trends with
population, on the strength of the pilot's 16 worlds. **That was wrong**,
and 60 worlds show it: within B, Spearman's rho is **+0.41** for aggregation
and **-0.56** for encounter. The claim is withdrawn rather than softened.
What replaces it is better than what it claimed, and is in the Results
below: the two indices are confounded with population in *opposite*
directions, so the confound argues for the aggregation result and against
the encounter one.

## Results: C7.1 Is Met, And One Half Of It Is Load-Bearing

`experiments/phase7-c71-confirmatory.campaign`, 5 conditions x 60 seeds
(1001..1060) x 20,000 ticks, 300 worlds, campaign hash `0xa6f5c0b90c1d8e48`.
Report: `experiments/results/phase7-c71-confirmatory-spatial.txt`.

**No world was excluded.** All 300 produced a defined index, so there are no
treatment-correlated exclusions to argue about. No world went extinct; the
smallest pooled sample behind any single index is 8,402 organism
observations.

| Contrast | Aggregation | Encounter |
|---|---|---|
| **A** full contest | **52/60**, -38.9%, CI95 [-2.704, -1.758] | **44/60**, -24.5%, CI95 [-0.893, -0.382] |
| **C** attacks fire, zero damage | 32/60, +2.2% | 25/60, +5.6% |
| **D** perception only | 22/60, +11.5% | 23/60, +4.7% |
| **E** no contest, reduced capacity | 36/60, -12.8% | 11/60, **+51.1%** |

Bar: 40 of 60 worlds down by at least 10 percent. Both indices clear it
under A, acceptance is conjunctive, so **C7.1 is met.**

Three things in that table matter more than the pass.

**Damage does it, not the attack action.** Condition C fires attacks at
thirteen times A's rate with damage set to exactly zero, and moves neither
index (+2.2 and +5.6 percent, both failing the bar). Condition D, which
makes the sensing channels live and no more, moves neither. Whatever
contest does to spatial structure, it does through damage.

**The aggregation half is confounded and the encounter half is not.**
Within condition B, aggregation rises with population (rho +0.41) and
encounter falls with it (rho -0.56); D reproduces both signs. A has roughly
a fifth of B's population. So population alone predicts A's aggregation to
fall - which is exactly what was observed, and therefore not evidence of
anything about contest. It predicts A's encounter index to *rise*, and it
fell by 24.5 percent. **The encounter result runs against its own confound.**

**Condition E is the same argument, measured rather than inferred.** A
contest-free world thinned by cutting carrying capacity moves the encounter
index +51.1 percent, in 47 of 60 worlds - the direction the population
relationship predicts, and the opposite of A. It also moves aggregation down
12.8 percent, most of the way to A's effect, without any contest in it at
all, and falls just short of the bar (36/60, p = 0.077).

So the defensible claim is narrower than "contest changes spatial
structure", and it is the one carried forward: **damage reduces short-range
co-occurrence, conditional on where organisms are, by about a quarter, and
the reduction is not attributable to the accompanying population decline
because that decline pushes the measure the other way.** The aggregation
result is reported as passing its prespecified bar while being confounded,
because that is what it is.

### C7.3, re-measured

D-052 recorded C7.3 unmet because the attack rate saturated. At this
configuration it does not. Under A the rate is **0.51 percent of the hard
cooldown ceiling** (49,388 attacks over 97.2 million organism-ticks);
under C, with damage removed, it is 6.62 percent - thirteen times higher but
still not saturating. The difference between the two is retaliation risk and
attacker mortality, both of which C removes.

That satisfies C7.3's first clause. Its second - that the rate "correlates
with local resource contention at the world level" - is **not** satisfied:
against a demand-over-supply proxy the correlation under A is only rho
**+0.19** (it is +0.52 under C, where attacking is nearly free). **C7.3
stays unmet**, now on its correlation clause rather than its saturation
clause, and is recorded as unmet rather than adjusted.

The D-052 figures were measured at a different and unrecorded
configuration whose median populations were 15 to 48; this campaign runs the
project's standard 500-organism, 256x256 tier, where the same conditions
give medians of 87 to 454. The two are not directly comparable, which is
itself the argument for the campaign files now living in `experiments/`.

## Remaining Work

1. ~~Implement the world-level spatial aggregation and encounter-avoidance
   index C7.1 needs.~~ **Done**: `crates/sim-analysis`, computed offline
   from a versioned spatial sample file (`ALSS`) written by the experiment
   harness. Positions are deliberately not in the event log; see the module
   header for why.
2. ~~Re-run the confirmatory campaign with the primary endpoint measured.~~
   **Done**; see Results above.
3. ~~Decide whether the saturating attack rate is a cost-structure problem
   worth changing.~~ **Decided: no change; `contest-behavior-v1` stands.**
   The saturation that motivated the question is not present at the standard
   tier - 0.51 percent of the ceiling under full contest - and appears only
   under condition C, where damage is switched off so nothing damps the
   action. Opening a `contest-behavior-v2` lineage would be tuning away a
   phenomenon that the full-contest condition does not exhibit, at the cost
   of a new replay lineage.

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
destroyed. Almost all of them land in Phases 11 to 13.

So Phase 7 delivers the **physics of damage and contest** and measures only
what that physics can establish on its own. The organized-conflict claims
move to Phase 13, where recognition, memory, and transmission exist. See
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

- [x] **C7.1 Contest changes spatial structure (primary).** **Met.** 52/60
      and 44/60 against a bar of 40, both indices, conjunctively. The
      encounter half runs against its population confound; the aggregation
      half runs with it and is reported as such. Under A, a
      world-level index of spatial aggregation and of encounter avoidance
      differs from B by at least the prespecified smallest effect of
      interest, in at least 20 of 30 worlds. Condition C distinguishes the
      energetic cost of attacking from its consequences. Reported with an
      interval and an equivalence result, so a null is interpretable.
- [ ] **C7.2 Contest is consequential, not lethal noise.** **Unmet on its
      tolerance clause, satisfied on its mechanism clause**, unchanged from
      D-052 and recorded rather than adjusted. Median population
      and median lifespan under A remain within the stated tolerance of B,
      or the difference is explained by a reported mechanism. A model that
      simply collapses populations has produced a mortality term, not
      conflict.
- [ ] **C7.3 Attack is used selectively rather than saturating.** **Unmet.**
      The rate is bounded away from zero and the ceiling at the standard
      tier (0.51 percent of it), but its correlation with world-level
      resource contention is only rho +0.19, so the second clause fails. The
      per-organism attack rate under A is bounded away from both zero and
      the ceiling, and correlates with local resource contention at the
      world level. Saturation is a reportable finding about the cost
      structure, not a failure to be tuned away.
- [x] **C7.4 Energy and health accounting is exact.** Met. The ledger stays exact
      to the milli-unit across damage, healing, death, carcass creation, and
      carcass consumption over a 10^6-tick run. Carcass energy never exceeds
      the source organism's recorded remaining transferable energy.
- [x] **C7.5 Determinism.** Met. Clean-process replay of the Phase 7 fixture;
      order-independence under storage permutation; pair-key symmetry;
      contest-disabled configs reproduce `0xff9dfcff5dffbf42` exactly.

### Explicitly not claimed here

Kin-biased grouping, directed inter-group violence, territoriality, and
coalition formation are **not** Phase 7 criteria. They require recognition
from perceptible cues, associative memory, and spatial memory, none of which
exist until Phases 11 to 13. Measuring them here would produce either a
null that says nothing or an artifact of the demes the world was seeded
with. They are Phase 13 criteria.

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
| Kin structure is read as kin *recognition* | Phase 7 has no recognition mechanism and claims none. Limited-dispersal kin structure is reported descriptively; recognition is a Phase 13 question with perceptible cues only |
| Cluster labels leak into behavior through the C7.2 statistic | Structural: the statistic is computed in the analysis crate over the event log, after the run |

## Rollback

The entire phase is one config section. Disabled, the world takes the Phase 2 code paths and reproduces its fixture. Health state is absent from
snapshots of disabled worlds.
