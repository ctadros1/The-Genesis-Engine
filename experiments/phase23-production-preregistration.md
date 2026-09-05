# Phase 23 production-dose pre-registration (C23.1, C23.2, C23.3)

**STATUS: LOCKED 2026-09-04**, after the pilot
(`experiments/phase23-production-pilot.campaign`, seeds 23901..23904,
nine arms, 100,000 ticks; archived at
`runs/phase23-production-pilot-0xca50383c46389d13`, 36 worlds, 0 failed,
8,537 s at 8 workers on the VM) and before any confirmatory world runs.
The pilot licensed the top dose, calibrated the SESOI and measured the
cost per world; where its findings broke a rule's premise, the rule is
amended below with the finding beside it, dated, and the original kept.

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

- shipped order: **P1** the shipped 2 (the control), **P64** 128 -
  refill against a single take of sixty-four percent per tick, the
  highest rung under a take - **P128** 256 (128 percent, one eater's
  take within a tick) and **P256** 512 (256 percent), the top dose
  **PT = P256** (pilot, below);
- the other order (youngest first, `physiology.intake_order
  descending`): **P1D** at the shipped 2, **P128D** and **P256D** at
  the two rungs past a take (**PTD = P256D**).

**The top dose, by rule (as drafted)**: the highest of P256, P128 at
which all four pilot worlds under both orders are generable and none
trips the entity-cap gate (final population at 4,000, or any capacity
rejection, or any materialization deferred for capacity). If neither
qualifies, PT is P64 and the phase reports that D-136's question cannot
be put inside the entity cap.

**Pilot (2026-09-04) and the amendment.** Every pilot world under every
order was generable. The entity-cap gate tripped in 3 of 4 worlds at
P16 and in 4 of 4 at P64, P128, P128D, P256 and P256D - final
population 4,000 in each - and in every one of them through the
materialization path alone: `capacity_rejections` 0, the manifest's
`transition_deferred_capacity` between 2.5 and 9.1 million per world
(the column the harness gained for this phase; without it the gate
would have read only the population clause). The rule as drafted
therefore qualifies no rung, and its fallback, P64, is capped as well:
the cap is a property of this field regime at every dose from 16x up,
not a defect of one rung, and dropping every capped dose would drop the
question. Amended: the entity-cap gate is reported per arm as the
ecology (C23.5) and disqualifies nothing; the top dose is **P256**, the
rung at which the pilot's born cohort lives to the age ceiling
(`max_age_ticks` 36,000) under both orders in 3 of 4 worlds, and
**P128/P128D are kept** because the pilot brackets the transition
between them and P256 (P128 ascending at the ceiling in 1 of 4, P128D
in 4 of 4); **P4 and P16 are dropped for cost** (a capped world costs
about 46 minutes on the VM; the sub-take shape they carry - a born
median 52 and 75 ticks below the control, intervals excluding zero on
4 seeds - is represented by P64 at 72 below). Seven arms, 210 worlds,
estimated 14 hours at 8 workers.

## Primary endpoint (C23.1)

Per world, `lifesim cohort`'s born median completed lifespan. PT
against P1, seed-paired: the count of pairs where PT's born median
exceeds P1's by the SESOI, against a **bar of 22 of 30** (alpha 0.008
under no directed effect; power 0.87 at a per-pair clearing rate of
0.8), with the median pair difference and its interval. P4, P16 and
P64 against P1 the same way, reported.

**The SESOI (as drafted)** is the larger of 20 ticks (a tenth of the
shipped born median) and a tenth of the pilot's median born median over
the PTD worlds - a tenth of the level at which the order contrast is
read. **Pilot and the amendment**: the pilot's P256D born medians are
36,000 in 4 of 4 worlds - the age ceiling - so the drafted formula gives
3,600 ticks for every contrast, and applied to the contrasts read at
other levels it is absurd on the pilot itself: the other order at the
shipped dose, a 1,905-tick effect on four of four pairs, would read as
"equivalent within the band", and the reference would count as reached
with a world 2,086 ticks below it. The formula named one level for
every contrast; the words named the level of each. Amended to what the
words say, one SESOI per contrast, the larger of 20 and a tenth of the
pilot's median born median over the worlds of the arm the contrast is
read against, fixed here from the pilot and never from the data
reduced: **21** for every contrast against the control (P1 at 213:
the rungs, the top dose, and P1D against P1), **20** for P128D against
P128 (P128 at 179), **3,600** for P256D against P256 (P256 at the
36,000 ceiling), **213** for P256 against the reference P1D (P1D at
2,130). The ceiling makes the order contrast at the top dose partly a
contrast of ceilings, so the reduction reports, beside every count, the
number of worlds per arm whose born median sits at the ceiling
(`--ceiling 36000`).

