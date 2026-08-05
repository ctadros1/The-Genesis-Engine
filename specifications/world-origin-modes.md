# World Origin Modes Specification

Status: design specification, not implemented. Policy version
`lifesim-origin-v1`. Phases 6 (framework, `random` and `seeded`) and 15
(`scratch`). Decision: ADR-0021.

## Problem

How a world begins is currently hard-coded. `Genome::founder` draws every
trait uniformly from the middle half of its range and every neural weight
from `[-0.5, 0.5]`, and `initial_organisms` of them appear simultaneously on
random land cells. There is no way to specify founder genomes, load a
founder population, seed distinct groups in distinct places, or begin from
anything other than fully-formed mobile organisms.

That is a limitation for the research programme, not just a missing
convenience. Testing kin-biased grouping and inter-group conflict (Phase 7)
is far sharper from several spatially separated founder demes than from one
well-mixed pool, and testing whether a behavior is reachable is a different
question depending on whether founders are naive or pre-adapted.

## The Three Modes

`origin.mode` is versioned config. Every mode is a **starting condition**,
never a trajectory: the simulation authors where the search begins and never
what it does next. See ADR-0012, and ADR-0018 for the one bounded exception
that applies to `scratch`.

| Mode | Founders | Phase |
|---|---|---|
| `random` | Bounded-random single body plan. The current behavior, preserved exactly | 6 |
| `seeded` | Biome-matched founder archetypes placed in the biomes they suit | 6 |
| `scratch` | No organisms. A chemistry field from which protocells may arise | 15 |

### Fixture preservation

`origin.mode = random` with default parameters reproduces today's founder
generation bit-for-bit, including `0xff9dfcff5dffbf42`. The origin section
participates in the config hash only when `mode != random` or any parameter
differs from the recorded defaults, following the D-014 precedent exactly.

## Mode `random`

Unchanged in behavior, but its constants become config instead of literals:

| Field | Default | Meaning |
|---|---|---|
| `trait_low_q16`, `trait_span_q16` | 0.25, 0.5 | Founder trait draws land in `low + u * span` |
| `neural_span` | 1.0 | Founder neural weights in `+/- span/2` |
| `deme_count` | 1 | Number of separated founder groups |
| `deme_radius_m` | world-wide | Placement radius around each deme centre |

`deme_count > 1` is the addition that matters. Deme centres are chosen
deterministically from the `FounderSeed` stream over valid land cells,
separated by at least `deme_min_separation_m`, and each deme's founders draw
from an independently offset sub-stream so demes start genetically distinct.
This gives Phase 7 the population structure its criteria need.

## Mode `seeded`: The Head Start

Founders are drawn from **archetypes**: named founder distributions with a
trait profile, a morphology profile (Phase 10 onward), and a biome affinity.

An archetype is a *distribution*, not an organism. It is a set of trait
means and variances plus a module layout template, and every founder is an
independent draw from it. Two founders of the same archetype are not
identical and neither is a designed creature.

### Archetype names never enter the kernel or any analysis

Archetypes may carry evocative names in config for human legibility
(`cold_large_grazer`, `arid_burrower`). Those names are **presentation
only**. The kernel stores an archetype ID for provenance and nothing reads
it; no rule, no input channel, no mating gate, and no analysis grouping may
consult it. Reports may state which archetypes seeded a run and may never
use an archetype as an explanatory category for a later observation.

This is the same line already drawn for similarity cluster labels
(ADR-0016), and for the same reason: the moment an authored label can
explain an outcome, the outcome stops being evidence.

Crucially, an archetype is not a claim about any real organism. No archetype
is named after an extant or extinct species, and no document may describe a
seeded run as containing mammoths, frogs, or trilobites. The archetypes are
functional trait distributions that a biome makes plausible.

### Biome matching

Each archetype declares a biome affinity vector over the biome registry
(`specifications/biome-and-climate.md`). Placement is deterministic:

