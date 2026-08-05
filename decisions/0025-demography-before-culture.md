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
  possible: ontogeny as developmental growth requires the Phase 10 module
  body, and sexual selection requires the Phase 13 perception channels.
  Neither can precede its dependency.
- **Split the phase along its dependency line** and move only the half that
  can move.

## Proposed Decision

Split the former Phase 13 in two.

**Phase 8, Demography And Life History** executes **after Phase 7
(contest), before Phase 9 (genome successor)**: allometric metabolism,
thermoregulation, senescence, non-food extrinsic mortality, a juvenile
mortality and maturation constraint, the life-history investment tradeoff,
and death-cause accounting as a reported distribution.

It has no unmet prerequisites. Allometry uses the existing body-scale gene,
senescence uses age, the life-history tradeoff uses the existing
reproduction-investment gene, and thermoregulation uses the temperature
field the Phase 6 climate slice already built. Its only prerequisite is
Phase 7, for damage as a non-food mortality source.

**Phase 14, Ontogeny And Sexual Selection** executes **after Phase 13
(social)**: developmental growth of a module body, mate choice on perceived
phenotype, and the optional disease slice. These are late because they
cannot be early, not because of a scheduling preference.

### The mechanism that motivates the split

Surplus does not come from more food. It comes from **mortality that is not
food-driven**. A population held below carrying capacity by predation,
senescence, or accident has per-capita abundance at the same food supply.
That is the ecological route to the slack every non-utilitarian and
culturally expensive behavior requires, and it is exactly what Phase 8
delivers.

The demographic half is therefore not a realism nicety. It is the
precondition under which the culture stack is measurable at all.

### What ADR-0017 keeps

Its precedence order (determinism first, bounded state second, measured cost
third) is unchanged. Its non-transfer rule is unchanged and now applies in
two smaller pieces rather than one large one. The genetics-realism placement
in Phase 9 is unchanged.

What is superseded is the single sentence placing physiology after the
culture stack, and only for the demographic half.

## Consequences

Positive: the culture stack runs in a world where its endpoints are
reachable; a null from Phases 11 to 13 becomes interpretable; the
`max_entities`-as-ecology artifact is caught before four phases of campaigns
depend on it; and the surplus question becomes measurable rather than
hypothetical.

Negative and accepted:

- **Per-organism cost arrives earlier and applies to every subsequent
  phase**, reducing generations reachable per unit of compute across the
  whole remaining programme rather than only the last few phases. This is
  the real price and it is not small.
- **Phase 7 results do not transfer across 8.** That is one phase of
  results rather than the four that would have been invalidated under the
  old order, which is the trade being made.
- Two phase documents where there was one, and a numbering that is no longer
  monotonic until it can be repaired.
- If demography turns out *not* to be gating the culture stack, this
  reordering bought nothing and cost throughput. The C8.1 and C8.2
  criteria are what determine that, and they are worth running regardless
  because the death-cause distribution is diagnostic of the world's health
  either way.

### Numbering

When this decision was taken the split files carried provisional `13a` and
`13b` labels. A full renumber touches the roadmap, decision log, backlog,
manifest, and every cross-reference, and those files were held uncommitted
by a concurrent Phase 6 implementation session, so renumbering then would
have clobbered in-progress work.

**The renumber has since been done** in one atomic pass, recorded as D-059.
`13a` became Phase 8 and `13b` became Phase 14, with 8-12 shifting up to
9-13 and 14-17 to 15-18. The execution order this ADR argues for did not
change; only the labels did.

Three prior renumbers each left defects that took a sweep to find, and any
future pass should check all three patterns explicitly:

- Multi-number lists where only the first number was mapped, because the
  separator pattern did not match the connector actually used (`", and N"`
  and `"N / M"` were both missed, producing duplicates like
  `"Phases 7, 12, and 12"`).
- Bare numbers in table cells and parentheticals, which stay put while
  their meanings move. Reassign these **by content**, never by the mapping:
  two such tables were stale from the original numbering and had survived
  two passes untouched.
- Numbers joined by a connector outside the separator set  -  `"Phase 8 or
  9"` mapped to `"Phase 9 or 9"`, which no duplicate-in-a-list check
  catches.

The execution order is stated at the top of each phase document and in
`docs/19-implementation-roadmap.md`, which is authoritative.

## Performance Implications

Negative and measured rather than estimated. Phase 8 records the
per-organism cost delta against Phase 7 and the resulting change in ticks
per second per world, and that figure now applies to Phases 9 through 17
rather than to Phase 17 alone.

Phase 8 additionally records the population equilibrium reached once
`max_entities` is raised above it, which is currently unknown and which
bounds the achievable population for every later phase.

## Operational Implications

None beyond the throughput reduction.

## Revisit Conditions

- C8.1 shows the death-cause distribution does not become mixed, meaning
  demography did not fix what it was moved to fix.
- The measured throughput cost is severe enough that running it before four
  phases is worse than running invalid campaigns, which would be a genuine
  and unpleasant trade to re-examine with numbers in hand.
- Ecology cannot be made to bind below an affordable `max_entities`, which
  would be a finding about world size rather than about demography.

## Evidence Required To Accept

- Phase 8 criteria C8.1 through C8.3: starvation ceases to dominate,
  population sits below carrying capacity, and ecology binds rather than the
  guard.
- The measured per-organism cost delta and its throughput consequence.
- Phase 7 fixture reproduced exactly with demography disabled.
