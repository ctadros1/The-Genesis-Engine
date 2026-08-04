# ADR-0020: Two-Regime Simulation For The Unicellular Phase

Status: Proposed
Date: 2026-08-04
Author: Origin-modes revision

## Context

The `scratch` origin mode begins with no organisms, so something must
happen between an empty world and the first individual-based organism.

The obstacle is scale, not biology. A microbe divides on the order of tens
of minutes and a macro-organism lives for years; a productive microbial
population is measured in millions per litre against a proven
individual-based tier of 500 to 2,000 entities. Per-individual microbes
would need several orders of magnitude more entities and ticks than the
kernel has ever been benchmarked at, for a phase expected to return null.

## Options Considered

- **One individual-based engine throughout**, with a configured tick meaning
  and very large population caps. Conceptually uniform. Requires millions of
  entities where 2,000 is the proven tier, and would consume the entire
  compute budget of the programme on its least tractable phase.
- **Skip abiogenesis**, starting `scratch` at simple unicellular
  individuals that already exist. Cheap, and it declines the part of the
  request that motivated the mode.
- **Two coupled regimes**: a field/population-level microbial phase over the
  raster, handing off to the existing individual engine.

## Proposed Decision

Adopt two coupled regimes, specified in
`specifications/unicellular-regime.md`.

The field regime holds per-cell chemistry concentrations and per-cell,
per-genotype-class microbial densities. The individual regime is the
existing engine, unchanged. Organisms consume from and excrete into the
field; field populations that cross an aggregation and complexity threshold
materialize as individuals.

Load-bearing elements:

- **No per-individual randomness in the field regime.** Field cost and field
  determinism are both independent of population size, which is the property
  that makes the regime affordable at all.
- **Everything fixed point.** The field integrates over horizons far longer
  than an organism lifetime, so Rule 7 applies with more force here than
  anywhere else in the project.
- **Canonical ordering throughout**: cells in ascending index, class flows in
  ascending `(cell_index, source_class, target_class)`, materialization in
  ascending `(cell_index, class_id)` with entity IDs from the existing
  shared monotonic counter.
- **Exact conservation across the boundary.** Mass and energy moving between
  field and individuals go through the same ledger, with rounding remainders
  assigned to the lowest new entity ID per the existing convention.
- **The threshold is a representation change, not an achievement.** Crossing
  it means the field representation is no longer adequate for what is there.
  The organism gains nothing, and Phase 15 tests transition neutrality
  directly by comparing a materialized organism against an otherwise
  identical normally-born one.
- **Materialized organisms are one-module bodies** (ADR-0019), so a unicell
  is an ordinary organism in the ordinary morphospace and going multicellular
  is ordinary structural mutation. There is no second transition mechanic.

## Consequences

Positive: a microbial phase becomes reachable in a finite campaign; the
individual engine is untouched; the transition needs no special
multicellularity machinery.

Negative and accepted:

- **The handoff is the most likely place in the entire programme for a
  determinism or conservation defect.** Two representations of the same
  matter, converting under a threshold, is exactly the shape of bug that
  passes casual testing and corrupts a long run. Phase 15's criteria are
  weighted heavily toward it.
- **Genotype classes are a discretization.** Microbial evolution in the
  field regime is movement between a bounded set of classes, not open-ended
  genome evolution. This is a real loss of realism, taken deliberately under
  the ADR-0017 precedence order (determinism first, bounded state second),
  and it means the field regime cannot itself demonstrate open-ended
  evolution. Only the individual regime can.
- **`field_steps_per_tick` is an abstraction, not a claim.** Running
  microbial dynamics at a multiple of the organism tick makes the phase
  reachable; it is not an assertion that the two timescales are correctly
  related to anything real, and no document may imply otherwise.
- Field state is **stored**, unlike derived bodies and biomes, because it
  cannot be recomputed. This adds a per-cell growth term to the snapshot.

Compatibility: the field regime is a config section, inert when disabled,
reproducing the Phase 13 fixture exactly. It does not alter the individual
regime's rules.

## Performance Implications

Field cost is proportional to cells times classes and independent of
organism population, verified by benchmark across population tiers in Phase
14. Snapshot growth from stored field state is measured there too.

The compute-cost risk in `docs/20-risk-register.md` is made materially worse
by this mode, and ADR-0018's mandatory unscaffolded control doubles the seed
cost of every transition claim on top of it.

## Operational Implications

Snapshot growth. Nothing else.

## Revisit Conditions

- Conservation across the boundary cannot be made exact, in which case the
  transition mechanism is wrong and needs redesign rather than a tolerance.
- Field cost proves proportional to population after all, indicating the
  regime separation leaked.
- Abiogenesis never fires at a useful rate even under the permitted
  scaffolding, in which case `scratch` starts at existing unicells and the
  origin-of-life step is dropped with the measurement recorded.

## Evidence Required To Accept

- Phase 14 and 15 acceptance criteria, in particular exact conservation over
  a 10^6-tick run including many transitions, transition neutrality, and
  field cost independence from population.
- Snapshot size and restore time with a populated field.
- Phase 13 fixture reproduced exactly with the regime disabled.
