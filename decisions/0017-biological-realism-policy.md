# ADR-0017: Biological Realism Policy

Status: Proposed
Date: 2026-08-04
Author: Goal revision

## Context

The project is to simulate biology and genetics as realistically as
possible. That instruction is easy to agree with and hard to act on, because
"as possible" is doing all the work and because realism trades directly
against three things the project already values: determinism, bounded state,
and throughput.

It also sits in tension with existing text.
`docs/00-project-vision.md` previously disclaimed "biologically faithful
evolution" alongside intelligence and consciousness, which conflates a
mechanism claim with an outcome claim. The mechanism can be faithful; the
outcomes are still simulation outcomes and are not evidence about real
biology.

Without a written policy, "realism" becomes an argument that can be made for
any addition, and the project accumulates expensive mechanisms nobody can
evaluate.

## Options Considered

- **Maximize realism.** Unbounded scope, and it breaks determinism the
  moment a mechanism needs a transcendental or an unbounded structure.
- **Keep the current abstractions.** Cheap, and it declines a stated
  project goal.
- **Realism as a standing policy with an explicit precedence order and a
  falsifiability test.**

## Proposed Decision

Adopt the policy in `docs/26-biological-realism-policy.md`.

Its substance:

**The default.** Where a mechanism has a well-established biological form
and an abstract shortcut, prefer the biological form unless the shortcut is
justified in writing with the cost that made it necessary.

**What realism means.** The *mechanism* resembles the biological one.
Parameter values do not correspond to any real organism and no document may
imply they do. The world is not a model of Earth. `docs/00-project-vision.md`
still governs: parameters are simulation design choices unless supported by
a cited model or a validation experiment.

**The falsifiability test.** A realistic mechanism should reproduce a
textbook result it was not tuned to produce. A mechanism that cannot be
checked against any known result is decoration, not realism. This turns the
policy into acceptance criteria rather than an aesthetic: Hardy-Weinberg
equilibrium at a neutral locus (C7.3), linkage decay with map distance
(C7.4), the allometric exponent (C11.1), the investment-versus-number
tradeoff (C11.2), and lifespan responding to extrinsic mortality (C11.3).

**Three hard constraints, in precedence order.**

1. *Determinism first.* A biologically faithful mechanism that cannot be
   replayed exactly loses to a less faithful one that can. Concretely: any
   quantity that accumulates over a lifetime is fixed point, because float
   accumulation over 10^5 ticks amplifies exactly the variance ADR-0011
   excludes.
2. *Bounded state second.* Every biological structure gets a configured cap
   with deterministic rejection and counting. Real genomes are effectively
   unbounded; ours cannot be, because unbounded genomes make snapshots,
   migrations, and memory budgets unprovable.
3. *Measured cost third.* A realism increment that measurably breaks the
   tick or checkpoint budget is deferred with its measurement recorded, not
   adopted and hoped about.

**No Lamarckian inheritance by default.** Recorded here as well as in
ADR-0014, because it is as much a realism decision as a learning one, and
because it is the property that keeps the culture question distinguishable
from the genetics question.

**Placement.** Genetics realism goes in Phase 7, because the genome is being
rewritten anyway and doing it twice is strictly worse. Physiology realism
goes in Phase 11, after the scale-sensitive culture experiments, because
each realism increment multiplies per-organism cost. The cost of that split
is accepted and stated: results from Phases 6 to 10 do not transfer across
Phase 11, and the campaigns that matter are re-run under
`lifesim-physiology-v1` before their results become standing findings.

## Consequences

Positive: realism claims become testable; the trade-offs are written down
once instead of relitigated per feature; the project can say specifically
what it does and does not model.

Negative and accepted:

- Diploidy roughly doubles genome storage.
- Meiosis, dominance expression, allometry, ontogeny, and hazard modelling
  all cost per-organism time, which reduces reachable generations per unit
  of compute. Phase 11 is required to report that number as a headline
  rather than a footnote, because it is the honest price.
- Several realism mechanisms are deliberately excluded (molecular genetics,
  physical morphology, material chemistry, real climate), and the exclusions
  are recorded so nobody has to rediscover the reasoning.

Compatibility: every realism increment is a config-gated section that is
inert when disabled, so earlier fixtures are preserved. Each starts a new
replay lineage when enabled, exactly as every other behavior policy does.

## Performance Implications

Negative and measured rather than estimated. Phase 7 and Phase 11 both
record the per-organism cost delta and the resulting change in ticks per
second per world.

## Operational Implications

Snapshot growth from diploidy and from physiological state. Covered by the
same checkpoint budget work ADR-0013, ADR-0014, and ADR-0015 depend on.

## Revisit Conditions

- A realism criterion fails in a way that indicates the mechanism rather
  than the world is wrong.
- Throughput cost reduces reachable generations below what the culture
  phases need, at which point the precedence order says the realism
  increment yields, not the science.
- A molecular or regulatory mechanism becomes necessary for a specific
  question, requiring its own ADR rather than an appeal to this one.

## Evidence Required To Accept

- The realism validation criteria across Phases 7 and 11: C7.3, C7.4,
  C11.1, C11.2, C11.3.
- Measured per-organism cost and throughput deltas at both supported tiers.
- Fixture preservation with every realism section disabled.
