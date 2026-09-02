# Phase 15 field-scaffold pre-registration (C15.3)

**LOCKED 2026-09-02, before any campaign world ran.** Committed ahead of
`experiments/phase15-field-scaffold.campaign` per the methodological law:
the observables, window, thresholds and expectations below are fixed
here; anything the campaign then shows is reported against them, never
fitted to them. The reduction script
(`experiments/results/phase15-field-reduction.py`) is committed alongside
this file, also before the campaign.

## Question

C15.3: abiogenesis occurs and persists, or is reported as not occurring -
the rate of formation and the persistence of the formed microbial
population, for the neutral condition and for scaffolded conditions at a
swept range of intensities, reported as a curve against scaffold
intensity. The reportable result is the difference between conditions,
never a scaffolded number alone (ADR-0018).

## World and base configuration

64x64 cells, phase-1 preset, 40 organisms, 60,000 ticks, 30 seeds per
condition (16001..16032 excluding 16005 and 16009, disjoint from every
prior decision range), the same seeds in every condition.

*Seed amendment, recorded 2026-09-02 before any campaign world ran:*
the range as first locked was 16001..16030; the campaign preflight
refused seeds 16005 and 16009 - both generate land fractions below the
worldgen minimum at 64x64 - and refused to run the whole design, so
zero worlds and zero data existed at amendment time. The two ungenerable
seeds are replaced by 16031 and 16032 (the next generable values, probed
by 1-tick world generation). Nothing else changed. The field stack: chemistry enabled at one
field step per tick, microbial classes 2x2x2, abiogenesis enabled, all
rate constants at their shipped defaults except `mutation_q16 = 4096` -
the shipped default (66) truncates to zero for densities below 993
milli, and abiogenesis seeds at most 1,000 milli with death applied
first, so the default would silently disable the mutation term at
exactly the densities the campaign creates (the trap is recorded in the
increment-2 commit; the choice is declared here, not discovered later).

Both coupling fractions are zero: organisms are present so the world is
a standard ecology, but they cannot touch the field and the field cannot
touch them, so every between-condition difference is the production
placement's and nothing else's. Population dynamics are identical across
conditions on the same seed (verified in the fixture: the neutral and
scaffold arms deposit identically and differ only in field state).

## Conditions

Production TOTAL is identical in every condition by construction (the
telescoping distribution); only its placement varies.

- **N** (neutral): spatially uniform abiotic production
  (`scaffold_patch_radius_cells 0`).
- **S1..S4**: the same total production concentrated into patches of
  radius 3 cells at contrast 2x, 4x, 8x, 16x (Q16 131072, 262144,
  524288, 1048576) - production inside patches versus outside, total
  held constant.

Plain-language description (the ADR-0018 naming test): "the same
substrate input concentrated into patches of radius 3 at contrast c".
It names an environmental structure, not an outcome; that patchy input
changes formation or persistence is the hypothesis under test.

## Observables (exact, from the `.alfd` series, interval 500)

Per world:

1. **Formation rate** = `fired` at the final sample, divided by
   (4096 cells x 60,000 ticks), reported per 10^6 cell-ticks.
2. **Persistence** (the stated window): the window is the final 10,000
   ticks (the last 20 samples plus the final one). A world carries a
   persistent population iff `microbial_milli > 0` at every window
   sample AND `microbial_milli` at the final sample is at least 10,000 -
   ten times the largest single seeding, so a lone just-fired seed
   cannot count as a persistent population.
3. **Sustainment ratio** = final `microbial_milli` / max(final
   `seeded_milli`, 1). Above 1, the standing population exceeds
   everything abiogenesis ever put in - growth-dominated rather than
   re-seeding-dominated.
4. **Occupancy** = final `occupied` / 4096.

Per condition: the fraction of 30 worlds satisfying (2), and the median
across 30 worlds of (1), (3), (4). The record is the five-point curve
(N, S1..S4) of each, and the per-S difference from N with its sign.

No acceptance bar is set: C15.3 is a measurement criterion, and the
curve with its controls IS the result, whichever way it runs.

## Expected outcomes, recorded in advance

The phase plan's a-priori text expected that under N abiogenesis either
never fires at a useful rate or produces populations that do not
persist. The shipped rate constants are evidently more permissive: the
48x48 fixture (near-uniform production, the same rates, 4,000 ticks)
already shows a self-sustaining field - sustainment about 7.6, occupancy
about 0.98. We therefore expect, and state before running:

- persistence fraction at or near 30/30 in EVERY condition including N
  (a ceiling); sustainment well above 1 everywhere;
- formation-rate and occupancy differences between S and N of unknown
  sign (concentration raises the local rate function where substrate
  piles up, and also deepens local depletion);
- if the ceiling holds, the persistence curve is flat, and the flat
  curve is the reported result. The discrepancy between the plan's
  expectation and the shipped constants' behaviour is reported as such -
  it is a fact about the chosen rate regime, not adjusted away by
  re-tuning after the fact.

## Hard gates

- Conservation: `World::check_invariants` enforces the exact field
  identity in-run (`check-interval 5000`); the reduction independently
  re-checks `produced + deposited == chem + microbial` at the final
  sample of every world and refuses the whole reduction on any defect.
- Completeness: the reduction requires all 150 series, each with the
  full sample count; anything missing or short is counted and fatal,
  never silently dropped.
- The campaign file's conditions must differ from N only in the two
  scaffold fields (the `vary` declaration and the comparison report's
  A5.6 check both say so).

## Out of scope here

C15.2 (population independence) is answered by the phase-15 benchmark
record; C15.1's long-horizon clause by the ledger-soak campaign
(`phase15-c151-ledger-soak.campaign`, 10^6 ticks with the invariant
checked every 5,000); C15.6 by the exchange test committed with
increment 3. The chemistry-as-food narrowing (ADR-0031 coupling v1) is
named in the findings.
