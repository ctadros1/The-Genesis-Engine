# Biological Realism Policy

## Standing Requirement

The project simulates biology and genetics as realistically as the
determinism contract and the compute budget allow. Where a mechanism has a
well-established biological form and an abstract shortcut, prefer the
biological form unless the shortcut is justified in writing with the cost
that made it necessary.

This is a standing policy, not a phase. It applies to every mechanism added
from Phase 7 onward and is the reason several design choices in
`specifications/genome-schema-2.md` and
`specifications/plasticity-and-learning.md` are more expensive than the
minimum that would work.

## What Realism Means Here, And What It Does Not

Realism means the *mechanism* resembles the biological one, so that the
model can reproduce known biological results as a validity check.

Realism does not mean parameter values are drawn from any real organism.
They are not, and no document may imply otherwise. It does not mean the
world is a model of Earth. It does not license claims about real ecology,
real evolution, or real cognition. `docs/00-project-vision.md` still governs:
parameters are simulation design choices unless explicitly supported by a
cited model or a validation experiment.

The practical test of realism is falsifiable: a realistic mechanism should
reproduce a textbook result it was not tuned to produce. The roadmap turns
this into acceptance criteria (Hardy-Weinberg equilibrium at a neutral
locus, linkage decay with map distance, the extrinsic-mortality/lifespan
relationship, the offspring-number/investment tradeoff). A mechanism that
cannot be checked against any known result is decoration, not realism.

## The Three Hard Constraints

Realism is bounded, in this order of precedence:

1. **Determinism first.** A biologically faithful mechanism that cannot be
   replayed exactly is rejected in favor of a less faithful one that can.
   ADR-0010 and ADR-0011 are not negotiable for realism. Concretely: any
   quantity that accumulates over an organism's lifetime is fixed point, not
   float, because float accumulation over 10^5 ticks amplifies exactly the
   variance the numeric policy is designed to exclude.
2. **Bounded state second.** Every biological structure gets a configured
   cap: chromosome count, locus count, node and edge count, object
   composition depth, carried mass. Real genomes are effectively unbounded;
   ours cannot be, because unbounded genomes make snapshots, migrations, and
   memory budgets unprovable. Caps are versioned config, and a mutation that
   would exceed one is rejected deterministically and counted, exactly as
   births are rejected at `max_entities`.
3. **Measured cost third.** A realism increment that measurably breaks the
   tick budget or the checkpoint budget at a supported tier is deferred with
   its measurement recorded, not adopted and hoped about.

## Where Realism Is Being Increased

| Area | Current (Phase 2) | Successor | Phase |
|---|---|---|---|
| Ploidy | Haploid; a single flat gene vector | Diploid, chromosomes, dominance | 9 |
| Recombination | Per-gene independent parent choice | Meiosis with crossover points and linkage | 9 |
| Mutation classes | Point mutation only | Point, duplication, deletion, insertion, transposition | 9 |
| Network structure | Fixed 20-16-12-12, human-authored | Grows and shrinks by gene duplication and deletion | 9 |
| Within-lifetime change | None; weights fixed at birth | Genome-encoded synaptic plasticity with neuromodulation | 11 |
| Reinforcement | None | Evolved neuromodulatory signal, never an authored reward | 11 |
| Metabolism | Linear basal cost with a body-scale multiplier | Allometric scaling with a configured exponent, thermoregulation | 8 (early) |
| Ontogeny | None; organisms are born adult-shaped | Juvenile constraint and maturation (8); developmental growth of a module body (14) | 8 / 14 |
| Senescence | Hard `max_age_ticks` cutoff | Age-dependent hazard; evolvable lifespan | 8 (early) |
| Sexual selection | Compatibility threshold only | Mate choice from perceived phenotype, costly signals | 14 (late) |
| Body structure | Small fixed parameter set, identical for every organism | Typed modules on a lattice, grown by a developmental genome | 10 |
| Morphogenesis | No developmental stage at all | A growth program executed from a single origin module | 10 |
| Climate | Seasons only; temperature field unimplemented | Moisture and temperature fields, biomes, long-timescale drift | 6 |
| Origin of life | Organisms exist at tick 0 by construction | Optional abiogenesis from a chemistry field | 15 |
| Major transitions | None | Unicell to differentiated multicell as ordinary morphological evolution | 16 |
| Disease | None | Transmissible load with contact-structured spread | 14 (optional slice) |
| Population regulation | Reproduction gated on energy alone; 99.9 percent starvation mortality | Non-food extrinsic mortality, senescence, juvenile mortality, life-history tradeoff | 8 (early) |

