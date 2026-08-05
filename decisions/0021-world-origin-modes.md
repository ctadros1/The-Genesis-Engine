# ADR-0021: World Origin Modes

Status: Proposed
Date: 2026-08-04
Author: Origin-modes revision

## Context

How a world begins is hard-coded: `Genome::founder` draws every trait from
the middle half of its range and every neural weight from `[-0.5, 0.5]`, and
`initial_organisms` of them appear at once on random land cells. There is no
way to specify founder genomes, seed distinct groups in distinct places, or
begin from anything other than fully-formed mobile organisms.

That is a research limitation, not just a missing convenience. Phase 7's
kin-biased grouping and inter-group conflict criteria are far sharper from
several separated founder demes than from one well-mixed pool, and whether a
behavior is reachable is a different question from naive founders than from
pre-adapted ones.

The user additionally asked for an Earth-analogue "head start" and a
from-scratch mode. Both are starting conditions, which is what makes them
admissible at all.

## Options Considered

- **Keep founders hard-coded.** No new surface, no new risk, and the
  population-structure question stays unaskable.
- **A single configurable founder distribution.** Covers `random` with demes
  and nothing else.
- **Three named origin modes with a shared founder framework.**

## Proposed Decision

Adopt three modes under `origin.mode`, specified in
`specifications/world-origin-modes.md`.

| Mode | Founders |
|---|---|
| `random` | Bounded-random single body plan. Current behavior, preserved exactly |
| `seeded` | Biome-matched founder archetypes placed in the biomes they suit |
| `scratch` | No organisms; a chemistry field from which protocells may arise |

Plus a founder-population file usable by any mode, so a campaign can start
from a previously evolved population without branching a full save.

### The boundary that makes this admissible

**Authoring where the search starts is not authoring the path it takes.** A
seeded world begins with adapted populations in matching biomes and is then
subject to exactly the same rules as a random one. Nothing about the
starting condition constrains, guides, or rewards what happens next.

Three consequences enforced rather than assumed:

- **Archetypes are distributions, not organisms.** Each founder is an
  independent draw from a trait and morphology distribution. Two founders of
  one archetype are not identical, and neither is a designed creature.
- **Archetype names are presentation only.** The kernel stores an archetype
  ID for provenance and nothing reads it: no rule, no input channel, no
  mating gate, no analysis grouping. This is the ADR-0016 line applied to a
  new label, for the same reason: the moment an authored label can explain
  an outcome, the outcome stops being evidence. A test asserts inertness by
  permuting archetype IDs and requiring identical trajectories.
- **No archetype is named after a real species**, and no document may
  describe a seeded run as containing mammoths or frogs. Archetypes are
  functional trait distributions a biome makes plausible, and describing them
  otherwise would import a claim the simulation cannot support.

### Ages are not a mode

The user-facing intuition of an ice age is delivered by biome structure at
generation plus climate drift during the run
(`specifications/biome-and-climate.md`). There is no age state, no age
transition, and nothing spawned or swapped because of one. An observer may
call a sustained cold stretch an ice age afterwards; that is Phase 17
segmentation over recorded history, a label applied to the past.

Had this been implemented as a world phase that triggers and populates, it
would be an era the simulation enters, which `docs/25` rules out and which
would make every result about climate adaptation a result about our
schedule instead.

## Consequences

Positive: founder population structure becomes controllable, which several
existing phase criteria need; the head start is available without
compromising the philosophy; `scratch` gets a home.

Negative and accepted:

- **A seeded run is a weaker basis for a reachability claim** than a random
  one, because the starting point was chosen. Any result from a seeded run
  reports its archetype set, and a claim of the form "behavior X evolved"
  means something different under `seeded` than under `random`. The reports
  must keep saying which.
- Archetype authoring is a recurring temptation to encode expectations.
  The inertness test is the guard, and it is a required test rather than a
  convention.
- Biome-matched seeding depends on biomes, which do not exist yet and are
  therefore in the same phase.

Compatibility: `origin.mode = random` with default parameters reproduces
today's founder generation bit-for-bit, including `0xff9dfcff5dffbf42`. The
origin section enters the config hash only when the mode or any parameter
differs from the recorded defaults, following D-014.

## Performance Implications

Negligible for `random` and `seeded`: founder generation is a one-time cost
at tick 0. `scratch` is dominated by the field regime and is covered by
ADR-0020.

## Operational Implications

Founder files are a new decoder and therefore new hostile-input surface,
held to the same fail-closed bounded-decode standard as the genome, save,
and protocol codecs.

## Revisit Conditions

- Archetype inertness fails, indicating a leak from provenance into
  behavior or analysis.
- Deme seeding proves insufficient for Phase 7's population-structure
  criteria, suggesting structure must be maintained rather than merely
  initialized.
- A research question needs a starting condition none of the three modes
  expresses.

## Evidence Required To Accept

- Phase 6 acceptance criteria: fixture preservation under `random`,
  order-independent founder generation, deme separation and genetic
  distinctness at tick 0, biome-matched placement or fail-closed, archetype
  ID inertness, and a fail-closed founder-file corruption sweep.
- Confirmation that a seeded run and a random run under otherwise identical
  config produce different config hashes.
