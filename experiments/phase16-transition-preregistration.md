# Phase 16 transition pre-registration (C16.5, C16.6)

**LOCKED 2026-09-02, before any confirmatory world ran.** Committed ahead
of `experiments/phase16-transition-confirmatory.campaign` per the
methodological law: the observables, decision rules and expectations
below are fixed here; anything the campaign then shows is reported
against them, never fitted to them. The reduction script
(`experiments/results/phase16-transition-reduction.py`) is committed
alongside, also before the campaign. The Stage B pilot that calibrated
this record (`experiments/phase16-transition-pilot.campaign`, seeds
17901, 17902, 17903, 17905 - 17904 generates no land at 64x64 - four
conditions, 16 worlds, 60,000 ticks) is archived under
`runs/phase16-transition-pilot-0xd113b9f4494ccde7`; its facts are quoted
where they set a value.

## Question

C16.5: `scratch` runs end to end - from chemistry, through abiogenesis and
the microbial field, to at least one materialized individual organism -
or reports precisely where it stops. C16.6: multicellularity is reached,
or reported as not reached - the fraction of seeds in which any lineage
exceeds one module, and the distribution of module counts over time, for
scaffolded and unscaffolded conditions.

Both are measurement criteria. The reportable quantity for C16.5 is the
difference between conditions (ADR-0018), and the interpretable
difference here is **T0 versus N**: a transition-disabled scratch world
holds zero organisms ever, by construction, so materialization under N
is what the transition adds. The scaffold arms (S2, S4) place the same
production into patches; D-128's ceiling and the pilot make the
a-priori expectation for the scaffold contrast flat.

## World and base configuration

64x64 cells, phase-2 preset, `origin.mode = scratch` with zero founders,
60,000 ticks, 30 seeds per condition (17001..17030, disjoint from every
prior decision range and from the pilot's; the generability probe of
2026-09-02 found every one of them generable at 64x64, so no amendment
is expected - should the preflight refuse one anyway, it is replaced by
the next generable value and the amendment recorded here before any
world runs), the same seeds in every condition.