**Ordering note (ADR-0025).** The demographic half of physiology executes
*before* the culture stack rather than after it. Surplus does not come from
more food; it comes from mortality that is not food-driven, and a population
held below carrying capacity has the per-capita slack that every
energetically expensive behavior requires. Running the culture stack in a
world where 99.9 percent of deaths are starvation would produce nulls caused
by the ecology rather than by anything about transmission.

## Structural Change By Duplication, Not By Graph Editing

The single most consequential application of this policy is in Phase 9. The
obvious way to make a network topology evolvable is a NEAT-style pair of
graph-editing operators: "add a node", "add a connection". That is an
authored editing scheme imposed on a graph.

The biological way is that networks grow because *genes* duplicate and then
diverge, and shrink because genes are deleted. Gene duplication followed by
divergence is the principal mechanism by which real regulatory and neural
complexity increased. Adopting it gives structural evolution and genetic
realism from one mechanism instead of two, and it makes the resulting
complexity growth a consequence of the genetics rather than a separate
authored feature.

The cost is honest and recorded in ADR-0013: duplication-driven growth is
slower and less directed than explicit add-node mutation, and the ALife
literature does not establish that indirect or duplication-based encodings
outperform direct ones at this scale. The mitigation is that both operator
sets act on the same variable-length locus list, so the explicit-insertion
operator remains available as a configured variation policy if duplication
alone proves too slow. That comparison is itself a Phase 9 experiment.

## No Lamarckian Inheritance By Default

Learned state is reset at birth. This is not an optimization; it is the
property that makes the project's central question meaningful. If learned
weights were inherited, a "discovery" would become a heritable trait, culture
would collapse into genetics, and Phase 13 would be unable to distinguish
transmission from inheritance. A `lamarckian_fraction` config field exists,
defaults to zero, and enabling it is an explicit experimental condition that
must be reported, never a default.

## Realism That Is Deliberately Not Attempted

Recorded so nobody has to rediscover the reasoning:

- **Molecular-level genetics.** No codons, transcription, translation,
  protein folding, or explicit gene regulatory network dynamics at chemical
  timescales. The cost is prohibitive at 10^3 to 10^4 organisms and the
  added realism does not change the questions being asked. A coarse
  regulatory locus type is an open question for a later schema, not a
  commitment.
- **Physical body simulation.** Morphology evolves as typed modules on a
  discrete lattice (Phase 10, ADR-0019), which is a real structural
  morphospace. It is deliberately **not** rigid-body or soft-body dynamics:
  modules confer capability and cost, and they do not swing, bend, or
  collide with each other. Full biomechanics would dominate the compute
  budget and displace the culture and cognition work, and remains a
  different project.
- **Open-ended microbial genome evolution.** The field regime evolves
  between a bounded set of genotype classes rather than over open genomes
  (ADR-0020). This is a deliberate realism loss taken under the precedence
  order below, and it means only the individual regime can demonstrate
  open-ended evolution.
- **A correct relationship between microbial and organism timescales.**
  `field_steps_per_tick` is the knob that makes a microbial phase reachable
  in a finite campaign. It is an abstraction, not a claim, and no document
  may imply otherwise.
- **Real chemistry for materials.** Materials carry abstract physical
  properties (hardness, mass, durability, energy content), not composition.
- **Real climate or hydrology.** `docs/04-simulation-model.md` already
  labels the climate model ecological simulation, not weather forecasting.
  That stands.

## Related Documents

- Epistemic position: `25-emergence-and-epistemic-position.md`
- Genetics: `08-genetics-and-evolution.md`,
  `specifications/genome-schema-2.md`
- Learning: `specifications/plasticity-and-learning.md`
- Demography and life history: `planning/phase-8-demography-and-life-history.md`
- Ontogeny and sexual selection: `planning/phase-14-ontogeny-and-sexual-selection.md`
- ADR-0013 (encoding), ADR-0014 (learning), ADR-0017 (this policy)
