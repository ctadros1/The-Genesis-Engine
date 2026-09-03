# Phase 20 lineage pre-registration (C20.1, C20.2, C20.3)

**STATUS: LOCKED 2026-09-03**, before any confirmatory world ran. The
pilot (`experiments/phase20-lineage-pilot.campaign`, seeds 20901..20904,
the shipped Phase 19 v2 world with the body-composition record on,
100,000 ticks; archived at `runs/phase20-lineage-pilot-0x06fb2fdcf55662df`)
was read against the decision tree in
`planning/phase-20-second-module-lineage.md` (written and committed
before the pilot ran, 41e9930); the reading and the branch are recorded
below and in the plan. No threshold is weakened after the data; a
different bound is a different phase.

## Question

Bodies above one module appear under coupling v2 once reproduction runs,
one organism at a time, and no multi-module lineage establishes itself
(D-133). The arithmetic rules price out (ADR-0035; a second module of
five of seven types leaves the basal multiplier on its clamp floor and
confers capability). Is the reason that a multi-module organism does not
reproduce (mate access), that its children are refused (viability), or
that its children do not carry the module (segregation) - and does any
lineage cross one reproduction?

## World

`experiments/phase20-lineage-confirmatory.campaign` (written after the
branch is read): the Phase 19 confirmatory's v2 base (64x64 scratch,
transition at defaults, floor 4,000, influx cap 64 per 100 ticks,
pairing threshold 7,000, consumption Q16_ONE, `max_entities` 4,000),
100,000 ticks, events on, **fifty seeds per arm** - 20001..20039
20041..20051 (20040 refused at preflight; every other seed probed
generable before this record) - because the primary endpoint is a rare
outcome whose baseline is expected at or near zero (ADR-0022's
rare-outcome clause).

## Stated before the pilot: the reproduction test's measurements

`crates/sim-core/tests/phase20_reproduction.rs` (the kernel's own
recombine, mutate, develop and node-budget sequence with keyed draws),
run 2026-09-03 before any pilot world:

| cross | draws | refused non-viable | refused budget | one module | two or more |
|---|---|---|---|---|---|
| unicell x unicell | 1,000 | 0 | 1 | 999 | 0 |
| two-module x unicell | 1,000 | 0 | 0 | 512 | 488 |
| two-module x itself | 1,000 | 0 | 2 | 231 | 767 |

The two-module genome (a duplicated gut) was found at keyed draw 6,398
from the unicell cross; its compatibility distance to the unicell is
0.0000 (threshold 0.5). Read: transmission is Mendelian (half against a
unicell, three quarters against itself), viability refuses nothing, and
mate compatibility bars nothing. The pilot therefore decides between
"a two-module organism never pairs" (Branch A, lever: the pairing
energy gate) and "second-generation organisms exist and Phase 19's peak
was a sampling artefact" (Branch D); Branches B and C are already
disfavoured by these numbers and would be a surprise the record must
explain.

## The branch taken: A by its letter, run as the one-arm shape

The pilot's U1 census (`lifesim lineage`, four worlds):

| seed | births | multi (all born) | multi parents | 2nd gen | multi median | cohort median (completed/censored) | born parents | births with a born parent | rej energy/cap/place | refused budget | compositions |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 20901 | 14,483 | 6 | 0 | 0 | 191 | 259 (4,160/151) | 1,803 | 10,417 | 0/0/1 | 1,215 | gut+gut x5, gut+motor x1 |
| 20902 | 17,981 | 7 | 0 | 0 | 218 | 238 (4,834/249) | 1,303 | 12,951 | 0/0/1 | 905 | gut+gut x7 |
| 20903 | 13,677 | 2 | 0 | 0 | 261 | 144 (1,326/51) | 834 | 8,630 | 0/0/0 | 810 | gut+gut x2 |
| 20904 | 11,174 | 2 | 0 | 0 | 133 | 140 (1,368/67) | 822 | 7,263 | 0/0/0 | 917 | gut+gut x2 |

`multi_parents` is zero in every world: Branch A by the tree's letter.
Its lever was the pairing energy gate; the gate refused no pairing in
any world, so a contrast on it would be a null bought in advance and it
is not run. What the census shows instead: born organisms do reproduce,
each rarely (born parents / births = 12.4, 7.2, 6.1, 7.4 percent), with
world-wide born median lifespans 293 / 235 / 220 / 210 ticks against a
trait-derived maturity of at least 400; a two-module organism's
lifespan matches its cohort's (median difference -13.5 ticks) and 0 of
17 reproducing has probability ~0.09 at the cohort's rate. Appearances:
17 in 57,315 births = 2.97 per 10,000 - about eight times Phase 19's
series-based count, which sampled every 500 ticks and missed most
~200-tick lives.

**The confirmatory is the shipped world alone**
(`experiments/phase20-lineage-confirmatory-alone.campaign`, 50 seeds),
and the two-arm file is removed from the tree with this record.

## Primary endpoint (every branch)

**C20.1**: second-generation multi-module organisms per world from
`lifesim lineage` - born, two or more modules, at least one parent with
two or more, and a module count not above that parent's (inherited, not
a fresh duplication). Beside it, on every branch: the birth-normalized
rate (per 10,000 births).

- **The decision rule (one arm, 50 worlds)**: the pooled
  second-generation rate per 10,000 births with its exact one-sided
  97.5 percent upper bound, against a **SESOI of 0.5 per 10,000
  births**, and beside it the **cohort prediction** computed by the
  reduction from the same campaign's own numbers:

      predicted rate = (appearances / births)
                     x (born parents / births)
                     x (births with a born parent / born parents)
                     x 0.5 (the reproduction test's transmission)

  On the pilot: 17 / 57,315 x 0.0831 x 8.24 x 0.5 = **1.016 per 10,000
  births**, 5.8 expected second-generation organisms over the pilot's
  births against 0 observed (Poisson probability ~0.003 - a deficit the
  four worlds cannot settle and the fifty are run to). Three readings,
  stated in advance: (i) the observed rate's interval contains the
  prediction - multi-module organisms reproduce like their cohort and
  lineages exist at the rarity the arithmetic says (Branch D's
  substance); (ii) the upper bound is below the SESOI while the
  prediction is above it - the second module costs reproduction, and
  C20.2 says how; (iii) zero everywhere with the prediction above the
  bound - a gate, named from the counters. The reproduction test's
  counts are the transmission term and are recorded as such.

## Reported beside it

- **C20.2**: multi-module completed lifespan against the matched
  one-module cohort (admitted within 2,000 ticks of a multi-module
  admission), completed and censored counts for both, per world and as
  a median of differences. Descriptive.
- **C20.3**: the composition multiset of multi-module bodies and the
  child-minus-parent added-type histogram. Descriptive.
- Per world: births, population, materialized, nonviable_bodies,
  refused_node_budget, the free-lunch gate (no world at the entity cap).

## Expected outcomes, recorded in advance

Branch B or C; a null under the equivalence bound; a mechanism named by
the reproduction test. Stated as the honest prior, not as a hope.

## Hard gates

Both identities exact in-run (`check-interval 10000`); every series,
event log and manifest present; the reduction refuses anything missing
or short; no world at the entity cap or reported as such.
