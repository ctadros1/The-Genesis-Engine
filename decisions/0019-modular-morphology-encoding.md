# ADR-0019: Modular Morphology With Developmental Encoding

Status: Proposed
Date: 2026-08-04
Author: Origin-modes revision

Extends ADR-0013 (variable-topology genome). Allocates the regulatory locus
type that ADR-0013 reserved and deferred.

## Context

Body plan is a small fixed parameter set. Nothing about structure can
evolve. `docs/02` deferred rich morphology entirely, and that deferral is
now blocking: the `scratch` origin mode is aiming at morphological
radiation, and the multicellularity transition needs a representation in
which "one cell" and "many differentiated cells" are the same kind of
object.

## Options Considered

- **Parameterized body plan.** Extend the genome with numeric morphology
  genes: segment count, limb count, sensor count, size. Cheap, deterministic,
  fits the existing architecture and save format with no new machinery.
  Enough to distinguish a swimmer from a walker; not enough for a genuinely
  novel body, and it gives no natural representation of a unicell.
- **Modular lattice with a developmental encoding.** An organism is typed
  modules on a discrete lattice, grown by a genome-encoded growth program.
- **Full physical body simulation.** Rigid or soft body dynamics. Would
  dominate the compute budget and displace the culture and cognition work
  that Phases 8 through 16 are built around.

## Proposed Decision

Adopt the modular lattice with a developmental encoding, specified in
`specifications/morphology-and-development.md` as genome schema 3.

Load-bearing elements:

- **Discrete lattice, integer coordinates.** Morphology is exactly
  representable, hashable, and comparable with no float geometry anywhere.
- **Bounded typed module registry**, versioned, each type conferring
  capability and costing mass and upkeep.
- **Derived phenotype.** Mass, speed, sensor range, intake, storage, basal
  cost, and controller node budget all become consequences of the module
  set rather than independent genes. Trade-offs become structural instead of
  authored; nothing states that a particular body is good.
- **Indirect developmental encoding.** The genome stores a growth program of
  regulatory loci, not a module list. This is what produces repetition,
  symmetry, and segmentation from small genetic changes, which is how
  morphological variation is generated and what makes the morphospace
  searchable.
- **Bodies are derived, not stored.** Development is a pure function of
  `(genome, config)`, so a body is recomputed on load exactly as phenotypes
  are today. This is a deliberate choice to avoid stacking a fourth growth
  term onto a snapshot budget already strained by ADR-0013, ADR-0014, and
  ADR-0015.
- **Explicitly not a physics simulation.** Modules confer capability; they
  do not swing, bend, or collide with each other. That boundary is what
  keeps the cost tractable and the determinism exact.

### The unification that justifies the cost

A one-module organism is legal and is exactly what a unicell is.
Multicellularity is evolving past one module with more than one type
present. There is no multicellularity mechanic, no threshold flag, and
nothing that detects or rewards the transition: it is a region of the same
morphospace, reached by ordinary structural mutation.

This is the strongest argument for the expensive option over the cheap one.
The parameterized body plan would have required a separate, authored
multicellularity mechanism, and an authored transition is precisely what
ADR-0012 forbids.

## Consequences

Positive: structure evolves; the transition needs no special case; the
brain-body cost coupling (neural modules carry upkeep) makes cognition
expensive in a way that is structural rather than stipulated.

Negative and accepted:

- **Indirect encodings are hard to analyze.** The genotype-phenotype map is
  many-to-one and discontinuous, and a single-locus mutation can produce a
  large phenotypic jump. Phase 9 measures that discontinuity distribution
  rather than assuming it is tolerable.
- Development runs per organism per birth, and per tick under incremental
  ontogeny. Per-organism cost becomes module-count dependent, so tick time
  acquires a second skewed distribution on top of the one variable topology
  introduces, and the two multiply.
- Three trait genes are retired (body scale, speed potential, sensor range),
  their roles now derived. Retired trait IDs are never reused.
- An invalid body is a non-viable organism whose birth is rejected. If the
  growth-program search produces a high non-viability rate, effective
  fecundity drops and the ecology shifts; Phase 9 reports the rate as a
  first-class metric.

Compatibility: genome schema 3 is config-selected. Schema 1 and schema 2
worlds keep their schemas, their decoders, and their fixtures permanently.
There is no migration between schemas, for the reason ADR-0013 already
records: a converted genome would be a record that never existed.

## Performance Implications

Unmeasured, and the largest single unknown in the programme after ADR-0013.
Phase 9 records development cost per birth, per-organism cost against module
count as a distribution, the interaction with controller cost, and the
non-viability rate. Caps are set from that measurement rather than guessed.

Snapshot size is deliberately unaffected.

## Operational Implications

Protocol change: render records carry a compact module summary, with deep
morphology on the HTTP detail path. Observer work to render variable bodies.

## Revisit Conditions

- Genotype-phenotype discontinuity is severe enough that selection cannot
  act, in which case a more constrained growth grammar or the parameterized
  fallback is reconsidered.
- Development cost or non-viability rate breaks the tick budget or the
  ecology at a supported tier.
- The lattice proves too restrictive for a research question, which would be
  an argument for physical morphology and therefore for a different project.

## Evidence Required To Accept

- Phase 9 acceptance criteria, in particular development purity and order
  independence, the discontinuity measurement, the non-viability rate, and
  the per-organism cost distribution.
- A one-module body producing sane derived attributes, since the
  unicellular case depends on it.
- Phase 8 fixture reproduced exactly with morphology disabled.