1. Classify every land cell into a biome at generation time.
2. For each archetype, materialize the candidate cell set for its affinity,
   sorted by `(cell_index)`.
3. Draw placements from the `FounderSeed` stream keyed on the prospective
   founder's entity ID.
4. Allocate founder entity IDs in ascending `(archetype_id, draw_index)`
   order so the population is reproducible regardless of iteration.

If the map is large enough to contain many biomes, a `seeded` world starts
with several distinct adapted populations in different places. That is the
"all biomes at once" case, and it falls out of biome-matched placement
rather than needing its own mechanism.

### What a "cold period" is, and is not

The user-facing intuition of an ice age is delivered by two independent
things, neither of which is an era:

- **Biome structure at generation.** Cold regions exist and are seeded with
  cold-affinity archetypes at tick 0.
- **Climate drift over the run.** Temperature and moisture vary on long
  timescales (`specifications/biome-and-climate.md`), so cold regions
  expand and contract and organisms adapt, migrate, or die.

No code ever reads "the world is in an ice age." There is no age state, no
age transition, and nothing is spawned or swapped because of one. An
observer may look at the event log afterwards and call a cold stretch an ice
age; that is Phase 17's job and it is a label applied to history, never a
state the simulation entered. See `docs/25-emergence-and-epistemic-position.md`.

## Mode `scratch`

The world begins with no organisms and a chemistry field. Protocells may
arise by abiogenesis; unicells may arise from protocells; multicellular
organisms may arise from unicells. Each step is physics with a probability,
not a scheduled event.

Specified in `specifications/unicellular-regime.md` and
`specifications/morphology-and-development.md`. Delivered in Phases 15 and
16, which are placed late for reasons stated in
`docs/19-implementation-roadmap.md`: nothing else depends on them, and they
are the least tractable work in the programme.

**This mode is expected to return null at one or more transitions**, and
that expectation is recorded in advance rather than discovered. The
scaffolding permitted under ADR-0018 applies here and only here, with its
mandatory unscaffolded control.

## Founder Population Files

Any mode may instead load founders from a file: a versioned, checksummed,
bounded-decode founder set carrying genomes, positions, and provenance. This
is how a campaign starts from a previously evolved population without
branching a full save.

Decode is fail-closed on the same terms as every other codec: magic, version,
counts, and checksum verified before allocation; every genome validated;
positions checked against terrain; no repair path. A founder file records the
run that produced it, so a pre-adapted starting condition can never be
mistaken for a naive one.

## Determinism

- New stream `FounderSeed` (22) owns archetype selection, deme centre
  choice, and founder placement. It is separate from `GenomeInit` (5) so
  that adding origin modes cannot shift the founder sequence of an existing
  `random` world.
- Founder entity IDs are allocated in a canonical order (`archetype_id`,
  then draw index), never in iteration order.
- Deme centres are drawn, then **sorted by cell index** before founders are
  assigned to them.
- Archetype and founder-file provenance enter the state checksum under
  `lifesim-origin-state-v1` only when `mode != random`.
- Every archetype definition and every scaffold parameter is inside the
  config hash, so two origin configurations are never the same experiment.

## Test Requirements

- `random` with defaults reproduces `0xff9dfcff5dffbf42` exactly.
- Founder generation is order-independent: permuting archetype iteration
  order produces an identical founder population.
- Deme separation is honored, and `deme_count > 1` produces measurably
  distinct genetic clusters at tick 0.
- Biome-matched placement puts every founder in a cell matching its
  archetype affinity, or fails closed with a typed error if no such cell
  exists.
- Founder-file decode: bounded, fail-closed, seeded corruption sweep with
  zero panics.
- Archetype IDs are provably inert: a run with archetype IDs permuted
  produces identical trajectories given identical founder genomes.
- `scratch` with abiogenesis disabled produces an empty world that remains
  valid, savable, and observable, exactly as an extinct world does today.
