# ADR-0025: Demography Executes Before The Culture Stack

Status: Proposed
Date: 2026-08-04
Author: Ordering revision

**Amends ADR-0017**, which placed physiology after the culture stack.
ADR-0017 keeps its status and the rest of its content; this record reverses
its placement argument for one half of that phase and states why.

## Context

The former Phase 13 sat after the culture stack (learning, artifacts,
social) on ADR-0017's reasoning: physiology changes the entire selection
landscape, so introducing it late avoids invalidating prior cross-condition
results, at the price of a stated non-transfer.

That argument contains an assumption that does not survive inspection: **it
assumes the culture-stack results would otherwise be valid.**

The Phase 2 long-run record says they would not be:

    199,871 starvation deaths
        180 old-age deaths
      population pinned at the 5,000 `max_entities` ceiling

Roughly 99.9 percent of mortality was starvation, and the population sat on
a process-safety guard rather than an ecological equilibrium. Three things
follow, and the third is decisive:

1. No per-capita surplus can exist. Every organism is one bad tick from
   death.
2. `max_entities` is functioning as the carrying capacity, so measured
   dynamics are an artifact of a memory limit.
3. **Every culture-stack criterion is energetically gated.** Costly
   signalling, object manipulation, carrying, placing, and social learning
   all cost energy that nobody has. Campaigns run in that world would
   return nulls caused by starvation rather than by anything about
   transmission or accumulation.

The last point is what forces the change. ADR-0022 A7 already committed the
project to distinguishing a negative result from an underpowered one. A null
produced by universal starvation is not a finding about culture; it is a
finding about the ecology, and it would be indistinguishable from the real
thing.

## Options Considered

- **Keep the existing order.** Cheapest in scheduling, and it spends the
  most expensive campaigns in the plan on a world where their endpoints
  cannot be reached for reasons unrelated to their hypotheses.
- **Move all of the former Phase 13 before the culture stack.** Not
  possible: ontogeny as developmental growth requires the Phase 9 module
  body, and sexual selection requires the Phase 12 perception channels.
  Neither can precede its dependency.
- **Split the phase along its dependency line** and move only the half that
  can move.

## Proposed Decision

Split the former Phase 13 in two.

**Phase 13a, Demography And Life History** executes **after Phase 7
(contest), before Phase 8 (genome successor)**: allometric metabolism,
thermoregulation, senescence, non-food extrinsic mortality, a juvenile
mortality and maturation constraint, the life-history investment tradeoff,
and death-cause accounting as a reported distribution.

It has no unmet prerequisites. Allometry uses the existing body-scale gene,
senescence uses age, the life-history tradeoff uses the existing
reproduction-investment gene, and thermoregulation uses the temperature
field the Phase 6 climate slice already built. Its only prerequisite is
Phase 7, for damage as a non-food mortality source.

**Phase 13b, Ontogeny And Sexual Selection** executes **after Phase 12
(social)**: developmental growth of a module body, mate choice on perceived
phenotype, and the optional disease slice. These are late because they
cannot be early, not because of a scheduling preference.

### The mechanism that motivates the split

Surplus does not come from more food. It comes from **mortality that is not
food-driven**. A population held below carrying capacity by predation,
senescence, or accident has per-capita abundance at the same food supply.
That is the ecological route to the slack every non-utilitarian and
culturally expensive behavior requires, and it is exactly what Phase 13a
delivers.

The demographic half is therefore not a realism nicety. It is the
precondition under which the culture stack is measurable at all.

### What ADR-0017 keeps

Its precedence order (determinism first, bounded state second, measured cost
third) is unchanged. Its non-transfer rule is unchanged and now applies in
two smaller pieces rather than one large one. The genetics-realism placement
in Phase 8 is unchanged.

What is superseded is the single sentence placing physiology after the
culture stack, and only for the demographic half.

## Consequences

Positive: the culture stack runs in a world where its endpoints are
reachable; a null from Phases 10 to 12 becomes interpretable; the
`max_entities`-as-ecology artifact is caught before four phases of campaigns
depend on it; and the surplus question becomes measurable rather than
hypothetical.

Negative and accepted:

- **Per-organism cost arrives earlier and applies to every subsequent
  phase**, reducing generations reachable per unit of compute across the
  whole remaining programme rather than only the last few phases. This is
  the real price and it is not small.
- **Phase 7 results do not transfer across 13a.** That is one phase of
  results rather than the four that would have been invalidated under the
  old order, which is the trade being made.
- Two phase documents where there was one, and a numbering that is no longer
  monotonic until it can be repaired.
- If demography turns out *not* to be gating the culture stack, this
  reordering bought nothing and cost throughput. The C13a.1 and C13a.2
  criteria are what determine that, and they are worth running regardless
  because the death-cause distribution is diagnostic of the world's health
  either way.

### Numbering

The files keep provisional `13a` and `13b` labels rather than being
renumbered into a monotonic sequence. A full renumber touches the roadmap,
decision log, backlog, manifest, and every cross-reference, and those files
were held uncommitted by a concurrent Phase 6 implementation session when
this decision was made. Renumbering them would have clobbered in-progress
work.

The execution order is stated at the top of each phase document and in
`docs/19-implementation-roadmap.md`, which is authoritative.

**The monotonic renumber remains outstanding**, and it should be done in one
atomic pass rather than incrementally. Two prior renumbers each left defects
that took a sweep to find: stale multi-number lists where only the first
number was mapped ("Phases 8, 8, and 10"), and table rows whose bare phase
numbers stayed put while their meanings moved. Check both patterns
explicitly afterwards. Suggested target sequence: 5 headless, 6
biomes/origins, 7 contest, 8 demography, 9 genome, 10 morphology, 11
learning, 12 artifacts, 13 social, 14 ontogeny/sexual selection, 15
abiogenesis, 16 multicellularity, 17 era detection, 18 parallelism.

## Performance Implications

Negative and measured rather than estimated. Phase 13a records the
per-organism cost delta against Phase 7 and the resulting change in ticks
per second per world, and that figure now applies to Phases 8 through 16
rather than to Phase 16 alone.

Phase 13a additionally records the population equilibrium reached once
`max_entities` is raised above it, which is currently unknown and which
bounds the achievable population for every later phase.

## Operational Implications

None beyond the throughput reduction.

## Revisit Conditions

- C13a.1 shows the death-cause distribution does not become mixed, meaning
  demography did not fix what it was moved to fix.
- The measured throughput cost is severe enough that running it before four
  phases is worse than running invalid campaigns, which would be a genuine
  and unpleasant trade to re-examine with numbers in hand.
- Ecology cannot be made to bind below an affordable `max_entities`, which
  would be a finding about world size rather than about demography.

## Evidence Required To Accept

- Phase 13a criteria C13a.1 through C13a.3: starvation ceases to dominate,
  population sits below carrying capacity, and ecology binds rather than the
  guard.
- The measured per-organism cost delta and its throughput consequence.
- Phase 7 fixture reproduced exactly with demography disabled.
