# ADR-0022: Research Findings Adopted And Declined

Status: Proposed
Date: 2026-08-04
Author: Research reconciliation

## Context

Six commissioned engineering-scientific reviews were delivered on 2026-08-04
covering neuroevolution and lifetime learning, artificial genetics and
lineages, cumulative culture and technology, mutable worlds and tool use,
social organization and conflict, and open-ended-evolution methodology. They
are indexed in `research/deep-research-index.md` and carried in the repo
under `.agents/skills/genesis-*/references/`.

They corroborate the plan's spine: analysis must never instruct, no global
group identifier, territory is not a world object, keyed counter-based
randomness, snapshots plus a bounded journal, independent-world parallelism
as the first scaling axis, three-factor plasticity with an evolved
modulator, and the explicit finding that generic physics plus neural agents
predictably yielding technology or civilization is **unsupported**.

They also contradict specific decisions. This record states which
contradictions are adopted, which are declined, and why, so that neither the
adoptions nor the refusals have to be re-derived later.

## Adopted

**A1. "Scarcity plus kin bias plus damage implies war" is withdrawn.**
`social_organization` section 1.3 explicitly rejects it, and lists roughly eleven
further dependencies for organized inter-group conflict: persistent
interaction communities, recognition, target discrimination, coalition
recruitment, numerical-advantage assessment, spatial memory, free-rider
suppression, and capturable rather than merely destroyable value. Most land
in Phases 10 to 12 in this plan. The claim was load-bearing in the roadmap's
justification for scheduling contest early, and it was wrong.

Phase 7 keeps its position, because health and damage as *physics* still
need no schema change and still make threat information real. Its behavioral
criteria are rewritten down to what is reachable without recognition or
memory, and the organized-conflict criteria move to Phase 12 where their
prerequisites exist.

**A2. Artifact-mediated transmission precedes signalling.** Triple-sourced:
`cumulative_culture` section 1.2 lists persistent generic artifacts inside the
*minimum viable transmission system*; `artifacts` section 1.7 puts social
transmission at step 25 of 30, after carrying, reuse, caching, structures,
and stigmergy; `social_organization` section 1.1 puts stigmergic cooperation before
communication.

The plan's prior justification assumed transmission means signalling. It
does not: **an artifact left behind is a transmission channel that requires
no perception of conspecifics at all**, and it is the easier one. Phases 11
and 12 swap, so artifacts land first and stigmergy becomes the first
transmission mechanism tested.

**A3. No direct genotype-distance perception.** `social_organization` section 1.1:
cues may be heritable phenotype components, movement styles, emissions,
carried objects, and location history, and **never** direct access to
genotype distance, pedigree, or observer labels. The plan supplied
`neighbour_kinship` as computed genetic distance and recorded the concern as
an open question with the wrong default. Kin recognition must be solvable
from perceptible cues or not solvable at all.

**A4. No privileged action labels.** `cumulative_culture` section 1.3 warns that a
privileged action identifier scripts the channel. The `neighbour_action`
one-hot action class is exactly that. Replaced with body motion, contact
events, object-state change, and carried-object cues.

**A5. The world is the experimental unit.** `oee_methodology` section 1.1 and section 5.2.
Organisms, births, encounters, and ticks are nested observations, and
counting them as replicates is pseudoreplication. Phase 10's C10.1 said
"measured per individual"  -  correct science, wrong statistical unit. Per
-individual quantities are computed then aggregated to a world-level
statistic, and the world remains the replicate.

**A6. Seed counts rise to 30 minimum, 50 for rare or fixation-driven
outcomes**, with a pilot preceding every confirmatory campaign and
simulation-based power analysis choosing the final number. The plan's
uniform "30 seeds" was arbitrary and 2.5 to 4 times under the recommended
minimum.

**A7. Conjunctive acceptance with a designated primary endpoint.** Each
phase names one primary endpoint; secondary metrics cannot rescue a failed
primary. Nulls report the smallest effect of interest, an interval, and an
equivalence result, so "no effect detected" is distinguishable from
"underpowered".

**A8. Structural identity is four fields, not one.**
`neuroevolution` section 1.6. The plan's single `innovation_id` conflated gene
lineage, homology class for alignment, structural signature, and mutation
event identity. Split into four, with alignment by homology class and IDs
derived by domain-separated hash over a canonical event key rather than a
global sequential counter.

