# Abiogenesis And The Unicellular Regime Specification

Status: design specification, not implemented. Phases 15 and 16. Policy
versions `lifesim-chemistry-v1`, `lifesim-microbial-v1`,
`lifesim-transition-v1`. Decision: ADR-0020.

## Problem

The `scratch` origin mode begins with no organisms. Something has to happen
between an empty world and the first individual-based organism, and it
cannot happen in the existing engine.

The obstacle is scale. A bacterium divides on the order of tens of minutes
and a macro-organism lives for years; a productive microbial population is
measured in millions per litre while the proven individual-based tier is
500 to 2,000 entities. Running per-individual microbes on a 100 ms tick
would need several orders of magnitude more entities and ticks than the
kernel has ever been benchmarked at, for a phase that is expected to return
null.

## The Two-Regime Engine

The world runs two coupled regimes over the same raster.

| | Field regime | Individual regime |
|---|---|---|
| Represents | Chemistry and microbial populations | Organisms |
| Granularity | Per cell, per genotype class | Per entity |
| State | Concentrations and densities, fixed point | The existing SoA arrays |
| Randomness | Per cell, not per individual | Per entity, as today |
| Cost | Proportional to cells x classes | Proportional to population |

The individual regime is the existing engine, unchanged. The field regime is
new. They are coupled in one direction most of the time (organisms consume
from and excrete into the field) and in both directions at the transition.

### Field regime state

Per raster cell:

- **Chemistry**: concentrations of a small bounded set of abstract
  substrates. Not real chemistry; a small set of interconvertible resource
  species with stoichiometric rules and an energy currency.
- **Microbial populations**: a density per **genotype class**, where a class
  is a bounded discretization of microbial genotype space (metabolic
  strategy, substrate affinity, replication rate, aggregation tendency).
  Classes are a fixed registry, not per-individual genomes.

Everything is fixed point. Densities, concentrations, and every accumulator
follow Rule 7, because the field integrates over very long horizons and
float accumulation there would be far worse than the per-lifetime case that
already forced fixed point on learned weights.

### Field regime update

Per tick, over cells in ascending cell index:

1. Diffusion of chemistry between adjacent cells, a fixed stencil with
   exactly conserved totals.
2. Abiotic reactions, deterministic rate laws in fixed point.
3. Per class: growth from available substrate, death, and mutation flow
   between neighbouring classes at a configured rate.
4. Aggregation pressure, a per-class term that responds to the environmental
   structure the scaffold config shapes.

Class-to-class flows apply in ascending `(cell_index, source_class,
target_class)` order. Nothing iterates a map.

## Abiogenesis

Protocells arise from chemistry, at a rate that is a function of local
conditions and never a scheduled event.

    protocell_rate(cell) = f(substrate concentration, energy gradient,
                             temperature, surface term)

The `Abiogenesis` stream (18) supplies the draw, keyed on `(seed, tick,
cell_index)`. When it fires, a seed density is added to a founder genotype
class in that cell. That is the whole mechanism: it produces a self-
replicating density in the field, not an entity.

Abiogenesis is config-gated and may be disabled, in which case `scratch`
produces an empty world that stays valid, savable, and observable, exactly
as an extinct world does today.

**Expected outcome, recorded in advance:** with a neutral chemistry
configuration, abiogenesis is expected to either never fire at a useful rate
or produce populations that do not persist. The scaffolding permitted under
ADR-0018 applies, with its mandatory unscaffolded control.

## The Transition To Individuals

The handoff is the hard part of this design and the most likely place for a
determinism or conservation defect.

### Trigger

A cell's genotype class crosses a configured **aggregation and complexity
threshold**: sustained density above a floor, an aggregation-tendency value
above a threshold, and a persistence requirement over a window.

The threshold is a physical condition, not a detector of success. Nothing
reads "this lineage has become multicellular" and grants anything. Crossing
the threshold means the field representation is no longer adequate for what
is there, so the representation changes. The organism gains nothing by
crossing it, and Phase 16's acceptance criteria are written to detect it if
it accidentally does.

### Materialization

In ascending `(cell_index, class_id)` order, for each triggering class:

1. Compute the biomass to convert, bounded by config.
2. Synthesize a schema-3 genome from the class's parameters through a
   documented, deterministic, versioned map. The resulting organism is a
   **one-module body** (`specifications/morphology-and-development.md`),
   which is exactly what a unicell is in the morphology representation.
3. Allocate entity IDs from the shared monotonic counter in that same
   canonical order.
4. Debit the field density and credit organism energy and mass so the
   ledger balances exactly, with the rounding remainder assigned to the
   lowest new entity ID, following the existing convention.

From that point the organism is an ordinary individual: it develops,
evolves, and may radiate into more modules through ordinary structural
mutation. **There is no second multicellularity mechanic.** Going from one
module to many is ordinary morphological evolution in the same morphospace,
which is the payoff of the unified representation.

### Reverse coupling

Organisms consume field chemistry and excrete into it, and their remains
return to it. Both directions run through the same exact ledger, so total
mass and energy are conserved across the regime boundary. This is the
invariant most at risk and Phase 16 tests it directly.

## Time Scaling

The field regime advances at a configured multiple of the individual tick,
`field_steps_per_tick`, so microbial dynamics can run faster than organism
dynamics without changing `dt` for organisms. The multiple is versioned
config and is inside the config hash.

This is an abstraction, and the documents say so: it is not a claim that the
two timescales are correctly related to anything real. It is the knob that
makes a microbial phase reachable in a finite campaign.

## Determinism

- New streams: `Abiogenesis` (18), `MicrobialField` (19), `Transition` (20).
- All field iteration is in ascending cell index; all class flows in
  ascending `(cell_index, source_class, target_class)`.
- No per-individual randomness exists in the field regime, so field cost and
  field determinism are both independent of population size.
- Materialization order is canonical and entity IDs come from the existing
  shared monotonic counter, so the transition cannot introduce
  order-dependence into the individual regime.
- Everything is fixed point; diffusion and reaction stencils conserve totals
  exactly by construction rather than approximately.
- Checksum sections `lifesim-chemistry-state-v1` and
  `lifesim-microbial-state-v1`, present only when the regime is enabled.
- Field state is **stored** in the save, unlike derived bodies: it cannot be
  recomputed from anything. This adds a per-cell growth term to the snapshot
  and Phase 15 measures it.

## Test Requirements

- Exact conservation: total mass and energy across chemistry, field
  populations, and individuals is invariant to the milli-unit over a
  10^6-tick run including many transitions.
- Diffusion conserves totals exactly under adversarial concentration
  gradients.
- Transition determinism: two cells triggering in the same tick materialize
  identically regardless of storage order.
- Transition neutrality: an organism materialized by transition has no
  advantage over an otherwise identical organism that was born normally.
  Tested by direct comparison of derived attributes and energy.
- Reverse coupling: organism consumption and death return exactly what they
  took.
- Field regime cost is independent of individual population, verified by
  benchmark across population tiers.
- Abiogenesis disabled produces a permanently empty but valid world.
- Save round trip with a populated field and mid-transition state.
- Field-regime disabled reproduces the Phase 13 fixture exactly.
