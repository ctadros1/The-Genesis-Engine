# Modular Morphology And Development Specification

Status: design specification, not implemented. Phase 9. Policy versions
`lifesim-morphology-v1`, `lifesim-develop-v1`; genome schema 3. Decision:
ADR-0019.

## Problem

Body plan is a small fixed parameter set: body scale, speed potential,
sensor range, and a handful of scalars. Every organism has the same
structure and only its numbers differ. Nothing about shape can evolve, and
`docs/02` currently defers rich morphology entirely.

That is a hard ceiling for the new goal in two ways. A world where structure
cannot change cannot produce the morphological radiation the `scratch` mode
is aiming at, and it has no representation in which "one cell" and "many
differentiated cells" are the same kind of object, which is exactly what the
multicellularity transition needs.

## The Representation

An organism is a set of **modules** occupying cells of a small fixed
lattice, connected through shared lattice edges.

    module = { lattice_position, module_type, scale, orientation, parameters }

The lattice is discrete (square or hex, config, fixed per world) and bounded
by `max_modules`. Positions are integer lattice coordinates, so morphology
is exactly representable, hashable, and comparable without any float
geometry.

This is deliberately **not** a physical body simulation. There is no rigid
body dynamics, no joint torque, no soft tissue. Modules confer capability
and cost; they do not swing. That distinction is what keeps the cost
tractable and the determinism exact, and it is the boundary ADR-0019
records.

### Module types

A bounded, versioned registry. Registry version enters the config hash.

| Type | Confers | Costs |
|---|---|---|
| `Structural` | Connection, mass, integrity | Mass, basal upkeep |
| `Sensory` | One bound input channel; range scales with module scale | Upkeep, and mass |
| `Motor` | Thrust; realized speed scales with motor mass over total mass | High upkeep, high energy per use |
| `Digestive` | Intake rate and assimilation efficiency | Upkeep |
| `Storage` | Energy capacity | Mass |
| `Reproductive` | Reproductive capability and per-offspring investment capacity | Upkeep |
| `Neural` | Controller node budget | Upkeep, disproportionate at scale |

A one-module organism is legal and is exactly what a unicell is. This is the
unification that makes the multicellularity transition expressible: a
unicell is a single undifferentiated module, and multicellularity is
evolving past one module with more than one type present. There is no
separate multicellularity mechanic, no threshold flag, and nothing that
detects or rewards the transition. It is simply a region of the same
morphospace.

### Phenotype derivation

Every runtime attribute that is currently a trait gene becomes a **derived
consequence of the module set**, computed by a pure deterministic function:

    mass          = sum over modules of (type_density * scale^3)
    max_speed     = f(total motor thrust / mass), clamped
    sensor_range  = per sensory module, from its scale and position
    intake_rate   = sum over digestive modules
    energy_max    = sum over storage modules
    basal_cost    = sum over modules of (type_upkeep * scale^3)
    node_budget   = sum over neural modules

Modules are iterated in ascending lattice index for every sum, so
float summation order is pinned exactly as it is for controller edges
(`specifications/determinism-extensions.md` Rule 6).

The trade-offs are then structural rather than authored. More motors means
speed and a metabolic bill. More neural modules means a bigger controller
and a disproportionate upkeep. Nothing anywhere states that a particular
body is good.

## Development: Genome To Body

Morphology is not stored directly in the genome. The genome stores a
**growth program** that is executed to produce the body. This is the
developmental encoding that genome schema 2 reserved as locus type tag 5 and
deferred; Phase 9 allocates it.

### Regulatory loci

    Regulatory { homology_id, gene_lineage_id, mutation_event_id,
                 condition, action, parameters }

A growth step reads local context (current module count, neighbour types,
position, developmental clock, and an energy-availability term) and may
place a module, differentiate one, change a scale, or terminate a branch.

Execution is a bounded deterministic loop:

1. Start from a single `Structural` module at lattice origin.
2. For `step` in `0..max_growth_steps`: evaluate every regulatory locus in
   ascending `homology_id` order against the current body; collect the
   matching actions; apply them in ascending `(locus_innovation_id,
   lattice_index)` order.
3. Stop at `max_growth_steps`, at `max_modules`, or when no locus matches.
4. Validate: connectivity, module count, at least one of any type the config
   requires for viability. An invalid body is a **non-viable organism**, not
   a repaired one: the birth is rejected with a typed reason and counted,
   exactly as a capacity rejection is.

Development runs once at birth by default. With Phase 13 ontogeny enabled it
runs incrementally across the lifespan, with growth consuming energy through
the existing ledger.

### Why indirect rather than direct

