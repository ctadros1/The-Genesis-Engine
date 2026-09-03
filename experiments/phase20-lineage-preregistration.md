# Phase 20 lineage pre-registration (C20.1, C20.2, C20.3)

**STATUS: DRAFT, NOT LOCKED.** The pilot
(`experiments/phase20-lineage-pilot.campaign`, seeds 20901..20904, the
shipped Phase 19 v2 world with the body-composition record on, 100,000
ticks) is read against the decision tree in
`planning/phase-20-second-module-lineage.md` - written and committed
before the pilot ran - and fills the `[PILOT]` slots; the record is then
LOCKED and committed before any confirmatory world runs.

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

## The branch taken

`[PILOT]` - recorded here with the pilot's U1 census (per world:
multi_total, multi_born, multi_parents, multi_offspring_total,
second_generation, cohort medians, compositions, added types, and the
manifest's nonviable_bodies and refused_node_budget) and the tree's
branch it lands on: A (no offspring -> contrast the pairing energy
gate lowered against the shipped world; see the plan's tree), B (offspring,
refused children -> shipped world alone + reproduction test), C
(offspring, viable one-module children -> shipped world alone +
reproduction test), or D (second-generation organisms exist -> two
horizons).

## Primary endpoint (every branch)

**C20.1**: second-generation multi-module organisms per world from
`lifesim lineage` - born, two or more modules, at least one parent with
two or more, and a module count not above that parent's (inherited, not
a fresh duplication). Beside it, on every branch: the birth-normalized
rate (per 10,000 births).

- Branch A: seed-paired directed count, open threshold minus shipped,
  SESOI `[PILOT]` worlds-with-any, bar `[PILOT]` of 50 with a
  simulation-based power statement from the pilot's per-birth
  appearance rate.
- Branches B and C: the shipped world alone on 50 seeds with an
  **equivalence bound**: the pooled second-generation rate below
  `[PILOT]` per 10,000 births (the SESOI), with its exact binomial
  upper interval, so "no transmission" is distinguishable from
  "underpowered"; the reproduction test's four counts (refused
  non-viable, refused budget, one module, two or more) over 1,000 draws
  against a unicell genome and 1,000 against itself are the named
  mechanism.
- Branch D: the count at 60,000 and at 100,000 ticks on the same seeds.

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
