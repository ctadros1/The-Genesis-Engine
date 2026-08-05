# ADR-0016: Analysis Observes, It Never Instructs

Status: Proposed
Date: 2026-08-04
Author: Goal revision

## Context

The project already holds this line for one case. `lifesim-similarity-v1`
computes species clusters offline, records them in reports, and never feeds
them back into mating rules, action eligibility, or behavior. That was
stated as a property of the similarity analysis specifically.

The new goal adds era detection and tradition detection, and will add more
analyses after that. The rule needs to be general, and it needs an
enforcement mechanism, because the pressure to add "just one small hook"
from analysis into the kernel will be strongest when a phase is returning
nulls and an analysis output looks like a useful signal.

The rule matters more than it might appear. An era that the simulation knows
about is an authored progression stage wearing an analysis costume: the
moment a rule can read an era, the era is causing outcomes and the whole
point of ADR-0012 is gone.

## Options Considered

- **Convention plus code review.** How the rule is currently held. Works
  until it does not, and the failure is silent.
- **A runtime assertion** that analysis output is not read during a tick.
  Catches some violations at test time and not all of them, and can be
  disabled.
- **Structural enforcement by crate dependency direction**, plus a required
  checksum-equality test.

## Proposed Decision

Analysis observes. It never instructs. Enforced structurally.

- All offline analysis lives in a separate crate, `sim-analysis`, which
  depends on `sim-core` and on the event-log reader. **Nothing in `sim-core`
  depends on `sim-analysis`.** The dependency direction makes feedback a
  compile error rather than a review finding, and the direction is asserted
  in CI.
- Analysis consumes the append-only event log and read-only snapshot views.
  It never holds a mutable handle to a world.
- Analysis draws from no `RngSystem` stream. If it needs sampling it uses a
  separate deterministic sampler seeded from its own report parameters, so
  an analysis can never perturb a world's stream sequence even accidentally.
- Analysis version strings are recorded in reports and are deliberately
  **not** folded into the config hash, because an analysis version can never
  affect a world. This is the inverse of the rule for behavior policies and
  the asymmetry is the point.
- Every analysis carries a required test asserting that world state
  checksums are bit-identical with the analysis enabled at any cadence and
  with it disabled, extending the existing Phase 2 analysis-neutrality test.
- No analysis output may become a simulation input, a config trigger, an
  intervention, or a rendering-driven behavior change.

Two boundary clarifications:

- **Genetic compatibility gating is not analysis.** It is computed directly
  from two genomes and would still function identically if every analysis
  module were deleted. Cluster labels never gate anything; genome-derived
  distance may.
- **Observer rendering may display analysis output.** Displaying a cluster
  color or a segment boundary in the browser is fine, because the observer
  cannot write to the world. What is forbidden is an analysis result
  reaching a rule.

## Consequences

Positive: results stay interpretable; the ADR-0012 boundary cannot be
eroded through the analysis side door; the least risky phase in the plan
(Phase 17) is least risky by construction.

Negative and accepted:

- Some analyses would genuinely be more efficient computed inside the tick
  where the data already sits. They are computed offline anyway, at a cost
  measured and recorded separately from tick cost.
- Crate separation adds a build boundary and some duplication of read-only
  view code.

Compatibility: no data or protocol impact. `lifesim-similarity-v1` already
satisfies the rule and moves into `sim-analysis` unchanged, keeping its
version string and its report format so existing reports stay comparable.

## Performance Implications

Analysis cost is measured and reported separately from tick cost, following
the existing similarity-analysis convention where the 2,048-sample bounded
run was recorded at 466 to 474 microseconds and 7.45 to 7.47 milliseconds
depending on sample size, entirely outside the tick. No tick-path cost is
permitted at all, and the checksum-equality test proves it.

## Operational Implications

None. Analysis is a separate command producing separate artifacts. Removing
it changes no world, no save, no protocol, and no config hash.

## Revisit Conditions

Effectively none for the rule itself. If a future research question appears
to require analysis feedback, that is a request to author progress and needs
its own ADR arguing against ADR-0012, not a relaxation of this one.

The enforcement mechanism may be revisited if crate separation proves
unworkable for a specific analysis, in which case an equivalent structural
guarantee must replace it before the separation is dropped.

## Evidence Required To Accept

- Phase 17 criteria C17.1 (checksum equality at every cadence) and C17.2
  (build-enforced dependency direction).
- `lifesim-similarity-v1` relocated to `sim-analysis` with its reports
  unchanged and its existing neutrality test still passing.
