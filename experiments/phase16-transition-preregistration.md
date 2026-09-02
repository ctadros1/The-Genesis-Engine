# Phase 16 transition pre-registration (C16.5, C16.6)

**STATUS: DRAFT, NOT LOCKED.** Written before the Stage B pilot
(`experiments/phase16-transition-pilot.campaign`, seeds 17901..17904,
disjoint from the confirmatory's). The pilot fills in the marked
`[PILOT]` facts; the record is then LOCKED and committed before any
confirmatory world runs (`experiments/phase16-transition-confirmatory.campaign`).
Nothing the pilot shows changes an observable's definition - it can only
change the horizon, a cap, or an expectation, and each such change is
recorded here with the pilot number that motivated it.

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
production into patches, and D-128's ceiling makes the a-priori
expectation flat.

## World and base configuration

64x64 cells, phase-2 preset, `origin.mode = scratch` with zero founders,
60,000 ticks, 30 seeds per condition (17001..17030, disjoint from every
prior decision range; any seed the preflight refuses is replaced by the
next generable value and the amendment recorded here before any world
runs, as Phase 15 did), the same seeds in every condition.

The stack: schema 2 with the Phase 10 morphology campaign's caps and
mutation rates (duplication 6,554, deletion 655, insertion and
transposition 0; regulatory mutation enabled - the operator that can
turn the unicell's one differentiation rule into a placement rule), the
Phase 15 field regime at its shipped rates (production 2 milli per
cell-step, `mutation_q16` 4,096 above its truncation floor), coupling v1
live in both directions (excretion and remains at 0.5) so the whole
cycle runs, and the transition at its defaults: check interval 100,
density floor 20,000 milli, persistence 5 checks, aggregation step 1
(the top of the default axis), 4,000 milli per organism, 4 organisms
per event, 64 per tick. The pairing threshold stays at its shipped
7,000 (a materialized organism starts at 4,000 and must eat up to it;
the unicell holds 12,000).

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

- **C16.5**: N is stated before the campaign as `[PILOT] N of 30` worlds
  materializing under the best scaffold condition; the expected result
  under T0 is 0 of 30 by construction, and that comparison is the
  finding. Given D-128 (persistence at the ceiling in every Phase 15
  condition including neutral), the a-priori expectation is that N also
  materializes in nearly every world, so the S-versus-N contrast is
  expected flat and is reported as a curve without a bar.
- **C16.6**: **expected null, stated in advance.** The count of worlds
  in which any lineage exceeds one module is reported per condition with
  no bar; the a-priori expectation is 0 of 30 in every condition, for two
  independent reasons the pilot will separate: a unicell may not reach
  maturity and reproduce at all in this ecology (no births - Phase 9's
  D-073 birth-limit shape), and if it does, structural evolution at
  these rates does not reach a population median (D-087). A nonzero
  count is reported as a measured reachability with the scaffold named.

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

`[PILOT]` - to be completed from the pilot's four seeds per condition:
first-materialization tick under N, sustained population, whether births
occur, whether the per-tick cap or `max_entities` deferred anything, and
the module-count distribution. The pilot cannot change an observable's
definition.

## Out of scope here

C16.1's long-horizon clause is the ledger-soak campaign
(`phase16-c161-ledger-soak.campaign`, 10^6 ticks with the invariant
checked every 5,000 - the conversion runs many times inside it); C16.2,
C16.3, C16.4 and C16.8 are the suite's and the verify script's; C16.7 is
the code-review criterion recorded in ADR-0032 and D-130.
