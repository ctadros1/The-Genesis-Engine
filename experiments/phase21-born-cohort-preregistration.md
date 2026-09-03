# Phase 21 born-cohort pre-registration (C21.1, C21.2, C21.3)

**STATUS: DRAFT, NOT LOCKED.** The pilot
(`experiments/phase21-born-cohort-pilot.campaign`, seeds 21901..21904,
the shipped Phase 20 world with the birth-site record on, 100,000
ticks) is read against the licence rule in
`planning/phase-21-born-cohort.md` - written and committed before the
pilot ran - and fills the `[PILOT]` slots; the record is then LOCKED
and committed before any confirmatory world runs.

## Question

A born organism lives a median ~268 ticks against a materialized one's
~1,784 with a comparable start (D-134; `runs/phase20-lineage-
confirmatory-0xa8d0b4c2ab68ba74/demography.txt`). Is the difference
made of the ground it is born on (food in its birth cell), the order it
eats in (it holds the highest ID in the world and both feeding passes
take in ID order - the youngest eats last, a fact pinned by
`tests/phase21_cohort.rs`), or the company it keeps (occupants of its
cell, all ahead of it)?

## World

Phase 20's base (64x64 coupled scratch, transition at defaults, shipped
field and mouth), 100,000 ticks, events on, **30 seeds** 21002..21031
(21001 refused at preflight; every seed probed generable). Two shapes,
one of which runs:

- **Observational** (`phase21-born-cohort-confirmatory-alone.campaign`):
  the shipped world alone, if the probe is not licensed.
- **Probe** (written only if licensed): O1 the shipped intake order
  against O2 youngest-first (`physiology.intake_order` Descending,
  `lifesim-intake-order-v2`, ALIF format 16), on the same 30 seeds.

## The licence rule, as written before the pilot

The probe is licensed if the pilot's per-world median block rho between
a born organism's completed lifespan and its occupants at birth is at
least as strong in magnitude as that between lifespan and food at
birth, taking the median over the four pilot worlds of each.

## Definitions (the census, `lifesim cohort`, lifesim-cohort-index-v1)

- Food at birth = primordial + monomer + biomass of the birth cell as
  the record carries it (waste, polymer, microbial density are not
  food and are reported apart).
- Block rho: Spearman (milli, ties averaged) over born organisms with a
  completed lifespan within fixed 20,000-tick admission blocks (a block
  with fewer than 30 such organisms is skipped and counted), the
  world's rho being the lower median over its qualifying blocks; the
  pooled whole-run rho reported beside it, never decided on.
- Maturity reached: completed lifespan at or past the record's own
  `maturity_ticks`, or censored past it.

## Primary endpoint (C21.1)

- Observational shape: per world, `rho_food_milli` and
  `rho_occupants_milli` (median block rhos). Count of worlds where
  `rho_food_milli >= SESOI_F` and, separately, where
  `rho_occupants_milli <= -SESOI_O`, against bars stated below.
  SESOI_F `[PILOT]`, SESOI_O `[PILOT]` (in rho milli; the smallest
  association that would matter is stated from the pilot's spread, not
  from its point estimates), bars `[PILOT]` of 30 with the binomial
  null clear rate stated.
- Probe shape: the born cohort's median completed lifespan, O2 minus
  O1, seed-paired directed count above SESOI `[PILOT]` ticks, bar
  `[PILOT]` of 30 with the sign-test power from the pilot's pair
  spread; the materialized cohort's median reported beside it.

**Expected, stated in advance**: place carries most of it - food rho
positive and clearing its SESOI in most worlds, occupants rho negative
and smaller - and the probe, if it runs, moves the born median up but
leaves it far below the materialized median.

## Reported beside it

C21.2 the site contrast (born vs materialized food medians and ratio;
occupancy medians; a magnitude reference, since materialization
selects the densest cells). C21.3 maturity reached and reproduced by
food quartile and by occupants 0/1/2/3+. The within-block rank
partials. Waste, polymer and microbial at the born sites.

## Power `[PILOT]`

## Hard gates

Both identities exact in-run (`check-interval 10000`); every event log
and manifest present; the reduction refuses anything missing or short;
no world at the entity cap.
