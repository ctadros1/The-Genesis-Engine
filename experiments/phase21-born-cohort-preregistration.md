# Phase 21 born-cohort pre-registration (C21.1, C21.2, C21.3)

**STATUS: LOCKED 2026-09-03**, before any confirmatory world ran. Two
pilots on the same four seeds, both archived: the first
(`runs/phase21-born-cohort-pilot-0xa7a9fd0e26445430`, the shipped world
with the record on) licensed the probe by the rule below; the second
(`runs/phase21-born-cohort-pilot-gate-0xdcc22be2abd31f7e`, O1 against
O2) supplied the pair spread the SESOI rule needs. Every rule, SESOI
and bar below was fixed before the pilot it draws on was read, and the
one expectation that was wrong is revised here with its reason, not
silently.

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

### Read 2026-09-03 against the pilot: LICENSED

`runs/phase21-born-cohort-pilot-0xa7a9fd0e26445430`, four worlds,
100,000 ticks, `lifesim cohort`:

| seed | born median / completed / censored | mat median | born food | mat food | born occ | rho food | rho occ | partial food | partial occ | matured by occ 0/1/2/3+ |
|---|---|---|---|---|---|---|---|---|---|---|
| 21901 | 244 / 15,868 / 528 | 1,762 | 0 | 12,677 | 1 | 290 | -281 | 132 | -135 | 893/1152/136/3 |
| 21902 | 192 / 8,971 / 356 | 1,709 | 0 | 15,519 | 1 | 297 | -344 | 137 | -283 | 512/626/60/5 |
| 21903 | 191 / 15,050 / 423 | 1,959 | 4 | 5,505 | 1 | 258 | -288 | 87 | -225 | 713/796/70/1 |
| 21904 | 228 / 12,190 / 405 | 1,702 | 0 | 12,632 | 1 | 256 | -324 | 145 | -213 | 722/846/72/2 |

Median block rho: food 274, occupants -306; |occupants| >= food in
every world and in the median - the probe is licensed. Two facts the
record carries forward: the median born organism starts in a cell with
**zero food** (primordial + monomer + biomass) against ~12,650 for a
materialized one, so the food quartiles collapse (q1 and q2 are both
zero); and with two or more occupants at birth almost no born organism
reaches maturity. Every block qualified (4 used, 0 skipped per world).

### The second pilot, required by the licence

The pair spread the probe's SESOI and power need cannot come from a
pilot without the probe arm, so the same four seeds ran again under
both orders (`phase21-born-cohort-pilot-gate.campaign`: O1 shipped, O2
youngest-first, the only difference between arms; 8 worlds, 0 failed):

| seed | O1 born median (completed/censored) | O2 born median (completed/censored) | delta | O1 mat | O2 mat | O1 rho food / occ | O2 rho food / occ |
|---|---|---|---|---|---|---|---|
| 21901 | 244 (15,868/528) | 2,171 (13,164/799) | +1,927 | 1,762 | 1,205 | 290 / -281 | 5 / 22 |
| 21902 | 192 (8,971/356) | 2,150 (9,640/809) | +1,958 | 1,709 | 1,176 | 297 / -344 | 39 / -60 |
| 21903 | 191 (15,050/423) | 2,037 (10,607/537) | +1,846 | 1,959 | 1,227 | 258 / -288 | -18 / 31 |
| 21904 | 228 (12,190/405) | 2,597 (10,134/891) | +2,369 | 1,702 | 1,170 | 256 / -324 | 21 / 7 |

Median pair difference +1,942.5 ticks (min +1,846, max +2,369);
materialized median delta -545. Born organisms reaching their own
maturity per world: O1 median ~1,611, O2 ~9,644; reproducing: ~880 to
~8,247. Under O2 both associations vanish (food rho median 13,
occupants 14.5): with the youngest eating first, where a newborn is
born and beside whom no longer predicts how long it lives.

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

- Observational shape (reported beside the probe on the O1 arm, which
  is the shipped world): per world, `rho_food_milli` and
  `rho_occupants_milli` (median block rhos). Count of worlds where
  `rho_food_milli >= 128` and, separately, where `rho_occupants_milli
  <= -140` - each SESOI half the first pilot's smallest magnitude (food
  256, occupants 281), so a world clears only if its association is at
  least half the weakest the pilot saw; bar 22 of 30 for each (the
  sign-test table below applies: under a null in which a world clears
  by chance half the time, 22 has probability 0.008). Stated now, from
  the first pilot, before the second pilot or any confirmatory world.
- Probe shape: the born cohort's median completed lifespan, O2 minus
  O1, seed-paired directed count above the SESOI of **971 ticks** (the
  rule below, evaluated on the second pilot). **Bar 22
  of 30**, fixed before the second pilot was read: under no directed
  effect a count of 22 or more has probability 0.0081 (21: 0.021; 20:
  0.049; 23: 0.0026), and the bar detects a per-pair clearing
  probability of 0.8 with power 0.87 and of 0.9 with 0.998 (0.7: 0.43,
  stated so a null at the bar is read as "fewer than four pairs in
  five clear the SESOI", not as "no effect"). The SESOI is set from the
  second pilot's pair spread as stated below, never from its point
  estimate. The materialized cohort's median is reported beside it and
  is expected not to move (a materialized organism is rarely the
  youngest in its cell).

**Expected, as first stated**: place carries most of it - food rho
positive and clearing its SESOI in most worlds, occupants rho negative
and smaller - and the probe, if it runs, moves the born median up but
leaves it far below the materialized median. **Revised at the second
pilot, before this lock, with the reason**: that expectation was wrong
in its second half. The probe moves the born median tenfold, above the
materialized median, and erases both associations; the order is the
term, and place and company were its shadow (the cell a newborn is
born into is grazed, in the same tick, by the organisms that eat
before it). The confirmatory's expected outcome is therefore: C21.1
met at the bar on the probe shape; the O1 arm clearing both
observational bars; the materialized median falling under O2 by a few
hundred ticks (what the youngest take, the eldest no longer get).

## Reported beside it

C21.2 the site contrast (born vs materialized food medians and ratio;
occupancy medians; a magnitude reference, since materialization
selects the densest cells). C21.3 maturity reached and reproduced by
food quartile and by occupants 0/1/2/3+. The within-block rank
partials. Waste, polymer and microbial at the born sites.

## Power

Observational shape (the O1 arm): each count-of-worlds bar of 22 of 30
is a binomial test against a null in which a world clears its SESOI by
chance half the time (probability 0.008 of 22 or more); the first
pilot cleared both SESOIs in 4 of 4 worlds at double the SESOI or
more, so the power at that rate is indistinguishable from one, and a
null at the bar would mean fewer than four worlds in five carry the
association at half the pilot's weakest strength. Probe shape: the sign-test table above; the SESOI in
ticks is the larger of 20 ticks (a tenth of the born median, the
smallest change in a ~200-tick life the record would call a change)
and half the second pilot's median pair difference - **971 ticks** (half
of 1,942.5). All four pilot pairs clear it by a factor of two; at a
per-pair clearing probability of 0.9 or above the bar of 22 has power
0.998 or better, and a confirmatory null at the bar would mean fewer
than four pairs in five clear 971 ticks, which the pilot makes
unlikely but the thirty-world run decides.

## Hard gates

Both identities exact in-run (`check-interval 10000`); every event log
and manifest present; the reduction refuses anything missing or short;
no world at the entity cap.