**Intervals** are 95 percent percentile intervals of the median paired
difference from 10,000 resamples seeded with 23, so the same numbers
always give the same interval.

**Expected, stated in advance**: met at every rung - production raises
every cohort's life. *Pilot observation (4 seeds, recorded before the
lock and not used to rewrite the expectation)*: the rungs below a take
LOWER the born median - P4 by 52, P16 by 75, P64 by 72 ticks, every
pair negative, intervals excluding zero - and P256 lifts it to the age
ceiling in 3 of 4 worlds (+35,775 median); P128 in 1 of 4. The
confirmatory decides; the expectation stands as written and, if the
pilot's shape holds, is refuted below a take and met above it.

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
on these seeds), and **P128D against P128** the same way (the further
pair the amendment keeps: the bracket below the top dose).

**Expected, stated in advance**: (i) or (iii). The arithmetic says a
cell refills for one eater at P128, but a newborn's cell refills for
it only after every elder sharing the cell has taken, and the born
cohort's disadvantage shrinks past a take without vanishing. (ii) at
PT would be the finding D-136 asked for, and it is stated here so it
cannot be narrated afterwards. *Pilot observation*: at P128 the order
still matters (3 of 4 pairs, P128D at the ceiling in 4 of 4, P128 in 1
of 4); at P256, 3 of 4 pairs are exactly 0 (both orders at the
ceiling) and one is +35,825 (the ascending world at 175). Born
occupants per site are 30 to 57 at these doses against 1 at the
shipped dose: the cell a newborn shares holds dozens, which is why one
take's refill (P128) is not yet enough under the shipped order.

## The reference, on the same seeds (C23.3)

PT against P1D, seed-paired: **reached** when the interval's lower
bound is at or above minus the SESOI (non-inferiority: the shipped
order at the top dose lives at least as long, less the SESOI, as the
other order did at the shipped dose); the count of pairs at or above
the reference less the SESOI reported. P1D's born median is read
against Phase 21's O2 (2,191 on seeds 21002..21031) and Phase 22's O2
(2,172.5 on 50 seeds), a byte-identical base on different seed sets -
an 18.5-tick spread that is the reference's own uncertainty, declared
here. **Expected**: not reached. *Pilot observation*: P1D's born median
on the four pilot seeds is 2,130 (1,986..2,261); P256 reaches it in 3
of 4 pairs.

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

`experiments/results/phase23-production-reduction.py` is new (extended
after the pilot with `--order-pairs`, `--ceiling` and one SESOI per
contrast, and re-pinned identically) and is checked on the archived Phase 21 confirmatory (O1 as the control, O2
as the order arm, `--pin-archive` because that manifest predates the
transition columns): born medians 222 against 2,191, materialized
medians 1,752.5 against 1,190.5, materialized counts 2,528 against
2,525.5, field mass 171,055,313.5 against 112,578,576.5 milli, born
occupants 1 against 1; 30 of 30 pairs above a 20-tick SESOI, 0 of 30
within it, delta median 1,969, interval [1,944, 2,007]; 1 against 16
worlds with any second-generation organism; the cap gate 0 of 30; the
field identity exact on all 60 series; worlds at the age ceiling 0 of
30 in both arms. It refuses the archive without the pin flag (60
refusals) and a cohort line removed (1 refusal). The confirmatory is
read without the pin flag; the header says which. The confirmatory
invocation, fixed here: `--control P1 --rungs P64,P128 --top P256
--top-order P256D --reference-order P1D --order-pairs P128D:P128:20
--ceiling 36000 --sesoi 21 --sesoi-top 3600 --sesoi-reference 213
--bar 22`.

## Hard gates

Both identities exact in-run (`check-interval 10000`); every event
log, field series and manifest present; the field identity exact at
every sample; the three materialized counts equal per world; the
reduction refuses anything missing; a world at the entity cap is
reported as such and its arm's counts read with it (the pilot says
every world past 16x will be).
