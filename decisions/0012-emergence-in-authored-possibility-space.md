# ADR-0012: Emergence Inside An Authored Possibility Space

Status: Proposed
Date: 2026-08-04
Author: Goal revision

## Context

The project's long-term ambition changed on 2026-08-04: organisms should be
able to evolve from simple foragers toward tool use, persistent structures,
transmitted knowledge, technological accumulation, territoriality, and
organized inter-group conflict.

An ambition of that shape has an obvious failure mode. The fastest way to
produce something that looks like technological progression is to author it:
a technology tree, a research prerequisite graph, era states, building
recipes. Systems built that way display progression and demonstrate nothing,
because the progression was the input.

The project needs a governing rule that makes the difference structural
rather than a matter of taste, because the pressure to author progress will
be strongest exactly when a phase returns a null result.

## Options Considered

- **Author a progression structure** (tech tree, recipes, era states) and
  let evolution navigate it. Produces visible results quickly and answers no
  interesting question.
- **Author nothing beyond current physics** and hope complexity appears.
  Guarantees a null result, because the physics contains no objects to
  manipulate, no channel to transmit on, and no within-lifetime change.
- **Author the possibility space, never the trajectory.** Define what is
  physically possible; define no progress structure at all.

## Proposed Decision

Author physics, not progress.

The simulation defines what is physically possible: that stone is hard and
can be struck, that an object can be held, carried, placed, and combined,
that a placed object persists, that a signal can be emitted and perceived,
that one organism's actions are visible to another, that a synapse can
change during a lifetime and that the rule governing that change is
inherited.

The simulation never defines a technology tree, a research prerequisite
graph, an era, a building recipe, a civilization stage, a tool category, or
a reward for inventing anything.

The operational test for any proposed mechanism: **can you name the specific
outcome it makes more likely?** If yes, it is authored progress and it is
rejected. A material registry entry is physics because it does not favor any
particular use. A recipe is progress because it names an outcome.

Two clarifications the boundary needs:

- **Genetic compatibility gating is physics.** Reproduction may be gated on
  compatibility computed directly from two genomes, because that is a
  physical fact about whether two records can recombine. It is not an
  analysis-assigned cluster label. The distinguishing test: if the analysis
  modules were deleted, compatibility gating would still function
  identically and clustering would not exist.
- **Analysis observes, never instructs.** Covered separately by ADR-0016
  because it needs its own enforcement mechanism.

## Consequences

Positive: results mean something. A finding of tool use under this decision
is a finding about evolution; under the authored alternative it would be a
finding about our design.

Negative and accepted:

- Several phases are likely to return null results, and the null is the
  correct outcome to report rather than a reason to relax the rule.
- Progress is slower and less directed than an authored structure would be.
- Some capacities sit close to the line and require an explicit argument.
  The clearest current case is plasticity rule form 5 (observational
  learning), which authors the *capacity* for a synapse to be driven by an
  observed conspecific action without authoring what is learned. Phase 11
  runs its presence and absence as experimental conditions rather than
  resolving it by assertion, which is the pattern this ADR recommends for
  every future borderline case.

Compatibility: no code or data impact. This is a design constraint on all
future phases, recorded here so that later pressure to author progress has
to argue against a written decision.

## Performance Implications

Indirect and real. Unauthored search needs more generations than an authored
progression, which is the direct cause of Phase 5's existence and of the
compute-cost risk in `docs/20-risk-register.md`.

## Operational Implications

None. No deployment, security, or infrastructure impact.

## Revisit Conditions

- Multiple phases return nulls and the evidence points to a specific
  authored capacity that would make the question askable without naming an
  outcome. Adding such a capacity is a new proposed ADR and a new condition,
  never a silent relaxation.
- A boundary case arises that the "can you name the outcome" test does not
  resolve.

## Evidence Required To Accept

- User approval, since this is a product-direction decision rather than a
  technical one.
- At least one phase completed under the rule, showing that its acceptance
  criteria can be stated and measured without an authored progression.
