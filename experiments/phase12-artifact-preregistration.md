# Phase 12 artifact campaign: C12.1, C12.2, C12.3 - pre-registration

Committed before the campaign was run. Companion to
`experiments/phase12-artifact-confirmatory.campaign` (campaign hash
**`0x36c2d68a9a0869da`**, preflighting as 4 conditions x 30 seeds = 120
worlds at 60,000 ticks; validated by preflight only, and the partial output
directory of the parse check was deleted). The decision rules and constants
below are the ones `sim_analysis::ArtifactPlan::preregistered` returns and
`lifesim artifact` echoes into its report; if the two ever disagree, this
document is what was pre-registered and the code is wrong.

## 1. What this campaign is for

The plan's three behavioural criteria for the artifact half, exactly as
written in `planning/phase-12-mutable-world-and-artifacts.md`, given their
controls and their thresholds. Nothing here changes a criterion's wording;
where the plan left something unstated (C12.1's effect size, C12.3's N,
C12.2's primary fitness measure and its matching rule) it is fixed here,
before the run, and the reasoning is written down. A null on any criterion
is a result and will be reported as one.

## 2. Arms

Four conditions, matched on seed (12001..12030), horizon (60,000 ticks),
and every config field but the three below:

| Arm | `artifact.max_composition_depth` | `artifact.ephemeral` | `artifact.inert` |
|---|---|---|---|
| A | 4 | false | false |
| B | 4 | true | false |
| C | 4 | false | true |
| D | 0 | false | false |

- **A**: artifacts persistent, combination enabled.
- **B**: an object dropped or placed is destroyed at the end of the tick it
  lands in, ledgered to dust; actions exist and cost the same. Persistence
  removed, nothing else.
- **C**: every requested pick-up, drop, place, strike and combine is
  charged and counted as a success and nothing in the world changes. The
  control that separates "actions fire" from "actions pay". Carcass objects
  still exist under C (they are ecology, not action), so C is not an
  object-free world; it is a world in which action has no effect.
- **D**: combination refused by the depth cap; simple objects only.

The base is the Phase 11 confirmatory base (128 x 128, 200 founders, capacity
240,000, physiology with the extrinsic hazard at 13, point rate 65535,
duplication 6554), chosen because it is a known-stable ecology with births
in the hundreds per 10,000 ticks and for nothing about objects; the
relocating patch is off. All four arms carry `genome2.mutation.binding_q16
= 16384` (0.25 per birth) so the mutational supply of object bindings is
common-mode.

**Why the bind rate is 0.25, and how it was chosen.** No schema-2 lineage
can bind a channel its founder did not without the `bind` operator (D-114),
so the rate is a floor on reachability, not a treatment. It was calibrated
on two pilot seeds disjoint from the thirty above (39321 and 30583, phase2
default world, 60,000 ticks) by a rule about the mechanism: "the median
pilot world binds and successfully fires `pick_up` inside the horizon". At
0.10 the thriving pilot reached its first pick-up near tick 40,000 (4 in
60,000); at 0.25 it reached 23; the collapsed pilot (population 7) reached
none at either. Higher rates were not tried. That is a lower bound on what
could have been used, and it is recorded so the null it may produce is read
as "at this supply" and not as "ever".

## 3. The phase's own prior, stated in advance

Unfavourable, and recorded rather than discovered. In the pilots, strikes
happened in the thousands, pick-ups in the tens, placements never,
combinations never. Every object action needs a lineage to bind its channel
by mutation and, for placement and combination, to bind *two* channels in
one line of descent and hold something at the moment the second fires -
the conjunction structure that produced Phase 11's reachability null
(D-099). Under C the counted rate is a firing rate, which selection reduces
only through the action cost; under A the counted rate is a success rate,
which selection raises only if what the action does pays. C12.1 therefore
asks a real question with a likely answer of "no at this supply and this
horizon", and the plan's risk table already names C12.3 as the criterion
most likely to return null.

## 4. Primary quantities, per world, then decision rules

The world is the replicate (ADR-0022 A5). Every quantity is reduced per
world from the manifest, the run's event log, and its final snapshot, by
`sim_analysis::world_artifact`, and the rules below count worlds. Extinct
worlds are analysed to their extinction and count against every bar;
nothing is excluded for having died.

**C12.1** - *successful pick-up + place + combine per million
organism-ticks* (`organism_ticks` is the sum of the population over every
tick, recorded by the scheduler), A minus C on the same seed. A world counts
when the difference is at least **10 ppm** (one successful action per
100,000 organism-ticks, roughly one per organism per 2.8 lifespans) in the
increasing direction. Bar **20 of 30**. Reported beside it, not decisive: the
same contrast on the *fire* rate (successes plus refusals of the same three
actions, from the log), so a reader can see whether A's lineages fire more
or less than C's as well as whether they succeed.

**C12.2 (a)** - median lifetime of placed-object episodes exceeds the median
organism lifespan, in **15 of 30** worlds (the plan's number). An episode
runs from `ObjectReleased{placed:true}` to the same id's next
`ObjectPickedUp` or `ObjectDestroyed`; an episode open at the horizon
counts at its observed length, and so does an organism alive at the
horizon; both censoring counts are reported. A world with no placed episode
counts as not meeting the clause.

**C12.2 (b)** - the plan's "measurable fitness difference" is fixed as
**reproductive output** (offspring per thousand ticks of life, from
`PairedBirth` parents in the log), and its matching rule as **stratification
by the capacity band of the birth cell** (`birth_band`, the terrain quintile
of baseline capacity, recorded at birth): placed objects sit where organisms
are, and organisms are where the food is, so an unstratified comparison
would measure the food. An organism is *exposed* if it spent at least **5%**
of its life in a cell holding a live placed object (`exposure_ticks` from
`ObjectExposure` at death or the final table for the living). The per-world
effect is the mean over bands, weighted by the smaller side's count, of the
exposed mean minus the unexposed mean; bands with an empty side are dropped
and counted. A world counts when the effect is positive **and** it had at
least **20 exposed organisms**; fewer counts as not showing the effect, never
as excluded. Bar **20 of 30**. Both halves are required, as the plan says.

**C12.3** - live composites of depth two or more per thousand living
organisms, sampled every 1,000 ticks from the log, averaged over the first
and the last third of the run. A world counts when the last-third mean
exceeds the first-third mean by **0.5 per thousand organisms** (500 milli).
**N = 20 of 30**: the smallest count with a one-sided binomial p below 0.05
at a null rate of one half, the same bar C12.1 states; chosen for that
reason and no other. Under D the count is asserted zero.

Analysis seed `0xa11fac750b1ec751`; paired intervals by
`lifesim-paired-stats-v1`.

## 5. What each criterion licenses, in the review's vocabulary

- C12.1 met would license review 10.2 Level 0/1 (interaction, controlled
  transport). Not tool use.
- C12.2 met would license "persistence plus a matched correlation with
  reproductive output" - review 10.2 Level 2 requires the removal branch,
  which does not exist (open question, `docs/21-open-questions.md`).
- C12.3 met would license "elaboration frequency rose" - structural nesting
  of depth two, not review 10.2 Level 7's causal-dependency depth and not
  Level 10's cumulative culture.

## 6. What a null means here

At this bind rate and horizon, in this ecology, with these costs. The
reachability diagnostic (`lifesim conjunction`-style census of which
lineages bound which object channels) is owed beside any null so that "the
world did not use objects" and "no lineage could reach the action" are told
apart. The costs and the rate are the two knobs a follow-up would move,
each as its own pre-registered campaign.
