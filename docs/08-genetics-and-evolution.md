# Genetics And Evolution

## Phase 2 Implementation Status (`lifesim-genome-v1`)

Genome schema 1 carries 14 normalized trait genes (pigmentation x2, body
scale, speed potential, sensor range, sensor sensitivity, metabolism,
thermal preference, diet affinity, approach tendency, defense tendency,
maturity, reproduction investment, reproduction cooldown) plus 696 neural
genes for topology 1. Paired-parent reproduction requires two living
compatible adults with mutual bounded intent, maturity, energy, completed
cooldown, pairing range, and capacity; pair selection is greedy in stable
entity-ID order with (distance, entity ID) tie-breaks. Recombination picks
each gene from either parent by a named draw; variation is
`uniform-bounded-v1`: per-gene probability 0.02, delta uniform in +/- sigma
(0.05 gene units for traits, 0.4 weight units for neural genes), clamped to
bounds. The mutation-distribution choice is itself versioned config policy
per this document; the Gaussian form below remains an alternative for a
future policy version. Thermal preference and defense tendency are stored
and inherited but behaviorally inert until their mechanics exist
(analysis-only, documented). Founders derive from named `GenomeInit`
streams; all child-keyed draws use the prospective child entity ID.

## Planned Successor: `lifesim-genome-v2` (Phase 9)

Schema 1 is a flat haploid vector: no chromosomes, no linkage, no dominance,
and point mutation only. Inheritance is per-gene independent parent choice,
which is free recombination, so linkage disequilibrium decays instantly and
a co-adapted gene set cannot be held together.

Phase 9 replaces it with a diploid chromosomal genome. The full design is
`specifications/genome-schema-2.md`; the decision is ADR-0013 and the
realism policy behind it is `docs/26-biological-realism-policy.md`.

| Aspect | Schema 1 | Schema 2 |
|---|---|---|
| Ploidy | Haploid | Diploid, homologous chromosomes paired by index |
| Structure | Flat vector, fixed lengths | Sorted typed locus lists, variable length |
| Recombination | Per-gene independent parent choice | Meiosis with crossover; adjacency in innovation order is linkage |
| Dominance | None | Per-locus evolvable gene, codominant through complete dominance continuously |
| Mutation | Point only (`uniform-bounded-v1`) | Point, duplication, deletion, insertion, transposition (`lifesim-structmut-v1`) |
| Network structure | Fixed by topology ID | Grows and shrinks by gene duplication and deletion |
| Channels | Hard-coded 20 in, 12 out | Versioned registry; organisms bind any subset |

The central design claim: **network growth by gene duplication is
simultaneously the realistic genetics answer and the structural evolution
answer.** Gene duplication followed by divergence is the principal mechanism
by which real regulatory and neural complexity increased, so adopting it
gives both properties from one mechanism instead of bolting a graph-editing
scheme onto a flat vector. The honest cost is that duplication-driven growth
is slower and less directed than explicit add-node mutation, and Phase 9
measures the comparison (C9.5) rather than asserting an answer.

Schema 1 decode, evaluation, fixtures, and tests stay in the build
permanently. There is no schema 1 to schema 2 migration: converting a
schema 1 genome would produce a record that never existed and a lineage
that cannot be replayed.

## Genome Philosophy

Genomes encode bounded trait and controller parameters, not a hand-authored fitness score. Survival and reproduction under the world rules create selection pressure. Genetic changes are experimental mechanisms; modelling a mechanism faithfully is a claim about the model, never about real biology.

From Phase 11 the genome also encodes **how learning works**: which edges are
plastic, which rule form each uses, its coefficients, and which nodes gate
it. There is still no fitness score and no reward function anywhere. The
signal that gates plasticity is an ordinary output of the organism's own
evolved network, so what counts as reinforcing is a matter of evolutionary
history rather than of our specification. See ADR-0014.

Learned state is **not** inherited. That is an invariant, not a default: if
learned weights passed to offspring, a discovery would become a heritable
trait and transmitted knowledge would be indistinguishable from genetics,
which would make the Phase 13 question unanswerable.

## Genome Sections

| Section | Examples | Purpose |
|---|---|---|
| Metadata | Genome schema version, lineage hash | Safe decoding and provenance |
| Morphology | Size, color/pattern, speed potential | Visible phenotype variation |
| Metabolism | Basal rate, storage, thermal preference | Energy and climate tradeoffs |
| Senses | Range, channel sensitivity | Local information tradeoffs |
| Ecology | Diet affinity, attack/defense balance | Emergent feeding roles |
| Reproduction | Maturity, threshold, cooldown, investment | Life-history tradeoffs |
| Controller | Weights and biases | Neural behavior |

## Inheritance

Phase 1 supports asexual cloning with mutation only to prove state, save, and lineage machinery. Phase 2 adds sexual reproduction: two valid adults within mating range, with mutual intent, energy, cooldown, and compatibility checks, create an offspring genome via deterministic crossover plus mutation. Parents pay explicit energy investment. Offspring has immutable parent IDs and birth event.

## Mutation

For gene g:

    g_next = clamp(g + Bernoulli(p_mutation) * Normal(0, sigma), min_g, max_g)

Mutation probability, magnitude, mutation distribution, and gene-specific ranges are config values. Deterministic RNG streams make outcomes replayable. Mutation must not alter schema lengths or introduce non-finite values.

## Crossover

Use per-gene deterministic parent selection initially, optionally with bounded blended values for continuous traits. Neural genes must preserve topology compatibility. Cross-schema mating is rejected or requires an explicit migration policy; do not concatenate incompatible arrays.

## Fitness And Drift

There is no numerical global fitness score. Measured reproductive success, survival, energy efficiency, and population share are analytics, not direct objectives. Genetic drift is expected in finite populations and should be reported as an observation with confidence/context, not a causal certainty.

## Evolution Safety

- Mutation cannot create invalid values or unbounded matrices.
- Reproduction checks process/entity capacity before allocating an offspring.
- Inbreeding/kin avoidance is deferred unless it is explicitly modelled and tested.
- Genetic diversity metrics use sampled/periodic analysis to avoid tick-time quadratic work.
- Changes to mutation/range policy change experiment semantics and must version configs.
