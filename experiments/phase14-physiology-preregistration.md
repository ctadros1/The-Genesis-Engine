# Phase 14 Confirmatory Campaign Pre-Registration

**Status: committed before any confirmatory world runs** (ADR-0022 A7).
Every parameter below was fixed from the Stage B pilot
(`experiments/phase14-physiology-pilot.campaign`, seeds 14901..14908,
disjoint from every decision range; reports committed as
`experiments/results/phase14-pilot-{development,assortment}.txt`), from
Gate E scripted fixtures, and from the ADR-0030 design record. Nothing
here was chosen after seeing a confirmatory world.

Detectors: `lifesim-development-v1` (C14.1) and `lifesim-assortment-v1`
(C14.2), read through `lifesim development` and `lifesim assortment`.
The contrast arithmetic is executed by the committed findings-reduction
script at reading time, exactly as Phase 13's was; its rules are this
document's.

## Arms and seeds

The confirmatory base is the pilot's base verbatim (the Phase 13 pilot
base minus its social and plasticity sections, plus morphology and the
physiology section) with **one stated change: the Phase 8 authored
juvenile hazard multiplier is set to 1.0** - the pilot ran with the
default 2x, so its juvenile-mortality gap was partly a config knob's;
the confirmatory removes the knob so any excess juvenile mortality is
the partially grown body's own. 50 seeds per arm, **15001..15050**,
matched across arms:

  A   ontogeny_enabled, mate_choice_enabled       (both ADR-0030 gates)
  B   both gates off                              (the Phase 8 baseline)
  P   A plus mate_choice_scramble                 (cue-blind choice)

150 worlds, 60,000 ticks each. Wall-clock measured by the pilot: 24
worlds in 27.8 minutes at 6 workers, so the campaign is ~2.5 hours at 8.

## Primary endpoint: C14.2, decided by A-versus-P alone

Per world, the assortment census's candidate-weighted mean deviation of
the chosen candidate's TRUE body-scale cue (cue 7) from its opportunity
mean: `dev7 = sum(n_i * chosen_i - sum_i) / sum(n_i)`, milli, truncated
toward zero, over choices with at least two candidates. Single-candidate
choices are excluded and counted; a world with no informing choice is
unusable and reported by seed.

**Why cue 7**: body scale is the phase's named perceivable trait - it is
heritable, genuinely costly (mass drives allometric upkeep), carried in
the schema-8/10 phenotype records, and it is what C14.3's ornament
question reads. The other eight cues are reported in full beside it and
decide nothing.

**Why A-versus-P is the whole decision**: the criterion's two clauses -
"pairings are non-random with respect to perceived phenotype" and "the
assortment disappears under P-scramble" - collapse into one contrast,
because P is the empirical null: its choices are cue-blind by
construction and it shares A's seed, ecology, cost and act. The pilot
proved the collapse is not hypothetical: with neutral founders A and P
are literally identical worlds, and BOTH carry a positive proximity
deviation (~+100 milli on cue 1) from the nearest-candidate tie-break -
a within-arm deviation against zero can never evidence choice. Cue 1 is
baseline-loaded and never decisive. There is no A-versus-B form: B has
no choices, so the assortment statistic does not exist there.

**Decision rule (locked):**
- D(s) = |dev7_A(s)| - |dev7_P(s)| per matched seed: does the informed
  arm carry MORE absolute scale-assortment than its cue-blind twin?
  Absolute values because an informed preference for small is as much
  assortment as one for large; the direction of the contrast (increase)
  is fixed from the design.
- SESOI: **10 milli absolute** - roughly 2% of the perceived scale
  cue's dynamic range and more than five pilot noise SDs (the pilot's
  paired |A|-|P| values on cue 7 were [0,1,1,1,1,1,1,6], sd 1.85).
- Bar: **at least 30 of 50 seeds** with D(s) >= +10 milli (the plan's
  own number). Stated honestly: the bar alone at the worst-case null
  rate 500 is p = 0.101, NOT a stand-alone 0.05 test the way Phase 13's
  20-of-30 was; the exact binomial tail is reported beside the count,
  and the simulated false-positive rate under both pilot-derived null
  models is below 1e-4 (`experiments/phase14-power-simulation.py`,
  committed with its output below).
- Power, simulated at world level per methodology review 7.12: ~0 for
  any true effect below the SESOI (the deliberate estimation ceiling),
  0.35/0.10 at exactly 10 milli under the resampled/Gaussian nulls, and
  1.00 from 12 milli up. A true effect smaller than the SESOI lands in
  estimation, not decision.