The stack: schema 2 with the Phase 10 morphology campaign's caps and
mutation rates (duplication 6,554, deletion 655, insertion and
transposition 0; regulatory mutation enabled - the operator that can
turn the unicell's one differentiation rule into a placement rule), the
Phase 15 field regime at its shipped rates (production 2 milli per
cell-step, `mutation_q16` 4,096 above its truncation floor), coupling v1
live in both directions (excretion and remains at 0.5) so the whole
cycle runs, and the transition at its defaults: check interval 100,
density floor 4,000 milli, persistence 5 checks, aggregation step 1
(the top of the default axis), 4,000 milli per organism, 4 organisms
per event, 64 per tick. The pairing threshold stays at its shipped
7,000 (a materialized organism starts at 4,000 and must eat up to it;
the unicell holds 12,000).

*The floor, and why it is 4,000.* ADR-0032 first set the default floor
at 20,000 milli, reading D-128's "~53x seeded" sustainment ratio as a
per-slot density. It is a total. The first pilot run (same seeds, floor
20,000) materialized nothing in any arm; the per-slot average density
at the horizon is ~9,400 milli, growing linearly at ~5,000 per 10,000
ticks. The floor is now one organism's energy - the minimal physical
condition the trigger can state - and every campaign file and the
default carry 4,000. Declared here, from the pilot, before the
confirmatory; not discovered after.

## Conditions

Production TOTAL is identical in every condition by construction; only
its placement and the transition gate vary.

- **T0**: transition disabled. The field-only scratch world; zero
  organisms ever.
- **N**: transition on, uniform production.
- **S2**: transition on, the same production concentrated into patches of
  radius 3 at 2x contrast - the one interior point below the 4x
  saturation D-128 found.
- **S4**: transition on, fully concentrated (4x).

Plain-language description (ADR-0018's naming test): "the same substrate
input concentrated into patches of radius 3 at contrast c". It names an
environmental structure, not an outcome.

## Observables (exact, from the `field-series 2` `.alfd` series, interval 500)

Per world, from the final sample unless stated:

1. **Materialized** = `materialized` > 0 at the final sample (C16.5's
   per-world reading), and the first sample tick at which it is positive.
2. **Final population**, **final `materialized` total**, **final
   `births` total** - the last says whether any materialized lineage
   reproduced at all, so a C16.6 null with zero births is reported as a
   different null from one with births.
3. **Multi-module reached** = the peak of `max_modules` over the series
   exceeds 1 (C16.6's per-world reading); **final multi-module fraction**
   = `multi_module` / `population` at the final sample, milli.
4. **Occupancy** = final `occupied` / 4096 (the field's own persistence,
   for continuity with Phase 15).

Per condition: the count of worlds satisfying (1) and (3), the count with
births, and the medians of the rest. The record is the four-point table
(T0, N, S2, S4) and the N-minus-T0 and S-minus-N differences with sign.

## Decision rules, stated in advance

- **C16.5**: the stated N is **28 of 30** worlds materializing under the
  best scaffold condition, and the same 28 of 30 is stated for N itself:
  the pilot materialized in 4 of 4 worlds under N, S2 and S4 alike
  (first materialization at ticks 24,500 / 23,500 / 22,500, medians),
  and the density that drives the trigger grows deterministically with
  production, so a world that fails to materialize would be a seed whose
  land or field differs qualitatively. T0's expected result is 0 of 30
  by construction, and T0-versus-N is the finding. The S-versus-N
  contrast is reported as a curve with no bar; a-priori flat.
- **C16.6**: **expected null, stated in advance: 0 of 30 in every
  condition** (the pilot: 0 of 4 in every arm, peak `max_modules` 1
  everywhere). The pilot also says which of the two reasons applies:
  births happen but rarely - 2 of 4 worlds per arm, medians 3.5 (N),
  50.5 (S2), 62.5 (S4) births per world against ~22,700 materialized -
  so structural evolution has almost no material to act on within the
  horizon (Phase 9's D-073 birth-limit shape). A nonzero count is
  reported as a measured reachability with the scaffold named; the
  birth counts are reported beside it either way.
- **The influx rate is the cap.** In every pilot world the
  `materialized` total rises by exactly 640 per 1,000 ticks after onset
  (64 per check, 10 checks) - the per-tick cap binds, so the standing
  population (~140-170) is the equilibrium of a configured influx
  against starvation, not a property of the field. Stated so it is not
  read as a finding about density.

No threshold is weakened after the data; a different bar is a different
experiment.

## Hard gates

- Conservation: `World::check_invariants` enforces both identities in-run
  (`check-interval 5000`); the reduction independently re-checks
  `produced + deposited - materialized_milli == chem + microbial` at the
  final sample of every world and refuses the whole reduction on any
  defect.
- Completeness: the reduction requires every series with the full sample
  count; anything missing or short is counted and fatal.
- The campaign file's conditions differ from N only in the declared
  `vary` fields (the parser refuses otherwise).
- `refused` admissions are a bug signal; the fixture asserts zero and any
  nonzero value in a campaign world is reported as a defect, not a
  result.

## Expected outcomes, recorded in advance

Materialization in 30 (or 29) of 30 under N, S2 and S4, none under T0;
first materialization between ticks 20,000 and 30,000; standing
populations of order 100-200 at the horizon, held by the influx cap
against starvation; births in a minority of worlds and in the tens at
most; no lineage above one module in any world. The phase's own
a-priori text ("expected to return null" on C16.6) holds; C16.5's
"scratch runs end to end" is expected to be met, and its interpretation
rests on T0.

## Out of scope here

C16.1's long-horizon clause is the ledger-soak campaign
(`phase16-c161-ledger-soak.campaign`, 10^6 ticks with the invariant
checked every 5,000 - the conversion runs many times inside it); C16.2,
C16.3, C16.4 and C16.8 are the suite's and the verify script's; C16.7 is
the code-review criterion recorded in ADR-0032 and D-130.
