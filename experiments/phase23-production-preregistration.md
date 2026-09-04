# Phase 23 production-dose pre-registration (C23.1, C23.2, C23.3)

**STATUS: DRAFT, NOT LOCKED.** The pilot
(`experiments/phase23-production-pilot.campaign`, seeds 23901..23904,
nine arms, 100,000 ticks) licenses the top dose, calibrates the SESOI
and measures the cost per world, each by a rule written here before it
runs; the record is then LOCKED and committed before any confirmatory
world runs.

## Question

The shipped intake order - the youngest eats last - is a sufficient
cause of the born cohort's short life and of the rarity of multi-
module lineages (D-135, D-136). It matters because a grazed cell
refills slowly against a take: the field produces 2 milli per cell per
step at one step per tick, a fed organism takes up to 200 per tick. On
the field's own terms: at what production does the order stop
mattering - and does production alone, under the shipped order, carry
the born cohort to where the other order carried it?

## World and arms

Phase 22's base, 100,000 ticks, events on, the field series every
1,000 ticks, on **30 matched seeds** 23001..23010 23012..23031 (23011
refused at preflight; every other seed probed generable), seven arms
differing in exactly two hashed fields, `chemistry.production_milli_per_step`
and `physiology.intake_order`:

- shipped order: **P1** the shipped 2 (the control), **P4** 8, **P16**
  32, **P64** 128 - refill against a single take of one, four, sixteen
  and sixty-four percent per tick - and **PT**, the top dose past one
  take: `[PILOT]` (P128 at 256, 128 percent, or P256 at 512, 256
  percent);
- the other order (youngest first, `physiology.intake_order
  descending`): **P1D** at the shipped 2 and **PTD** at the top dose.

**The top dose, by rule**: the highest of P256, P128 at which all four
pilot worlds under both orders are generable and none trips the
entity-cap gate (final population at 4,000, or any capacity rejection,
or any materialization deferred for capacity). If neither qualifies,
PT is P64 and the phase reports that D-136's question cannot be put
inside the entity cap. A dose dropped by this rule is reported; none
is dropped after the lock.

## Primary endpoint (C23.1)

Per world, `lifesim cohort`'s born median completed lifespan. PT
against P1, seed-paired: the count of pairs where PT's born median
exceeds P1's by the SESOI, against a **bar of 22 of 30** (alpha 0.008
under no directed effect; power 0.87 at a per-pair clearing rate of
0.8), with the median pair difference and its interval. P4, P16 and
P64 against P1 the same way, reported.

**The SESOI** is the larger of 20 ticks (a tenth of the shipped born
median) and a tenth of the pilot's median born median over the PTD
worlds - a tenth of the level at which the order contrast is read:
`[PILOT]` ticks.

**Intervals** are 95 percent percentile intervals of the median paired
difference from 10,000 resamples seeded with 23, so the same numbers
always give the same interval.

**Expected, stated in advance**: met at every rung - production raises
every cohort's life.

## The order at the top dose (C23.2)

PTD against PT, seed-paired, three readings stated in advance:

- (i) **the order still matters**: the count of pairs where PTD's born
  median exceeds PT's by the SESOI is at or above 22 of 30;
- (ii) **the order stopped mattering**: the interval of the median
  paired difference lies within minus SESOI and plus SESOI
  (equivalence), with the count of pairs within that band reported;
- (iii) **undecided**: neither, reported as such - a smaller campaign
  cannot tell "no effect" from "underpowered", and this one says which
  it is by (ii)'s band.

P1D against P1 reported the same way (Phase 21's contrast replicated
on these seeds).

**Expected, stated in advance**: (i) or (iii). The arithmetic says a
cell refills for one eater at P128, but a newborn's cell refills for
it only after every elder sharing the cell has taken, and the born
cohort's disadvantage shrinks past a take without vanishing. (ii) at
PT would be the finding D-136 asked for, and it is stated here so it
cannot be narrated afterwards.

## The reference, on the same seeds (C23.3)

PT against P1D, seed-paired: **reached** when the interval's lower
bound is at or above minus the SESOI (non-inferiority: the shipped
order at the top dose lives at least as long, less the SESOI, as the
other order did at the shipped dose); the count of pairs at or above
the reference less the SESOI reported. P1D's born median is read
against Phase 21's O2 (2,191 on seeds 21002..21031) and Phase 22's O2
(2,172.5 on 50 seeds), a byte-identical base on different seed sets -
an 18.5-tick spread that is the reference's own uncertainty, declared
here. **Expected**: not reached.

## Reported beside it

C23.4 per arm, worlds with any second-generation two-module organism
and the pooled count (Phase 20's census), read against Phase 22's 8 of
50 shipped and 36 of 50 under the other order. C23.5 per arm:
population, materialized count (cohort completed plus censored, equal
to the manifest's `transition_materialized` and the field series' final
`materialized` or the reduction refuses), births, field mass (final
chem plus microbial from the field series), born-site food and born
occupants medians, the entity-cap gate as defined above and the
per-tick materialization deferral beside it. The materialized median
beside the born median at every arm.

## The reduction, pinned before the lock

`experiments/results/phase23-production-reduction.py` is new and is
checked on the archived Phase 21 confirmatory (O1 as the control, O2
as the order arm, `--pin-archive` because that manifest predates the
transition columns): born medians 222 against 2,191, materialized
medians 1,752.5 against 1,190.5, materialized counts 2,528 against
2,525.5, field mass 171,055,313.5 against 112,578,576.5 milli, born
occupants 1 against 1; 30 of 30 pairs above a 20-tick SESOI, 0 of 30
within it, delta median 1,969, interval [1,944, 2,007]; 1 against 16
worlds with any second-generation organism; the cap gate 0 of 30; the
field identity exact on all 60 series. It refuses the archive without
the pin flag (60 refusals) and a cohort line removed (1 refusal). The
confirmatory is read without the pin flag; the header says which.

## Hard gates

Both identities exact in-run (`check-interval 10000`); every event
log, field series and manifest present; the field identity exact at
every sample; the three materialized counts equal per world; the
reduction refuses anything missing; a world at the entity cap is
reported as such and its arm's counts read with it.
