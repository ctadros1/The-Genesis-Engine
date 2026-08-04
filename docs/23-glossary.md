# Glossary

| Term | Meaning |
|---|---|
| World lineage | A world plus its parent/branch provenance and compatible history |
| Tick | One fixed ordered simulation step |
| Strict mode | Determinism mode with controlled ordering/RNG and declared replay guarantee |
| Environmental cell | Raster location storing terrain/climate/resource state |
| Organism | Mobile simulated entity with genome, phenotype, controller, and lifecycle state |
| Genome | Versioned inheritable encoded trait/controller parameters |
| Phenotype | Derived visible/functional traits at runtime |
| Intent | Bounded requested action emitted by a neural controller before validation/resolution |
| Keyframe | Self-contained visible-state WebSocket baseline |
| Delta | Incremental stream update applied after a keyframe |
| World epoch | Server lifecycle counter that invalidates stale stream sequences |
| Experiment config | Versioned parameters/seed/build declaration for a run |
| Intervention | Audited user/system mutation such as a food injection or weather change |
| Species cluster | Analytical genetic grouping, not an authoritative simulation faction |
| Capacity event | Explicit record that configured process safety limits affected behavior |
| Replay lineage | The compatible chain of snapshot, config, build, seed, and interventions |
| Authored possibility space | The physics the simulation defines. What is possible is authored; what happens inside it is not |
| Authored progress | A technology tree, research graph, era state, recipe, or civilization stage. Permanently out of scope. Test: if you can name the outcome a mechanism makes more likely, it is authored progress |
| Segment | A statistically detected regime change in event rates, found post hoc by offline analysis. Never called an era, an age, or a stage, and never named after a human historical period |
| Tradition | A behavioral variant that is locally concentrated, persists beyond the individuals performing it, and is **not explained by the local genotype distribution**. The genetic control is part of the definition, not an optional check |
| Innovation ID | A world-global monotonic identifier for a node or edge locus, allocated at birth. Equal IDs mean homologous loci, which is how structurally different genomes align during meiosis |
| Locus | The unit of mutation, crossover, and expression in genome schema 2. Typed: trait, node, edge, or IO binding |
| Plastic edge | A network edge whose weight changes within a lifetime under a genome-encoded rule. Learned state is reset at birth and is never inherited |
| Modulatory node | A node whose activation gates plastic updates. It is what the organism treats as reinforcing, and it is evolved, never authored |
| Condition | A named config delta with its own canonical hash, used as a control or ablation. Two conditions are never the same experiment |
| Ablation | A condition that removes exactly one mechanism so its contribution can be measured. Required for every behavioral acceptance criterion |
| Baseline terrain | Terrain regenerated from (seed, config), still checksum-verified in save format 2 |
| Composed terrain | Baseline plus the stored organism-made modification delta, separately checksummed |
| Origin mode | How a world begins: `random`, `seeded`, or `scratch`. A starting condition, never a trajectory |
| Archetype | A founder trait and morphology **distribution** with a biome affinity. Not an organism, not a species, and its name is presentation only: no rule or analysis may read an archetype ID |
| Deme | One of several spatially separated founder groups, each drawing from an independently offset sub-stream so they start genetically distinct |
| Scaffold | Environmental or selective structure deliberately shaped toward a major transition (ADR-0018). Must be describable without naming its target, is always reported as a condition, and always requires an unscaffolded control |
| Climate drift | A stateless deterministic quasi-periodic temperature and moisture term evaluated from tick alone. **Not an age or era**: no code reads it as a world state |
| Module | One typed unit of an organism's body, occupying a lattice cell. Confers capability and costs mass and upkeep; it does not swing, bend, or collide |
| Morphospace | The space of bodies reachable within the module caps. A one-module body is a unicell, so multicellularity is a region of the same space rather than a separate mechanic |
| Field regime | The population-level microbial and chemistry simulation over the raster, with no per-individual randomness, coupled to the individual regime |
| Genotype class | A bounded discretization of microbial genotype space in the field regime. Deliberately not open-ended, and only the individual regime can demonstrate open-ended evolution |
| Materialization | The conversion of field density into individual organisms at the aggregation threshold. A representation change, never an achievement, and tested for neutrality |
