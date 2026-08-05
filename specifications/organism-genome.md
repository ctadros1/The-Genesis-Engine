# Organism Genome Specification

## Phase 2 Implementation Notes (schema 1)

Implemented in `sim-core::genome` as `lifesim-genome-v1`: magic `ALGN`,
schema version 1, topology ID 1, 14 trait genes, 696 neural genes, FNV-1a64
checksum; canonical little-endian encoding of exactly 2,862 bytes. Decoding
verifies magic, schema, topology, counts, total length, and checksum before
any allocation, then validates finiteness and bounds of every value; every
failure is a typed `GenomeError` and nothing is repaired. The canonical
genome hash covers the encoded header and values. Recombination and bounded
variation return validated genomes by construction and are covered by a
deterministic malformed-input harness plus regression cases.

## Schema 2 (Phase 9)

Genome schema 2 is specified in full in
`specifications/genome-schema-2.md`. Summary of what changes here:

- Magic becomes `ALG2`; the header gains ploidy, chromosome count, and a
  channel/activation registry version, and the flat `trait_count` /
  `neural_count` fields are replaced by per-chromosome locus counts.
- Trait genes become `Trait` loci carrying value **and dominance**; neural
  parameters become `Node`, `Edge`, and `IoBinding` loci.
- Encoded length is variable, so `total_len` is a header field verified
  against the buffer before any allocation, and every per-chromosome locus
  count is checked against `max_loci_per_chromosome` before allocating it.

The Validity and Compatibility rules below apply unchanged and are the
reason schema 2's decode is specified the way it is: reject unknown schema,
registry, or channel IDs; reject inconsistent lengths, invalid checksum,
non-finite scalars, out-of-range values, unsorted loci, and dangling edge or
binding references; never repair; return validated genomes or a typed error.

On cross-version compatibility, this document's existing rule is satisfied
by rejection: **worlds are schema 1 or schema 2 by config, there are no
mixed-schema worlds, and there is no schema 1 to schema 2 genome
migration.** Converting a schema 1 genome would produce a record that never
existed and a lineage that cannot be replayed.

## Header (Schema 1)

| Field | Type | Validation |
|---|---|---|
| magic | fixed bytes | must match genome format |
| schema_version | unsigned integer | registered supported version |
| topology_id | unsigned integer | known neural topology |
| trait_count | unsigned integer | exact schema count |
| neural_count | unsigned integer | exact expected weights/biases |
| checksum | fixed digest | verified before use |

## Trait Genes

Each gene is a finite normalized f32 with explicit min/max and phenotype mapping. Initial groups: pigmentation/pattern, body scale, speed potential, sensory range/sensitivity, metabolism, thermal preference, diet affinity, attack/defense tendency, maturity threshold, reproduction investment/cooldown.

## Validity

Reject unknown schema/topology, inconsistent lengths, invalid checksum, non-finite scalar, out-of-range gene, or decoded allocation over configured limit. A loader does not repair arbitrary genome bytes. Mutation/crossover functions return validated genomes or a typed error; they never produce an unchecked slice.

## Compatibility

Genome versions are compatible only through registered transforms. Cross-version mating uses a declared migration to one target schema or is rejected. Genome hash uses canonical encoded bytes plus schema version.
