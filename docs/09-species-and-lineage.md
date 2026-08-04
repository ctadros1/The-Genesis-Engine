# Species And Lineage

## Phase 2 Implementation Status

Live ancestry state per organism is bounded: up to two immutable parent
IDs, creation tick, genome hash, ancestry depth (max of parent depths plus
one), and a child counter. The authoritative history is the append-only
event stream (`PairedBirth` with parent IDs, genome hash, per-parent energy
investment, and variation counts; `PairRejected` with a typed reason;
`Death`). No unbounded in-memory genealogy exists. Similarity clustering is
implemented as `lifesim-similarity-v1`: offline, deterministic, bounded
sampling in stable ID order, normalized trait distance (controller-weight
contribution configurable and 0 by default), threshold union-find into
connected components, labels assigned by first appearance. Reports record
algorithm, analysis tick, config hash, schema, sample policy, threshold,
and cluster sizes. Labels are observations only and never feed back into
behavior.

## Lineage

Each organism has a stable ID, up to two immutable parent IDs, birth tick, genome hash, and world lineage ID. The authoritative lineage source is the append-only birth/death event stream plus snapshot references. The live in-memory graph keeps only indexed relationships needed for current inspection; it must not grow unbounded through duplicated ancestry lists.

## Analysis Observes, It Never Instructs

The rule this document has held for similarity clustering since Phase 2 is
now a project-wide principle with structural enforcement (ADR-0016). Era
detection and tradition detection (Phase 12,
`specifications/era-and-tradition-detection.md`) obey it identically:

- Analysis lives in a separate `sim-analysis` crate that `sim-core` does not
  depend on, so feedback is a compile error rather than a review finding.
- Analysis draws from no RNG stream and holds no mutable world handle.
- Analysis version strings are recorded in reports and deliberately excluded
  from the config hash, because an analysis version can never affect a
  world. This is the inverse of the rule for behavior policies.
- A required test asserts world checksums are bit-identical with analysis
  enabled at any cadence and disabled.

One boundary case, stated because it looks like a violation and is not:
**genetic compatibility gating for reproduction is physics, not a label.**
It is computed directly from two genomes and would function identically if
every analysis module were deleted. Cluster labels never gate anything;
genome-derived distance may.

An era is a narrative an observer detects post hoc from the event log. It is
never a state the simulation enters and no organism is ever told about one.
Reports call detected regimes "segments" and never name them after human
historical periods.

## Species Labels

Species are analytical clusters, not hard simulation factions. The initial system periodically samples living genomes, computes a normalized genetic distance, and assigns provisional cluster labels. Cluster membership does not change action eligibility, mating rules, or rendering unless a configuration explicitly says so.

## Genetic Distance

For normalized gene vector values a and b:

    distance(a, b) = sum(w_i * abs(a_i - b_i)) / sum(w_i)

Weights and included genes are schema/versioned. Do not compare lengths or meanings across incompatible genome versions. Neural weight contribution can be downweighted or omitted in the first visualization because high-dimensional controller noise can obscure interpretable trait divergence.

## Clustering Policy

Run clustering outside the hot tick path at a bounded cadence or on demand. Start with a deterministic threshold/connected-components approach or a bounded hierarchical method. Benchmark before adopting more expensive clustering. Labels include cluster algorithm, config hash, sample policy, and analysis tick.

## Family Inspection

The observer should show direct parents, offspring count, known descendants at a bounded depth, and ancestry links retrieved on demand. It must not attempt to render an unbounded global genealogy in a live frame. Exported lineage data can support deeper offline analysis.

## Traditions Require A Genetic Control

From Phase 9, a claim that a behavior is a transmitted tradition rather than
an inherited trait requires a genotype-matched control: the variant's
frequency in the local group must exceed its frequency in a cohort of
organisms elsewhere matched on genetic distance to that group's genotype
distribution. A behavior shared by close kin is a plausible inherited trait.

This is a required report field, not a recommended practice. A tradition
finding without its control statistic, matching tolerance, and cohort size
fails report validation.

## Genetic Distance Across Variable Structures

From Phase 7 genomes have different structures, so the existing normalized
gene-vector distance is not defined between them. Distance becomes

    distance = w_t * trait_distance + w_s * structural_distance

where `structural_distance` is the fraction of innovation IDs not shared
between two expressed networks. Weights are Q16 config. The existing rule
still applies: do not compare lengths or meanings across incompatible genome
schema versions.

## Interpretation Caveats

A colored cluster is a visualization aid, not evidence of real-world speciation. Reports must distinguish genetic distance, phenotype variation, ecological role, and lineage relation rather than treating them as synonyms. A detected segment is not an era, and a signal channel is not a language.