A direct encoding, one locus per module, cannot easily produce repeated
structure: doubling a limb count means duplicating every locus of that limb.
A growth program produces repetition, symmetry, and segmentation from small
genetic changes, which is how real morphological variation is generated and
is what makes the morphospace searchable at all.

The cost is honest and recorded in ADR-0019: indirect encodings are harder
to analyze, the genotype-phenotype map is many-to-one and discontinuous, and
small genetic changes can produce large phenotypic jumps.

Two commissioned reviews (`genetics` section 1.6, `neuroevolution` section 1.4)
recommend against making a developmental program a baseline encoding at all.
ADR-0022 D1 records why that is partially declined for morphology and fully
adopted elsewhere, and what it costs:

- The **controller** stays directly encoded. The developmental program is
  scoped to morphology only, because the one-module-is-a-unicell unification
  is what removes the need for an authored multicellularity mechanic.
- The program is declared as a **bounded versioned module** carrying every
  field `genetics` section 1.6 requires: maximum expansion steps, maximum emitted
  modules, deterministic rule-match and conflict order, phenotype-overflow
  behavior, canonical intermediate representation, compiler and registry
  versions, and provenance links from each emitted module back to the locus
  that generated it.
- The **direct parameterized body plan is retained as a specified
  fallback**, not a hypothetical one.
- Phase 9's discontinuity measurement is a **gate**, not a metric. If a
  typical single-locus mutation produces an unrelated body, the encoding has
  failed its own premise and the fallback is taken.

## Interaction With Existing Systems

| System | Effect |
|---|---|
| Genome (schema 2) | Gains regulatory loci; becomes schema 3. Trait loci for body scale, speed potential, and sensor range are **retired**, their roles now derived from modules. Retired trait IDs are never reused |
| Controller | Sensory modules bind input channels; motor modules bind action channels. Node budget from neural modules caps controller size, so brain size costs body |
| Energetics | Basal cost, intake, and storage all derive from modules; the existing ledger paths are unchanged in form |
| Save format | Bodies are derived and **not stored**: they are recomputed from the genome on load, like phenotypes today. Only the developmental clock is stored under Phase 13 |
| Protocol | Render records carry a compact module summary, not the full lattice. Deep morphology stays on the HTTP detail path |
| Similarity | Genetic distance gains a morphological component: lattice-occupancy difference over the union of occupied positions |

## Determinism

- New stream `Morphogenesis` (17) for any stochastic developmental term.
  The default policy is fully deterministic development with no draws; the
  stream exists so that adopting developmental noise later cannot renumber.
- Regulatory loci evaluate in ascending `homology_id` order; actions apply
  in ascending `(locus_homology_id, lattice_index)` order. Body
  construction never depends on iteration order.
- All module sums iterate in ascending lattice index.
- Lattice positions are integers; no float geometry anywhere in morphology.
- Development is a pure function of `(genome, config)`, so a body recomputed
  after restore is bit-identical, and bodies are excluded from the save for
  exactly that reason.
- Checksum section `lifesim-morphology-state-v1` carries only the
  developmental clock and the non-viability counters, present only when the
  morphology section is enabled.

## Cost, Stated Honestly

This is the most expensive item in the programme after the genome
successor.

- Development runs per organism per birth, and per tick under incremental
  ontogeny. It is bounded by `max_growth_steps` and `max_modules`, both
  config, both enforced by deterministic rejection.
- Per-organism cost becomes a function of module count, so tick time
  develops the same skewed distribution that variable topology introduces.
- Neural node budget derived from neural modules means controller cost and
  body cost are coupled; the two skews multiply.
- Save size does not grow, because bodies are derived. This is a deliberate
  design choice made specifically to avoid stacking a fourth growth term on
  the snapshot budget already strained by ADR-0013, ADR-0014, and ADR-0015.

Phase 9 measures all of it before the caps are fixed.

## Test Requirements

- Development is a pure function: same genome and config gives the same
  body, and a body recomputed after save and restore is identical.
- Order independence: permuting regulatory locus storage order gives an
  identical body.
- Connectivity and viability validation rejects disconnected or invalid
  bodies with typed reasons; no repair path exists.
- Caps enforce deterministically, count, and event; no cap silently
  exceeded.
- Phenotype derivation bounds: no derived attribute leaves its clamp for any
  body reachable within the caps.
- A one-module body is legal, viable, and produces sane derived attributes,
  since it is the unicellular case.
- Ledger exactness with growth energy flowing through it.
- Genotype-phenotype discontinuity is measured and reported: the
  distribution of phenotypic distance produced by single-locus mutations.
- Morphology disabled reproduces the Phase 8 fixture exactly.
