# Phase 19 consumption pre-registration (C19.1, C19.3, C19.4, C19.5)

**STATUS: LOCKED 2026-09-03**, before any confirmatory or soak world ran.
Everything below was fixed from the Stage B pilot
(`experiments/phase19-consumption-pilot.campaign`, seeds 19901 19903
19904 19905, archived under `runs/phase19-consumption-pilot-*`), whose
seeds are disjoint from the confirmatory's. No threshold is weakened
after the data; a different bar is a different phase.

## Question

Does letting an organism eat the substrate in its own cell (coupling v2,
ADR-0034) change what a materialized unicell's life is: its lifespan
(C19.3, primary), whether it reproduces (C19.4), and whether any lineage
leaves one module (C19.5, C16.6 re-asked)? The control is coupling v1 on
the same seeds - the Phase 16 world byte for byte except the mouth
(`scripts/verify-phase19-determinism.sh` clause 3 pins that v1 IS the
Phase 16 fixture).

## World and arms

`experiments/phase19-consumption-confirmatory.campaign`: the Phase 16
confirmatory's scratch base (64x64, 60,000 ticks, shipped field rates,
transition at its defaults, floor 4,000, influx cap 64 per 100 ticks,
pairing threshold 7,000 untouched, `max_entities` 4,000), two arms on 30
matched seeds 19001..19008 19010..19022 19024..19032: **v1**
(`chemistry.consumption_fraction_q16` 0) and **v2** (65,536; yield
39,322 = 0.6). Events, snapshots and a 500-tick field series on.

### Seed amendment, before any world ran

The declared sets were 19901..19904 and 19001..19030. A one-tick
preflight on 2026-09-03 refused 19902, 19009 and 19023 (generated land
fraction outside [6554, 58982] of 65536 at 64x64); 19905, 19031 and 19032
replaced them, the next seeds up, probed generable the same way. No
campaign world had run.

### Analysis change made after the pilot, before this lock

`lifesim demography` index v1 started a life only at a `Birth` or
`PairedBirth` record, so a scratch world reported "0 completed" over
~22,600 deaths and C19.3 was uncomputable. Index v2
(`lifesim-demography-index-v2`, report 2) starts a life at a
`Materialized` record too and adds a materialized-only median beside
the all-organism one. Worlds without materialization reduce identically
under both. The pilot numbers below are index v2's; the change is a
definition, not a threshold, and it is recorded here because it was made
with pilot data in view.

## Observables and the arithmetic stated before the run

A unicell (constant genome map, `GENOME_MAP_VERSION` 1) has basal cost
200 milli per tick and digestive capability 1,000 milli per tick; at
yield 0.6 a gut fed from the field nets up to 400 milli per tick where
substrate is present, so a materialized organism (4,000 milli) can
reach the pairing threshold (7,000) in ~8 ticks of full feeding: the
threshold IS reachable from the field alone (C19.4's stated
precondition). Biomass fills the capability first; the field only feeds
where biomass is gone. Under v1 the same organism starves at ~200
ticks (D-130), a quarter of its trait-derived maturity (800).

1. **C19.3 (primary)**: `materialized_median_lifespan` per world (the
   lower median of completed materialization-to-death lifespans, index
   v2), v2 minus v1, seed-paired. **SESOI +200 ticks** - one v1
   lifetime, i.e. the mouth must at least double the life. **Bar: 28
   of 30 pairs** above the SESOI (the Phase 16 C16.5 bar; a sign count
   of 28 of 30 has probability 4.3e-7 under no directed effect).
   Pilot: 4 of 4 pairs, deltas 1,129 / 1,217 / 1,353 / 1,457 (v1
   medians 183-200; v2 1,329-1,644). Even a quarter of the pilot effect
   clears the SESOI in every pair; power at the bar is not the risk,
   the risk is a seed whose v2 world materializes little (pilot 19904:
   278 completed) - a world with **no** completed materialized lifespan
   in either arm is reported and the pair counted as not clearing.
   Censored materialized organisms are counted beside the median, never
   imputed.
2. **C19.4**: `births` per world (final `.alfd` sample), v2 minus v1.
   **SESOI +100 births** (the v1 pilot maximum was 11; a hundred is an
   order of magnitude above it). **Bar: 28 of 30 pairs.** Pilot: 4 of 4,
   deltas 5,425-5,894.
3. **C19.5 (C16.6 re-asked)**: worlds with any body above one module
   (peak `max_modules` > 1 over the series), per arm, with the birth
   counts beside it. The plan and the draft of this record expected a
   null again; **the pilot revised that before this lock**: v2 3 of 4,
   v1 0 of 4. Stated now: **v2 at least 15 of 30, v1 at most 2 of 30**.
   Mechanism reading, not a criterion: a materialized organism always
   starts with one module (the constant map), so a second module is a
   born offspring's - Phase 9's genome duplication at
   `duplication_q16` 6,554 acting on the first lineages that reproduce.
   The final `multi_module` fraction and peak module count are reported
   per world.
4. **Free-lunch gate**: no world in either arm at the entity cap
   (population 4,000); the final field mass (chem + microbial) reported
   per arm so grazing is visible (pilot: v2 ~78M milli against v1
   ~406M).
5. **C19.1 soak**: `experiments/phase19-c191-ledger-soak.campaign`, one
   v2 world, 10^6 ticks, `check-interval 10000`: both identities exact
   at every check or the phase fails. The confirmatory reduction also
   re-derives the field identity with the consumed term at every one of
   the 120 samples of all 60 series.

## Decided outside this record

C19.2 (neutrality under v2), C19.6 (the exchange test), C19.7 (fixtures)
and C19.8 (cost) are decided by tests, the verify script and the
benchmark; their results are recorded in the findings and the plan.

## Reduction

`experiments/results/phase19-consumption-reduction.py <dir>
<demography.txt> v1 v2 19001..19032/19009,19023 60000 500 4096 4000 200
28 100 28` - it counts, it does not decide, and it refuses any series
that is missing, short, or breaks the identity.

## Expected outcomes, recorded in advance

C19.3 met; C19.4 met; C19.5 present under v2 and absent under v1 as
bounded above; free-lunch gate clear; soak exact.