```
effect_milli resampled_power gaussian_power
0  0.000 0.000    10 0.346 0.099
2  0.000 0.000    12 1.000 1.000
4  0.000 0.000    15 1.000 1.000
6  0.000 0.000    20 1.000 1.000
8  0.000 0.000
```

**Expected outcome, stated in advance: null.** The pilot showed
preference has not left neutral within the horizon (every A-vs-P paired
deviation delta within +/-21 milli of zero on every cue), and no
authored assortative advantage exists for selection to find - the
project authors physics, never progress. The campaign's value if null is
the controlled baseline every later selection-coupling lever will be
contrasted against, plus the C14.1 measurement, which does not depend on
preference evolving.

## C14.1: juveniles measurably constrained

Read from `lifesim development` over arm A (B beside it as the
everyone-adult baseline; its juvenile columns are zero by construction):

- **Decision clause (mortality)**: juvenile mortality (deaths per
  million sampled observations) exceeds adult mortality within-world in
  **at least 30 of 50 A worlds**, with the authored multiplier at 1.0
  (exact binomial beside; the pilot saw 8 of 8 with gaps
  +5,803..+62,164 micro, but under the 2x knob - the confirmatory's
  gap, if it holds, is the body's).
- **Constraint clauses (capacity, sensor range)**: verified as exact
  derivations at Gate E - a grown prefix's carry capacity and sensor
  range are monotone non-decreasing in the prefix by construction
  (`phase14_ontogeny.rs`, `ontogeny.rs` unit tests) - and reported from
  the campaign as the completions/window counts that show the juvenile
  state actually occurred at scale (pilot: ~120k completions per world,
  ~155k juvenile observations).
- **Speed is reported direction-free, with its mechanism named**: under
  prefix growth realized juvenile speed is NOT monotone-lower - a motor
  module activating early in a light prefix carries a higher thrust/mass
  ratio than the finished body (pilot: juveniles ~87 vs adults ~61
  milli-metres per tick, consistent across all 16 gate-on worlds). The
  criterion's wording ("differ from adults by the configured amount")
  is satisfied by the Gate E derivation identities, not by a realized
  census direction; recorded here as the as-built reading, before data.

## C14.3: costly display - expected null, descriptive

Reported from the tag-26 phenotype records and censuses: the body-scale
trajectory under A versus B (does preference drag scale beyond the
survival optimum the B arm settles at). No threshold; expected null,
stated in advance; a positive would require its own replication before
any claim.

## C14.4: disease

Deferred with its slice (ADR-0030 decision 3); reported as deferred.

## C14.5: exactness and determinism

The 10^6-tick soak (`experiments/phase14-c145-ledger-soak.campaign`,
seed 14201, both gates on, ledger checked every 5,000 ticks) passes by
the kernel's own invariant; fixture replay, storage permutation, and
disabled-gate equality are carried by the committed suite and
`scripts/verify-phase14-determinism.sh` on both hosts.

## Reading notes, locked with the rules

- Cue values in the MateChoice record are f32 cues times 1,000
  truncated toward zero; the census arithmetic is exact from there. The
  truncation convention is part of the statistic's definition.
- The TRUE cues are recorded even under P's scramble; the scramble
  changes which candidate wins, never what is recorded about it.
- `single_candidate` counts are part of every report: the pilot's ~93%
  single-candidate rate is why the statistic weights by candidates and
  why power rides on the ~9,000 informing choices per world.
- A null with the A and P worlds byte-identical (choices_total equal,
  every deviation equal) is the neutral-equivalence signature: it says
  preference never left neutral, which is a reachability statement
  about selection on the band, not a failure of the mechanism - the
  mechanism's correctness is pinned by Gate E and the fixture, not by
  this campaign.
- `died_growing` equals juvenile deaths when every juvenile death is a
  growth-incomplete organism; a divergence would mean adults died while
  classed juvenile and is a detector fault to investigate, not a result.

## Claim ceilings

A passing C14.2 licenses "pairings are non-random with respect to a
perceived cue, and the information is what the scramble destroys" -
never "mate quality", "sexual selection succeeded", or any fitness
claim. C14.1's mortality clause licenses "juveniles die more", with the
authored juvenile multiplier neutralized so the excess is the body's,
not a config knob's. C14.3's ceiling: "scale drifted (or did not) under
choice", never "ornament".