**A9. Hybrid controller evaluation.** `neuroevolution` section 1.8 recommends
canonical topological order for zero-delay edges plus prior-state buffers
for delayed and recurrent edges. The plan's all-synchronous update was
chosen to avoid topological sorting, at the cost of one edge of propagation
per tick. The hybrid is strictly more capable and remains deterministic
under a canonical order, so the open question is closed in its favor.

**A10. Inheritance-mode controls.** `genetics` section 1.2: crossover is not
universally beneficial and paired reproduction should not imply mandatory
crossover. Clonal and single-parent whole-genome modes are added as
first-class controls.

**A11. Bounded event memory.** `cumulative_culture` section 1.2 lists it in the
minimum viable transmission system. Recurrent activations alone do not
provide a retrievable trace of state-action-outcome events.

**A12. Intermediate culture rungs.** The plan jumped from "transmission
occurs" to "depth-2 composites increase". `cumulative_culture` section 1.4 supplies
the missing rungs: payoff-sensitive competition between variants, and
retention of a single socially acquired improvement, before any cumulative
claim.

**A13. Novelty is not progress.** `oee_methodology` section 1.1.3. The morphology
criterion measured module-count divergence, which is novelty without
demonstrated consequence. Structural change must now show ecological or
fitness consequence and persistence to count.

**A14. Removal mutations from the first version**, plus explicit evaluation
cost and optional parsimony pressure (`neuroevolution` section 1.7). The plan had
deletion; the cost pressure and the reachability of pruning were
under-specified.

## Declined, with reasons

**D1. "Do not make a developmental program the baseline successor genome"**
(`genetics` section 1.6, `neuroevolution` section 1.4). **Partially declined.**

Both reports argue against developmental encodings as a *controller*
baseline. The plan uses one for *morphology*, where the argument is
materially different: the modular lattice is what makes a one-module body a
unicell, which is what lets the multicellularity transition require no
authored mechanic at all. Replacing it with a parameterized body plan would
force an authored transition, which ADR-0012 forbids.

What is adopted from the finding: the controller itself stays directly
encoded (it already was), the developmental program is scoped to morphology
only, it is declared as a bounded versioned module with every field
`genetics` section 1.6 requires, and the direct parameterized body plan is retained
as a specified fallback. Phase 9's genotype-phenotype discontinuity
measurement is promoted from a reported metric to a **gate**: if a typical
single-locus mutation produces an unrelated body, the encoding has failed
and the fallback is taken.

**D2. Full 2.5D representation with height intervals, a collider palette,
and a bond/support graph** (`artifacts` section 1.5). **Deferred, not declined.**

The recommendation is sound and stacking genuinely cannot be expressed
without height. It is also a change to collision, movement, perception,
rendering, and the save format simultaneously, arriving in a plan that
already has four unmeasured cost multipliers. A single height-interval field
per object plus a support relation is recorded as the minimum viable subset
and scheduled as a Phase 11 stretch item gated on the Phase 11 cost
measurement. Until it lands, the plan does not claim stacked construction.

**D3. Preregistration infrastructure in full** (`oee_methodology` section 1.3).
**Adopted in substance, declined in ceremony.** Immutable experiment plans,
locked seed registries, and frozen detector configs are adopted. Formal
external preregistration is not, because this is a private single-operator
project and the cost is not repaid. The requirement kept is that the plan
and thresholds are committed to the repository before the campaign runs, so
the git history serves the same function.

## Consequences

Positive: the plan's criteria become defensible rather than merely
falsifiable, and the two orderings that were wrong are corrected before any
implementation depends on them.

Negative and accepted:

- **Seed counts nearly triple.** Combined with ADR-0018's mandatory
  unscaffolded controls, this is a large multiplier on a compute ceiling
  already recorded as unresolved. The honest consequence is fewer claims,
  not more compute, and the roadmap now says so.
- Phase 7 becomes a smaller phase than advertised. That is the correct size
  for what it can actually establish.
- Phase 12 becomes the largest behavioral phase, carrying both transmission
  and organized conflict.

Compatibility: no code, schema, protocol, or fixture impact. This is a
planning-document change.

## Revisit Conditions

- A declined finding is shown to matter by a measurement, in particular D1
  if Phase 9's discontinuity gate fails.
- A further review contradicts an adoption here.

## Evidence Required To Accept

- User approval of the reordering and the reduced Phase 7 scope.
- The revised criteria surviving one phase's execution without needing a
  threshold change after data collection.
