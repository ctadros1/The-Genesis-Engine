# ADR-0018: Scaffolded Possibility Space For Major Transitions

Status: Proposed
Date: 2026-08-04
Author: Origin-modes revision

**Amends ADR-0012.** ADR-0012 keeps its status and its text; this record
states the one bounded exception and the conditions attached to it. Read
both together.

## Context

ADR-0012 says: author physics, never progress. Its operational test is
"can you name the specific outcome this mechanism makes more likely? If
yes, it is authored progress and it is rejected."

The `scratch` origin mode (`specifications/world-origin-modes.md`) asks the
simulation to cross the major evolutionary transitions: from chemistry to
protocell, from protocell to reproducing unicell, from unicell to
differentiated multicell. Those transitions are among the least tractable
problems in artificial life. A world whose environment is chosen without
regard to them will, on any realistic compute budget, cross none of them,
and the phase will return a null that says nothing except that the search
was not given a reason to move.

The user's decision, recorded 2026-08-04, is to shape the environment
deliberately toward those transitions rather than leave it neutral. That
decision fails ADR-0012's test as written. It is recorded here as an
amendment rather than absorbed by reinterpretation, because quietly
loosening the rule that keeps the project's results meaningful is exactly
the failure mode ADR-0012 exists to prevent.

## Options Considered

- **Neutral environment, no scaffolding.** Keeps ADR-0012 intact. Almost
  certainly produces a null at every transition, and an uninformative one.
- **Scaffold freely.** Choose whatever makes transitions happen, including
  direct rewards for the target trait. Produces visible results and
  destroys their meaning.
- **Scaffold the environment only, under a stated boundary, with a
  mandatory unscaffolded control.**

## Proposed Decision

Adopt the third option, bounded as follows.

### Permitted

Shaping **environmental and selective structure**: resource distribution and
patchiness, spatial structure and barriers, mortality regimes, nutrient
gradients, disturbance frequency, niche dimensionality, and the physical
constants that govern all organisms equally.

### Forbidden, unchanged from ADR-0012

- Any reward, energy bonus, fitness term, or survival advantage granted
  **for possessing the target trait**.
- Granting the trait, or any part of it, rather than making it reachable.
- A recipe, stage, grade, ladder, or checkpoint of any kind.
- Any mechanism that reads whether an organism has made a transition and
  changes a rule as a result.

### The naming test

A scaffold must be describable **without naming its target**. If the only
honest description of a config is "conditions that favor multicellularity,"
it is authored progress. If it is "patchy resources with high between-patch
variance and a dispersal cost," it is an environment, and the fact that
theory predicts it favors aggregation is a hypothesis being tested rather
than an outcome being purchased.

Every scaffold config carries a plain-language description that passes this
test, and that description is part of the record.

### The evidence conditions

These are what keep a scaffolded result honest, and they are not optional.

1. **The scaffold is a condition, always reported.** Any result obtained
   under scaffolding names the scaffold, its parameters, and its config
   hash. A scaffolded transition is never reported as simply "a transition
   occurred."
2. **An unscaffolded control is mandatory.** Every transition claim runs the
   same seeds under a neutral environment. The reportable quantity is the
   difference between conditions, not the scaffolded result alone.
3. **A scaffolded-only result is explicitly the weaker claim** and is
   written that way: "the transition occurred in N of 12 scaffolded seeds
   and 0 of 12 unscaffolded seeds" states both the finding and its
   dependence on the scaffold.
4. **Scaffold strength is swept, not fixed.** Where feasible, report the
   transition rate across a range of scaffold intensities, so the result is
   a curve rather than a single point. A transition that appears only at the
   most extreme scaffold setting is a different and much weaker finding than
   one that appears across the range.

## Consequences

Positive: the transition phases become answerable rather than
pre-determined to null. The boundary and the control keep the results
interpretable.

Negative and accepted:

- **Every claim from the scaffolded phases is weaker than an unscaffolded
  one would have been**, and the documents must keep saying so. The risk is
  not that the rule is broken once; it is that "scaffolded" quietly falls
  out of the summary three documents downstream.
- The naming test is a judgment call at the margin. A reviewer who thinks a
  scaffold names its target should be able to say so and have the config
  changed or the description rewritten.
- Scaffolding doubles the seed cost of every transition claim, because the
  unscaffolded control runs the same seed set.

Compatibility: no code, data, schema, or protocol impact. Scaffolds are
config, hashed like any other config, and every scaffold intensity is its
own condition with its own hash.

## Performance Implications

The mandatory unscaffolded control doubles campaign cost for the affected
phases, and the intensity sweep multiplies it further. This compounds the
compute-cost risk already recorded as unresolved in
`docs/20-risk-register.md`.

## Operational Implications

None.

## Revisit Conditions

- A transition occurs under a neutral environment, making the scaffold
  unnecessary for that transition; the scaffold is then dropped and the
  result restated as the stronger claim.
- Scaffolding is found to be doing more work than intended, for example a
  config that passes the naming test but on inspection grants an effective
  advantage to the target trait. That is a defect and the config is
  withdrawn.
- The amendment is being invoked outside the major-transition phases. It is
  scoped to those; any wider use needs its own record.

## Evidence Required To Accept

- User approval, since this changes a product-direction decision.
- At least one transition phase completed with its unscaffolded control run
  and both results reported.
- A written scaffold description for each scaffold config that passes the
  naming test under review.
