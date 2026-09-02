# ADR-0031: The Field Regime's Concrete Design (Phase 15)

Status: accepted 2026-09-02. The design authority for Phase 15 remains
`specifications/unicellular-regime.md` (ADR-0020, ADR-0018); this record
pins the concrete choices that specification deliberately left open, so
the implementation cannot pick them silently. Where this record and the
specification disagree, the disagreement is a defect in this record.

## Substrate registry (`lifesim-chemistry-v1`, registry version 1)

Four abstract substrates, ids permanent:

  0  S_PRIMORDIAL  abiotically produced (the scaffold's lever), inert food
                   for the simplest metabolic strategy
  1  S_MONOMER     produced from S_PRIMORDIAL by the one abiotic reaction;
                   the richer metabolic input
  2  S_POLYMER     produced only by microbial growth (a byproduct with
                   higher energy content); the surface term of the
                   abiogenesis rate function reads it
  3  S_WASTE       every metabolism's sink; decays abiotically back to
                   S_PRIMORDIAL at a configured rate, closing the cycle

One abiotic reaction table, fixed and versioned: `S_PRIMORDIAL ->
S_MONOMER` (rate law linear in concentration, Q16 rate per field step)
and `S_WASTE -> S_PRIMORDIAL` (the recycling term). Stoichiometry is
1:1 in mass everywhere, so conservation is addition and subtraction of
the same integers - by construction, not by tolerance. Concentrations
are i64 milli-units per cell (Rule 7).

## Genotype-class registry (`lifesim-microbial-v1`, registry version 1)

Classes are the cross product of three small axes, generated in a fixed
order so `class_id` is permanent for a given axis configuration (the axis
sizes are config, inside the hash, swept by the campaign per the plan):

  substrate preference  {S_PRIMORDIAL, S_MONOMER}          (2 default)
  replication rate      {low, high}                        (2 default)
  aggregation tendency  {low, high}                        (2 default)

Eight classes by default. Mutation flow moves density only between
classes differing in exactly one axis step, at one configured Q16 rate,
applied in ascending `(cell_index, source_class, target_class)`. Growth
converts preferred substrate to class density at the class's replication
rate times a configured yield; death returns a configured fraction of
density to S_WASTE and the rest to S_PRIMORDIAL - every flow names its
source and sink, so the mass ledger closes term by term.

## Update order and buffering

Per field step (`field_steps_per_tick` of them per world tick, config,
u32 >= 1, hashed): diffusion, then abiotic reactions, then per-class
growth/death/mutation, then the aggregation term - each pass reading a
committed buffer and writing the next (double-buffered), so storage
order cannot influence results and the C15.7 permutation clause is a
property of the structure. Diffusion is the von Neumann 4-neighbour
stencil: each cell sends `concentration * rate_q16 >> 16` per neighbour,
truncated; what is subtracted from the source is exactly what is added
to the destinations, and the truncation remainder simply stays home -
conservation by construction. Boundary cells send nothing off-map.

## Abiogenesis

The rate function reads, per cell: S_PRIMORDIAL and S_MONOMER
concentration, the temperature field where climate is enabled (a neutral
constant otherwise), and S_POLYMER as the surface term - each mapped
through a configured Q16 weight, summed, capped. The `Abiogenesis`
stream (18) draws keyed `(seed, tick, cell_index)`; a firing adds a
configured seed density to the founder class (lowest class id whose
preference is S_PRIMORDIAL) and debits the same mass from S_PRIMORDIAL,
so even genesis conserves. Gated by `chemistry.abiogenesis_enabled`;
disabled leaves `scratch` a valid empty world (C15.5).

## Coupling surface (v1)

Phase 15 ships the organism-to-field direction only; materialization
(field-to-organism) is Phase 16's, with the `Transition` stream (20)
reserved now:

- **Excretion**: a configured fraction of every organism's basal cost is
  deposited as S_WASTE in its cell, through the ledger.
- **Remains**: a configured fraction of energy removed at death deposits
  as S_PRIMORDIAL in the death cell.
- Organism consumption FROM the chemistry field is a config term that
  ships at zero: the biomass field remains the food model, and wiring
  chemistry into feeding is its own future increment with its own
  conservation tests. C15.6's exchange test exercises excretion, remains
  and (in Phase 16) materialization.

## Streams, sections, formats

- RNG streams exactly as the specification names them: `Abiogenesis` =
  18, `MicrobialField` = 19 (allocated now, unused by the deterministic
  default policy), `Transition` = 20 (reserved for Phase 16). Stream 17
  stays unallocated; numbers are permanent, not dense.
- ALIF format 11: the chemistry config block appended to the prefix
  chain, `SECTION_CHEMISTRY` (19) carrying per-cell concentrations and
  the chemistry counters, `SECTION_MICROBIAL` (20) carrying per-cell
  per-class densities and the microbial counters - stored, never
  recomputed, per ADR-0020. Checksum tags `lifesim-chemistry-state-v1`
  and `lifesim-microbial-state-v1`, present only when enabled. The
  retained-writer/refusal/migration machinery follows formats 9 and 10.
- Benchmark schema 9 rows per the plan (field p50/p95 against cells,
  classes, `field_steps_per_tick`; population-independence across
  tiers; snapshot growth per cell).

## Scaffold parameterization (ADR-0018)

The neutral arm N runs spatially uniform abiotic S_PRIMORDIAL
production. Scaffold arms concentrate the same TOTAL production into
patches: `scaffold_patch_radius_cells` and
`scaffold_patch_contrast_q16` (production inside patches versus
outside, total held constant so no arm is richer). The plain-language
description - "the same substrate input concentrated into patches of
radius r at contrast c" - names no target; that patchy energy sources
favour local density persistence is the hypothesis under test, not an
outcome purchased. Intensity is swept over contrast per ADR-0018's
condition 4; every S arm runs its N control on the same seeds.

## Consequences

- Every mass term has a named source and sink, so C15.1's conservation
  is checkable term by term rather than only in total.
- The default eight classes are small enough that the field's cost
  envelope (cells x classes x steps) stays inside the environment
  phase's existing budget expectations; the class-count sweep is where
  the discretization-too-coarse risk gets its measurement.
- Deferring chemistry-as-food keeps Phase 15's conservation surface
  minimal; the cost is that C15.6 in this phase certifies a narrower
  exchange than the specification's eventual full coupling, and the
  findings must say so.

## As built (2026-09-02 amendment)

Where the implementation and this record's letter diverged, the record
owns it here rather than silently:

- **Formats.** This record said "ALIF format 11" for the whole phase.
  Implementation shipped three: 11 (chemistry, `SECTION_CHEMISTRY` 19),
  12 (microbial, `SECTION_MICROBIAL` 20), 13 (the coupling fractions,
  byte-shaped, no section) - the Phase 14 precedent (ADR-0030 across
  formats 9 and 10), each with its own retained writer, refusal and
  migration. Benchmark schema stayed at 10: the phase adds no tick
  phase, and the marker-record format is unchanged (the "schema 9" line
  above was stale when written).
- **Aggregation term.** The update order names an aggregation pass; v1
  ships the aggregation *axis* as a heritable class dimension (mutation
  flows across it) with no separate spatial aggregation pass - the axis
  is registry-real, behaviourally inert until a later increment gives it
  a pass of its own. Nothing hashed changes when that lands; the pass
  will be config-gated.
- **Abiogenesis rate inputs.** The temperature term ships at weight
  zero implicitly: the rate function reads the three substrate
  concentrations only. Wiring the climate field in is a config change
  (a fourth weight) for a phase that needs it.
- **Excretion's basal share.** "Fraction of basal cost" is implemented
  as the cost before movement, carrying and crowding join the bill -
  which includes the allometric and thermal terms - floored by what the
  organism actually paid. Starvation deaths deposit zero remains
  (energy floors at zero before death).
- **A truncation floor worth knowing.** The default `mutation_q16` (66,
  0.001) moves nothing below 993 milli of density; abiogenesis seeds at
  most 1,000 with death applied first, so campaigns that want the
  mutation term live must set the rate above the floor (the fixture and
  the pre-registered campaign use 4096) or seed harder. Declared in the
  pre-registration, not discovered after.
- **C15.1 as an invariant.** The exact field identity (joint with the
  microbial half, chemistry-only without) is enforced by
  `World::check_invariants`, so campaign check-intervals and every
  test's invariant sweep carry it; the reduction re-checks it
  independently from the recorded series.
