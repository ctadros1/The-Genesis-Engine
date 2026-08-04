# Artificial Genetics and Lineages for The Genesis Engine

**Deliverable:** Engineering-oriented scientific review  
**Target system:** The Genesis Engine, a deterministic long-running artificial-life simulation  
**Research date:** 2026-08-04  
**Encoding:** UTF-8  
**Status:** Design review and experimental roadmap; not a claim that any proposed mechanism guarantees open-ended evolution

---

## How to read this review

The following claim markers are used throughout:

- **[E] Evidence:** a result or design lesson supported by cited empirical, computational, or theoretical work.
- **[J] Engineering judgment:** a recommendation for The Genesis Engine derived from the evidence and the engine's determinism, persistence, and scale constraints.
- **[S] Speculation:** a plausible research direction whose value in this engine remains unestablished.

Citations use stable bibliography keys such as **[NEAT02]**. Full citations, DOI identifiers, relevance, and limitations appear in Section 20.

A crucial distinction is maintained throughout:

1. **Historical homology** means two loci descend from the same ancestral locus.
2. **Structural analogy** means two loci currently have similar structure or function, possibly through independent convergence.
3. **Researcher-defined similarity** is an offline analytical classification.
4. **Reproductive compatibility** is a simulation mechanism and therefore part of the causal world.

Those categories must never be collapsed into one identifier or one “species” field.

---

# 1. Executive summary

## 1.1 Recommended baseline

**[J] Adopt a direct, tagged, modular graph genome with chromosome-like ordering as a secondary organizational layer.** The baseline should be called something explicit in the implementation, such as **Tagged Modular Graph Genome, schema 1** (`TMGG-1`). It should contain:

1. A canonical schema and semantic-version header.
2. One or more ordered chromosome-like segments.
3. First-class module records with typed interfaces.
4. Directly encoded capability, neural-node, neural-connection, plasticity, neuromodulation, signaling, and object-interaction genes.
5. Stable historical locus identifiers inherited across descent.
6. Distinct origin-innovation identifiers, duplication-family identifiers, and structural signatures.
7. Explicit enabled/disabled state rather than deletion as the only way to suppress genes.
8. Bounded, versioned phenotype compilation.
9. Mutation and recombination performed by an external, versioned genetics policy, with only carefully bounded parts of that policy made heritable later.
10. Content-addressed canonical genome storage plus separate transmission and organism identities.
11. Append-only reproduction, mutation, and lineage-event records.
12. Named, keyed, counter-based random streams whose keys are derived from scheduler-independent event identifiers.

This combines the strongest practical properties of direct graph encodings and NEAT-style historical markings—debuggability, local structural mutation, and homology-aware crossover—with explicit modules that make duplication and recombination less destructive [NEAT02; MISEVIC06; MERLEVEDE19]. It avoids making a developmental program, artificial chemistry, or unconstrained regulatory network the only path from genotype to phenotype. Those encodings can produce useful regularity and compression, but they substantially enlarge the failure surface for determinism, schema migration, phenotype explosion, causal debugging, and crossover [STANLEY03; CPPN07; HORNBY03; HYPERNEAT09].

## 1.2 What should not be assumed

**[E] Crossover is not universally beneficial.** It can assemble useful combinations and reshape genetic architecture, but it can also destroy coadapted structures, especially when loci are strongly epistatic or alignment is poor. Digital-organism work shows context-dependent benefits and substantial changes to modularity and epistasis under sexual recombination; evolutionary-computation work proves benefit only for particular problem structures, not as a universal law [MISEVIC06; DOERR08; MERLEVEDE19].

**[J] Paired reproduction should therefore not imply mandatory intramodule crossover.** The engine should support multiple inheritance modes:

- clonal inheritance;
- paired whole-genome inheritance from one selected parent as a control;
- biparental module assortment;
- homologous, module-aware crossover;
- rare non-homologous rearrangement as an experimental operator.

The world genetics policy determines which modes exist. If inheritance-mode probabilities later become heritable, that is a separately versioned experiment.

**[E] Evolvable mutation rates do not reliably optimize long-term adaptation.** In digital organisms on rugged landscapes, selected mutation rates can settle far below rates that maximize long-run adaptation [CLUNE08]. **[J]** Mutation-rate evolution should initially remain disabled. If enabled, it should be bounded, costly or otherwise causally accountable, measured against fixed-rate controls, and incapable of changing identifier generation, serialization, or RNG semantics.

**[E] Robustness and evolvability have a time- and environment-dependent relationship.** Neutral networks and mutational robustness may slow early adaptation while improving access to later innovations; in other settings the benefit is absent or reversed [ELENA08; DRAGHI10; LABAR17]. **[J]** Do not optimize directly for “robustness” as though it were universally good. Measure mutation-neighborhood viability, diversity accumulation, and post-shift adaptation separately.

**[E] Explicit modular syntax does not guarantee evolved modular function.** Modularity can be promoted by modularly varying environments, connection costs, duplication, and specialization pressures, but the effect depends on the evolutionary regime [KASHTAN05; CLUNE13; ESPINOSA10]. **[J]** Modules should primarily provide stable interfaces, lineage units, and safe recombination boundaries. Whether functional modularity emerges must be tested.

## 1.3 Identity and ancestry model

The baseline requires separate identifiers for separate questions:

| Identifier | Meaning | Inherited? | Content-addressed? |
|---|---|---:|---:|
| `WorldLineageId` | Replay branch and causal world namespace | N/A | Derived from fork history |
| `OrganismId` | One embodied individual | No | No |
| `GenomeContentId` | Canonical genome bytes and semantic bundle | Shared by identical genomes | Yes |
| `GenomeTransmissionId` | One inherited genome instance at one birth | No | No |
| `ReproductionEventId` | One scheduler-independent birth event | No | Deterministically derived |
| `MutationEventId` | One mutation attempt or application | No | Deterministically derived |
| `LocusId` | Historical identity of one orthologous gene position | Yes until duplication/replacement | No |
| `InnovationId` | Origin event of a structural novelty | Yes with descendants of that novelty | Deterministically derived |
| `GeneFamilyId` | Duplication family/paralogy relationship | Yes | Derived from origin history |
| `StructuralSignature` | Current structural/semantic equivalence key | Recomputed | Yes |
| `ArtifactId` | Persistent object or construction lineage, if tracked | No | Policy-dependent |

**[J] A locus created by independent convergence must not receive the same historical identifier as a preexisting locus merely because its endpoints or function match.** Crossover may optionally use structural signatures as a fallback compatibility rule, but the event record must say that the match was analogical rather than homologous.

## 1.4 Determinism model

Every reproduction event is planned in a stable order before any parallel offspring construction. A recommended event root is:

```text
ReproductionEventId =
    H(
      domain = "genesis.reproduction-event.v1",
      world_lineage_id,
      genetics_policy_epoch,
      tick,
      reproduction_phase,
      reproduction_slot_key,
      canonical_parent_0_id,
      canonical_parent_1_id_or_zero,
      offspring_ordinal
    )
```

Parents are canonically ordered by stable `OrganismId` unless the world contains a genuine semantic role such as gestator, spore donor, or provisioning parent. Semantic roles are stored independently from canonical ordering.

Each random draw is keyed by:

```text
RNGKey =
    (
      world_seed,
      world_lineage_id,
      genetics_policy_epoch,
      reproduction_event_id,
      stream_name,
      draw_index
    )
```

Named streams include at least:

```text
reproduction.mode
recombination.chromosome_pairing
recombination.module_choice
recombination.breakpoint_count
recombination.breakpoint_position
recombination.allele_choice
mutation.count
mutation.operator
mutation.target
mutation.numeric_step
mutation.structural_parameters
mutation.strategy_parameter
development.initialization
development.step
learning.noise              # only if stochastic learning is explicitly allowed
```

Adding a draw to `mutation.numeric_step` must not perturb `recombination.module_choice`. Random distributions, bounded-integer sampling, weighted choice, hashing, fixed-point formats, and canonical ordering are all versioned algorithms, not unspecified library behavior.

## 1.5 Mutation validity policy

Use the following hierarchy:

1. **Construct from valid candidate sets** whenever possible.
2. **Apply a deterministic transactional closure** where only one semantically valid repair exists.
3. **Reject the mutation attempt** when no valid candidate exists or a hard budget would be exceeded.
4. **Never apply heuristic repair whose outcome depends on traversal order or an arbitrary “best guess.”**
5. **Record every attempted mutation**, including a rejection or a selected no-op, with a reason code.

Examples:

- Adding a connection chooses from a canonically sorted set of currently valid source-target pairs.
- Splitting a connection constructs one new node and two new edges atomically.
- Deleting a node deterministically removes its incident edges as part of the same transaction.
- Deleting a capability removes its owned ports and dependent edges in a declared closure.
- A developmental expansion that exceeds its budget fails explicitly; it is not silently truncated.

## 1.6 Developmental and regulatory encodings

**[J] Do not make a developmental program the baseline successor genome.** Instead, reserve versioned gene kinds for bounded developmental modules or indirect generators. A future module may generate repeated neural or morphological structure using a CPPN, grammar, graph-rewrite rule set, or regulatory program [CPPN07; HYPERNEAT09; GRUAU94; GE98]. Such a module must declare:

- maximum expansion steps;
- maximum emitted modules, nodes, and edges;
- deterministic rule-match and conflict order;
- phenotype overflow behavior;
- canonical intermediate-state representation;
- its compiler and function-registry versions;
- provenance links from generated phenotype elements to generating loci.

This captures the benefits of regularity and reuse without making every genome opaque.

## 1.7 Species and observer classifications

The engine should not store one authoritative `species_id`.

Reproduction-affecting mechanisms may include:

- local mate access;
- heritable mate preferences;
- explicit homologous-coverage requirements;
- module-interface compatibility;
- gamete or signaling traits;
- hybrid viability that follows from the phenotype.

Offline analysis may separately compute:

- genetic-distance clusters;
- topology clusters;
- behavioral clusters;
- ecological niche clusters;
- ancestral clades;
- radiations and extinction events;
- researcher-selected “species” partitions.

**[J] Offline clusters must be written to an observer namespace that the simulation cannot read.** If a similarity threshold is later used to gate mating, it becomes a causal genetics policy, receives a new policy version, and forks the replay lineage.

## 1.8 Experimental gate before adoption

The baseline should not be considered validated until it passes, at minimum:

- at least \(10^7\) randomized mutation attempts with zero malformed accepted genomes;
- serial versus multithreaded byte-identical reproduction tests;
- parent-order symmetry tests for role-free reproduction;
- recombination comparisons against no-crossover and module-assortment controls;
- bloat and phenotype-expansion stress tests;
- exact ancestry and locus-contribution verification on brute-force-checkable populations;
- bottleneck, environmental-shift, and high-mutation stress experiments;
- schema migration, policy-fork, and fail-closed loading tests.

The numerical thresholds proposed in Section 14 are engineering acceptance gates, not biological constants.

---

# 2. Scope and terminology

## 2.1 Scope

This review concerns the artificial genotype, its inheritance, and the records required to reconstruct descent in a deterministic artificial-life world. It covers:

- direct and indirect encodings;
- variable-length genomes;
- paired, sexual, and asexual reproduction;
- homology and crossover;
- structural and parametric mutation;
- duplication, deletion, and regulatory/developmental encodings;
- plasticity and neuromodulation parameters;
- evolvable capabilities and signaling traits;
- robustness, neutrality, modularity, epistasis, and pleiotropy;
- mutation-rate evolution;
- species, niches, and observer classifications;
- lineage, relatedness, and event storage;
- deterministic implementation, replay, and schema evolution;
- experiments and property-based tests.

It does **not** attempt to reproduce molecular genetics, meiosis, transcription, protein chemistry, or embryology for their own sake. A biological mechanism is relevant only when it contributes a useful artificial-system property: locality, recombination stability, modularity, innovation, robustness, interpretable ancestry, or a controlled source of developmental bias.

## 2.2 Core terms

### Genotype and phenotype

- **Genotype:** the canonical, heritable data structure.
- **Phenotype:** the executable organism configuration produced by a versioned phenotype compiler. This includes neural topology, initial parameters, enabled capabilities, signaling interfaces, object-interaction channels, and learning-rule configuration.
- **Lifetime state:** non-heritable state accumulated after birth, including synaptic weights altered by learning, neuromodulator state, memories, damage, and acquired objects. It is persisted for exact save/restore but excluded from the birth genotype unless an explicit experiment defines inheritance.

### Gene, locus, allele, module, and chromosome

- **Gene:** one typed heritable record.
- **Locus:** a historical identity maintained through ordinary inheritance. A parameter change creates a new allele at the same locus; duplication creates a new locus.
- **Allele:** the current payload at a locus.
- **Module:** a first-class set of genes with a typed interface and internal provenance.
- **Chromosome-like segment:** an ordered container used for linkage, breakpoint placement, and bulk duplication/deletion. It is an artificial data organization, not a claim to model DNA.
- **Innovation:** a structural origin event that creates a new locus, module, capability, or developmental rule.
- **Homology:** common descent from the same locus or module locus.
- **Paralogy:** similarity caused by duplication within a lineage.
- **Analogy:** independently produced structural or functional similarity.

### Pleiotropy and epistasis

- **Pleiotropy:** one gene affects multiple phenotypic traits or behaviors. In the proposed architecture this often occurs through shared modules, shared neuromodulation, or one capability feeding multiple downstream paths.
- **Epistasis:** the effect of one allele depends on other alleles. Neural connections, module interfaces, regulatory rules, and capability-port bindings can all create strong epistasis.
- **Linkage:** loci tend to be inherited together because they occupy the same segment or module.
- **Linkage disequilibrium:** combinations of alleles occur more or less often than expected from their marginal frequencies.

### Robustness, evolvability, and neutrality

- **Mutational robustness:** phenotype or fitness changes little under mutation.
- **Developmental robustness/canalization:** phenotype remains stable despite genetic or environmental perturbation during phenotype construction.
- **Evolvability:** a population or genotype can generate heritable adaptive variation. It is not identical to raw mutation rate, diversity, or robustness.
- **Neutral mutation:** a mutation has no measured effect under the current evaluation conditions. Neutrality is environment- and measurement-dependent.
- **Neutral network:** mutationally connected genotypes with equivalent measured phenotype or fitness.
- **Degeneracy:** structurally different components can perform overlapping functions under some conditions while differing under others.
- **Exaptation:** a feature evolved under one context later contributes to another.
- **Genetic assimilation:** a phenotype initially dependent on learning or environmental induction becomes more reliably genetically produced over evolution.
- **Developmental bias:** the encoding and development map make some phenotypic variants easier to generate than others.

### Reproduction terms

- **Asexual/clonal reproduction:** one genome is copied and mutated.
- **Paired reproduction:** two organisms participate in a reproduction event. This does not, by itself, specify how many genomes contribute or whether crossover occurs.
- **Biparental inheritance:** both parents contribute genetic material to the offspring.
- **Crossover:** an offspring sequence or graph is assembled from multiple parental regions.
- **Assortment:** whole segments or modules are selected from either parent without crossing within the selected unit.
- **Homologous recombination:** crossover occurs between ancestrally corresponding regions.
- **Non-homologous recombination:** regions are joined without common-locus alignment.
- **Hybrid incompatibility:** combinations from divergent parents produce reduced viability or performance. A parser failure is not a scientifically useful hybrid incompatibility; accepted offspring should remain structurally well-formed.

### Population and lineage terms

- **Pedigree:** parent-child graph.
- **Phylogeny:** inferred or exact branching history, depending on the data and reproduction mode.
- **Clade:** an ancestor and descendants under a declared ancestry definition.
- **Pedigree relatedness:** expected shared ancestry derived from the parent graph.
- **Identity by descent (IBD):** shared loci inherited from a common ancestor.
- **Genetic similarity:** present-day payload similarity, which may not imply close ancestry.
- **Effective population size:** a population-genetic measure of drift intensity; it is not necessarily the headcount.
- **Bottleneck:** a temporary severe reduction in reproducing lineages or effective population size.
- **Founder effect:** a new population begins from a small, nonrepresentative subset of a source population.
- **Radiation:** rapid production and persistence of multiple descendant lineages, usually accompanied by ecological, behavioral, or structural divergence.
- **Extinction:** a lineage or observer-defined group has no extant descendants.

## 2.3 The relevant meaning of “deterministic”

The Genesis Engine's requirement is stronger than “the same statistical distribution.” Exact determinism requires:

- identical reproduction decisions;
- identical random draws;
- identical mutation targets and magnitudes;
- identical identifiers;
- identical canonical genome bytes;
- identical phenotype expansion;
- identical lineage and event records;
- identical outcomes across container layouts, thread schedules, and supported platforms.

The implementation must not depend on:

- hash-map iteration order;
- unstable sorting;
- address order;
- thread completion order;
- unversioned random distributions;
- platform floating-point edge cases;
- implicit locale or text normalization;
- wall-clock time;
- a process-global innovation counter;
- “repair” procedures that select the first match from an unordered collection.

## 2.4 Evidence boundaries

Most cited results come from a particular task, representation, population regime, or digital-evolution platform. The following translation rules are used:

1. A result from NEAT supports the value of historical markings and complexification in NEAT-like graph evolution; it does not prove that all graph genomes should use identical operators.
2. A result from Avida supports a possible evolutionary dynamic in self-replicating programs; it does not establish the same quantitative threshold for embodied neural organisms.
3. A result from biological population genetics motivates measurements of drift, bottlenecks, and relatedness; it does not require molecular mechanisms.
4. A result from generative encodings supports regularity and reuse under suitable tasks; it does not show that indirect encodings are universally more evolvable.
5. A proof that crossover helps one problem class does not establish a general benefit.
6. An offline cluster is descriptive unless the engine explicitly uses it in mating or selection.

---

# 3. Lessons from artificial-life genome systems

## 3.1 NEAT: historical markings, complexification, and innovation protection

**[E]** NEAT demonstrated a practical combination of direct graph encoding, structural mutations that add nodes and connections, historical innovation numbers for crossover alignment, and population partitioning that temporarily protects new structures [NEAT02]. Its important engineering lessons are broader than its exact algorithm:

- structural evolution needs a way to distinguish inherited homology from accidental positional correspondence;
- newly added structure is initially fragile and can be lost before optimization;
- topology and parameters should be evolved together;
- starting from minimal networks can reduce early search complexity;
- the mechanisms interact, so isolated adoption may not reproduce the full effect.

**[J] Translation for Genesis:** use historical locus identities, but do not copy NEAT's global mutable innovation counter. Generate innovation identities from deterministic reproduction and mutation events. Use explicit module interfaces and lineage provenance in addition to innovation numbers. Innovation protection, if used, should be a versioned population policy rather than an implicit property of the genome schema.

A limitation of classic NEAT for Genesis is that “matching genes” are primarily an optimization device. Genesis also needs exact historical reconstruction, duplication relationships, save compatibility, and independent-world isolation. One identifier is insufficient for all of those purposes.

## 3.2 Avida and Tierra: executable genomes, mutation load, bloat, and exact ancestry

**[E]** Tierra and Avida treat genomes as executable programs that copy, mutate, compete, and evolve [RAY91; AVIDA04]. These systems establish that digital evolution can support:

- endogenous replication;
- heritable instruction sequences;
- insertion, deletion, and substitution;
- complex epistasis;
- neutral and deleterious regions;
- long experiments with complete instrumentation;
- replayable mutation and ancestry analysis.

Avida studies also show that:

- genome complexity and robustness can evolve through interactions among instructions [LENSKI99];
- high mutation rates can favor flatter, more mutation-tolerant genotypes rather than the highest isolated peak [WILKE01];
- robustness can help or hurt later adaptation depending on horizon and environment [ELENA08];
- genome length can expand or contract under mutation, selection, and instruction-level biases [GUPTA16];
- sexual recombination changes epistasis and modular organization [MISEVIC06];
- small populations can settle on drift-robust regions of a fitness landscape [LABAR17].

**[J] Translation for Genesis:** treat “unused” or disabled genes as potentially meaningful neutral variation, but impose explicit storage and phenotype budgets. Do not equate genome length with complexity. Store exact event histories so that apparent innovations can be decomposed into substitutions, duplication, deletion, recombination, and migration events.

Tierra and Avida also warn against making the genotype too close to a low-level virtual machine unless executable self-replication is itself a research target. The Genesis organism genome should describe capabilities and controllers, not contain unrestricted code that can escape static validation or make execution cost unbounded.

**[E]** A recent self-replicating-neural-network model made mutation an endogenous consequence of imperfect learned genotype copying and reported adaptation, clonal interference, epistasis, and evolution of both mutation rate and the distribution of fitness effects [SHVARTZMAN24]. **[J]** This demonstrates that endogenous mutation is a credible artificial-life research direction, but it is not a suitable Genesis baseline: the learned replication kernel, source-code decoding, and external training stack make mutation provenance and toolchain-stable replay harder to audit than explicit named operators.

**[E]** Large-scale Lenia evolution experiments also produced an early phase of diversity and novelty followed by domination by fast-expanding patterns under the authored ecology [CHAN23]. **[J]** More compute, implicit self-reproduction, and a localized genotype do not by themselves guarantee continuing innovation. Resource conservation, energy accounting, interaction structure, and suppression of trivial expansion exploits are genome-external experimental variables.

## 3.3 Direct graph neuroevolution

Direct encodings represent each node, connection, and parameter explicitly. They have several advantages for a deterministic simulator:

- local mutation is easy to define;
- serialization is straightforward;
- phenotype size is close to genotype size;
- individual causal changes are inspectable;
- hard limits are easy to enforce;
- ancestry of structural elements can be retained.

Their primary weaknesses are:

- repeated structure requires repeated genes;
- large networks can become expensive;
- crossover is destructive without historical alignment;
- structural motifs are not automatically reused;
- many structurally distinct but behaviorally similar variants can accumulate.

**[J]** These weaknesses motivate first-class modules and duplication, not immediate replacement by a wholly indirect encoding.

## 3.4 Developmental and generative encodings

Artificial embryogeny, cellular encoding, grammars, CPPNs, HyperNEAT, and generative robot representations encode rules or patterns that produce a larger phenotype [STANLEY03; GRUAU94; GE98; CPPN07; HYPERNEAT09; HORNBY03]. Demonstrated advantages include:

- compact descriptions of repeated structure;
- regularity, symmetry, and geometric bias;
- reuse of modules or subprograms;
- coordinated large-scale variation;
- scalable generation of connectivity patterns.

The costs are equally important:

- one small genotypic mutation can have a global phenotypic effect;
- crossover may alter rule composition nonlocally;
- genotype size does not bound phenotype size unless explicit budgets exist;
- debugging requires reconstructing developmental causality;
- neutral variation may be extremely large;
- schema changes can reinterpret old programs;
- grammar or rewrite ambiguity can introduce ordering dependence;
- regulatory attractors may be sensitive to initialization and numeric details.

**[J]** Genesis should support generative modules as a typed extension after the direct baseline is validated. Each generator must expose a deterministic expansion contract and provenance mapping.

## 3.5 Modular robot encodings and gene duplication

Generative representations for robot morphology show how repeated parts and regular structures can be expressed compactly and varied coherently [HORNBY03]. Biological and computational work on gene duplication supports a general innovation pathway:

1. duplicate an existing functional unit;
2. retain overlapping function temporarily;
3. allow neutral or weakly selected divergence;
4. lose, partition, or specialize functions;
5. sometimes acquire a new role [FORCE99; LYNCH00; POSADAS22].

This is not a guarantee. Most duplicates may be deleted, silenced, or remain redundant. Duplication can also cause rapid bloat and increase evaluation cost.

**[J] Translation for Genesis:** module duplication should be a first-class atomic operator with:

- new historical locus IDs;
- retained `GeneFamilyId`;
- explicit `derived_from` links;
- copied internal relative topology;
- deterministic rebinding of external interfaces;
- immediate hard-budget checks;
- balanced deletion pressure;
- separate metrics for duplicate retention, divergence, specialization, and cost.

## 3.6 Recombination in digital and evolutionary systems

**[E]** Sexual reproduction in digital organisms can weaken epistasis and produce more modular architectures under some regimes [MISEVIC06]. Homology-aware variable-length crossover must balance two competing goals: preserve corresponding regions and reshuffle variants [MERLEVEDE19]. Homologous and size-fair crossover reduce destructive size growth in genetic programming compared with unconstrained subtree exchange [LANGDON00; POLI04]. Crossover can be provably useful on selected problem structures [DOERR08].

**[E]** A neuroevolution study inferred communities of functionally dependent connection weights and used those communities as crossover masks; on 8- and 10-bit parity benchmarks, the linkage-aware operator produced more successful mixing than less informed crossover [QIAO23]. **[J]** This supports functional linkage groups as a credible future operator, not as baseline homology. Online linkage inference is computationally expensive, environment-dependent, and causal; offline researcher-defined communities must not silently become reproductive units.

The evidence does not support “crossover on” as a default scientific truth. It supports making recombination:

- aligned by meaningful homology;
- tested against no-crossover controls;
- aware of modules and linkage;
- optional under high epistasis;
- measured by child viability and long-term innovation, not only immediate fitness.

## 3.7 Neutrality, error thresholds, and survival of the flattest

Neutral networks can allow populations to spread through genotype space without immediate fitness change and can increase access to novel phenotypes [VANNIM99; DRAGHI10]. At high mutation rates, populations may favor broad robust regions over sharper high-fitness peaks [WILKE01]. Error-threshold theory describes regimes where mutation overwhelms faithful inheritance [EIGEN71], but its quantitative formulas assume specific sequence and fitness models.

**[J] Translation for Genesis:**

- measure the empirical relationship among per-locus mutation, genome size, offspring viability, and lineage persistence;
- scale mutation policy carefully as genomes grow;
- do not import a biological error-threshold number;
- retain bounded neutral state, such as disabled genes and duplicate variants;
- distinguish phenotype-neutral, behavior-neutral, and fitness-neutral mutations;
- monitor the active-to-total gene fraction and mutation load.

## 3.8 Regulatory networks and artificial chemistries

Gene-regulatory and artificial-chemistry systems demonstrate that interacting rules can generate stable organizations, self-maintaining sets, and complex dynamics [FONTANA94; CILIBERTI07]. Artificial chemistry is typically defined by a molecule set, reaction set, and reactor dynamics; in AlChemy, lambda expressions interact constructively and can form self-maintaining organizations [FONTANA94].

Applicable lessons are:

- constructive interactions can create higher-level organization not named in advance;
- multiple lower-level descriptions may map to similar stable organizations;
- self-maintenance and repair can emerge from network closure;
- the genotype-phenotype boundary can itself become evolvable.

However, those systems also introduce open-ended execution, state explosion, difficult validation, and large equivalence classes.

**[J]** Artificial chemistry is not justified as the baseline genome encoding. It is relevant later if Genesis evolves lower-level replicators, metabolic construction rules, or nested individuals. The successor genome should leave room for typed regulatory and developmental modules without assuming chemistry-like evaluation.

## 3.9 Major evolutionary transitions

Major transitions involve formerly independent replicating units becoming parts of a higher-level individual, often with new inheritance, conflict mediation, and division of labor. Digital systems have evolved kin groups, communication, reproductive division of labor, apoptosis, and multicellular life histories when group formation and reproduction are available [WILLENSDORFER08; MORENO22].

**[J]** The genome should not encode a “major transition” flag. It should support the prerequisites that make one representable:

- nested or composite identity;
- module and subgenome provenance;
- explicit reproductive ownership;
- communication and signaling traits;
- conflict and resource-transfer mechanisms;
- lineage queries at multiple levels;
- observer metrics that detect a shift in selection level.

These are future requirements. They do not justify adding biological chromosome machinery to the initial successor genome.

## 3.10 Signaling, tags, and kin recognition

Coevolving arbitrary tags can support kin-biased or tag-biased cooperation, but tags are vulnerable to false signaling and do not equal genealogical relatedness [AXELROD04; SCOTT22]. Digital sexual signaling can be gained or lost depending on ecological context [WEIGEL15].

**[J]** Heritable signaling traits should be ordinary capability and parameter genes. Organisms may observe signals if the physics exposes them. Offline lineage tools may compute exact relatedness, but the controller should not receive omniscient pedigree distance unless the experiment intentionally grants that sensor. A visible tag is a cue; it is not ground truth.

---

# 4. Genome-representation options

## 4.1 Comparison matrix

Ratings are relative to Genesis's requirements. “High” is favorable unless the column says “risk.” A representation can be combined with others; the recommended baseline is a hybrid.

| Representation | Mutation locality | Recombination stability | Modularity | Structural innovation | Neutral variation | Serialization / migration | Deterministic processing | Size control | Debugging | Ancestry reconstruction | Thousands of organisms |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Ordered gene list | High for parameter edits; medium for insert/delete | Low without positional homology; medium with segments | Low–medium | Medium | High | High | High | High | High | Medium | High |
| Tagged gene list | High | High when tags represent homology | Medium | High | High | High | High | High | High | High | High |
| Direct graph genome | High | Low without historical alignment; high with tags | Medium | High | Medium–high | High | High | High | High | High | High |
| Chromosome-like segments | High within loci | Medium–high if segment homology is stable | Medium | High through segment duplication/rearrangement | High | High | High | Medium–high | Medium–high | High | High |
| Explicit module encoding | High within module; medium at interfaces | High for module-level assortment; medium inside modules | High | High | High | High | High | High | High | High | High |
| Developmental program | Low–variable; mutations can be global | Low–medium unless rule homology is explicit | High potential | Very high | Very high | Medium–low | Medium with strict semantics | Low unless bounded | Low | Medium–low | Medium |
| Regulatory network | Medium–low; attractor effects can be global | Low–medium | High potential | High | Very high | Medium | Medium; numeric and update order matter | Medium–low | Low–medium | Medium | Medium |
| Grammar-based encoding | Medium; production changes can be global | Medium if productions are tagged | High | High | High | Medium | High if parsing/rewrite order is frozen | Medium | Medium–low | Medium | Medium–high |
| Tree encoding | Medium | Medium with homologous/size-fair crossover; low with arbitrary subtree crossover | Medium–high | High | High | High | High | Medium; bloat common | Medium | Medium | Medium |
| Indirect phenotype generator / CPPN | Low–medium; coordinate-wide effects | Low–medium unless graph genes are tagged | High regularity | High | High | Medium | High if function set and queries are fixed | Medium; phenotype budget required | Medium–low | Medium | Medium–high |
| Artificial chemistry / constructive system | Low and context-dependent | Unclear; conventional crossover may be meaningless | Emergent rather than explicit | Very high potential | Very high | Low | Low–medium unless heavily constrained | Low | Low | Low–medium | Low for baseline |

### Matrix interpretation

- **Ordered lists** are easy to implement, but raw position should not be treated as ancestry after insertion, deletion, or duplication.
- **Tags and historical IDs** materially improve variable-topology crossover and lineage reconstruction [NEAT02].
- **Direct graphs** fit neural controllers and typed capabilities, but need modules to avoid flat, highly epistatic genomes.
- **Chromosome-like segments** are useful for linkage and multipoint crossover; their order should be canonical and evolvable only through explicit rearrangement operators.
- **Modules** provide the best immediate tradeoff for Genesis. They are not a replacement for gene-level historical identity.
- **Developmental, regulatory, grammar, and indirect encodings** are promising alternatives for repetition and scale, but they require stronger budgets and provenance.
- **Artificial chemistry** offers lessons about constructive closure, not a practical initial serialization format.

## 4.2 Ordered gene lists

An ordered gene list is a sequence of typed records. Order may determine evaluation, linkage, or merely serialization.

### Strengths

- simple canonical encoding;
- efficient append, delete, scan, and segment operations;
- straightforward multipoint crossover;
- compact storage;
- easy schema migration when records are self-describing.

### Weaknesses

- positional indices become unstable after insertion and deletion;
- independent insertions shift downstream positions;
- duplication creates ambiguous positional correspondence;
- uniform or multipoint crossover can cut coadapted structures;
- order can accidentally become semantic.

### Genesis use

**[J]** Use ordered lists *inside* chromosome-like segments, but give every record a stable locus ID. Record order is a linkage coordinate, not the sole identity. Evaluation order must be derived canonically from typed dependencies, not inherited accidentally from serialized order, except where order is an explicit evolvable trait.

## 4.3 Tagged genes

A tagged gene carries an identifier that survives ordinary inheritance. NEAT's historical markings are the canonical example [NEAT02].

### Strengths

- reliable alignment of inherited homologs;
- precise mutation ancestry;
- deterministic merge-like crossover;
- insertion does not shift identity;
- easy detection of disjoint and excess genes.

### Weaknesses

- identifier policy can become a bottleneck;
- global counters are hostile to parallel determinism and independent worlds;
- one tag is often overloaded to mean identity, function, and structural equivalence;
- duplication needs new-copy semantics.

### Genesis use

**[J]** Tagged genes are required. Use distinct fields:

```text
locus_id                 # orthologous historical identity
origin_innovation_id     # structural origin event
gene_family_id           # duplication family
derived_from_locus_id    # immediate duplication/replacement source, optional
structural_signature     # recomputed current structure, not ancestry
```

## 4.4 Graph genomes

A graph genome directly stores nodes and edges. It naturally represents variable neural topology, regulatory dependencies, capability bindings, and module interfaces.

### Strengths

- direct phenotype mapping;
- local add/delete/split mutation;
- graph validity can be statically checked;
- execution cost is predictable;
- causal provenance can follow nodes and edges.

### Weaknesses

- graph edit distance is expensive and ambiguous;
- crossover can create dangling references or disrupt cycles;
- recurrent networks require explicit scheduling semantics;
- flat graphs encourage pervasive pleiotropy.

### Genesis use

**[J]** The graph is the core representation for controllers and regulatory wiring. All references use stable IDs in the serialized genome. The compiled phenotype remaps stable IDs to compact local array indices for execution.

## 4.5 Chromosome-like segments

Segments are ordered collections with linkage and crossover boundaries.

### Strengths

- natural unit for multipoint crossover;
- bulk duplication and deletion;
- supports linkage evolution;
- partitions large genomes for storage and mutation.

### Weaknesses

- arbitrary segment boundaries may not match functional modules;
- rearrangements complicate homology;
- too many tiny segments reduce meaning;
- physical position can be falsely treated as exact ancestry.

### Genesis use

Use a small number of explicit segments with stable `SegmentLocusId`s. A segment contains module references and possibly segment-scoped genes. Breakpoints occur only at declared boundaries in the baseline. Fine-grained intramodule crossover requires locus alignment, not raw byte offsets.

## 4.6 Module-based encodings

A module is a named-by-ID, typed subgraph with input/output/modulatory ports and internal genes.

### Strengths

- module-level duplication and deletion;
- stable recombination units;
- reduced accidental cross-module rewiring;
- easier causal debugging and provenance;
- supports repeated capability-controller complexes;
- exposes functional and historical hierarchy.

### Weaknesses

- fixed interfaces can constrain innovation;
- module boundaries can become arbitrary or stale;
- shared modules can create hidden pleiotropy;
- interface mutation may be highly disruptive.

### Genesis use

Modules are first-class but evolvable. A module has:

```text
ModuleGene {
    module_locus_id
    module_family_id
    segment_locus_id
    module_kind
    interface_ports[]
    internal_gene_refs[]
    regulatory_inputs[]
    enabled
    provenance
}
```

Module kinds are versioned semantic categories, not named technologies. Examples include neural subgraph, sensory transducer, action controller, signaling transducer, plasticity controller, object-interaction controller, or bounded generator.

## 4.7 Developmental programs

A developmental program iteratively constructs phenotype components.

### Strengths

- coordinated structural growth;
- repetition and hierarchy;
- developmental bias;
- compact genome-to-large-phenotype maps;
- possible canalization.

### Weaknesses

- nonlocal mutation effects;
- expansion can diverge or explode;
- hidden state complicates checksums and replay;
- developmental timing becomes another causal system;
- difficult migration when rule semantics change.

### Genesis use

**[J]** Experimental only. A developmental module must be total over accepted genomes: it either produces a valid phenotype within fixed limits or returns a deterministic failure code before organism insertion. No unbounded loops, recursion, dynamic code, or unspecified rule conflicts.

## 4.8 Regulatory networks

Regulatory networks determine which genes or modules are active as a function of state.

### Strengths

- context-dependent expression;
- temporal programs;
- reuse of structural components;
- multiple phenotypes from one genome;
- rich epistasis and neutral variation.

### Weaknesses

- attractor dynamics can be fragile;
- update order matters;
- hidden cycles complicate validation;
- high pleiotropy;
- crossover can disrupt global regulatory context.

### Genesis use

Begin with bounded gates and neuromodulatory control rather than a fully general gene-regulatory network. A regulatory gene should use fixed-point parameters, a finite versioned function set, declared update phases, and hard state bounds.

## 4.9 Grammar-based encodings

A grammar maps a genotype to a derived structure through productions [GE98].

### Strengths

- syntactic validity can be built into the grammar;
- reusable production rules;
- variable structure;
- compact recursive patterns.

### Weaknesses

- grammar changes alter the meaning of every genome;
- codon wrapping and unused productions can create large neutrality;
- a local production change may alter the whole derivation;
- crossover at raw sequence positions may not preserve semantics.

### Genesis use

A grammar must be bundled with an immutable grammar semantic version. Old genomes cannot be reinterpreted under a changed grammar. Tagged production rules and bounded derivation are mandatory.

## 4.10 Tree encodings

Trees represent nested expressions, rules, or components.

### Strengths

- natural hierarchy;
- subtree duplication and replacement;
- straightforward canonical serialization;
- executable or generative semantics.

### Weaknesses

- arbitrary subtree crossover causes bloat and semantic disruption;
- repeated references require DAG extensions;
- deep trees can be expensive;
- exact homologous alignment is nontrivial.

### Genesis use

Useful for bounded expression genes, activation formulas, or developmental rules. Use typed nodes, maximum depth, maximum node count, tagged subtrees, and homologous or size-fair crossover [LANGDON00; POLI04].

## 4.11 Indirect phenotype generators

CPPNs and HyperNEAT generate connectivity from coordinates or other structural queries [CPPN07; HYPERNEAT09; ESHYPER12].

### Strengths

- regularity and symmetry;
- large phenotypes from compact genomes;
- coordinate-aware patterns;
- coordinated changes.

### Weaknesses

- requires meaningful geometry;
- small mutations may affect many connections;
- phenotype extraction thresholds introduce discontinuities;
- generated provenance is less direct;
- recurrent or nongeometric controller structure may not fit.

### Genesis use

A credible alternative for sensory sheets, body-relative spatial fields, repeated actuator arrays, or morphologies if Genesis later has a stable geometric substrate. It is a poor universal encoding for arbitrary object-interaction and signaling logic. Use it as a module type, not the sole genome.

## 4.12 Recommended representation decision

**Baseline:** tagged direct graph genes inside explicit modules and chromosome-like segments.

**Credible alternative A:** the same baseline with bounded CPPN/generative modules for repeated geometric structures.

**Credible alternative B:** a module grammar with tagged productions and direct graph leaves.

**Credible alternative C:** a bounded regulatory-developmental layer that selects and duplicates direct modules over developmental time.

**Not recommended initially:** unrestricted artificial chemistry, unrestricted executable genome code, raw-position crossover without historical tags, or a whole-genome indirect encoding with no direct provenance.


# 5. Reproduction and crossover

## 5.1 Separate mating, parenthood, and inheritance

A common design error is to use one boolean called `sexual` to control several different mechanisms. Genesis should separate:

1. **Mate choice:** how another organism is selected.
2. **Reproduction participation:** which organisms pay costs or supply resources.
3. **Legal parentage:** which organisms are recorded as parents.
4. **Genetic contribution:** which genomes contribute inherited loci.
5. **Assortment:** selection of whole parental units.
6. **Crossover:** within-unit exchange.
7. **Post-recombination mutation:** new variation after inheritance.
8. **Developmental initialization:** construction of the offspring phenotype.
9. **Provisioning or gestation:** non-genetic parental effects, if later modeled.

This separation permits scientifically interpretable experiments. A paired event can be compared with and without genetic crossover while holding mate choice and reproductive costs constant.

## 5.2 Recommended inheritance modes

| Mode | Parents recorded | Genetic contributors per offspring | Within-module crossover | Primary use |
|---|---:|---:|---:|---|
| `CLONAL` | 1 | 1 | No | Asexual baseline |
| `PAIRED_SINGLE_GENOME` | 2 | 1 | No | Isolate effects of pairing/mate choice from biparental inheritance |
| `PAIRED_WHOLE_MODULE_ASSORTMENT` | 2 | Usually 2 | No | Low-destructiveness biparental baseline |
| `PAIRED_HOMOLOGOUS_CROSSOVER` | 2 | Usually 2 | Yes, homologous loci only | Recommended sexual treatment |
| `PAIRED_SEGMENT_MULTIPOINT` | 2 | Usually 2 | At segment boundaries and aligned loci | Test linkage and chromosome-like behavior |
| `PAIRED_UNIFORM_LOCUS` | 2 | Usually 2 | Per matched locus | Destructiveness control, not preferred baseline |
| `PAIRED_NONHOMOLOGOUS` | 2 | Usually 2 | Yes, without historical homology | Rare rearrangement experiment |
| `PAIRED_GENERATOR_RULE_CROSSOVER` | 2 | Usually 2 | Tagged rule/subtree units | Only for developmental encodings |

**[J]** `PAIRED_SINGLE_GENOME` is a scientifically useful control even though only one genome contributes to each offspring. If the simulation's definition of “paired reproduction” requires genetic contribution from both parents, call this mode something else in user-facing analysis, but retain it internally to isolate causal effects.

For modes that claim biparental inheritance, the policy should state whether every offspring must contain at least one inherited unit from each parent. Enforcing that rule can itself create bias when only one homologous unit exists. Therefore the event should record actual contribution fractions rather than infer them from parent count.

## 5.3 Canonical parent ordering and semantic roles

For role-free reproduction:

```text
canonical_parent_0 = min(parent_organism_id)
canonical_parent_1 = max(parent_organism_id)
```

Parent order is used only for deterministic serialization and RNG keys. The allele choice algorithm must be symmetric under swapping the parents before canonicalization.

If the world later has a genuine role:

```text
semantic_roles = {
    initiator: organism_id?,
    gestator: organism_id?,
    resource_provider: organism_id?,
    signal_sender: organism_id?
}
```

Those roles are stored separately. A semantic role may affect provisioning, developmental state, or inheritance only when the policy explicitly says so. It must never be smuggled in through “parent A” versus “parent B.”

## 5.4 Homologous recombination

### Definition

Two loci are exactly homologous for crossover when their `LocusId` values match. Two modules are exactly homologous when their `ModuleLocusId` values match. Descendants preserve those identifiers under parameter mutation and ordinary inheritance.

Duplication creates:

- a new `LocusId` or `ModuleLocusId`;
- the same or derived `GeneFamilyId`;
- an explicit `derived_from` link;
- a new origin event.

Thus duplicates are paralogs, not exact homologs.

### Alignment algorithm

For each compatible homologous module pair:

1. Canonically sort genes by `(gene_kind, locus_id)`.
2. Merge the two lists by `locus_id`.
3. Classify entries as:
   - matching homolog;
   - present only in parent 0;
   - present only in parent 1;
   - incompatible type under the same ID, which is a corruption and must fail closed.
4. Select alleles for matching homologs under a versioned rule.
5. Select or omit unmatched loci under an explicit excess/disjoint rule.
6. Validate module interfaces and budgets.
7. Record the inherited parent for each locus.

This is \(O(G_0 + G_1)\) after canonical sorting or when genes are stored in canonical order.

### Structural fallback alignment

Independently evolved loci can share a `StructuralSignature` without sharing ancestry. Optional fallback alignment may match such loci if:

- the module type and interface are compatible;
- the structural signature is exact under a versioned definition;
- no exact historical homolog exists;
- the policy records `alignment_kind = ANALOGICAL`;
- tie resolution is deterministic.

**[J]** Keep structural fallback disabled in the first baseline. It can conceal convergent origin and create arbitrary pairings when multiple analogous loci exist.

## 5.5 Innovation-number alignment

Classic NEAT aligns genes using innovation numbers [NEAT02]. Genesis should preserve the core idea while changing the implementation:

- `InnovationId` is an origin-event identity, not a process-global counter.
- A locus created by that innovation inherits the ID through descent.
- Multiple offspring produced by the *same planned mutation event* receive the same result only if the event semantics explicitly create a shared innovation. Ordinarily each offspring mutation is its own event.
- Identical mutations in separate lineages receive distinct `InnovationId`s.
- A `StructuralSignature` supports analysis of convergence without falsifying ancestry.

This model is more precise than assigning the same innovation number to structurally identical mutations observed during one generation. It sacrifices some convenient crossover matching but protects lineage truth.

## 5.6 Segment-based and multipoint crossover

Segment crossover treats ordered chromosome-like coordinates as linkage information.

A deterministic baseline algorithm:

1. Pair homologous segments by `SegmentLocusId`.
2. Enumerate valid breakpoint boundaries in canonical order.
3. Draw breakpoint count from `recombination.breakpoint_count`.
4. Sample distinct boundaries using a specified bounded-integer algorithm.
5. Sort breakpoints.
6. Draw starting donor from `recombination.allele_choice`.
7. Alternate donors at breakpoints.
8. Within mixed modules, use homologous-locus alignment or prohibit cuts.

Advantages:

- linkage blocks can be preserved;
- multiple variants can be reshuffled;
- the operator has a clear sequence interpretation.

Failure modes:

- segment order may no longer reflect functional homology after rearrangement;
- a breakpoint can split a strongly epistatic module;
- insertions and deletions make raw coordinate alignment ambiguous;
- size can drift if unequal regions are exchanged.

**[J]** Baseline breakpoints should occur between modules. Intramodule multipoint crossover should be an experimental mode using locus alignment.

## 5.7 Uniform crossover

Uniform crossover chooses each matched allele independently from a parent.

Advantages:

- high mixing;
- simple and symmetric;
- useful as an experimental upper bound on recombination intensity.

Disadvantages:

- destroys linkage;
- breaks coadapted parameter sets;
- can produce incoherent plasticity and regulatory configurations;
- exacerbates epistasis;
- makes module inheritance difficult to interpret.

**[J]** Do not use uniform crossover as the default. Retain it as a control for measuring how much the baseline benefits from linkage and module boundaries.

## 5.8 Module-level crossover and assortment

Module-level inheritance is the safest initial biparental mechanism:

1. Align exact homologous modules.
2. Choose one whole allele of each module from a parent.
3. For modules present in only one parent, apply the policy's inclusion probability.
4. Resolve external bindings through stable interface-port IDs.
5. Reject only if no deterministic valid binding exists; do not guess.
6. Optionally require each parent to contribute at least one module when feasible.
7. Apply mutation after assembly.

Benefits:

- preserves internal coadaptation;
- makes contribution maps compact;
- supports duplication families;
- minimizes malformed children;
- provides a strong no-intramodule-crossover baseline.

Costs:

- low mixing when genomes contain few large modules;
- selection may freeze large linkage blocks;
- module boundaries can become a source of developmental constraint.

**[S]** Evolvable module fission/fusion operators may later change linkage structure. They should be separate from ordinary crossover and heavily instrumented.

## 5.9 Non-homologous recombination

Non-homologous exchange can create major innovations but has high destructiveness. Candidate mechanisms include:

- moving a module to a different segment;
- inserting a copied segment from one parent into the other;
- exchanging modules with compatible typed interfaces despite distinct ancestry;
- fusing two modules through a newly created adapter;
- translocating a regulatory region.

**[J]** These should be rare, versioned structural mutation/rearrangement operators, not routine crossover. The operator must construct a valid typed result and record both source regions. It must not merge arbitrary bytes or silently drop unresolved references.

## 5.10 Crossover in variable-length genomes

Variable-length crossover has two competing objectives [MERLEVEDE19]:

- **preserve homology:** corresponding inherited regions remain aligned;
- **create recombination:** parental variants are actually reshuffled.

Too much positional conservatism yields little recombination. Too much free exchange destroys correspondence. Genesis can manage the tradeoff through hierarchy:

1. exact module-locus alignment;
2. exact gene-locus alignment inside modules;
3. module-level assortment for unmatched modules;
4. optional structural-signature fallback;
5. rare non-homologous rearrangement.

Length control must be explicit. The offspring size check occurs before commit, and the result is never silently truncated. Policies may:

- reject the crossover and fall back to one parental module;
- redraw a different legal breakpoint using a separate, predefined retry ordinal;
- deterministically choose the smaller legal result;
- abort the birth if the reproductive mechanics allow failure.

The fallback policy is versioned and recorded. Unbounded “retry until success” is prohibited because it obscures distribution and can consume variable computation.

## 5.11 When crossover helps

Crossover is most likely to help when:

- homologous regions are accurately aligned;
- modules contain internally epistatic genes and interfaces are relatively weakly coupled;
- useful alleles occur in different parents;
- the environment recombines subproblems;
- population diversity is sufficient;
- recombination breaks unfavorable linkage;
- offspring can survive long enough for mixed structures to be optimized;
- selection or ecology protects rare structural combinations.

Sexual digital organisms evolved weaker epistasis and more modular architecture in one Avida regime [MISEVIC06]. This is evidence that recombination can reshape the genome to become more recombination-tolerant, not proof that every initial genome will tolerate crossover.

## 5.12 When crossover destroys useful structures

Crossover is likely destructive when:

- parent topologies have little exact homology;
- parameters are strongly coadapted;
- module interfaces are not stable;
- one gene has broad pleiotropic effects;
- uniform crossover separates plasticity rules from the structures they regulate;
- developmental programs have nonlocal rule interactions;
- the population has recently accumulated unique innovations;
- genomes approach hard size limits;
- parent ecological adaptations are incompatible;
- the child's compiled phenotype is valid but behaviorally incoherent.

Immediate child fitness is not the only metric. A destructive operator can still be useful if it rarely creates large innovations. Therefore evaluate:

- accepted-child structural validity;
- birth viability;
- early-life survival;
- parental performance retention;
- longer-term descendant contribution;
- novel module combinations;
- fixation or persistence of recombinant innovations;
- population extinction risk.

## 5.13 Reproduction without crossover

Asexual reproduction remains a scientifically important mode and control:

- it preserves coadapted structures;
- mutation effects are easier to attribute;
- lineage is a tree rather than a pedigree DAG;
- it can adapt efficiently when beneficial mutations arise independently;
- it may accumulate deleterious changes under small populations or high mutation;
- it cannot directly combine beneficial variants from different lineages.

The engine should support worlds or demes with different modes. Mode transitions should not be inserted implicitly. A change from asexual to sexual inheritance alters causal semantics and must be a genetics-policy fork unless the capacity to reproduce in different modes is already encoded and present from the beginning.

## 5.14 Assortative mating, inbreeding, and kin avoidance

### Assortative mating

Assortative mating can be based on heritable signals, behavior, location, phenotype, or compatible interfaces. It can promote divergence, but it also reduces eligible mates and can fragment small populations [ANDERSON14; DECARA08].

Implementation options:

- controller-mediated mate choice from observable cues;
- a world-level compatibility filter;
- a heritable preference vector;
- a minimum homologous-module coverage;
- spatially local mating.

**[J]** Prefer controller-mediated choice and physical locality when possible. A world-level genetic-distance threshold is a strong causal intervention and must be explicit.

### Inbreeding

Inbreeding increases recent shared ancestry and can expose recessive-like incompatibilities only if the encoding contains dominance or diploidy. In a haploid direct genome, inbreeding mainly reduces diversity and increases identity by descent.

Do not add dominance solely to imitate biology. Measure:

- pedigree kinship;
- locus-level shared ancestry;
- diversity loss;
- offspring viability;
- deleterious-load accumulation.

### Kin avoidance

Three distinct mechanisms must not be confused:

1. **Omniscient kin avoidance:** the engine computes pedigree relatedness and filters mates.
2. **Cue-based kin avoidance:** organisms observe heritable tags, location, familiarity, or signals.
3. **Offline kin analysis:** researchers measure relatedness after the fact.

**[J]** Cue-based behavior is the most compatible with the engine's embodied-cognition goals. Omniscient filtering is acceptable only as an explicit experimental population policy.

## 5.15 Sexual selection and signaling

Sexual selection requires variation in mate choice, signals, and reproductive success. A minimal implementation needs:

- observable heritable signaling traits;
- signaling costs or physical constraints;
- controller access to observations;
- mate-choice actions;
- reproduction outcomes tied to those actions;
- exact parentage and reproductive-success records.

Do not label signals as “fitness” or “quality.” They are physical traits whose meaning emerges from behavior. Digital work shows that signaling can be lost when ecological or reproductive context changes [WEIGEL15].

## 5.16 Compatibility thresholds and hybrid incompatibility

A compatibility threshold may use:

- proportion of exact homologous modules;
- required interface compatibility;
- maximum unresolved port bindings;
- segment pairing availability;
- explicitly evolved recognition signals;
- developmental compatibility.

A raw whole-genome distance is a weak basis because it conflates neutral divergence, duplication, convergence, and functional incompatibility.

**Recommended baseline compatibility:**

```text
compatible =
    required_reproductive_capabilities_present
    AND homologous_module_coverage >= policy_minimum
    AND all mandatory module interfaces can be bound
    AND predicted_genome_size <= hard_limit
```

This is evaluated in canonical order without randomness.

Hybrid incompatibility should preferably arise after a structurally valid child is built:

- mismatched neural parameters;
- conflicting regulatory gates;
- incompatible sensory-action coordination;
- ecological mismatch;
- reduced fertility in descendants.

A compile error is a software invariant violation, not an evolutionary phenotype. If a world intentionally permits nonviable zygotes, represent that as a valid `NONVIABLE_DEVELOPMENT` outcome with complete lineage records, not malformed data.

## 5.17 Recommendation

**Baseline inheritance policy:**

- support `CLONAL`, `PAIRED_WHOLE_MODULE_ASSORTMENT`, and `PAIRED_HOMOLOGOUS_CROSSOVER`;
- start production experiments with clonal and module-assortment modes;
- enable intramodule crossover only after destructiveness tests;
- keep non-homologous recombination rare and experimental;
- do not make mode probabilities heritable initially;
- record per-locus contribution maps;
- preserve exact parentage even when a parent contributes no loci;
- use no observer-defined species label in mate choice.

---

# 6. Mutation operators

## 6.1 General mutation transaction

Every mutation attempt is a transaction with this logical form:

```text
MutationAttempt {
    mutation_event_id
    reproduction_event_id
    mutation_ordinal
    operator_id
    operator_semantics_version
    rng_stream_root
    pre_genome_checksum
    canonical_candidate_count
    selected_candidate_key?
    sampled_parameters
    outcome: Applied | Rejected | NoCandidate | BudgetExceeded
    rejection_code?
    created_ids[]
    deleted_or_disabled_ids[]
    post_genome_checksum
}
```

Procedure:

1. Derive `MutationEventId`.
2. Select `operator_id` from an integer-weighted canonical table.
3. Enumerate valid candidates in canonical order.
4. If there are no candidates, emit `NoCandidate`.
5. Select a candidate with a versioned bounded-integer method.
6. Sample mutation parameters from the operator-specific named stream.
7. Construct a candidate result in isolation.
8. Run structural, semantic, and budget validation.
9. Commit atomically or emit a rejection.
10. Compute the canonical checksum.
11. Append the event record.

A failed mutation does not disappear from history. Whether a failed attempt consumes the organism's mutation count is a policy decision and must be frozen.

## 6.2 Distribution principles

### Numeric mutations

Normal-distribution libraries are risky for exact cross-platform replay because transcendental implementations and sampling algorithms may differ. Recommended fixed-point distributions:

1. **Discrete Laplace / two-sided geometric local step**
   - draw sign;
   - draw magnitude from a geometric tail using integer arithmetic;
   - add in the parameter's fixed-point unit;
   - clamp or reject under a declared boundary rule.

2. **Rare bounded reset**
   - with small integer probability, draw uniformly from the full legal range.

3. **Log-space step for positive scale parameters**
   - add a fixed-point step to `log2(parameter)` or to a discrete exponent/mantissa representation;
   - decode with a frozen integer algorithm or table.

This produces mostly local changes with occasional large exploration. Exact probabilities and algorithms are part of `operator_semantics_version`.

### Structural mutation counts

Use an explicit distribution such as:

- Bernoulli per offspring for each operator family;
- Poisson-like count implemented by a frozen integer algorithm;
- categorical count table such as `{0,1,2,3+}` with fixed integer weights.

Do not let mutation count scale implicitly with container length unless that is the declared scientific policy. Track both:

- per-genome event rates;
- per-locus exposure rates.

## 6.3 Operator matrix

| Operator | Expected distribution | Locality | Main failure modes | Invalid-state handling | Cost | RNG streams | Crossover interaction | Bloat risk | Key invariants |
|---|---|---|---|---|---|---|---|---|---|
| Numeric parameter | Discrete local step + rare reset | High | saturation, zeroing, unstable time constants | Clamp or reject by parameter contract | \(O(1)\) | `mutation.target`, `mutation.numeric_step` | Alleles remain homologous | None | value in range; canonical fixed-point |
| Neural weight | Local fixed-point step + rare reset/sign change | High behaviorally variable | dead/saturated network, oscillation | Range contract; no NaN | \(O(1)\) | same as numeric | Coadaptation can be broken by crossover | None | exact weight format; endpoint refs valid |
| Node insertion | Choose enabled edge; split atomically | Medium–high | phenotype disruption, recurrent-cycle change | Construct only from valid splittable edges | \(O(1)\) plus index update | `mutation.target`, `mutation.structural_parameters` | New locus unmatched until inherited | Moderate | old edge disabled; two valid edges; IDs unique |
| Connection insertion | Uniform/weighted over valid missing pairs | High structurally | dense graphs, forbidden cycles, duplicate edge | Enumerate valid pairs; no heuristic repair | \(O(N^2)\) naïve, lower with indexes | `mutation.target`, structural params | Unmatched locus may be inherited/discarded | High | endpoints exist; type and recurrence legal |
| Node deletion | Choose deletable node; closure removes incident edges | Medium | cascade loss, required-path deletion | Transactional deterministic closure | \(O(\deg v)\) | `mutation.target` | Deletes homolog, creating presence/absence allele | Reduces | protected ports intact; no dangling refs |
| Connection deletion | Choose enabled deletable edge | High | disconnected capability, dead module | Allow behavioral damage; structural validation only | \(O(1)\) | `mutation.target` | Presence/absence polymorphism | Reduces | no dangling refs; required hard edges protected |
| Module duplication | Choose duplicable module; deep copy + rebind | Medium | rapid bloat, aliasing IDs, duplicated external side effects | Atomic construction; fail on budget/interface ambiguity | \(O(\lvert M\rvert)\) | target + structural params | Creates paralog; module crossover must distinguish family/homology | Very high | all copied IDs new; family/source retained |
| Module deletion | Choose nonrequired module; dependency closure | Medium–low | large functional loss, orphan interfaces | Deterministic closure or reject if mandatory | \(O(\lvert M\rvert + deps)\) | `mutation.target` | Large presence/absence variation | Reduces | no unresolved refs; required capabilities remain if policy says so |
| Sensory capability add/change | Choose capability type and legal transducer/interface | Medium | free information, orphan neural ports, dimension mismatch | Capability owns ports; construct complete bundle | \(O(M)\) | operator, target, structural params | Keep capability and owned interface linked | Moderate | physics-authorized channel; ports bound or explicitly unbound |
| Sensory capability delete | Choose deletable capability; remove owned ports/edges | Medium–low | broad behavioral collapse | Transactional closure | \(O(deps)\) | target | Presence/absence module | Reduces | no sensor data exposed after deletion |
| Action capability add/change | Choose physics-authorized action and controller interface | Medium | free actuation, invalid object interaction, unbounded force | Construct from capability registry and bounds | \(O(M)\) | operator, target, structural params | Keep actuator and controller port linked | Moderate | action exists in world physics; parameter bounds |
| Action capability delete | Remove capability and dependent ports | Medium–low | immobility or reproductive failure | Transactional closure; mandatory reproductive actions policy-specific | \(O(deps)\) | target | Presence/absence module | Reduces | no stale commands or ports |
| Plasticity-rule mutation | Local coefficient/rule-ID mutation | Low–medium; lifetime effects can be global | unstable learning, runaway weights, invalid modulator ref | Rule registry + bounded coefficients + valid channels | \(O(1)\) to \(O(E)\) validation | target, numeric step | Must remain linked to affected synapses/modules | None–moderate | rule exists; state bounds; exact reset at birth |
| Neuromodulation mutation | Add/delete channel, change production/receptor params | Low–medium | global pleiotropy, channel aliasing, instability | Typed channel ownership and bounded update | \(O(E)\) | target, structural params | Prefer module-level linkage | Moderate | producers/receptors valid; update phase defined |
| Regulatory mutation | Rule/edge/threshold change | Low–medium | attractor shift, nontermination, hidden cycles | Bounded rule set and update steps | \(O(R)\) | target, numeric/structural params | Crossover highly epistatic | Moderate | total deterministic evaluation |
| Developmental rule mutation | Tagged production edit, add/delete/duplicate | Low; phenotype effects may be global | expansion explosion, malformed phenotype | Preflight budget; explicit development failure | potentially high | target, structural params, `development.*` | Only homologous tagged-rule crossover | Very high phenotype risk | termination/budget; provenance map |
| Mutation-rate mutation | Bounded log-space step | High on meta-parameter; global descendant effect | zero-rate lock-in, mutator runaway, short-term bias | Hard bounds; optional cost; no effect on current event | \(O(1)\) | `mutation.strategy_parameter` | Alters future mutation exposure, not current crossover semantics | Indirect | rate in bounds; applied next reproduction only |
| Segment duplication/deletion | Whole aligned region | Low–medium | extreme bloat, linkage disruption | Atomic budget check and reference closure | \(O(\lvert S\rvert)\) | target, structural params | Changes future breakpoint map | Very high | new segment IDs; internal provenance |
| Module fission/fusion | Partition/merge with adapters | Low | arbitrary boundary, interface explosion | Construct from canonical cut/merge candidates | high | target, structural params | Alters recombination units | Mixed | all dependencies assigned; adapters explicit |

## 6.4 Numeric parameters and neural weights

Each numeric field has a schema-level contract:

```text
NumericContract {
    fixed_point_format
    minimum
    maximum
    local_step_distribution_id
    reset_distribution_id
    boundary_mode: Clamp | Reflect | Reject
    default_value
}
```

**[J]** Use fixed-point arithmetic for heritable and runtime neural parameters when the engine's deterministic kernel already relies on fixed point. A parameter's bit pattern is its canonical value; there is no NaN or negative zero.

For weights, consider three operators rather than one overloaded mutation:

- `WEIGHT_LOCAL_STEP`;
- `WEIGHT_RESET`;
- `WEIGHT_DISABLE_OR_ENABLE` if disabled edges are retained.

This makes event histories interpretable and operator rates independently testable.

## 6.5 Node insertion

Recommended edge-split operator:

```text
Before:
    source --(weight w, plasticity p)--> target

After:
    source --(w1, p1)--> new_node --(w2, p2)--> target
    old_edge.enabled = false
```

Possible initialization strategies:

- identity-preserving where the activation and fixed-point range permit;
- near-identity with explicit bounded error;
- neutral disabled insertion followed by later enablement;
- random local initialization.

**[J]** Use an identity-preserving or near-identity strategy for the baseline and measure actual phenotype equivalence. The exact transformation depends on activation semantics and should be separately versioned. Do not claim function preservation for recurrent, normalized, gated, or plastic edges without a test.

Invariants:

- one new node locus;
- two new edge loci;
- old edge retained disabled for homology, unless the policy deletes it explicitly;
- no duplicate IDs;
- source and target types compatible;
- recurrent class preserved or deliberately changed;
- module and segment ownership unambiguous;
- phenotype remains within limits.

## 6.6 Connection insertion

The candidate set should be generated from typed ports and node roles:

```text
valid_pair(source, target) =
    source.can_emit
    AND target.can_receive
    AND type_compatible(source.output_type, target.input_type)
    AND not forbidden_self_loop
    AND recurrence_policy_allows(source, target)
    AND no identical enabled edge
    AND module_interface_policy_allows(source, target)
```

Canonical candidate key:

```text
(module_locus_id, source_locus_id, target_locus_id, edge_kind)
```

To avoid \(O(N^2)\) scans for every mutation, maintain derived indexes keyed by type and module. Derived indexes are caches and excluded from genome checksums. Rebuilding them must produce the same candidate order.

## 6.7 Node and connection deletion

Deletion should distinguish:

- **disable:** preserve historical locus and payload but remove expression;
- **delete:** remove the locus from the current genome while ancestry remains in event storage.

Disabled genes create neutral variation and facilitate homologous crossover with descendants that retain the locus. Permanent deletion controls bloat.

Recommended policy:

- use disablement for recent structural changes and reversible expression;
- allow permanent deletion after a configurable age or through a separate operator;
- cap disabled-gene count and age;
- never garbage-collect a locus without a recorded deletion event.

Deleting a node atomically removes or disables incident edges. Whether downstream disconnected nodes are also removed must be a distinct operator; recursive “cleanup” can erase large regions and introduce hidden bias.

## 6.8 Module duplication

Atomic duplication procedure:

1. Select a module from canonical duplicable candidates.
2. Allocate a new `ModuleLocusId` from the mutation event.
3. Create a new or inherited `GeneFamilyId` according to the family rules.
4. Deep-copy internal genes.
5. Allocate new `LocusId`s for every copied gene.
6. Preserve `derived_from_locus_id` for each copy.
7. Copy internal edges using a deterministic old-to-new ID map.
8. Handle external ports under a declared mode:
   - duplicate unbound;
   - bind to the same external source;
   - create a duplicated adapter;
   - reject if the interface requires unique ownership.
9. Validate size and capability constraints.
10. Commit and record a compact copy map.

The operator must not reuse IDs or share mutable storage between copies.

## 6.9 Module deletion

A module is deletable only if:

- it is not the sole provider of a schema-required root interface;
- all incoming and outgoing dependencies have a deterministic closure;
- deletion fits the world's reproduction semantics;
- no live shared reference would remain.

Behaviorally catastrophic deletion is allowed if structurally valid. “Required” should mean required for the phenotype compiler, not required to survive.

## 6.10 Sensory, action, signaling, and object-interaction mutations

Capabilities should be typed genes backed by an immutable capability registry. The registry defines physical possibilities, not named technologies.

Example capability descriptors:

```text
CapabilityDescriptor {
    capability_kind_id
    physics_semantics_version
    parameter_contracts[]
    emitted_observation_ports[]
    accepted_command_ports[]
    energy_and_time_cost_contract
    spatial_or_contact_requirements
    object_material_constraints?
}
```

Possible evolvable capability traits include:

- sensor presence;
- field of view or range within physical bounds;
- resolution or sampling frequency;
- receptor selectivity;
- actuator strength, speed, or precision;
- grasp/hold/place interface;
- contact or impact action;
- signaling emission channel, amplitude, duration, or pattern;
- receptor sensitivity;
- object-interaction effector geometry, if the body model supports it.

**[J]** Adding a capability creates the capability and owned neural interface together. It must not expose information or action outside the world physics. A capability's existence is heritable; its lifetime calibration or wear state is not automatically heritable.

## 6.11 Plasticity and neuromodulation mutations

A plasticity gene should reference a rule in an immutable, versioned registry rather than store executable code. A generic local rule can encode fixed-point coefficients over declared variables:

```text
delta_weight =
    f(
      pre_activity,
      post_activity,
      reward_or_modulator_channels[],
      current_weight,
      eligibility_trace,
      fixed_coefficients[]
    )
```

The rule contract declares:

- update phase;
- state variables;
- fixed-point formats;
- saturation behavior;
- decay;
- lifetime initialization;
- whether stochasticity exists;
- maximum work per synapse per tick.

Mutations may:

- change coefficients;
- switch among compatible rule IDs;
- add/remove a modulator receptor;
- add/delete a neuromodulator channel;
- change eligibility decay;
- change which edges express the rule;
- duplicate a plasticity module.

Failure modes include runaway saturation, global pleiotropy, and crossover separating a rule from its target edges. Prefer module-scoped plasticity controllers and explicit target sets.

Learned weights are saved in organism lifetime state for exact restore. They are not copied into offspring initial weights unless a separately versioned Lamarckian experiment is enabled. Work comparing inheritance of acquired neural changes in changing environments cautions that direct inheritance can reduce stability [SASAKI99].

## 6.12 Regulatory and developmental mutations

Regulatory mutation should operate on typed bounded primitives:

- add/delete regulatory edge;
- change threshold;
- change gate function;
- duplicate regulatory submodule;
- change expression target;
- change developmental timing within finite bounds.

Developmental rule mutation should never modify an unversioned source string. Use canonical typed trees or records. Rule matching is deterministic, and all conflicts resolve by a specified key such as:

```text
(priority, rule_locus_id, target_provenance_id)
```

No result depends on insertion order.

## 6.13 Mutation-rate evolution

Possible heritable strategy genes:

```text
MutationStrategyGene {
    operator_family_multipliers[]
    genome_wide_count_scale
    duplication_scale
    deletion_scale
    numeric_step_scale
    recombination_mode_weights[]
}
```

Risks:

- selection favors immediate fidelity over long-term adaptability;
- mutators hitchhike with beneficial changes;
- near-zero rates freeze lineages;
- high rates drive mutational meltdown;
- larger genomes receive disproportionate mutation load;
- meta-mutation can create abrupt global changes.

Recommended safeguards:

- hard lower and upper bounds;
- mutations affect only future reproductions;
- integer/log-space representation;
- explicit cost only if physically meaningful or experimentally justified;
- no ability to modify ID generation, RNG algorithms, validation, or schema;
- separate operator-family rates so insertion and deletion can balance;
- compare against the best fixed-rate grid.

**[J]** Keep this feature dormant in the baseline schema or include the record with a fixed policy override. Enable only through an experimental policy fork.

## 6.14 Attempt, reject, repair, or construct-valid?

| Situation | Preferred response | Rationale |
|---|---|---|
| Add edge | Construct from valid pair set | Invalid edges need never exist |
| Split edge | Atomic constructor | One well-defined closure |
| Delete node | Atomic delete plus incident-edge closure | Deterministic and local |
| Delete capability | Atomic delete plus owned-port closure | Ownership provides one valid closure |
| Module duplication | Atomic deep-copy constructor | Prevent aliasing |
| Development exceeds budget | Reject development/offspring explicitly | Truncation changes semantics |
| Ambiguous external port rebinding | Reject mutation | Heuristic choice creates hidden bias |
| Parameter beyond range | Contract-specific clamp, reflect, or reject | Must be specified per field |
| Cycle where recurrence forbidden | Exclude candidate | Construct-valid |
| Unknown rule or schema ID | Fail load | Fail closed |
| Crossover child over hard limit | Apply declared fallback or reject birth | No silent pruning |
| Duplicate historical ID in one genome | Treat as corruption | Cannot be repaired without falsifying ancestry |

The preferred architecture is **valid by construction**, with a small number of deterministic transactional closures. Rejection is scientifically cleaner than ambiguous repair.

## 6.15 Mutation testable invariants

Every accepted mutation must satisfy:

1. Canonical serialization parses and reserializes identically.
2. All stable IDs are unique within their declared namespace.
3. Every reference resolves to a compatible record.
4. Every created locus points to exactly one origin event.
5. Every inherited locus existed in at least one parent.
6. Every duplicate has a new locus ID and a valid family/source relation.
7. Numeric values are representable and in range.
8. Hard genome and phenotype budgets are met.
9. Required compiler roots exist.
10. No cache or traversal order affects the checksum.
11. Reapplying the event to the same pre-genome yields identical bytes.
12. Replaying with a different thread schedule yields the same result.
13. The event's before and after checksums match the actual genomes.
14. Rejected mutations leave genome bytes unchanged.
15. The operator consumes draws only from its declared streams.
16. Mutation-event ordinals are contiguous and unique within the reproduction event.

---

# 7. Modularity, duplication, and developmental encodings

## 7.1 Modularity as an engineering property

“Modularity” has at least four meanings:

1. **Serialization modularity:** records are grouped into independently parseable modules.
2. **Recombination modularity:** modules are inherited as units.
3. **Phenotypic modularity:** internal effects are stronger than cross-module effects.
4. **Functional modularity:** modules contribute separable behavioral functions.

The baseline can guarantee the first two. The last two are evolutionary outcomes and must be measured.

A module boundary should have:

- typed input, output, and modulatory ports;
- explicit ownership;
- bounded fan-in and fan-out if desired;
- internal locus list;
- external dependency list;
- module-level enabled state;
- family and duplication provenance;
- optional developmental generator;
- compiled local-index map.

## 7.2 Pressures that can support modularity

Evidence suggests modularity can emerge under:

- modularly varying goals [KASHTAN05];
- selection for low connection cost [CLUNE13];
- specialization across tasks or environments [ESPINOSA10];
- recombination that penalizes dispersed epistasis [MISEVIC06];
- duplication followed by divergence [FORCE99; POSADAS22].

Genesis analogues include:

- environments where resource and behavioral demands recombine;
- physical wiring, energy, latency, or maintenance costs;
- module duplication;
- local mate recombination;
- changing niches;
- explicit but minimal interface costs.

**[J]** Do not add an arbitrary “modularity reward” to organism fitness. If modularity is desired as an emergent outcome, use causal physical or computational costs that have meaning in the simulation, or keep modularity only as an offline metric.

## 7.3 Module interface evolution

Interfaces can become a bottleneck if frozen. Safe evolvable operations include:

- add an optional port;
- delete an unused optional port;
- duplicate a port and its binding;
- change a port's bounded parameter;
- create a typed adapter module;
- split a broad port into typed subports;
- merge compatible ports through a declared operator.

Unsafe operations include changing a port type while retaining stale bindings or selecting a “closest” type implicitly.

Port identity should distinguish:

```text
port_locus_id            # ancestry
port_type_id             # semantic type
direction                # input/output/modulatory
cardinality              # one/many
ownership                # module/capability
```

## 7.4 Duplication as a path to innovation

Duplication can provide a temporarily redundant copy that changes with less immediate cost [FORCE99; LYNCH00]. In Genesis, duplication is especially useful for:

- neural subcircuits;
- repeated sensor-controller pairs;
- signaling channels;
- object-manipulation controllers;
- plasticity modules;
- developmental rules;
- body parts, if morphology becomes evolvable.

Metrics should distinguish:

- duplicate survival time;
- sequence/parameter divergence;
- interface divergence;
- expression divergence;
- functional specialization;
- subfunctionalization;
- neofunctionalization;
- deletion or silencing;
- cost paid during redundancy.

A duplicate should not be counted as innovation merely because it exists. Innovation requires a persistent new phenotype, behavior, ecological role, or reusable structural contribution.

## 7.5 Deletion and genome-size control

Deletion is not only a cleanup mechanism. It can:

- remove costly redundancy;
- specialize duplicates;
- simplify regulatory interactions;
- alter recombination units;
- create loss-of-function adaptations;
- expose previously masked pathways.

Genome-size control should use several layers:

- hard serialized-size and phenotype-size caps;
- balanced insertion/duplication and deletion operators;
- explicit runtime/energy costs where physically meaningful;
- size-fair recombination;
- disabled-gene retention limits;
- observer metrics for active versus total genes;
- optional periodic *recorded* compaction that creates a new schema/migration event, never invisible garbage collection.

Do not forcibly delete the “least useful” gene based on an evaluator when a cap is hit. That creates an external optimizer and biases evolution.

## 7.6 Pleiotropy and epistasis management

Pleiotropy is not inherently bad; shared components can create efficient coordination. Excessive, accidental pleiotropy makes mutation and recombination destructive.

Architecture can influence pleiotropy through:

- module-scoped parameters;
- typed interfaces;
- explicit shared modulators;
- local regulatory scope;
- immutable capability ownership;
- adapters rather than arbitrary cross-module edges;
- separate initial and lifetime-learning parameters.

Epistasis should be measured using sampled single and pair mutations:

```text
epistasis(A, B) =
    effect(A+B) - effect(A) - effect(B)
```

The exact effect measure may be log fitness, survival probability, task score, or phenotype distance. For stochastic environments, use paired deterministic seed sets.

## 7.7 Developmental bias and canalization

A developmental map changes which phenotypes are reachable by small mutations. It can create regularity, coordinated changes, and robustness, but also blind spots.

Practical mechanisms:

- bounded repetition counts;
- symmetry or coordinate generators;
- module templates;
- conditional expression gates;
- deterministic growth rules;
- developmental timing;
- duplicated rules with divergence.

Canalization can be measured as low phenotype variance under:

- genotypic perturbations;
- developmental-state perturbations;
- environmental perturbations during development.

**[J]** Do not add explicit “canalization genes” initially. Allow robustness to emerge from redundancy, gating, and modular construction, then measure it.

## 7.8 Grammar and tree alternatives

A credible module-grammar alternative would encode:

```text
ProductionRule {
    rule_locus_id
    left_symbol_type
    right_hand_typed_tree
    priority
    maximum_applications
    enabled
}
```

The derivation starts from a fixed root symbol and applies rules in canonical order until no rule applies or a bound is reached.

Requirements:

- no implicit wrapping;
- no grammar changes without semantic version changes;
- stable rule IDs;
- maximum depth, node count, and applications;
- deterministic tie resolution;
- generated-element provenance.

Tree crossover should be homologous or size-fair. Arbitrary subtree crossover is retained only as an ablation because of bloat and disruption [LANGDON00; POLI04].

## 7.9 CPPN and geometric alternatives

A CPPN can query coordinates and emit connectivity or material decisions [CPPN07; HYPERNEAT09]. It becomes attractive when Genesis has:

- regular sensory surfaces;
- repeated limbs or effectors;
- spatially embedded neural substrates;
- morphology or construction templates;
- stable coordinate systems.

A bounded CPPN module should declare:

- input coordinate semantics;
- output channels;
- activation-function registry;
- query ordering;
- connection-expression threshold;
- maximum emitted elements;
- handling of equal coordinates and symmetries;
- provenance from generated element to CPPN outputs and source loci.

The phenotype checksum includes the fully expanded graph, not only the CPPN genome.

## 7.10 Regulatory-developmental alternative

A future alternative could combine:

- direct modules as building blocks;
- a regulatory network selecting module expression;
- bounded duplication or placement during development;
- typed signals and developmental time;
- no unrestricted code.

This provides context-dependent expression and repeated structure while keeping the executable units inspectable. It is a credible second-generation experiment after the direct baseline.

## 7.11 Artificial chemistry as a future substrate

Artificial chemistry is relevant if Genesis later seeks:

- self-maintaining molecular-like organizations;
- lower-level replicators;
- metabolism-like construction;
- evolving genotype-phenotype mappings;
- transitions in individuality.

It is not needed to encode neural topology, sensor presence, object manipulation, or signaling. Introducing it early would multiply state space and verification cost without a clear baseline benefit.

## 7.12 Major transitions and nested genomes

If organisms later become composites, the lineage schema may need:

```text
CompositeOrganism {
    composite_organism_id
    member_organism_ids[]
    reproductive_owner
    germline_genome_ids[]
    somatic_or_worker_member_ids[]
    formation_event_id
}
```

This is a future extension. The current successor genome should reserve namespace/versioning capacity but not model cells, germlines, or chromosomes unless the world contains those entities.

---

# 8. Robustness and evolvability

## 8.1 Robustness is multidimensional

Measure at least:

- structural validity after mutation;
- developmental completion;
- birth viability;
- behavioral performance retention;
- ecological fitness retention;
- recovery after damage;
- robustness of lifetime learning;
- lineage persistence under mutation.

A genome can be structurally robust but behaviorally fragile. A phenotype can be robust in one environment and brittle after a shift.

## 8.2 Neutral networks

**[E]** Neutral networks can support drift through genotype space and provide access to new phenotypic neighborhoods [VANNIM99; DRAGHI10]. In digital organisms, robust genotypes can accumulate variation that later supports adaptation, but the effect depends on time and environment [ELENA08].

Practical sources of bounded neutrality:

- disabled genes;
- duplicate modules with overlapping function;
- alternate routes in a controller;
- unused optional ports;
- parameter changes below behavioral resolution;
- synonymous structural organization that compiles identically;
- dormant regulatory branches;
- signaling traits not currently observed.

Risks:

- genome bloat;
- storage cost;
- low active-to-total ratio;
- hidden recombination load;
- observer misclassification of “complexity.”

**[J]** Permit neutrality but do not inject unlimited junk loci. Monitor genotype diversity, phenotype diversity, active fraction, and adaptation after shifts.

## 8.3 Degeneracy

Degeneracy differs from exact redundancy. Two components overlap under some conditions but diverge under others. It can support robustness and later specialization.

Genesis mechanisms:

- two sensors responding to overlapping ranges;
- alternate action pathways;
- modules with partially overlapping outputs;
- different plasticity rules yielding similar baseline behavior;
- multiple signaling channels;
- different object-interaction strategies.

Degeneracy should emerge from capabilities and network organization. A dedicated “degeneracy flag” would be biological decoration.

## 8.4 Robustness versus adaptability

Evidence is mixed and time-dependent [ELENA08; DRAGHI10; LABAR17]:

- robust neighborhoods can reduce the immediate supply of phenotypic variation;
- neutral drift can accumulate hidden diversity;
- high mutation can select flat but lower peak regions;
- small populations may persist only on drift-robust regions;
- complex environments can change the relationship.

Therefore report:

- one-step beneficial, neutral, deleterious, and lethal mutation fractions;
- multi-step reachable phenotype diversity;
- early and late adaptation after a shift;
- robustness at fixed mutation rates;
- robustness after population bottlenecks;
- lineage survival.

Do not collapse these into one “evolvability score” used by selection.

## 8.5 Developmental bias

Every encoding biases variation. Direct graph mutation favors local additions and deletions. Module duplication favors repeated structures. CPPNs favor regular geometric patterns. Grammars favor structures expressible by their productions.

**[J]** Make bias explicit and test it:

- sample mutation neighborhoods from matched phenotypes under different encodings;
- compare phenotype-distance distributions;
- compare invalid and lethal fractions;
- compare innovation retention;
- inspect unreachable or rarely reached phenotype classes.

Developmental bias is useful when it aligns with the world's recurring structure. It is harmful when it prevents irregular solutions.

## 8.6 Canalization

Canalization can emerge from:

- saturation;
- redundant paths;
- threshold gates;
- module boundaries;
- negative feedback;
- learning that compensates for initial variation;
- developmental attractors.

But saturation can also create dead networks, and broad feedback can create instability.

Measure robustness across perturbation amplitudes and identify whether stability comes from meaningful organization or numerical clipping.

## 8.7 Exaptation

An exaptation is identified retrospectively:

1. a trait or module arose and persisted under an earlier context;
2. the environment or behavior changed;
3. the preexisting structure contributed to a new function.

Lineage and phenotype provenance are essential. Genesis should not contain an `exaptation` gene. Offline analysis can detect candidate events by comparing historical module use, environmental conditions, and descendant contribution.

## 8.8 Genetic assimilation and the Baldwin effect

Learning can smooth evolutionary search by allowing partially suitable inherited controllers to perform better during life. Over generations, evolution may produce better initial parameters or structures, reducing dependence on learning [DOWNING04; SASAKI99].

Implementation requirements:

- separate initial heritable parameters from learned lifetime values;
- exact birth-state and death-state snapshots for analysis;
- plasticity-rule genes;
- environmental exposure records;
- no automatic inheritance of learned weights;
- experiments comparing plastic and fixed controllers;
- metrics for performance before learning, after learning, and across generations.

A decrease in learning dependence is not automatically assimilation; it may reflect easier environments or changed behavior. Use matched evaluations.

## 8.9 Gene duplication

Duplication can increase robustness immediately through redundancy but also increase mutational targets and cost. Over time, duplicates may:

- remain redundant;
- partition function;
- specialize expression;
- acquire a new function;
- become disabled or deleted.

The engine should not protect duplicates indefinitely. Their retention must follow organism-level consequences and population dynamics.

## 8.10 Evolvable mutation rates

Mutation-rate evolution is a second-order trait because it changes descendant variation. Selection often acts on short-term lineage success, not experimenter-valued long-term innovation [CLUNE08].

Practical design:

- represent rates as bounded fixed-point multipliers;
- apply them to future births only;
- separate parameter and structural rates;
- limit per-generation change;
- prevent direct mutation of validation or RNG semantics;
- compare with fixed-rate controls;
- detect hitchhiking;
- detect mutational meltdown and zero-rate lock-in.

Mutation-rate genes can be useful research variables, but they are not required for open-endedness.

## 8.11 Pleiotropy

Pleiotropy can arise from:

- one sensor feeding many modules;
- one neuromodulator changing many synapses;
- shared action outputs;
- global regulatory gates;
- one developmental rule emitting many structures.

Record dependency graphs so offline analysis can estimate the phenotypic reach of a locus. High dependency degree is not identical to actual causal effect; perturbation tests are needed.

## 8.12 Epistasis

Strong epistasis is expected in neural and regulatory systems. It affects:

- crossover destructiveness;
- mutation-effect distributions;
- duplication divergence;
- speciation;
- hybrid fitness;
- robustness.

Experimental measurement should sample:

- single mutations \(A\), \(B\);
- combined mutation \(A+B\);
- effects in multiple genetic backgrounds;
- within-module versus cross-module pairs.

A modular genome should reduce average cross-module epistasis relative to within-module epistasis if its boundaries are functionally meaningful.

## 8.13 Mutation-selection balance and error thresholds

At a stable mutation regime, deleterious input and selection can reach a balance. Genome growth changes per-genome mutation exposure if rates are per locus. Conversely, a fixed per-genome count reduces per-locus exposure as genomes grow.

Genesis should explicitly choose and report:

- event count per genome;
- event probability per locus;
- operator-specific rates;
- genome-length scaling;
- mutation-effect distribution;
- reproductive output and population size.

Empirically map:

```text
mutation regime
    -> viable offspring fraction
    -> mean performance
    -> lineage extinction
    -> genome-size distribution
    -> diversity
```

Do not assign a theoretical Eigen threshold directly to a variable-topology embodied system [EIGEN71].

## 8.14 Genetic drift

Drift is unavoidable in finite populations and becomes stronger when reproductive success is highly unequal. It can:

- eliminate beneficial innovations before fixation;
- fix mildly deleterious variants;
- reduce diversity;
- amplify founder effects;
- change genome architecture;
- interact with robustness [LABAR17].

Genesis can measure drift exactly because parentage is known. Useful quantities include:

- variance in lifetime reproductive success;
- number of unique genetic contributors per generation window;
- lineage-frequency change under measured selection;
- effective population-size estimators;
- coalescence depth;
- allele-frequency trajectories;
- loss probability for new neutral or beneficial loci.

## 8.15 Population bottlenecks and founder effects

A headcount bottleneck is not enough. Record:

- number of surviving organisms;
- number of distinct parent lineages;
- distinct genome contents;
- module-family diversity;
- locus heterozygosity or analogous diversity;
- ecological and spatial coverage;
- reproductive variance after recovery.

Founder effects can be detected when a new deme or region descends from a small subset whose allele frequencies differ from the source. Exact lineage data allow direct measurement rather than inference.

## 8.16 Genome bloat

Bloat can arise from:

- duplication exceeding deletion;
- disabled genes protected by homology;
- neutral introns;
- tree/subprogram crossover;
- developmental rules that expand phenotype;
- selection exploiting more computation or state;
- hard caps that produce edge effects.

Metrics:

- serialized bytes;
- active genes;
- disabled genes;
- compiled nodes/edges;
- development steps;
- execution time;
- energy cost if modeled;
- offspring mutation load;
- marginal performance per gene/module;
- ancestry of added material.

Never infer sophistication from size alone. Digital GP can produce enormous programs without corresponding useful complexity [LANGDON19].

## 8.17 Robustness and evolvability recommendation

The baseline should provide:

- exact historical IDs;
- bounded disabled loci;
- module duplication and deletion;
- local fixed-point mutation;
- explicit plasticity rules;
- direct phenotype provenance;
- no fitness bonus for robustness or modularity;
- offline mutation-neighborhood assays;
- experimental protection mechanisms only when tested.

The engine should make robustness and evolvability *measurable*, not hardcode a preferred answer.


# 9. Species, niches, and observer classifications

## 9.1 There is no single species concept

In artificial systems, “species” can refer to different partitions:

| Classification | Definition | May affect reproduction? | Recommended status |
|---|---|---:|---|
| Reproductive compatibility | Organisms can produce an accepted or viable offspring under causal mechanics | Yes | Mechanistic, versioned |
| Genetic-distance cluster | Genomes are near one another under a metric | No by default | Offline observer |
| Behavioral cluster | Organisms behave similarly under an assay | No | Offline observer |
| Ecological niche cluster | Organisms use similar resources, spaces, or interactions | No | Offline observer |
| Network-topology cluster | Controllers share graph features | No | Offline observer |
| Ancestral clade | Organisms descend from a common ancestor under a rule | No | Exact/offline ancestry |
| Researcher-defined cluster | User-selected algorithm or threshold | No | Offline observer |
| NEAT-like innovation-protection group | Similar genotypes compete preferentially within a group | Yes | Explicit evolutionary policy, not observer taxonomy |
| Heritable mating preference group | Organisms choose similar signals or traits | Yes, through behavior | Emergent causal outcome |
| Spatial mating neighborhood | Only local organisms can mate | Yes | World physics/ecology |

The same population can have several valid partitions simultaneously. A behavioral cluster may cut across clades. A clade can occupy multiple niches. Two structurally similar genomes can be independently convergent.

## 9.2 Reproductive compatibility

A reproductive-compatibility rule is causal. It may use only data the declared policy is allowed to inspect.

Possible components:

- presence of required reproductive capabilities;
- minimum exact homologous-module coverage;
- compatible module-interface types;
- segment pairing;
- developmental viability;
- controller-selected mate acceptance;
- signaling and recognition;
- spatial proximity;
- resource availability.

**[J]** Avoid a single weighted whole-genome distance threshold as the baseline. It is easy to implement but difficult to interpret and vulnerable to genome bloat, neutral genes, and convergent topology.

A baseline compatibility report should return structured reasons:

```text
CompatibilityReport {
    result: Compatible | Incompatible | PolicyError
    exact_homologous_module_fraction
    exact_homologous_locus_fraction
    required_capability_status[]
    unresolved_mandatory_interfaces[]
    predicted_size_status
    reason_codes[]
}
```

This report is deterministic and can be stored with failed and successful reproduction attempts if the observer protocol needs it.

## 9.3 Speciation protection and niching

NEAT-style speciation protects new topology by reducing immediate competition with established structures [NEAT02]. Fitness sharing and niching in evolutionary computation similarly maintain multiple search regions.

For Genesis, these mechanisms are not neutral observer tools. If they alter competition, mate access, reproduction, or resource allocation, they are part of the world's evolutionary physics.

Potential policies:

- local competition only;
- resource niches that naturally separate lineages;
- explicit compatibility mating;
- offspring protection for novel structural innovations;
- fitness sharing by a declared metric;
- novelty or quality-diversity selection in an external evolutionary experiment.

**[J]** Prefer ecological and spatial mechanisms over an abstract species-protection layer in the main artificial-life world. Use explicit protection only in controlled experiments designed to determine whether structural innovations otherwise disappear too quickly.

## 9.4 Assortative mating and divergence

Assortative mating can promote reproductive divergence, but outcomes depend on ecology, dimensionality, gene flow, and the source of preference [ANDERSON14; DECARA08]. In digital organisms, ecological divergence can create stronger postzygotic isolation than mutation-order divergence under tested conditions [ANDERSON14].

Genesis experiments should separate:

- ecological divergence with no mating preference;
- mating preference with no ecological divergence;
- both together;
- geographic separation;
- mutation-order divergence under identical ecology.

Do not label a cluster a species until the criterion is stated. “Species under reproductive-compatibility criterion v3” is meaningful; bare `species_id = 7` is not.

## 9.5 Hybrid incompatibility

Hybrid reduction can arise through:

- incompatible module combinations;
- disrupted coadapted parameters;
- regulatory conflict;
- developmental timing mismatch;
- capability-controller mismatch;
- ecological intermediacy;
- signaling/preference mismatch in descendants.

The engine must keep accepted hybrid genomes structurally valid. Track:

- predevelopment incompatibility reason;
- development failure;
- birth viability;
- lifetime performance;
- fertility;
- descendant contribution;
- locus ancestry and parent contributions.

This permits analysis of prezygotic and postzygotic isolation without treating a serialization error as biology.

## 9.6 Behavioral and ecological clusters

Behavioral clustering may use:

- trajectory features;
- action distributions;
- sensor-response mappings;
- object-interaction sequences;
- signaling behavior;
- social-network position;
- learned-policy probes.

Ecological clustering may use:

- resource consumption;
- habitat occupancy;
- trophic interactions;
- construction or niche modification;
- temporal activity;
- competition coefficients.

These analyses are observer constructs. Store:

```text
ObserverClusterRecord {
    analysis_run_id
    observer_schema_version
    source_world_lineage_id
    source_tick_or_interval
    clustering_method_id
    feature_schema_id
    distance_metric_id
    parameters
    member_organism_ids[]
    confidence_or_stability_metrics
}
```

No simulation component may query `ObserverClusterRecord`.

## 9.7 Network topology clusters

Topology similarity can be useful for identifying convergent controller forms, but it should not be treated as ancestry. Possible offline descriptors:

- module count and types;
- node/edge counts;
- recurrent motifs;
- degree distributions;
- graphlets;
- module-interface graph;
- plasticity-rule distribution;
- spectral or embedding descriptors;
- exact structural signatures.

Graph isomorphism or learned embeddings can group independently evolved structures. The output belongs in the observer namespace.

## 9.8 Ancestral clades

An ancestral clade is defined from exact parentage or locus ancestry. In asexual reproduction, the organism graph is a tree or forest. In biparental reproduction, it is a DAG, and “clade” requires a convention:

- all descendants of one organism;
- descendants inheriting at least one locus from an ancestor;
- descendants inheriting a particular module;
- maternal/paternal role lineages, if semantic roles exist;
- genome-content descent;
- dominant-contribution descent.

Every query must state the convention. A two-parent pedigree does not have a unique tree without choosing a projection.

## 9.9 True ancestry versus similarity

The engine must enforce the following rules:

- Same `LocusId` implies inherited historical continuity.
- Same `StructuralSignature` does not imply common-locus ancestry.
- Same `GeneFamilyId` implies a recorded duplication-family relation, not necessarily identical function.
- Similar genome checksum is not a meaningful relation; a checksum is equality only.
- Same behavior does not imply same genotype.
- Same observer cluster has no causal meaning.
- Same visible tag does not imply kinship.
- Exact parentage comes only from reproduction records.

## 9.10 Kin recognition and relatedness

### Offline exact relatedness

The observer can compute:

- pedigree kinship;
- ancestor overlap;
- locus-level identity by descent;
- module contribution;
- generation distance;
- most recent common ancestors;
- inbreeding-like coefficients under a declared haploid/diploid model.

### Organism-accessible cues

Organisms may use:

- heritable signals;
- phenotype similarity;
- location and familiarity;
- parental imprinting or learned recognition;
- direct observation of reproduction;
- persistent artifacts or nest association.

Tags can support cooperation but can be exploited [AXELROD04; SCOTT22]. Therefore cue-based kin recognition should be allowed to succeed or fail through evolution.

### Recommendation

Do not feed exact pedigree relatedness into the neural controller by default. If an experiment exposes it, define a specific sensor with noise, range, cost, and versioned semantics.

## 9.11 Detecting radiations, bottlenecks, and extinctions

### Radiation

A candidate radiation should require:

- one ancestral lineage or narrow ancestor set;
- a short time interval;
- multiple descendant branches;
- persistence beyond a minimum horizon;
- measurable divergence in ecology, behavior, structure, or reproductive compatibility.

Report sensitivity to thresholds. A burst of transient mutants is not necessarily a radiation.

### Bottleneck

Detect from:

- reproducing-lineage count;
- effective contributor count;
- genome-content diversity;
- locus and module-family diversity;
- coalescence depth;
- reproductive skew.

A population can retain headcount while suffering a genetic bottleneck if few organisms produce descendants.

### Extinction

For exact clades, extinction occurs when no extant organism carries a declared ancestry relation to the clade. For observer clusters, extinction depends on the clustering method and time resolution and must be labeled as such.

## 9.12 Recommendation

- Do not place `species_id` in the heritable genome.
- Store exact causal compatibility outcomes and parentage.
- Keep all clustering outputs in a read-prohibited observer store.
- Version every distance metric and feature schema.
- Use exact ancestry for relatedness and clades.
- Use visible cues for embodied kin recognition.
- Treat NEAT-like speciation as an explicit experiment, not taxonomy.
- Fork the policy lineage if an observer metric becomes causal.

---

# 10. Lineage and ancestry representation

## 10.1 Design goals

The lineage system must support:

- exact parentage;
- asexual and paired reproduction;
- per-locus and per-module inheritance;
- mutation-event provenance;
- duplication families;
- genome deduplication;
- common-ancestor queries;
- bottleneck and founder analysis;
- replay verification;
- schema and policy forks;
- optional artifact lineage;
- long-running storage without keeping every full genome in every organism row.

## 10.2 Identifier hierarchy

### World and policy

```text
WorldLineageId          # causal replay branch
WorldInstanceId         # one executing save/restore instance, optional
GeneticsPolicyId        # immutable policy bundle content ID
GeneticsPolicyEpoch     # local epoch number within a world lineage
SchemaVersion
SemanticsVersion
```

### Organisms and genomes

```text
OrganismId
GenomeContentId
GenomeTransmissionId
PhenotypeContentId
LifetimeStateSnapshotId
```

### Events

```text
ReproductionEventId
MutationEventId
MigrationEventId
PolicyForkEventId
DevelopmentEventId      # optional if development is separately journaled
```

### Heritable structure

```text
SegmentLocusId
ModuleLocusId
LocusId
InnovationId
GeneFamilyId
PortLocusId
StructuralSignature
```

### Artifacts

```text
ArtifactId
ArtifactTransformationEventId
ArtifactContributionEdgeId
```

All IDs include a domain separator and scheme version. IDs from different kinds are not interchangeable even if their bit widths match.

## 10.3 Content identity versus historical identity

Two organisms can have byte-identical genomes. Store one `GenomeContent` record and two `GenomeTransmission` records.

```text
GenomeContent {
    genome_content_id          # canonical hash
    canonical_bytes_location
    schema_bundle
    phenotype_content_id
    size_metrics
}

GenomeTransmission {
    genome_transmission_id
    organism_id
    genome_content_id
    reproduction_event_id
    pre_mutation_parent_contributions
}
```

`GenomeContentId` answers “are the canonical genomes identical?”  
`GenomeTransmissionId` answers “which inheritance instance did this organism receive?”  
`LocusId` answers “does this gene copy descend from the same historical locus?”

Do not use a content hash as a lineage identifier.

## 10.4 Append-only lineage tables

A relational or columnar representation can use the following logical tables.

### `organisms`

```text
organism_id
world_lineage_id
birth_tick
death_tick?
reproduction_event_id
genome_transmission_id
phenotype_content_id
spatial_birth_key
semantic_parent_roles?
```

### `reproduction_events`

```text
reproduction_event_id
world_lineage_id
policy_id
policy_epoch
tick
reproduction_slot_key
inheritance_mode
canonical_parent_0_id
canonical_parent_1_id?
semantic_roles
pre_mutation_child_genome_id?
post_mutation_child_genome_id
offspring_ordinal
event_chain_checksum
```

### `parent_edges`

```text
reproduction_event_id
parent_organism_id
parent_role
genome_contribution_numerator
genome_contribution_denominator
module_contribution_count
locus_contribution_count
```

### `mutation_events`

Fields from Section 6.1.

### `genome_contents`

Canonical bytes, schema bundle, content checksum, compact metrics, and storage location.

### `genome_transmissions`

One row per organism birth, linking the content to the event.

### `locus_origins`

```text
locus_id
locus_kind
origin_innovation_id
origin_mutation_event_id
origin_genome_content_id
derived_from_locus_id?
gene_family_id
```

### `locus_transmissions`

```text
genome_transmission_id
locus_id
source_parent_organism_id?
source_parent_locus_id?
allele_payload_hash
alignment_kind
```

Storing every locus transmission may be expensive. See Section 10.8 for compression.

### `policy_forks`

```text
new_world_lineage_id
parent_world_lineage_id
fork_tick
fork_event_id
old_policy_id
new_policy_id
reason
checkpoint_checksum
```

### `observer_clusters`

Physically or logically separate and unreadable by the simulation.

## 10.5 Per-locus contribution maps

A child genome can record inheritance compactly as runs or module maps:

```text
ModuleContribution {
    child_module_locus_id
    source:
        Parent0
        Parent1
        Recombinant
        NewMutation
    source_module_locus_ids[]
    locus_runs[]
}
```

For direct tagged genes, a merge-alignment operation can emit sorted runs:

```text
(start_key, end_key, parent_source)
```

Mutations then overlay changed or created loci. This is more compact than one row per locus when modules are inherited intact.

## 10.6 Innovation and duplication lineage

A structural origin record should distinguish:

- new connection;
- split-edge node;
- new capability;
- module duplication;
- segment duplication;
- module fission/fusion;
- developmental rule;
- migration-created replacement.

Example:

```text
InnovationRecord {
    innovation_id
    mutation_event_id
    innovation_kind
    created_locus_ids[]
    source_locus_ids[]
    gene_family_id?
    structural_signature_after
    policy_id
}
```

A duplication family is a DAG if duplicated copies are duplicated again. Store immediate `derived_from` edges and compute family membership from the root family ID.

## 10.7 Common-ancestor queries

### Asexual organisms

Asexual parentage permits:

- binary lifting;
- Euler-tour intervals;
- generation-depth arrays;
- lowest-common-ancestor algorithms;
- compact tree-sequence-like storage.

### Paired reproduction

A pedigree DAG can have many common ancestors. Queries must specify:

- **pedigree MRCA set:** most recent organisms that are ancestors of all queried organisms;
- **closest common ancestor by minimum maximum distance;**
- **locus MRCA:** most recent origin/transmission ancestor for a particular locus;
- **module MRCA;**
- **dominant-contribution ancestor;**
- **projected lineage tree** under a declared parent-selection rule.

Algorithms can use:

- generation/time bounds;
- ancestor-set intersection;
- cached compressed ancestor sketches;
- exact traversal for bounded windows;
- offline indexes for extant populations.

Do not present one arbitrary projected parent as the “true” phylogeny.

## 10.8 Exact lineage storage and compression

### Full exact history

The simplest trustworthy model is append-only full event storage. Birth insertion is \(O(1)\) plus contribution data. Old records move to immutable cold storage but are not discarded.

### Content-addressed genomes

Store canonical genome bytes once per `GenomeContentId`. Identical descendants share the blob.

### Event deltas and snapshots

A genome may be reconstructed from parent content plus recombination and mutation events. To bound reconstruction time:

- write a full canonical snapshot at configurable intervals or when delta depth exceeds a threshold;
- keep all events;
- verify reconstructed checksum against `GenomeContentId`.

Snapshot cadence is a storage optimization and must not change simulation behavior.

### Tree-sequence ideas

Succinct tree sequences can compress ancestry by recording how genome intervals inherit through time [KELLEHER16; KELLEHER18]. They are extremely effective when genomes have stable linear coordinates and recombination breakpoints.

Genesis's graph/module genome lacks one universal base-pair coordinate. A partial adaptation is possible:

- treat each segment as a coordinate space;
- treat module/locus order as intervals;
- record parent edges over runs of homologous loci;
- start a new coordinate lineage after rearrangement;
- keep explicit records for graph-specific structural innovations.

**[J]** Use tree-sequence-inspired run compression, not an assumption that the entire graph genome is one stable linear chromosome.

### Hereditary stratigraphy

Hereditary stratigraphy stores inherited annotations that support approximate phylogenetic inference in distributed populations [MORENO22HS]. It is useful when exact central tracking is impossible.

Genesis is centrally simulated and already requires exact replay. **[J]** Use exact records as the source of truth. Hereditary stratigraphy is optional for distributed shards, exported organisms, or resilience tests, and its estimates must never overwrite exact ancestry.

**[E]** A tracking-enabled asynchronous island-model genetic algorithm has demonstrated approximate phylogenetic observability at wafer scale using hereditary annotations, including runs with populations up to millions of agents [MORENO24]. **[J]** This strengthens the case for hereditary stratigraphy as an export, sharding, or extreme-scale fallback. It does not displace exact reproduction events in worlds where Genesis controls every birth.

## 10.9 Relatedness estimation

### Pedigree kinship

For a haploid clonal lineage, relatedness follows the tree. For biparental haploid assortment, calculate expected or realized contribution using actual inherited loci.

### Realized identity by descent

Recommended realized relatedness between organisms \(i\) and \(j\):

```text
IBD(i, j) =
    weighted_sum(shared historical locus ancestry)
    / weighted_sum(comparable expressed or total loci)
```

Weights may be:

- all loci equal;
- module-normalized;
- active loci only;
- capability loci;
- genome-byte contribution.

The metric version and weighting must be reported.

### Structural similarity

Compute separately using allelic payload distance, graph descriptors, or module signatures. Never call it IBD.

## 10.10 Founder, bottleneck, radiation, and extinction queries

The exact tables permit:

- founder set of a spatial deme;
- fraction of current loci derived from each founder;
- number of effective ancestors over a time window;
- surviving innovation families;
- clade diversification rates;
- lineage-through-time curves;
- extinction time of an exact clade;
- behavioral/ecological radiation overlays;
- allele-frequency and module-family trajectories.

## 10.11 Artifact lineage

Persistent tools, structures, and modified objects may later mediate behavior. Artifact lineage can record:

```text
Artifact {
    artifact_id
    creation_tick
    creator_organism_ids[]
    parent_artifact_ids[]
    material_component_ids[]
    current_state_content_id
}

ArtifactTransformationEvent {
    event_id
    tick
    acting_organism_ids[]
    input_artifact_ids[]
    output_artifact_ids[]
    operation_physics_id
    event_checksum
}
```

This supports questions such as:

- Was an artifact copied, repaired, combined, or independently reinvented?
- Which organism lineage used or modified it?
- Did a construction tradition outlive its creators?
- Did a genome lineage inherit an environment altered by ancestors?

Artifact lineage is not genetic lineage. Cross-links are useful, but IDs and causal semantics remain distinct.

## 10.12 Event-chain integrity

Each event can contribute to an append-only hash chain or Merkle structure:

```text
event_chain_checksum =
    H(
      "genesis.event-chain.v1",
      prior_chain_checksum,
      canonical_event_bytes
    )
```

For parallel phases, compute leaf event hashes by preassigned event order and combine them in a deterministic Merkle tree. This detects event loss, reordering, or corruption.

## 10.13 Recommendation

- Keep exact append-only parentage and mutation events.
- Deduplicate genome contents by canonical checksum.
- Separate genome content, transmission, organism, locus, and innovation identities.
- Record module/locus parent contributions.
- Use run compression and snapshots, not lossy ancestry deletion.
- Keep observer clusters separate.
- Adapt tree-sequence concepts only where stable segment coordinates exist.
- Retain artifact lineage as a separate optional graph.
- Make all query definitions explicit in biparental pedigrees.

---

# 11. Recommended successor genome architecture

## 11.1 Architecture name and boundary

Recommended working name:

> **TMGG-1: Tagged Modular Graph Genome, schema version 1**

TMGG-1 is a deterministic data format and genotype-to-phenotype contract. It is not a claim to model DNA.

The architecture has four layers:

1. **Canonical genome document**
2. **Genetics policy bundle**
3. **Phenotype compiler**
4. **Lifetime organism state**

Only the canonical genome document is inherited. Policy and compiler versions are referenced in the causal world bundle. Lifetime state is saved for restoration but reset at birth.

## 11.2 Canonical header

```rust
struct GenomeHeader {
    magic: [u8; 8],
    genome_schema_version: u32,
    genome_semantics_version: u32,
    phenotype_compiler_version: u32,
    capability_registry_version: u32,
    activation_registry_version: u32,
    plasticity_registry_version: u32,
    regulatory_registry_version: u32,
    fixed_point_profile_version: u32,
    canonical_encoding_version: u32,
    id_scheme_version: u32,
    checksum_algorithm_version: u32,
    originating_policy_id: PolicyId,
    hard_limit_profile_id: LimitProfileId,
}
```

Some fields may live in the save's semantic bundle rather than repeat in each genome blob. The canonical bytes must nevertheless commit to all semantics required to interpret the genome.

## 11.3 Top-level structure

```rust
struct TaggedModularGraphGenome {
    header: GenomeHeader,
    segments: Vec<SegmentGene>,
    modules: Vec<ModuleGene>,
    capabilities: Vec<CapabilityGene>,
    nodes: Vec<NodeGene>,
    edges: Vec<EdgeGene>,
    plasticity_rules: Vec<PlasticityGene>,
    neuromodulators: Vec<NeuromodulatorGene>,
    regulatory_rules: Vec<RegulatoryGene>,
    developmental_generators: Vec<DevelopmentalGene>,
    strategy_genes: Vec<StrategyGene>,
    provenance: GenomeProvenance,
}
```

Vectors are serialized in canonical order, not runtime insertion order.

## 11.4 Segment gene

```rust
struct SegmentGene {
    segment_locus_id: SegmentLocusId,
    origin_innovation_id: InnovationId,
    gene_family_id: GeneFamilyId,
    ordinal_key: CanonicalOrdinal,
    module_locus_ids: Vec<ModuleLocusId>,
    enabled: bool,
    linkage_parameters: LinkageParameters,
}
```

`ordinal_key` is an evolvable linkage coordinate only through explicit segment-rearrangement operators. Canonical serialization sorts by `(ordinal_key, segment_locus_id)`.

## 11.5 Module gene

```rust
struct ModuleGene {
    module_locus_id: ModuleLocusId,
    origin_innovation_id: InnovationId,
    gene_family_id: GeneFamilyId,
    derived_from_module_locus_id: Option<ModuleLocusId>,
    segment_locus_id: SegmentLocusId,
    module_kind: ModuleKindId,
    interface_ports: Vec<PortGene>,
    internal_locus_ids: Vec<LocusId>,
    regulatory_input_ids: Vec<LocusId>,
    enabled: bool,
}
```

`module_kind` is a semantic type used for validation and compilation. It does not name a technology or behavior.

## 11.6 Capability genes

```rust
struct CapabilityGene {
    locus_id: LocusId,
    origin_innovation_id: InnovationId,
    gene_family_id: GeneFamilyId,
    module_locus_id: ModuleLocusId,
    capability_kind_id: CapabilityKindId,
    parameter_values: Vec<FixedParameter>,
    observation_ports: Vec<PortGene>,
    command_ports: Vec<PortGene>,
    enabled: bool,
}
```

Capability categories may include perception, action, signaling, and object interaction, but the actual kinds come from physics:

- detect a local field;
- detect contact or material properties;
- emit a bounded signal;
- apply force through an effector;
- grasp/release;
- hold/carry;
- place;
- strike;
- join or separate objects when material physics permits.

The registry must not include named “axe,” “spear,” “house,” or “crafting recipe” genes.

## 11.7 Neural node gene

```rust
struct NodeGene {
    locus_id: LocusId,
    origin_innovation_id: InnovationId,
    gene_family_id: GeneFamilyId,
    module_locus_id: ModuleLocusId,
    node_kind_id: NodeKindId,
    activation_function_id: ActivationFunctionId,
    bias: Fixed,
    time_constant: Fixed,
    initial_state: Fixed,
    port_bindings: Vec<PortBinding>,
    enabled: bool,
}
```

Node types and activations come from immutable registries. Any new activation semantics require a new registry version and policy branch.

## 11.8 Edge gene

```rust
struct EdgeGene {
    locus_id: LocusId,
    origin_innovation_id: InnovationId,
    gene_family_id: GeneFamilyId,
    module_locus_id: ModuleLocusId,
    source_port_locus_id: PortLocusId,
    target_port_locus_id: PortLocusId,
    initial_weight: Fixed,
    edge_kind_id: EdgeKindId,
    recurrence_class: RecurrenceClass,
    plasticity_gene_locus_id: Option<LocusId>,
    neuromodulator_receptor_ids: Vec<LocusId>,
    enabled: bool,
}
```

A compiled phenotype may flatten this to compact indices. Stable IDs remain in provenance maps.

## 11.9 Plasticity gene

```rust
struct PlasticityGene {
    locus_id: LocusId,
    origin_innovation_id: InnovationId,
    gene_family_id: GeneFamilyId,
    module_locus_id: ModuleLocusId,
    rule_id: PlasticityRuleId,
    coefficients: Vec<Fixed>,
    state_initializers: Vec<Fixed>,
    modulator_channel_ids: Vec<LocusId>,
    target_selector: PlasticityTargetSelector,
    enabled: bool,
}
```

`target_selector` must be explicit and deterministic: a list of edge locus IDs, module scope, or typed selector with stable semantics.

## 11.10 Neuromodulator gene

```rust
struct NeuromodulatorGene {
    locus_id: LocusId,
    module_locus_id: ModuleLocusId,
    channel_type_id: ModulatorChannelTypeId,
    production_rule_id: ProductionRuleId,
    production_parameters: Vec<Fixed>,
    decay: Fixed,
    saturation_bounds: FixedRange,
    receptor_scope: ReceptorScope,
    enabled: bool,
}
```

Update order is fixed by the runtime semantics: observe, integrate neural state, compute modulators, apply plasticity, or another declared phase sequence. It may not depend on edge iteration order.

## 11.11 Regulatory gene

Baseline regulatory genes should be bounded gates:

```rust
struct RegulatoryGene {
    locus_id: LocusId,
    module_locus_id: ModuleLocusId,
    gate_function_id: GateFunctionId,
    input_port_ids: Vec<PortLocusId>,
    target_locus_ids: Vec<LocusId>,
    coefficients: Vec<Fixed>,
    update_phase: RegulatoryPhase,
    enabled: bool,
}
```

A fully general regulatory network is a future policy. TMGG-1 can reserve the record kind while the baseline limit profile permits zero or a small bounded number.

## 11.12 Developmental gene

```rust
struct DevelopmentalGene {
    locus_id: LocusId,
    module_locus_id: ModuleLocusId,
    generator_kind_id: GeneratorKindId,
    canonical_program: TypedBoundedProgram,
    max_steps: u32,
    max_emitted_modules: u32,
    max_emitted_nodes: u32,
    max_emitted_edges: u32,
    conflict_rule_id: ConflictRuleId,
    enabled: bool,
}
```

The direct baseline sets `developmental_generators = []`. Keeping the record in the schema may be useful only if unknown disabled kinds are still rejected; never accept an unknown generator because it is disabled unless the schema explicitly defines safe preservation semantics.

## 11.13 Strategy gene

```rust
struct StrategyGene {
    locus_id: LocusId,
    operator_family_rate_multipliers: Vec<BoundedLogFixed>,
    recombination_mode_weights: Vec<FixedWeight>,
    enabled: bool,
}
```

Baseline policy may ignore or prohibit these. If ignored, the semantics must say so; an old genome cannot silently begin expressing them after a software update.

## 11.14 Genome provenance

```rust
struct GenomeProvenance {
    genome_transmission_id: GenomeTransmissionId,
    reproduction_event_id: ReproductionEventId,
    parent_contribution_summary: Vec<ParentContribution>,
    applied_mutation_event_ids: Vec<MutationEventId>,
    source_schema_version: u32,
    migration_event_id: Option<MigrationEventId>,
}
```

Content-addressed canonical genome bytes should not include organism-specific transmission IDs if identical genomes are intended to deduplicate. Therefore divide:

- **heritable canonical content**, hashed as `GenomeContentId`;
- **transmission envelope**, containing reproduction and migration provenance.

## 11.15 Canonical serialization

Rules:

1. Fixed byte order, preferably little-endian if consistent with the engine.
2. Fixed integer widths or a fully specified canonical varint.
3. No native Rust memory layout.
4. No pointer values, padding, or hash-map order.
5. Text identifiers normalized or, preferably, replaced by numeric registry IDs.
6. Vectors sorted by declared stable keys.
7. Duplicate keys rejected.
8. Optional fields encoded with one canonical form.
9. Fixed-point values encoded as raw signed integers.
10. Unknown required record kinds rejected.
11. Domain-separated checksum over exact canonical bytes.
12. Runtime caches excluded.
13. Schema and semantic bundle committed in the hash.

Example:

```text
GenomeContentId =
    H(
      "genesis.genome-content.v1",
      canonical_encoding_version,
      semantic_bundle_content_id,
      canonical_heritable_genome_bytes
    )
```

## 11.16 Phenotype compilation

Compilation phases:

1. Validate header and semantic bundle.
2. Validate ID uniqueness and reference closure.
3. Validate segment/module ownership.
4. Validate capability registry entries and physical bounds.
5. Expand bounded developmental modules.
6. Resolve module interfaces in canonical order.
7. Build compact node and edge arrays.
8. Establish deterministic evaluation schedule.
9. Initialize plasticity and modulator state.
10. Produce phenotype provenance map.
11. Compute `PhenotypeContentId`.
12. Cache by `(GenomeContentId, compiler_bundle_id)`.

```text
PhenotypeContentId =
    H(
      "genesis.phenotype-content.v1",
      phenotype_compiler_version,
      registry_bundle_id,
      canonical_expanded_phenotype_bytes
    )
```

## 11.17 Hard limits

A `LimitProfile` should specify:

- serialized genome bytes;
- segments;
- modules;
- loci by kind;
- nodes;
- edges;
- ports;
- cross-module edges;
- regulatory state;
- developmental steps;
- emitted phenotype size;
- runtime operations per tick;
- lifetime plasticity state bytes.

**[J]** Limits are engineering controls, not biological facts. Start conservatively and tune with Section 14. A sample bring-up profile might allow tens of modules, hundreds of nodes, and low thousands of edges, but no number should become part of scientific interpretation until benchmarked.

## 11.18 Credible alternatives

### Alternative A: TMGG plus CPPN modules

Use direct genes for arbitrary control and CPPNs for spatially regular sensor, morphology, or connection fields.

### Alternative B: Tagged module grammar

Use typed grammar productions to assemble direct modules. Better compression; harder causal tracing.

### Alternative C: Regulatory developmental graph

Use direct modules plus a bounded regulatory program that controls expression, duplication, and placement.

### Alternative D: Pure asexual direct graph

No crossover, no chromosome segments, historical IDs retained for lineage. Simplest and strongest determinism; lower direct combination of lineages.

### Alternative E: Diploid allele pairs

Maintain two alleles per locus with dominance and meiosis. This is scientifically interesting for masking deleterious variants and recombination but doubles state and adds dominance, segregation, and phase complexity. **Not recommended without a specific research objective.**

## 11.19 Why this baseline is preferred

TMGG-1 provides:

- high mutation locality;
- stable homology;
- module-level duplication and crossover;
- exact ancestry;
- bounded execution;
- straightforward canonical serialization;
- explicit capability evolution;
- plasticity and neuromodulation support;
- future developmental extension points;
- practical operation for thousands of organisms;
- fail-closed migration.

It does not guarantee open-ended evolution, modularity, speciation, tool use, or cumulative culture. It makes those outcomes representable and measurable without scripting them.

---

# 12. Deterministic reproduction and mutation protocol

## 12.1 Protocol overview

A reproduction phase is divided into:

1. **Intent collection**
2. **Canonical conflict resolution**
3. **Event-ID assignment**
4. **Parent/genome snapshot binding**
5. **Offspring construction**
6. **Mutation**
7. **Phenotype compilation**
8. **Commit**
9. **Event-chain update**

No step may depend on thread completion order.

## 12.2 Reproduction intents

```rust
struct ReproductionIntent {
    tick: Tick,
    phase: ReproductionPhase,
    requester_id: OrganismId,
    proposed_partner_id: Option<OrganismId>,
    target_slot_key: ReproductionSlotKey,
    requester_local_ordinal: u32,
    requested_offspring_count: u32,
    semantic_roles: SemanticRoles,
}
```

`requester_local_ordinal` is generated deterministically by the organism's action protocol, not an atomic counter.

Canonical intent sort key:

```text
(
  tick,
  phase,
  target_slot_key,
  min(requester_id, partner_id_or_requester),
  max(requester_id, partner_id_or_requester),
  requester_id,
  requester_local_ordinal
)
```

World-specific collision rules choose winning intents in this order. Losers receive deterministic outcome records if reproduction failures are observable.

## 12.3 Reproduction event ID

For each accepted intent and offspring ordinal:

```text
ReproductionEventId = H128_or_H256(
    "genesis.reproduction-event.v1",
    world_lineage_id,
    genetics_policy_id,
    genetics_policy_epoch,
    tick,
    phase,
    target_slot_key,
    canonical_parent_0_id,
    canonical_parent_1_id_or_zero,
    requester_id,
    requester_local_ordinal,
    offspring_ordinal
)
```

Use at least 128 bits with collision detection; 256 bits simplifies risk management. The hash algorithm and encoding are versioned.

## 12.4 Named RNG root

Use a counter-based generator or keyed pseudorandom function:

```text
random_block =
    PRF(
      algorithm_version,
      key = H(
        "genesis.rng-root.v1",
        world_seed,
        world_lineage_id,
        policy_epoch,
        reproduction_event_id,
        stream_name
      ),
      counter = draw_index
    )
```

Requirements:

- independent streams;
- random access by draw index;
- no mutable shared generator;
- fixed endianness;
- fixed conversion to integers and fixed-point;
- test vectors in the repository;
- algorithm version in saves.

## 12.5 Stream keys

Recommended streams and subkeys:

| Stream | Additional stable key |
|---|---|
| `reproduction.mode` | offspring ordinal |
| `recombination.segment_pair` | segment-pair canonical key |
| `recombination.breakpoint_count` | segment-pair key |
| `recombination.breakpoint_position` | segment-pair key, breakpoint ordinal |
| `recombination.module_choice` | child module slot |
| `recombination.locus_choice` | module locus, child locus |
| `recombination.excess_gene_choice` | unmatched locus ID |
| `mutation.count` | operator family |
| `mutation.operator` | mutation ordinal |
| `mutation.target` | mutation event ID, candidate-set checksum |
| `mutation.numeric_step` | mutation event ID, field ID |
| `mutation.structural_parameters` | mutation event ID, parameter ordinal |
| `mutation.strategy_parameter` | mutation event ID |
| `development.initialization` | organism ID, developmental module ID |
| `development.step` | organism ID, developmental module ID, step |
| `learning.noise` | organism ID, lifetime tick, rule/channel ID |

Subkeys prevent iteration over one module from shifting random choices for another.

## 12.6 Stable candidate ordering

Every random selection is over a canonical candidate list. Examples:

```text
node candidates:
    (module_locus_id, node_locus_id)

edge candidates:
    (module_locus_id, source_port_locus_id, target_port_locus_id, edge_kind)

module candidates:
    (segment_ordinal, module_locus_id)

numeric field candidates:
    (gene_kind, locus_id, field_id)
```

A candidate-set checksum may be recorded:

```text
CandidateSetId =
    H("genesis.candidate-set.v1", operator_id, canonical_candidate_keys)
```

This makes replay diagnostics precise. A mismatch reveals that validation or ordering changed before a random index was applied.

## 12.7 Weighted selection

Weights are nonnegative fixed-width integers. Selection:

1. Canonically order candidates.
2. Sum weights in a specified wide integer type with overflow checks.
3. Draw a uniform integer in `[0, total_weight)`.
4. Choose the first cumulative range containing the draw.

The bounded-uniform algorithm is versioned. Rejection sampling is acceptable inside an isolated stream, but the algorithm and consumed draw indices must be deterministic. A fixed-consumption method is easier to audit.

## 12.8 Canonical parent symmetry

For role-free modes, enforce the property:

```text
reproduce(parent_x, parent_y, event_context)
==
reproduce(parent_y, parent_x, event_context)
```

after canonicalization.

The actual event ID includes sorted parent IDs, not call-site order. If one parent wins a tie, the tie rule uses a random bit from a named stream or a stable ID rule declared by the policy.

## 12.9 Mutation event IDs

```text
MutationEventId =
    H(
      "genesis.mutation-event.v1",
      world_lineage_id,
      policy_epoch,
      reproduction_event_id,
      mutation_ordinal,
      operator_family_or_placeholder
    )
```

To avoid circularity when operator choice is random, either:

- derive the event ID before operator choice using the ordinal only, then record the chosen operator; or
- derive a base attempt ID from the ordinal and a final event ID including the operator.

Recommended:

```text
MutationAttemptId = H(... reproduction_event_id, mutation_ordinal)
MutationEventId   = H(... mutation_attempt_id, operator_id)
```

Created IDs derive from the final event ID.

## 12.10 Innovation and locus IDs

```text
InnovationId =
    H(
      "genesis.innovation.v1",
      world_lineage_id,
      id_scheme_version,
      mutation_event_id,
      innovation_kind,
      innovation_output_ordinal
    )

LocusId =
    H(
      "genesis.locus.v1",
      innovation_id,
      locus_kind,
      locus_output_ordinal
    )
```

For duplication:

```text
new_locus_id = H(
    "genesis.duplicated-locus.v1",
    mutation_event_id,
    source_locus_id,
    duplicate_output_ordinal
)
```

The duplicate retains a `GeneFamilyId` derived from the original family root.

Benefits:

- no global counter;
- no scheduling dependence;
- independent worlds cannot collide if namespaced;
- exact reconstruction;
- deterministic parallel generation.

Collision handling:

- detect duplicate ID during validation;
- fail the reproduction event with an explicit fatal determinism error;
- never choose another random ID, because that would hide a hash or implementation fault.

## 12.11 Compact sequential ID alternative

Sequential IDs are possible with a two-phase planner:

1. Canonically plan all reproduction and mutation events.
2. Count maximum IDs required per event.
3. Assign deterministic prefix-sum ranges.
4. Construct in parallel within assigned ranges.

This yields compact IDs but requires accurate preplanning and complicates variable-result operators. **[J]** Prefer event-derived hash IDs unless storage measurements show a material problem.

## 12.12 Parallel reproduction

Recommended sequence:

```text
A. freeze parent genome-content references for the phase
B. collect intents
C. canonical-sort and resolve conflicts
D. assign reproduction IDs and RNG roots
E. construct child genomes in parallel, writing only private buffers
F. validate and compile phenotypes in parallel
G. canonical-sort results by reproduction event ID
H. commit accepted births to preassigned world slots
I. append event records in canonical order
J. compute deterministic phase checksum
```

No worker writes to a shared innovation registry. No worker chooses an organism ID from an atomic increment.

## 12.13 Organism ID assignment

Recommended:

```text
OrganismId =
    H(
      "genesis.organism.v1",
      world_lineage_id,
      reproduction_event_id,
      offspring_ordinal
    )
```

Founder IDs derive from the world initialization event and canonical founder ordinal.

## 12.14 Deterministic phenotype development

If development uses no randomness, output is a pure function of genome and compiler bundle.

If it uses stochastic development, all draws are keyed by:

```text
(world_seed, world_lineage_id, organism_id,
 developmental_module_locus_id, developmental_step,
 stream_name, draw_index)
```

Developmental updates use:

- declared synchronous or asynchronous semantics;
- canonical target order;
- fixed-point math;
- bounded steps;
- explicit conflict resolution;
- explicit overflow failure.

## 12.15 Lifetime learning determinism

Learning-state updates require:

- deterministic observation ordering;
- deterministic neural update phases;
- fixed-point arithmetic;
- named RNG if noise exists;
- canonical edge and rule order;
- exact save of weights, traces, modulators, and learning counters;
- separation from heritable initial parameters.

A restored organism must resume with identical next-tick updates.

## 12.16 Tie resolution

Tie hierarchy:

1. semantic rule;
2. stable numeric comparison;
3. canonical stable ID;
4. named random stream if the experiment requires unbiased choice.

Never resolve by “first in container.”

## 12.17 Failure outcomes

Possible deterministic outcomes:

```text
AppliedBirth
NoCompatibleMate
ConflictLost
InheritanceConstructionRejected
GenomeBudgetExceeded
DevelopmentFailed
PhenotypeCompileFailedInvariant
MutationRejected
PolicyVersionUnavailable
SchemaUnsupported
FatalDeterminismViolation
```

Expected evolutionary failures are data. Invariant and schema failures are engine errors and should stop or quarantine according to fail-closed policy.

## 12.18 Checksums

### Semantic bundle

```text
SemanticBundleId =
    H(
      schema version,
      phenotype compiler version,
      capability registry,
      activation registry,
      plasticity registry,
      regulatory registry,
      fixed-point profile,
      RNG algorithm,
      genetics policy,
      limit profile
    )
```

### Genome

```text
GenomeContentId =
    H("genesis.genome-content.v1",
      SemanticBundleId,
      canonical_heritable_bytes)
```

### Phenotype

```text
PhenotypeContentId =
    H("genesis.phenotype-content.v1",
      SemanticBundleId,
      canonical_expanded_phenotype_bytes)
```

### Reproduction result

```text
ReproductionResultChecksum =
    H("genesis.reproduction-result.v1",
      reproduction_event_id,
      parent_genome_ids,
      inheritance_record,
      mutation_event_ids,
      child_genome_content_id,
      phenotype_content_id,
      outcome)
```

## 12.19 Multiple worlds

Every world has:

- its own `WorldLineageId`;
- its own seed;
- its own policy bundle;
- no mutable global counters;
- no shared RNG state;
- no shared writable innovation registry.

Immutable registries may be shared by content ID. Caches may be shared only if keyed by all semantics and unable to change results.

## 12.20 Policy changes and replay-lineage forks

A policy change creates:

```text
NewWorldLineageId =
    H(
      "genesis.policy-fork.v1",
      parent_world_lineage_id,
      fork_tick,
      checkpoint_checksum,
      old_policy_id,
      new_policy_id,
      fork_reason
    )
```

The new branch may share history before the fork, but it is not the same replay lineage. Analyses must not compare event IDs across the fork as though unchanged semantics continued.

---

# 13. Versioning and migration requirements

## 13.1 Separate syntax from semantics

At minimum, version:

- binary/serialization schema;
- canonical encoding;
- gene-kind semantics;
- phenotype compiler;
- capability registry;
- activation functions;
- plasticity rules;
- regulatory rules;
- fixed-point arithmetic profile;
- RNG algorithm and distributions;
- mutation operators;
- recombination operators;
- hard-limit policy;
- checksum and ID schemes;
- lineage-event schema;
- observer-analysis schema.

Changing a default value can be a semantic change even when binary parsing is unchanged.

## 13.2 Immutable semantic bundles

A save references an immutable `SemanticBundleId`. Loading requires one of:

1. exact support for that bundle;
2. a deterministic explicit migration to a new bundle;
3. rejection.

Do not load old bytes under new defaults.

## 13.3 Migration object

```text
GenomeMigration {
    migration_id
    source_semantic_bundle_id
    destination_semantic_bundle_id
    migration_code_version
    source_genome_content_id
    destination_genome_content_id
    transformed_locus_map[]
    created_locus_ids[]
    removed_locus_ids[]
    warning_codes[]
    migration_checksum
}
```

A migration is a pure function over canonical input bytes and immutable migration parameters.

## 13.4 Migration rules

- Every migrated genome receives a new `GenomeContentId`.
- Historical `LocusId`s are preserved when semantic identity truly persists.
- If a gene is replaced by a new semantic construct, create a new locus and record `derived_from`.
- New required fields receive values only from an explicit migration rule.
- Removed fields remain recoverable through the source blob and migration record.
- Unknown rule IDs cause rejection.
- Migration cannot consult wall-clock time or mutable configuration.
- Migration output is canonical and checksum-verified.
- Migration is tested for repeatability.
- A migration should be idempotent with respect to its destination, or refuse already migrated input.
- Migration does not overwrite the old save.

## 13.5 Fail-closed loading

Reject or quarantine when:

- magic/version is unknown;
- required registry is missing;
- checksum fails;
- duplicate stable IDs occur;
- a reference is unresolved;
- numeric encoding is invalid;
- hard limits are exceeded without a declared legacy profile;
- a mutation or lineage event has inconsistent checksums;
- event chain is broken;
- semantic bundle is unavailable;
- migration is ambiguous;
- an unknown gene kind could affect phenotype.

Preserving an unknown disabled gene verbatim is safe only if the schema explicitly defines opaque preservation and guarantees that it cannot become enabled or affect canonical interpretation. The baseline should reject unknown gene kinds.

## 13.6 No silent reinterpretation

Examples of prohibited silent reinterpretation:

- treating an old activation-function ID as a new function;
- changing fixed-point scale while retaining bytes;
- using a new edge-update order;
- changing an object-interaction capability's physical meaning;
- changing crossover excess-gene rules;
- changing mutation-rate scaling;
- compiling old developmental grammar under new productions;
- applying a new size cap by silently deleting genes;
- treating old innovation numbers as event-derived IDs without migration;
- switching RNG library versions.

## 13.7 Policy epochs

Within one world lineage, an epoch identifies an immutable active policy. A causal policy change should generally fork the lineage. Minor changes that cannot affect simulation results may update observer metadata without a fork.

A safe rule:

> If a change can alter any future organism, random choice, identifier, genome, phenotype, or event record from the same checkpoint, it requires a new world lineage.

## 13.8 Replay after policy changes

To replay old history:

- load the old semantic bundle;
- run the old policy implementation;
- verify event and state checksums.

If old executable semantics are no longer shipped, the engine can inspect and migrate the checkpoint but cannot claim exact replay. The report should say `REPLAY_UNAVAILABLE_OLD_SEMANTICS`, not approximate success.

## 13.9 Genome and phenotype checksum migration

A schema migration changes the genome checksum even if the compiled phenotype is intentionally equivalent. Record both:

```text
source_genome_content_id
destination_genome_content_id
source_phenotype_content_id
destination_phenotype_content_id
phenotype_equivalence_test_id
equivalence_result
```

Byte equality and behavioral equivalence are different.

## 13.10 Lineage separation across migration

Two options:

1. **Administrative migration branch:** migrate a checkpoint and create a new `WorldLineageId`.
2. **Offline genome conversion:** convert genomes for analysis without executing descendants.

Do not splice migrated descendants into the original exact replay chain.

## 13.11 Schema evolution strategy

Recommended discipline:

- append new optional record kinds only with explicit semantics;
- reserve numeric ID ranges but do not assign meaning before versioning;
- keep parsers small and total;
- maintain golden corpus saves for every supported version;
- maintain mutation/recombination test vectors;
- make old policy bundles content-addressed artifacts;
- document deprecation and exact-replay support windows;
- never mutate saves in place.

## 13.12 Migration acceptance tests

A migration is releasable only if:

- all golden source genomes migrate deterministically;
- source blobs remain unchanged;
- destination canonical roundtrip is stable;
- all lineage mappings resolve;
- all created/deleted loci are accounted for;
- exact replay branches are clearly separated;
- equivalent phenotypes match their declared comparison;
- corrupted and unknown inputs fail closed;
- serial and parallel batch migration results are identical;
- repeated migration produces the same destination IDs.


# 14. Experimental program

## 14.1 Experimental principles

Every genetics feature should pass two distinct gates:

1. **Implementation validity:** the feature is deterministic, structurally safe, and measurable.
2. **Scientific utility:** the feature produces a repeatable benefit or informative dynamic relative to a simpler control.

A negative result is valid. The engine should not retain crossover, mutation-rate evolution, or developmental encoding merely because biology contains an analogue.

### Common protocol

- Use at least 30 independent deterministic world seeds per treatment for exploratory experiments and 100 or more for high-variance extinction/speciation experiments, unless power analysis supports another count.
- Use identical seed panels across paired treatments.
- Separate initialization RNG, environmental RNG, reproduction RNG, and assay RNG.
- Report medians, distributions, confidence intervals, effect sizes, extinction frequency, and lineage-level outcomes.
- Predeclare primary metrics and stopping horizons.
- Evaluate both static and changing environments.
- Archive policy bundle, source commit, build fingerprint, semantic bundle, seeds, checkpoints, event-chain checksums, and analysis code.
- Repeat a subset across thread counts and supported platforms.
- Use common starting ancestors and independently evolved starting populations where appropriate.
- Do not select only surviving runs when comparing adaptation; extinction is an outcome.

The numerical gates below are **[J] provisional engineering criteria**, not universal evolutionary constants. They should be adjusted after pilot variance estimates, but not after seeing the final treatment result.

## 14.2 Experiment matrix

### Experiment 1 — Mutation validity and deterministic replay

**Question:** Can every mutation operator produce only valid accepted genomes and replay exactly?

**Treatments / ablations**

- valid-by-construction candidate enumeration;
- attempt-and-reject;
- deterministic transactional repair;
- each operator individually;
- mixed operator schedules;
- serial and multiple thread counts.

**Protocol**

Generate random valid genomes across the limit profile, including edge cases near zero and maximum size. Apply at least \(10^7\) mutation attempts across operators. Replay every sampled event from the recorded pre-genome and event ID.

**Metrics**

- malformed accepted genome count;
- rejected attempt rate and reason distribution;
- before/after checksum agreement;
- event replay byte equality;
- candidate-set checksum equality;
- runtime per attempt;
- mutation distribution bias;
- invariant coverage.

**Acceptance criteria**

- zero malformed accepted genomes;
- zero checksum or replay mismatches;
- zero thread-count differences;
- every rejection has a stable reason code;
- candidate selection frequencies agree with the specified distribution within statistical tolerance;
- no operator has an unexplained rejection rate above 5% away from hard-limit boundary cases;
- any deterministic repair must produce exactly one documented closure.

### Experiment 2 — Recombination destructiveness

**Question:** Which crossover operator preserves viable structure while still generating useful combinations?

**Treatments / ablations**

- clonal mutation;
- paired single-genome inheritance;
- whole-module assortment;
- homologous tagged crossover;
- segment multipoint crossover;
- uniform-locus crossover;
- non-homologous crossover at low rate;
- parent genomes with low, medium, and high divergence.

**Metrics**

- structural acceptance;
- developmental completion;
- birth viability;
- early survival;
- child performance relative to geometric or arithmetic parental baseline;
- exact module retention;
- cross-module broken-binding count;
- recombinant novelty;
- descendant contribution after 10, 50, and 200 generations;
- population extinction.

**Acceptance criteria for baseline crossover**

- zero malformed accepted children;
- median birth viability no more than 15 percentage points below module assortment at the target divergence range;
- at least 90% of children preserve all mandatory interfaces;
- homologous/module crossover outperforms uniform crossover on viability and module retention;
- after an environmental recombination challenge, crossover yields at least a 10% median increase in persistent useful module combinations or a predeclared equivalent effect without more than a 5 percentage-point extinction increase;
- if these criteria fail, use module assortment or no crossover as baseline.

### Experiment 3 — Crossover versus no crossover within paired reproduction

**Question:** Does crossover add value beyond mate choice, reproductive cost, and two-parent participation?

**Treatments**

Hold mate-choice behavior, reproduction cost, parent availability, and mutation constant:

- paired single-genome inheritance;
- paired whole-module assortment;
- paired homologous crossover.

**Metrics**

- adaptation rate;
- genetic and phenotype diversity;
- beneficial-variant combination rate;
- mutational load;
- offspring viability;
- lineage turnover;
- reproductive skew.

**Acceptance criteria**

Crossover earns a default role only if it shows a preregistered benefit in at least two qualitatively different environments and is noninferior on extinction and viability. Otherwise retain it as an optional world policy.

### Experiment 4 — Sexual versus asexual reproduction

**Question:** Under what population and environmental regimes does paired inheritance help?

**Factorial design**

- reproduction: clonal versus paired module assortment versus paired homologous crossover;
- population size: small, medium, large;
- environment: static, periodically shifting, modularly varying;
- spatial structure: well mixed versus local;
- epistasis: low versus high starting architecture;
- mate scarcity: low versus high.

**Metrics**

- time to adaptation;
- standing diversity;
- fixation time;
- clonal interference;
- deleterious load;
- genome modularity;
- extinction;
- reproduction cost;
- eligible-mate fraction.

**Acceptance criteria**

No universal winner is expected. The experiment is accepted when it identifies stable interaction effects and reproduces under a second seed panel. Paired inheritance should become a main-world default only if its benefits survive realistic mating and reproduction costs. Otherwise support mixed or world-specific modes.

### Experiment 5 — Genome bloat and size control

**Question:** Which controls prevent unproductive genome and phenotype growth without suppressing innovation?

**Treatments / ablations**

- hard cap only;
- balanced duplication/deletion;
- disabled-locus age limit;
- module cost or runtime cost where physically meaningful;
- size-fair crossover;
- no deletion;
- no duplication;
- direct versus developmental encoding.

**Metrics**

- serialized bytes;
- modules, nodes, edges, and disabled loci;
- active-to-total locus ratio;
- compiled phenotype size;
- execution time and memory;
- offspring mutation load;
- innovation rate;
- fitness/performance;
- fraction of births rejected at cap;
- lineage persistence.

**Acceptance criteria**

- 95th-percentile genome and phenotype size remains below 70% of the hard cap in equilibrium treatments;
- fewer than 1% of births are rejected solely because of size outside intentional stress conditions;
- median active-to-total locus ratio remains above 0.5, or dormant material demonstrates a measured adaptive benefit;
- no-control treatment shows significantly greater bloat, establishing that the intervention is causal;
- size control must not reduce persistent structural innovation by more than 20% relative to the best controlled alternative;
- no silent pruning.

### Experiment 6 — Neutral drift

**Question:** Does bounded neutral variation increase future adaptation rather than only storage cost?

**Treatments**

- disabled loci removed immediately;
- disabled loci retained with bounded lifetime;
- duplicate modules allowed;
- neutral parameter resolution coarsened versus fine;
- direct genotype diversity with identical compiled phenotype.

After a neutral-drift phase, apply several environmental shifts.

**Metrics**

- genotype diversity at fixed phenotype;
- phenotype diversity after shift;
- time to first adaptive innovation;
- final adaptation;
- genome size;
- recombination load;
- lineage diversity.

**Acceptance criteria**

A neutral-retention policy is justified if it produces at least 2× neutral genotype diversity and at least a 10% improvement in median post-shift adaptation speed or persistent innovation, while increasing median genome cost by no more than 25% and not increasing extinction. If it only adds size, shorten retention or remove it.

### Experiment 7 — Functional modularity

**Question:** Do explicit modules correspond to reduced cross-module epistasis and improved adaptation?

**Treatments / ablations**

- flat tagged graph;
- explicit modules with unrestricted cross-links;
- explicit modules with typed adapters;
- modularly varying environment versus fixed environment;
- connection/runtime cost on versus off.

**Metrics**

- within- versus cross-module edge density;
- within- versus cross-module epistasis;
- module reuse;
- adaptation after subproblem recombination;
- crossover viability;
- graph modularity metrics;
- performance cost.

**Acceptance criteria**

Module architecture is functionally supported if:

- cross-module epistasis is at least 20% lower than within-module epistasis;
- reacquisition after recombined environmental demands is at least 25% faster than the flat control;
- static-environment performance is no more than 10% lower;
- results persist under at least two module metrics and are not solely caused by a hard ban on cross-links.

### Experiment 8 — Module duplication

**Question:** Does module duplication improve structural innovation and robustness enough to justify bloat risk?

**Treatments**

- no duplication;
- node/edge additions only;
- module duplication plus deletion;
- module duplication without deletion;
- segment duplication at rare rate.

**Metrics**

- duplicate retention time;
- divergence;
- subfunctionalization;
- new capability/controller combinations;
- robustness;
- bloat;
- extinction;
- descendant contribution of duplicate-derived loci.

**Acceptance criteria**

Duplication is justified if duplicate-derived persistent innovations occur at least 2× as often as matched-size node/edge controls, while the 95th-percentile size and evaluation cost remain within the bloat gate. Duplication without deletion should fail the bloat control; if it does not, inspect whether duplication is effectively inactive.

### Experiment 9 — Evolved mutation rates

**Question:** Can bounded heritable mutation rates improve adaptation without runaway or lock-in?

**Treatments**

- fixed low, medium, high, and empirically best rate;
- bounded heritable global rate;
- bounded per-operator-family rates;
- heritable rates with and without a physically justified cost;
- smooth, rugged, static, and changing environments.

**Metrics**

- evolved rate distribution;
- distance from best fixed rate;
- adaptation and final performance;
- extinction;
- mutator hitchhiking;
- near-zero lock-in;
- mutational meltdown;
- genome-size interaction.

**Acceptance criteria**

Enable in production only if:

- median long-run outcome is no more than 10% below the best fixed treatment across the intended environment class;
- extinction does not increase by more than 5 percentage points;
- fewer than 5% of surviving populations hit either hard rate bound for more than half the run;
- the result reproduces from multiple initial rates;
- per-family evolution provides benefit beyond a global rate;
- otherwise keep rates fixed and expose the negative result.

### Experiment 10 — Mutation robustness

**Question:** How tolerant are evolved genomes to each mutation family?

**Treatments**

- direct flat graph;
- tagged modular graph;
- modular graph with duplication;
- bounded indirect generator;
- plasticity on/off.

For sampled evolved genomes, enumerate or sample one-step and two-step neighborhoods.

**Metrics**

- valid, lethal, deleterious, neutral, and beneficial fractions;
- phenotype-distance distribution;
- performance decay by mutation magnitude;
- recovery through lifetime learning;
- module-specific fragility.

**Acceptance criteria**

The baseline should achieve at least 99.9% structural validity for operator-generated one-step mutations and a birth-viability target selected from pilot ecology. A representation is preferred only if robustness does not come from inability to change phenotype: report beneficial and novel fractions alongside neutral fraction.

### Experiment 11 — Structural innovation survival

**Question:** Do new nodes, modules, capabilities, and developmental rules survive long enough to be optimized?

**Treatments**

- no protection;
- spatial/local competition;
- NEAT-like compatibility protection;
- age-layered or novelty protection;
- ecological niche availability;
- module mutation disabled as control.

**Metrics**

- innovation survival at 1, 10, 50, and 200 generations;
- eventual fixation or stable polymorphism;
- descendant count;
- performance trajectory;
- bloat;
- diversity;
- extinction.

**Acceptance criteria**

A protection mechanism is justified if it at least doubles 20-generation survival of initially weak but later beneficial innovations and increases persistent adaptive innovations, while reducing elite performance by no more than 10% and not preserving neutral bloat at the same rate. Ecological/local protection is preferred when effects are comparable.

### Experiment 12 — Speciation mechanisms

**Question:** Which causal mechanisms produce persistent reproductive divergence without arbitrary population fragmentation?

**Treatments**

- no compatibility gate;
- homologous-module threshold;
- module-interface compatibility;
- controller-mediated assortative mating;
- spatial isolation;
- ecological divergence;
- combinations;
- offline clustering only as negative causal control.

**Metrics**

- eligible-mate fraction;
- realized gene flow;
- hybrid viability and fertility;
- genetic, behavioral, and niche divergence;
- number and persistence of reproductively isolated groups;
- extinction and mate failure;
- threshold sensitivity.

**Acceptance criteria**

A mechanism qualifies as stable reproductive divergence if:

- reduced gene flow persists for at least 200 generations after removing transient geographic separation, where applicable;
- within-group mating remains feasible for at least 80% of reproductive attempts;
- between-group hybrid reduction follows declared causal incompatibilities rather than parser failures;
- population-wide extinction does not increase by more than 5 percentage points;
- offline clusters alone have zero causal effect by construction.

### Experiment 13 — Diversity maintenance

**Question:** Which mechanisms preserve useful diversity without replacing natural selection with an external optimizer?

**Treatments**

- baseline local ecology;
- global competition;
- fitness sharing;
- novelty search;
- quality-diversity archive;
- spatial demes;
- resource niches.

**Metrics**

- lineage diversity;
- genome and phenotype diversity;
- behavioral repertoire;
- niche occupancy;
- best and median performance;
- innovation rate;
- extinction;
- archive dependence.

**Acceptance criteria**

For the main world, prefer mechanisms grounded in spatial/resource ecology. An explicit diversity mechanism is justified only if it produces at least 2× persistent lineage or behavioral diversity with no more than 10% loss in top performance and no loss of exact determinism. Observer-only archives must not feed back unless the world is explicitly an evolutionary-search experiment.

### Experiment 14 — Bottleneck and founder recovery

**Question:** How do encoding and reproduction policies affect recovery after severe diversity loss?

**Treatments**

- bottlenecks retaining 50%, 10%, 1%, and one founder;
- random founders versus high-fitness founders versus spatially biased founders;
- clonal, module-assortment, and crossover reproduction;
- duplication on/off;
- neutral retention on/off.

**Metrics**

- time to recover 80% of pre-bottleneck mean performance;
- extinction;
- genome, locus, module-family, and behavioral diversity;
- founder contribution;
- inbreeding/IBD;
- innovation rate;
- coalescence depth.

**Acceptance criteria**

A robust baseline should recover at least 80% of pre-bottleneck performance within the preregistered horizon after a 90% bottleneck in at least 90% of runs. The one-founder treatment is descriptive, not a required pass. Recovery must not be reported without diversity loss and founder dominance.

### Experiment 15 — Environmental shifts

**Question:** Which genome mechanisms support adaptation to abrupt and recurring changes?

**Treatments**

- static versus abrupt shift versus recurring shift;
- fixed weights versus plasticity;
- direct versus modular versus bounded developmental encoding;
- duplication on/off;
- neutral drift phase on/off;
- asexual versus paired inheritance.

**Metrics**

- immediate performance loss;
- within-lifetime recovery;
- generational adaptation latency;
- extinction;
- assimilation of learned behavior;
- reuse/exaptation of preexisting modules;
- diversity.

**Acceptance criteria**

A mechanism is beneficial if it improves median recovery time by at least 20% over the simpler control across two shift classes, without reducing static-environment performance by more than 10% or increasing extinction. Separate lifetime learning from genetic adaptation.

### Experiment 16 — Plasticity and neuromodulation

**Question:** Do evolvable learning rules improve embodied adaptation without destabilizing replay or creating inherited-state leakage?

**Treatments**

- fixed neural weights;
- fixed local plasticity rule;
- evolvable plasticity coefficients;
- evolvable rule selection;
- neuromodulated plasticity;
- inherited learned weights as a deliberately separate Lamarckian treatment.

**Metrics**

- performance before and after learning;
- learning speed;
- weight saturation;
- energy/computation cost;
- environmental-shift response;
- genetic assimilation;
- replay equality;
- offspring initial-state equality;
- extinction.

**Acceptance criteria**

- exact replay across save/restore and thread counts;
- zero leakage of parental learned weights in Darwinian treatments;
- at least 15% improvement in dynamic-environment lifetime performance relative to fixed weights;
- no more than 10% static-environment penalty;
- bounded weights and state in all runs;
- Lamarckian inheritance remains off unless it independently passes.

### Experiment 17 — Capability evolution

**Question:** Can sensors, actions, signals, and object-interaction capabilities evolve without invalid interfaces or free unphysical powers?

**Treatments**

- fixed capability set;
- evolvable presence/absence;
- evolvable bounded parameters;
- module duplication;
- capability cost on/off where physical costs exist;
- object-rich and object-poor environments.

**Metrics**

- valid capability mutations;
- orphan-port count;
- capability diversity;
- runtime/energy cost;
- behavior and niche innovation;
- persistent structural innovation;
- unused-capability bloat.

**Acceptance criteria**

- zero accepted orphan or unregistered capabilities;
- every action and sensor maps to a physics contract;
- capability evolution produces at least one repeatable persistent behavioral or ecological innovation in the enabling environment;
- unused-capability fraction remains below the genome-bloat gate or shows later exaptation;
- no capability provides information unavailable through world physics.

### Experiment 18 — Developmental and indirect encodings

**Question:** Do bounded generators provide useful regularity and scale beyond the direct modular baseline?

**Treatments**

- direct TMGG;
- TMGG plus CPPN module;
- tagged module grammar;
- bounded regulatory-developmental module;
- matched phenotype-size and matched compute budgets.

**Metrics**

- genotype bytes;
- phenotype size;
- compile time;
- mutation locality;
- structural validity;
- regularity;
- behavioral performance;
- innovation survival;
- debugging/provenance completeness;
- bloat and expansion failure.

**Acceptance criteria**

An indirect encoding earns inclusion if it achieves at least a 2× phenotype-to-genotype compression or a 20% adaptation advantage on repeated/geometric tasks, while:

- development completes within budget in at least 99.9% of accepted births;
- compile cost remains within the declared runtime budget;
- every generated element has provenance;
- irregular-task performance is not reduced by more than 15%;
- replay is byte-identical.

### Experiment 19 — Error-threshold and mutation-selection map

**Question:** Where does inheritance fidelity collapse for genomes of different sizes and encodings?

**Treatments**

- multiple per-genome and per-locus mutation rates;
- multiple genome sizes;
- direct, modular, and developmental representations;
- small and large populations;
- clonal and recombining reproduction.

**Metrics**

- viable offspring;
- mean performance;
- mutational load;
- lineage extinction;
- active information retention;
- genome size;
- survival-of-the-flattest signatures;
- diversity.

**Acceptance criteria**

This experiment maps a safe operating envelope rather than seeking one outcome. The production default should sit below the empirically observed collapse region with at least a 2× margin in the primary mutation-load measure, unless high-mutation dynamics are the experiment's target.

### Experiment 20 — Drift, population size, and reproductive skew

**Question:** How do small populations and unequal reproduction alter genome architecture?

**Treatments**

- multiple census sizes;
- controlled reproductive variance;
- spatial versus well-mixed;
- clonal versus paired;
- robustness mechanisms on/off.

**Metrics**

- effective contributor count;
- fixation of mildly deleterious mutations;
- drift robustness;
- diversity;
- genome size;
- extinction;
- mutational-neighborhood distribution.

**Acceptance criteria**

The implementation passes if exact pedigree-derived measures agree with simulated neutral expectations in simple benchmark worlds. Scientific results should reproduce the expected qualitative increase in drift with lower effective contributor count; deviations require explanation, not forced fitting.

### Experiment 21 — Lineage correctness and query performance

**Question:** Are exact ancestry, IBD, and common-ancestor queries correct and scalable?

**Treatments / comparisons**

- append-only exact graph;
- brute-force reference on small populations;
- run-compressed contribution storage;
- snapshot intervals;
- optional hereditary stratigraphy approximation;
- asexual and biparental modes.

**Metrics**

- parentage errors;
- locus-origin errors;
- MRCA query correctness;
- IBD correctness;
- storage bytes per birth;
- query latency;
- reconstruction depth;
- approximation error.

**Acceptance criteria**

- exact methods match brute force for every small test;
- zero broken provenance edges;
- genome reconstruction checksum always matches content ID;
- routine birth overhead remains under 5% of total simulation time in representative workloads, or a separately budgeted limit;
- approximate methods are clearly labeled and never overwrite exact truth.

### Experiment 22 — Serial/parallel determinism

**Question:** Does scheduling affect genetics or lineages?

**Treatments**

- one thread;
- all supported thread counts;
- randomized worker scheduling;
- reordered internal container insertion;
- different allocator/layout stress;
- save/restore at reproduction phase boundaries.

**Metrics**

- world-state checksum;
- event-chain checksum;
- organism IDs;
- genome and phenotype IDs;
- mutation and innovation IDs;
- lineage rows;
- RNG test vectors.

**Acceptance criteria**

Byte-identical equality for every compared checkpoint and event log. Any mismatch is a release-blocking defect.

## 14.3 Cross-experiment dashboards

Maintain a standard genetics dashboard with:

- population and effective contributor count;
- births, failed births, and death causes;
- genome and phenotype size distributions;
- mutation attempts/applied/rejected by operator;
- offspring validity and viability;
- exact lineage diversity;
- module-family diversity;
- active/disabled locus ratio;
- crossover contribution fractions;
- homologous coverage;
- innovation birth and survival curves;
- extinction and bottleneck indicators;
- replay/checksum status;
- compute and storage overhead.

## 14.4 Avoiding misleading conclusions

- A higher mean fitness among survivors can hide higher extinction.
- More genome diversity may be neutral bloat.
- More modules may be arbitrary partitioning.
- More robust genomes may generate fewer useful phenotypes.
- Faster adaptation may come from more computation per organism.
- “Species count” depends on the definition.
- A lineage radiation may be transient.
- Crossover may appear beneficial because mate choice differs.
- Plasticity may hide poor inherited controllers.
- A developmental encoding may win only because its phenotype budget is larger.
- An evolved mutation rate may maximize short-term lineage survival while reducing long-term adaptation.
- A founder population may recover performance while never recovering diversity.

All reported claims should include the relevant denominator, time horizon, extinction handling, and policy version.

---

# 15. Property-based test candidates

## 15.1 Canonical encoding

1. **Round-trip idempotence:** parse, canonicalize, serialize, parse, and serialize produces identical bytes.
2. **Permutation invariance:** arbitrary input record order produces identical canonical bytes.
3. **No duplicate keys:** duplicate locus, module, segment, or port IDs are rejected.
4. **Optional-field canonicality:** absent and default are not encoded in two semantically equivalent ways unless explicitly permitted.
5. **Fixed-point canonicality:** every numeric bit pattern round-trips; invalid encodings are rejected.
6. **Checksum domain separation:** a genome, phenotype, event, and artifact with the same payload cannot share a cross-domain hash input.
7. **Cache exclusion:** adding, removing, or reordering runtime indexes does not change content IDs.
8. **Semantic-bundle commitment:** changing any interpreting registry or policy changes the semantic-bundle ID.

## 15.2 Reference closure and typing

9. Every edge source and target port exists.
10. Every port belongs to exactly one declared owner unless shared ownership is an explicit type.
11. Every module belongs to one segment.
12. Every internal locus belongs to exactly one module unless a shared-gene mechanism is explicitly introduced.
13. Every plasticity target resolves to compatible edges.
14. Every modulator receptor resolves to an existing channel type.
15. Every capability kind exists in the referenced registry.
16. Every action and observation port has a compatible physical type.
17. No forbidden recurrence exists.
18. Required compiler roots exist.
19. Developmental emissions satisfy the same reference and type invariants as direct genes.

## 15.3 Historical identity

20. Every new locus has exactly one origin innovation.
21. Every inherited locus existed in a parent with the same historical ID.
22. Every duplicate has a new locus ID.
23. Every duplicate records its immediate source and family.
24. Independent structurally identical insertions have distinct locus and innovation IDs.
25. Structural signatures can match while ancestry IDs differ.
26. Allelic parameter mutation preserves `LocusId`.
27. Semantic replacement creates a new locus unless an explicit migration preserves identity.
28. Deleted loci never reappear with the same ID except by inheritance from a parent that retained them.
29. A locus ID is never used for a different gene kind.

## 15.4 Recombination

30. **Parent-swap symmetry:** role-free reproduction is invariant to call-site parent order.
31. **Contribution accounting:** every child locus is attributed to parent 0, parent 1, or a recorded mutation.
32. **Module-assortment integrity:** whole-module mode never mixes internal loci.
33. **Homolog alignment:** exact matching occurs only on equal `LocusId`.
34. **Analogical alignment labeling:** structural fallback, when enabled, records `ANALOGICAL`.
35. **Breakpoint legality:** every breakpoint is in the canonical valid boundary set.
36. **No dangling child references:** crossover cannot create unresolved ports or gene references.
37. **Budget compliance:** crossover never silently truncates.
38. **Deterministic fallback:** over-limit or incompatible crossover always chooses the same declared outcome.
39. **No parent contribution fabrication:** a recorded two-parent contribution equals actual inherited loci.
40. **Semantic-role isolation:** role fields do not affect inheritance unless the policy explicitly references them.

## 15.5 Mutation

41. **Replay exactness:** applying the event record to the pre-genome yields the post-genome checksum.
42. **Rejected immutability:** rejected mutation leaves bytes unchanged.
43. **Candidate determinism:** reconstructed candidate-set checksum matches.
44. **Stream isolation:** adding draws to one named stream does not alter other streams.
45. **Operator isolation:** disabling one operator changes only operator selection according to the versioned weight normalization.
46. **Split-edge closure:** exactly one old edge is disabled and the declared nodes/edges are created.
47. **Node-delete closure:** all incident edges are handled; unrelated edges are unchanged.
48. **Capability-delete closure:** owned ports and dependent references are handled.
49. **Module-copy freshness:** copied IDs are all new; no mutable aliasing exists.
50. **Numeric range:** every accepted numeric mutation lies inside its contract.
51. **No NaN-like states:** fixed-point and enumerated values cover all accepted states.
52. **Mutation ordinal uniqueness:** ordinals are contiguous within an event.
53. **No silent no-op:** an unchanged result has an explicit no-op/rejection reason.
54. **Current-event isolation:** mutation-rate mutations affect future events only.
55. **Hard-limit atomicity:** any over-limit transaction leaves the pre-genome intact.

## 15.6 Development and phenotype

56. Same genome and semantic bundle compile to identical phenotype bytes.
57. Development terminates within declared steps.
58. Overflow has one explicit failure outcome.
59. Generated elements have provenance.
60. Rule-conflict outcome is invariant to input container order.
61. Compiled local indices are deterministic.
62. Phenotype checksum changes when executable semantics change.
63. Lifetime state is initialized exactly from phenotype plus organism/event context.
64. Learned state is absent from offspring genotype in Darwinian mode.
65. Save/restore before and after a learning update yields identical next state.

## 15.7 IDs and parallelism

66. Event-derived IDs are identical across thread counts.
67. Independent worlds with different `WorldLineageId`s cannot generate equal IDs for corresponding events under the scheme.
68. Founder IDs are stable.
69. Policy fork produces a new world-lineage ID.
70. Hash collision detection fails closed.
71. No process-global counter affects IDs.
72. Reordered worker completion leaves event order and checksums unchanged.
73. Preassigned target-slot conflicts resolve identically.
74. RNG test vectors match on every supported platform.
75. Save/restore does not reset or shift draw indices.

## 15.8 Versioning and migration

76. Unknown required schema rejects.
77. Unknown registry ID rejects.
78. Migration is deterministic across thread counts.
79. Migration preserves declared loci and remaps replaced loci correctly.
80. Migration source remains unchanged.
81. Repeated migration yields the same destination or explicit already-migrated refusal.
82. Old and new replay lineages are distinct.
83. Equivalent-phenotype migration passes only the declared equivalence test.
84. Corrupted event chain rejects.
85. Unsupported exact replay is reported, not approximated.

## 15.9 Lineage

86. Parent edges point backward in time.
87. Organism pedigree is acyclic.
88. Every nonfounder organism has one reproduction event.
89. Every reproduction event's parents existed and were eligible at the event snapshot.
90. Genome transmission points to the child's genome content.
91. Locus origin precedes every transmission.
92. Brute-force MRCA matches optimized query on small DAGs.
93. Realized IBD equals direct locus comparison on small genomes.
94. Content-deduplicated genomes retain distinct transmission records.
95. Extinct clade queries agree with extant-descendant scans.
96. Founder contribution sums to the declared comparable genome weight.
97. Snapshot-plus-delta reconstruction matches content checksum.
98. Pruning indexes never delete source-of-truth events.
99. Observer cluster records cannot be resolved through simulation APIs.
100. Artifact lineage IDs cannot be mistaken for genetic locus IDs.

## 15.10 Metamorphic tests

Useful transformations with expected invariant outcomes:

- permute record input order;
- swap role-free parent arguments;
- change thread count;
- rebuild all caches;
- serialize and reload at each reproduction subphase;
- duplicate a genome blob reference without changing content;
- replace hash-map implementations;
- vary allocator behavior;
- migrate a batch in different chunk orders;
- execute query indexes from scratch versus incremental updates.

Any change to causal output under these transformations is a defect unless the policy explicitly declares the transformation semantic.

---

# 16. Performance and storage implications

## 16.1 Population scale

For thousands of simultaneously reproducing organisms, the primary costs are:

- phenotype execution every tick;
- phenotype compilation after new genomes;
- mutation candidate enumeration;
- recombination alignment;
- canonical serialization and hashing;
- lineage/event writes;
- save checkpoints;
- offline ancestry queries.

The genome representation should optimize execution separately from historical storage.

## 16.2 Dual representation

Use:

1. **Canonical historical genome:** stable IDs, provenance, typed records.
2. **Compiled execution phenotype:** dense arrays and compact local indices.

Example compiled edge:

```rust
struct RuntimeEdge {
    source_index: u32,
    target_index: u32,
    weight: Fixed32,
    plasticity_index: u32_or_sentinel,
    flags: u16,
}
```

Historical 128- or 256-bit IDs need not be read during every neural update.

## 16.3 Complexity

Approximate costs:

| Operation | Expected cost |
|---|---:|
| Numeric mutation | \(O(1)\) after candidate indexing |
| Node split | \(O(1)\) plus local index updates |
| Node deletion | \(O(\deg v)\) plus validation |
| Edge insertion candidate enumeration | \(O(N^2)\) naïve; lower with typed pair indexes |
| Module duplication | \(O(\lvert M\rvert)\) |
| Module deletion | \(O(\lvert M\rvert + dependencies)\) |
| Homologous crossover | \(O(G_0 + G_1)\) over sorted loci |
| Module assortment | \(O(M_0 + M_1)\) |
| Canonical serialization | \(O(G)\) if already sorted; \(O(G \log G)\) otherwise |
| Direct phenotype compile | \(O(N + E)\) |
| Bounded development | \(O(steps + emitted\ structure)\) |
| Birth lineage append | \(O(1)\) plus contribution summary |
| Exact DAG ancestry query | Potentially large; use offline indexes/caches |

## 16.4 Candidate indexes

Derived indexes can include:

- nodes by module and port type;
- valid source/target type classes;
- deletable nodes and edges;
- duplicable modules;
- numeric fields by mutation contract;
- exact homolog maps;
- external module dependencies.

Indexes are rebuilt deterministically and excluded from checksums. Their iteration API returns canonical sorted keys.

## 16.5 Genome memory model

Stable IDs dominate naïve record size. A serialized edge with 128-bit locus, source, target, and module IDs can require tens of bytes before parameters. Runtime Rust structs can be larger due to alignment.

Mitigations:

- canonical blob uses compact integer coding or per-genome ID dictionaries;
- execution phenotype remaps IDs to local 32-bit indices;
- repeated metadata lives in module tables;
- content-address identical genomes;
- store event deltas;
- compress cold canonical blobs;
- retain full hashes in index tables while blobs use local references.

Do not reduce historical ID width without collision analysis and an explicit scheme version.

## 16.6 Content-addressed deduplication

Identical genomes can occur through:

- clonal reproduction with no applied mutation;
- rejected mutation;
- convergent content;
- migration equivalence.

Store one canonical blob per `GenomeContentId`. Distinct organisms keep separate transmission and lifetime state.

A compiled phenotype cache keyed by `(GenomeContentId, SemanticBundleId)` can eliminate repeated compilation.

## 16.7 Delta storage

A child genome can be represented as:

- parent content references;
- inheritance contribution map;
- mutation event list;
- resulting content checksum.

For routine loading, periodically materialize a canonical snapshot. Use a deterministic snapshot rule based on delta depth or content size, but snapshot timing must not affect causal state or IDs.

## 16.8 Lineage storage growth

Per birth, fixed rows include:

- organism;
- reproduction event;
- one or two parent edges;
- genome transmission;
- zero or more mutation events.

Per-locus transmission is potentially expensive. Prefer:

- module contribution records;
- sorted locus-run records;
- mutation overlays;
- explicit records for rearrangements.

Full detailed provenance can be cold-stored asynchronously after deterministic event generation. If write timing differs, canonical event ordering and content must not.

## 16.9 Query indexes

Offline indexes may include:

- organism-to-parent adjacency;
- parent-to-child adjacency;
- generation/depth bounds;
- extant-descendant counts;
- locus-origin maps;
- module-family trees;
- ancestor sketches;
- deme-founder maps;
- event tick ranges.

Indexes are disposable derivatives. Source event tables and checksums are authoritative.

## 16.10 Developmental encoding cost

A compact genome can emit a large phenotype. Therefore budget both:

- genotype size;
- phenotype size;
- development work;
- runtime work.

A compression ratio is not free performance. An indirect encoding may reduce storage while increasing compile and execution cost.

## 16.11 Mutation-rate and genome-size coupling

If each locus mutates independently, mutation enumeration can become \(O(G)\) per birth. Alternatives:

- sample event count from a per-genome distribution;
- sample target loci from indexed classes;
- use mathematically specified skip sampling for per-locus Bernoulli exposure;
- mix per-genome structural events and per-locus numeric exposure.

The selected method changes evolutionary dynamics and must be a policy, not only an optimization.

## 16.12 Parallelism

Parallelize private work:

- child genome construction;
- validation;
- phenotype compilation;
- hashing;
- compression.

Serialize or deterministically reduce:

- intent conflict resolution;
- event ordering;
- world-slot commit;
- phase checksum;
- lineage log order.

## 16.13 Performance budgets

Track:

- nanoseconds or cycles per neural edge update;
- phenotype compile time;
- mutation time by operator;
- crossover time by genome size/divergence;
- bytes per genome content;
- bytes per birth event;
- lineage query latency;
- checkpoint size and write time;
- cache hit rates;
- percentage of total simulation time spent in genetics.

**[J]** A provisional target is for birth-time genetics and exact lineage writing to consume less than 5–10% of representative simulation time. This is a budget, not a scientific requirement; if organism cognition dominates, a higher fraction may be acceptable.

## 16.14 Storage retention policy

Recommended tiers:

- hot: extant organisms, recent ancestors, active genome blobs, indexes;
- warm: recent event blocks and snapshots;
- cold immutable: full historical event blocks, old genome blobs, policy bundles;
- observer: separate analytical outputs and derived clusters.

Never delete exact history merely because an observer index can approximate it unless the project explicitly changes its research requirement.

---

# 17. Failure modes and risks

## 17.1 Representation failures

### Raw positional homology

**Failure:** insertion shifts positions; crossover aligns unrelated genes.  
**Mitigation:** stable locus IDs, segment/module hierarchy, explicit analogical fallback.

### Identifier overloading

**Failure:** one “innovation number” is used for ancestry, function, equality, and module family.  
**Mitigation:** separate `LocusId`, `InnovationId`, `GeneFamilyId`, and `StructuralSignature`.

### Flat-graph pleiotropy

**Failure:** arbitrary cross-links make every mutation and crossover global.  
**Mitigation:** typed module interfaces, adapters, cross-module metrics.

### Opaque indirect encoding

**Failure:** small gene changes rewrite most of the phenotype; ancestry cannot explain behavior.  
**Mitigation:** bounded generators, generated-element provenance, direct baseline.

## 17.2 Recombination failures

### Structure shredding

**Failure:** uniform or positional crossover breaks coadapted modules.  
**Mitigation:** module assortment, historical alignment, destructiveness experiments.

### False homology

**Failure:** convergent structures receive the same historical ID.  
**Mitigation:** event-derived origin IDs; structural signature separate.

### Frozen linkage blocks

**Failure:** modules become too large to recombine, blocking useful allele exchange.  
**Mitigation:** measured module fission/fusion or intramodule homologous crossover experiments.

### Hybrid parser failures

**Failure:** “hybrids” fail because references are malformed.  
**Mitigation:** construct-valid crossover; represent nonviability as a valid outcome.

### Parent-order bias

**Failure:** parent A contributes more because of call order.  
**Mitigation:** canonical parent ordering and swap-symmetry tests.

## 17.3 Mutation failures

### Ambiguous repair bias

**Failure:** repair chooses an arbitrary closest port or first valid edge.  
**Mitigation:** reject ambiguous mutations; canonical valid constructors.

### Deletion cascades

**Failure:** recursive cleanup removes unrelated structure.  
**Mitigation:** narrow ownership closure; separate garbage-collection operators.

### Duplication aliasing

**Failure:** duplicated modules share IDs or mutable state.  
**Mitigation:** deep copy, new IDs, property tests.

### Capability leakage

**Failure:** a sensor exposes global state or an actuator bypasses physics.  
**Mitigation:** immutable physics capability registry and bounded parameters.

### Mutation-rate runaway or lock-in

**Failure:** rates hit zero or destructive maxima.  
**Mitigation:** bounds, future-only effect, fixed-rate controls, experimental gate.

### Distribution drift

**Failure:** library update changes random-normal sampling.  
**Mitigation:** frozen integer distributions and test vectors.

## 17.4 Genome bloat failures

- disabled loci accumulate indefinitely;
- duplication outpaces deletion;
- developmental phenotype grows while genotype remains small;
- crossover creates very large programs;
- hard caps cause repeated rejected births;
- size becomes a proxy for access to more compute.

Mitigations:

- separate genotype, phenotype, and runtime budgets;
- balanced operators;
- retention age;
- size-fair crossover;
- explicit computational/physical costs;
- active/total metrics;
- no silent pruning.

## 17.5 Developmental and regulatory failures

### Nontermination or explosion

Use hard step and output limits; fail explicitly.

### Ordering dependence

Specify synchronous/asynchronous phases and canonical conflict resolution.

### Semantic migration drift

Bundle immutable grammar/rule registries with old saves.

### Hidden numeric instability

Use fixed point, bounded state, and saturation tests.

### Untraceable phenotype

Store provenance from every emitted element to generator loci and steps.

## 17.6 Robustness and evolvability failures

### Robust but inert genomes

High neutral fraction may mean mutation cannot change behavior. Measure beneficial and phenotype-novel fractions.

### Innovation protection freezes mediocrity

Protection can preserve weak structures and bloat. Compare eventual descendant contribution and cost.

### Neutral bloat mistaken for evolvability

Require post-shift benefit.

### Short-horizon selection of low mutation rates

Compare evolvable rates with empirically best fixed rates [CLUNE08].

## 17.7 Species and lineage failures

### Observer feedback leak

A cluster label accidentally gates mating or selection. Enforce API and storage separation.

### Similarity mistaken for ancestry

Use exact locus and parent records.

### One-parent projection treated as true phylogeny

Biparental ancestry is a DAG. Label any projection.

### Tag similarity treated as kinship

Tags are observable cues and may be deceptive.

### Global species thresholds fragment populations

Track eligible mates and extinction; use ecological locality where possible.

### Lineage storage explosion

Use content addressing, run compression, snapshots, and cold storage without deleting source events.

## 17.8 Determinism failures

- shared global innovation counter;
- atomic organism ID increment;
- hash-map candidate order;
- unstable sorting ties;
- thread-dependent commit order;
- floating-point mutation distributions;
- platform-specific activation behavior;
- variable hidden RNG consumption;
- retries without bounded ordinal;
- cache state included in checksum;
- policy default changes;
- old genome silently recompiled under new semantics.

Every one is release-blocking for exact replay.

## 17.9 Save and migration failures

- unsupported old registry;
- missing migration provenance;
- in-place save rewrite;
- partial migration;
- silently changed hard limits;
- copied old IDs assigned new meaning;
- checksums not recomputed;
- replay lineage not forked.

Use fail-closed loading and immutable source saves.

## 17.10 Scientific interpretation failures

- selecting runs after extinction;
- reporting mean fitness without compute cost;
- counting transient mutants as innovations;
- calling clusters species without criterion;
- calling duplicates new functions;
- calling learned behavior genetic adaptation;
- treating genome length as complexity;
- assuming crossover caused a difference when mate choice changed;
- comparing encodings with unequal phenotype budgets;
- using offline labels as organism knowledge.

---

# 18. Claims not supported by current evidence

The following claims should not appear in design documentation as facts:

1. **Sexual reproduction is generally superior to asexual reproduction.** Benefits depend on ecology, epistasis, mutation, population size, and cost.
2. **Paired reproduction should always use crossover.** Pairing, biparental contribution, assortment, and crossover are separable.
3. **Uniform crossover is a neutral default.** It imposes strong assumptions about linkage and epistasis.
4. **Historical innovation IDs alone solve all homology.** Duplication, deletion, convergence, and module rearrangement need richer provenance.
5. **Structurally identical genes share ancestry.** Convergence can produce analogy without homology.
6. **Explicit modules guarantee functional modularity.** They guarantee data boundaries, not evolutionary function.
7. **Modularity always increases evolvability.** It can constrain solutions or impose performance cost.
8. **Gene duplication usually yields innovation.** Many duplicates are deleted, silenced, or remain redundant.
9. **Neutral variation is always useful.** It may only create bloat and mutation load.
10. **Robustness always promotes evolvability.** Effects depend on horizon, environment, mutation, and population.
11. **More robust genomes are necessarily fitter.** Robustness can favor flatter but lower peaks.
12. **Evolvable mutation rates optimize long-term adaptation.** Rugged landscapes can select suboptimal rates [CLUNE08].
13. **Higher mutation rates create more useful novelty.** They also increase deleterious load and extinction.
14. **A single theoretical error threshold applies to Genesis.** The engine's variable graph genotype violates many simple sequence-model assumptions.
15. **Larger genomes or phenotypes are more complex.** Bloat can be nonfunctional.
16. **Indirect encodings are more scalable in every domain.** Their bias helps regular tasks and can harm irregular ones.
17. **Developmental programs automatically create canalization.** They can be highly sensitive or unstable.
18. **Regulatory networks are more biologically realistic and therefore better.** Biological fidelity is not the objective.
19. **Artificial chemistry is necessary for open-ended evolution.** It is one research substrate, not an established requirement.
20. **NEAT-like speciation is a natural species model.** It is primarily an optimization/protection mechanism.
21. **A genetic-distance cluster is a species.** It is one observer partition.
22. **Behavioral similarity implies genetic similarity.** Different controllers can converge.
23. **Visible tags provide exact kin recognition.** Tags can mutate and be exploited.
24. **Exact pedigree relatedness should be available to organisms.** That grants an omniscient sensor unless physically modeled.
25. **Hybrid compile failures are meaningful reproductive isolation.** They are usually representation defects.
26. **A lineage tree is exact under paired reproduction.** Exact pedigree is a DAG.
27. **Tree-sequence compression directly applies to arbitrary graph genomes.** It requires adaptation to stable segment coordinates.
28. **Hereditary stratigraphy should replace exact lineage tracking.** It is an approximation useful when exact centralized records are impractical.
29. **A schema migration can preserve replay without a lineage fork.** Changed causal semantics produce a new replay branch.
30. **An old genome can be safely loaded using current defaults.** Silent reinterpretation violates determinism.
31. **Open-ended technological or cultural evolution will emerge once the genome is expressive.** The genome is only one prerequisite; ecology, embodiment, objects, learning, population structure, and selection pressures matter.
32. **Major evolutionary transitions can be induced by a genome flag.** They concern changes in individuality, inheritance, and conflict mediation and must be detected from dynamics.

---

# 19. Open questions

## 19.1 Baseline architecture

1. What are the engine's realistic per-organism node, edge, module, and capability budgets on target hardware?
2. Should chromosome-like segments be present in TMGG-1 or introduced only after module assortment is validated?
3. Should disabled loci be retained indefinitely, for a fixed age, or under a size-dependent budget?
4. Are module interfaces primarily neural ports, capability bindings, or a more general typed signal system?
5. Does the engine need cross-module recurrent edges in the first version?

## 19.2 Homology and IDs

6. Is exact historical `LocusId` sufficient for baseline crossover, or is structural fallback needed for independently convergent modules?
7. How should module fission and fusion map old loci to new module identities?
8. Should a copied module retain one family ID indefinitely, or should deeply diverged copies start subfamilies?
9. Is a 128-bit event-derived ID sufficient under the maximum projected birth count, or should all causal IDs use 256 bits?
10. Should compact local IDs be persisted inside genome blobs or regenerated at compile time?

## 19.3 Reproduction

11. Does current paired reproduction require both parents to contribute genes to every offspring?
12. What physical or energetic cost distinguishes paired reproduction from cloning?
13. Is mate choice controller-mediated, world-policy-mediated, or both?
14. Can organisms produce broods, and if so should recombination share a meiosis-like plan across siblings?
15. Should recombination modes be fixed per world, heritable, or chosen behaviorally?
16. What level of divergence should block homologous crossover versus merely reduce hybrid performance?
17. Is non-homologous rearrangement necessary before object and morphology evolution exist?

## 19.4 Mutation

18. What fixed-point mutation step distributions produce useful locality in the current neural controller?
19. Should node insertion attempt function preservation, neutral disabled insertion, or random activation?
20. Which capabilities are structurally mandatory for birth, if any?
21. How should deletion treat reproductive capabilities: allow sterile organisms, reject the mutation, or rely on phenotype viability?
22. Should mutation count scale per genome, per locus, or by operator family?
23. What disabled-gene retention policy best balances homology and bloat?
24. Should module duplication copy external bindings or begin unbound?
25. What physical costs, if any, justify capability and network size costs?

## 19.5 Plasticity and development

26. Which plasticity-rule family is sufficiently expressive but bounded for the first implementation?
27. Are neuromodulator channels global, module-scoped, spatial, or typed?
28. Does any stochastic learning remain after adopting fixed-point deterministic controllers?
29. Should learned weights ever influence reproduction indirectly through parental behavior only, or also through explicit inherited initialization experiments?
30. Which geometry exists that would make CPPN or HyperNEAT-style generators meaningful?
31. What developmental provenance granularity is affordable?
32. Should a development failure produce no organism, a nonviable organism record, or a valid inert phenotype?
33. Can developmental timing interact with the environment before birth, and if so how is that state persisted?

## 19.6 Species and populations

34. Is any explicit reproductive-compatibility filter scientifically necessary, or can physical locality and behavior produce sufficient divergence?
35. What relatedness cues can organisms physically observe?
36. How should effective population size be estimated in overlapping-generation spatial populations?
37. What constitutes an ecological niche in the planned world state?
38. What persistence horizon distinguishes radiation from transient diversification?
39. Should innovation protection ever exist in the main world, or only in controlled evolutionary-search modes?
40. How will population bottlenecks arise naturally, and which artificial bottlenecks are needed for experiments?

## 19.7 Lineage and storage

41. What ancestry retention horizon and storage budget are acceptable?
42. Is full per-locus transmission needed online, or can it be reconstructed from module runs and mutation events?
43. Which exact common-ancestor queries must be interactive rather than offline?
44. How will lineage data be partitioned across save files and observer databases?
45. Should artifact lineage share the event-chain infrastructure with genetic lineage?
46. What happens when organisms or artifacts move between separately simulated worlds?
47. Is hereditary stratigraphy useful for exported organisms or distributed simulation shards?

## 19.8 Versioning

48. How long will exact executable support for old semantic bundles be retained?
49. Which changes require migration versus permanent legacy interpretation?
50. Should limit-profile changes always fork the world lineage?
51. How are policy bundles packaged so old saves remain self-describing without embedding executable code?
52. What is the governance process for assigning registry IDs?
53. How will deterministic test vectors be preserved across Rust/compiler/toolchain updates?

## 19.9 Scientific uncertainty

54. Which representation yields the best tradeoff between mutation locality and structural innovation in the actual Genesis ecology?
55. Does module duplication produce useful novelty before object interaction and morphology are rich?
56. Does crossover select for more recombination-tolerant architectures in this system?
57. Does lifetime plasticity accelerate or delay inherited controller evolution?
58. Does neutral retention improve adaptation after realistic environmental shifts?
59. Do evolvable mutation rates settle below long-term optima as in Avida?
60. Can reproductive divergence emerge without an explicit compatibility threshold?
61. Which apparent “species” classifications remain stable across metrics?
62. Will a more expressive genome create persistent innovation, or only a larger neutral state space?

These questions should be answered by the experimental program rather than by adding biological mechanisms preemptively.



# 20. Annotated bibliography

## 20.1 Source-selection and interpretation notes

This bibliography prioritizes primary research papers, original system descriptions, and foundational theoretical work. Review articles were used only for orientation and are not substituted for the primary sources below. The annotations distinguish three questions:

1. **What the source actually establishes.**
2. **What engineering lesson can reasonably be transferred to The Genesis Engine.**
3. **What the source does not establish.**

A digital-evolution result is evidence about the particular representation, mutation process, population regime, task ecology, and selection mechanism used in that study. It is not automatically a law of all artificial-life systems. Likewise, results from molecular population genetics supply conceptual and mathematical warnings, but their numerical thresholds should not be copied into a digital genome without calibration. The recommended baseline in this review is therefore an engineering synthesis, not a direct implementation of any one publication.

DOI identifiers are supplied where available. Several foundational conference chapters and books have no DOI.

---

## 20.2 Direct, indirect, modular, and developmental encodings

### [NEAT02] Stanley, K. O., and Miikkulainen, R. (2002). “Evolving Neural Networks through Augmenting Topologies.” *Evolutionary Computation* 10(2): 99–127. doi:10.1162/106365602320169811.

**Contribution.** Introduces NEAT’s direct variable-topology encoding, historical markings for aligning genes during crossover, incremental structural complexification, and fitness sharing through a topology-distance species mechanism. The paper includes ablations showing that its components work as an interacting package in the reported control task.

**Genesis relevance.** This is the strongest direct precedent for preserving the historical identity of structural innovations across descendants. It supports stable locus or innovation tags, homology-aware alignment, minimal initial structures, and explicit protection of new structures in controlled experiments. It also demonstrates that a variable neural topology can remain serializable and inspectable.

**Transfer limit.** NEAT’s global innovation-number allocator, fitness-sharing population manager, and fitter-parent crossover rule are algorithm-specific. A long-running deterministic world cannot copy a process-global mutable counter or assume a generation-based optimizer. Historical identity should be derived from world-local reproduction events, while observer species clusters should not become causal merely because NEAT uses them operationally.

### [STANLEY03] Stanley, K. O., and Miikkulainen, R. (2003). “A Taxonomy for Artificial Embryogeny.” *Artificial Life* 9(2): 93–130. doi:10.1162/106454603322221487.

**Contribution.** Organizes developmental and generative encodings according to features such as cell fate, spatial interaction, temporal development, and genotype-to-phenotype mapping. It clarifies that “developmental encoding” is not one mechanism but a broad design space with materially different mutation and scaling properties.

**Genesis relevance.** The taxonomy is useful for preventing an imprecise decision to “add development.” It motivates explicit choices about growth substrate, developmental steps, local context, termination, and provenance. These decisions must be versioned because they define phenotype semantics.

**Transfer limit.** The work is a conceptual taxonomy, not evidence that developmental encodings outperform direct encodings in an open-ended ecological simulation. It does not resolve deterministic execution, genome migration, save-state requirements, or safe size bounds. Those are Genesis-specific engineering obligations.

### [CPPN07] Stanley, K. O. (2007). “Compositional Pattern Producing Networks: A Novel Abstraction of Development.” *Genetic Programming and Evolvable Machines* 8: 131–162. doi:10.1007/s10710-007-9028-8.

**Contribution.** Defines CPPNs as function networks whose activation functions can generate regular spatial patterns, including symmetry, repetition, and repetition with variation. The representation compresses a potentially large phenotype into a smaller pattern-generating genotype.

**Genesis relevance.** CPPNs are a credible future option when the world has a meaningful coordinate system for morphology, sensor placement, or neural substrate geometry. They can create correlated structural variation that would require many coordinated mutations in a direct encoding. A bounded CPPN module could generate one clearly scoped phenotype region while the rest of the genome remains direct.

**Transfer limit.** CPPNs introduce global and nonlocal mutation effects: one generator change can alter many phenotype elements. They also require a stable query geometry and thresholding semantics. The paper does not show that CPPNs are superior for arbitrary behavioral controllers, capability sets, or long-running lineage reconstruction. They should not be the baseline before the relevant geometry exists.

### [HYPERNEAT09] Stanley, K. O., D’Ambrosio, D. B., and Gauci, J. (2009). “A Hypercube-Based Encoding for Evolving Large-Scale Neural Networks.” *Artificial Life* 15(2): 185–212. doi:10.1162/artl.2009.15.2.15202.

**Contribution.** Introduces HyperNEAT, which queries a CPPN with geometric coordinates to generate connection patterns over a neural substrate. It demonstrates how regularity and geometry can scale an indirect neural encoding beyond one-gene-per-weight representations.

**Genesis relevance.** The work supports an optional phenotype generator for spatially embedded controller regions, especially where repeated sensors, body plans, or topographic neural maps make geometric regularity useful. It also illustrates that the substrate definition is part of the effective genome semantics even when it is not encoded in the genome.

**Transfer limit.** HyperNEAT assumes experimenter-specified substrate coordinates and generally does not make all neuron placement decisions evolvable. It is not evidence that indirect encoding will improve open-ended evolution in a mutable ecology. A Genesis implementation would need deterministic coordinate enumeration, fixed-point activation, bounded output size, and an auditable generated-to-source provenance map.

### [ESHYPER12] Risi, S., and Stanley, K. O. (2012). “An Enhanced Hypercube-Based Encoding for Evolving the Placement, Density, and Connectivity of Neurons.” *Artificial Life* 18(4): 331–363. doi:10.1162/ARTL_a_00071.

**Contribution.** Extends HyperNEAT by deriving neuron placement and varying density from the encoded pattern rather than requiring a completely fixed substrate. The reported experiments show that an indirect encoding can elaborate neural geometry and exploit geometric regularity.

**Genesis relevance.** ES-HyperNEAT is a credible advanced alternative if direct neural graphs become a demonstrated scaling bottleneck. It suggests a way to evolve repeated or spatially organized networks without directly storing every node and edge.

**Transfer limit.** Its phenotype extraction algorithm, thresholds, quadtree-like subdivision, and geometry are substantial hidden semantics. Small parameter changes can cause large structural changes. The approach therefore increases migration, checksum, debugging, and replay obligations. It should be tested under equal phenotype-compute and phenotype-size budgets rather than assumed to be more evolvable because it is more compressed.

### [GRUAU94] Gruau, F. (1994). “Automatic Definition of Modular Neural Networks.” *Adaptive Behavior* 3(2): 151–183. doi:10.1177/105971239400300202.

**Contribution.** Presents cellular encoding and graph-grammar development of neural networks, including automatically defined reusable subnetworks. It is an early demonstration that a developmental grammar can duplicate and compose neural modules.

**Genesis relevance.** The paper supports first-class module definitions, module calls or instantiation, and duplication as structural operators. It also shows why module interfaces and expansion limits must be explicit: a compact genotype can generate a much larger controller.

**Transfer limit.** The evolved locomotion domains and grammar are bounded optimization problems. The work does not establish that grammar-derived modules remain stable under sexual recombination or long-term neutral drift. For Genesis, a grammar should be an optional bounded module type, not an unbounded replacement for an inspectable direct genome.

### [GE98] Ryan, C., Collins, J. J., and O’Neill, M. (1998). “Grammatical Evolution: Evolving Programs for an Arbitrary Language.” In *EuroGP 1998*, Lecture Notes in Computer Science 1391: 83–96. doi:10.1007/BFb0055930.

**Contribution.** Introduces grammatical evolution, mapping a linear genotype through a formal grammar to a phenotype program. The method separates the genetic representation from the syntax of the generated artifact and can guarantee grammar-level syntactic validity.

**Genesis relevance.** Grammar-based encodings offer a way to construct only syntactically valid developmental programs or repeated structures. They also make the grammar version a clear, explicit semantic dependency.

**Transfer limit.** Syntactic validity is not semantic validity, bounded execution, or useful behavior. Codon wrapping and mapping choices can create strong biases, discontinuities, and neutral regions. A changed grammar silently changes phenotype meaning, so grammar IDs and mapping policies must be immutable within a replay lineage. Grammar-based genomes are a credible alternative only when the generated language is narrow and resource-bounded.

### [HORNBY03] Hornby, G. S., Lipson, H., and Pollack, J. B. (2003). “Generative Representations for the Automated Design of Modular Physical Robots.” *IEEE Transactions on Robotics and Automation* 19(4): 703–719. doi:10.1109/TRA.2003.814502.

**Contribution.** Compares generative representations for evolving modular robot designs and demonstrates how reusable developmental descriptions can express repeated structures and regularity.

**Genesis relevance.** The work supports module duplication and generative descriptions for morphology or construction-like repeated form. It provides a concrete reason to consider indirect encodings once the phenotype contains repeated physical structures rather than only a neural graph.

**Transfer limit.** The robot design domain, evaluation procedure, and morphology primitives are highly structured by the experimenter. The results do not show that a generative encoding is universally more evolvable. A Genesis implementation must avoid smuggling named technologies or task-specific forms into its grammar and must count generated phenotype cost, not merely genome length.

---

## 20.3 Digital organisms, genome dynamics, and long-running evolution

### [RAY91] Ray, T. S. (1991). “An Approach to the Synthesis of Life.” In C. G. Langton, C. Taylor, J. D. Farmer, and S. Rasmussen, eds., *Artificial Life II*, Santa Fe Institute Studies in the Sciences of Complexity, Proceedings Volume X, pp. 371–408. Addison-Wesley.

**Contribution.** Describes Tierra, in which self-replicating machine-code organisms mutate and compete for computational resources. Tierra demonstrated ecological interactions, parasitism, genome-length change, and evolutionary dynamics in an instruction-level digital medium.

**Genesis relevance.** Tierra is a warning and an inspiration: meaningful evolutionary novelty can arise when replication and resource competition are mechanistic, but outcomes are tightly shaped by instruction-set semantics, memory protection, mutation rules, and scheduler design. It supports preserving complete causal records rather than inferring that visually similar programs share ancestry.

**Transfer limit.** Tierra’s instruction ecology is not a direct model of embodied organisms with neural controllers. Claims of open-endedness or biological equivalence should not be imported. Its low-level self-modifying code also has a much larger invalid-state and security surface than the proposed typed Genesis genome.

### [AVIDA04] Ofria, C., and Wilke, C. O. (2004). “Avida: A Software Platform for Research in Computational Evolutionary Biology.” *Artificial Life* 10(2): 191–229. doi:10.1162/106454604773563612.

**Contribution.** Provides the primary platform description for Avida: self-replicating instruction-sequence organisms, explicit mutation, population competition, configurable environments, and detailed ancestry and phenotype analysis.

**Genesis relevance.** Avida demonstrates the scientific value of exact digital heredity, controllable mutation spectra, line-of-descent reconstruction, and large replicated experiments. It motivates storing mutation events and ancestry separately from observer classifications. Its experimental discipline is more transferable than its instruction genome.

**Transfer limit.** Avida rewards defined logic operations in many canonical experiments, whereas Genesis aims to author physics and affordances rather than named tasks. Avida’s genome and execution model should therefore inform test design, mutation accounting, and lineage analysis, not dictate the world’s phenotype language or fitness mechanism.

### [LENSKI99] Lenski, R. E., Ofria, C., Collier, T. C., and Adami, C. (1999). “Genome Complexity, Robustness and Genetic Interactions in Digital Organisms.” *Nature* 400: 661–664. doi:10.1038/23245.

**Contribution.** Uses exhaustive and sampled mutational neighborhoods of digital organisms to compare simple and more complex evolved genotypes. It finds common genetic interactions and different robustness properties under the studied selection regimes.

**Genesis relevance.** The study supports measuring pleiotropy, epistasis, lethal mutation fraction, and multi-mutation interaction directly rather than assuming additivity. It also supports storing enough genotype provenance to reconstruct the line of descent leading to complex architectures.

**Transfer limit.** Complexity and robustness were operationalized within Avida’s instruction and reward system. The study does not imply that larger Genesis genomes will be more robust, or that epistasis has one predictable sign. The relevant measures must be recomputed for Genesis phenotypes and ecologies.

### [WILKE01] Wilke, C. O., Wang, J. L., Ofria, C., Lenski, R. E., and Adami, C. (2001). “Evolution of Digital Organisms at High Mutation Rates Leads to Survival of the Flattest.” *Nature* 412: 331–333. doi:10.1038/35085569.

**Contribution.** Demonstrates in digital organisms that, at sufficiently high mutation rates, a genotype with lower peak replication performance but a more robust mutational neighborhood can outcompete a sharper, higher peak.

**Genesis relevance.** The result motivates reporting lineage-level reproductive success and mutational-neighborhood robustness separately from an individual’s immediate performance. It also provides a clear experimental template for identifying an error-load regime.

**Transfer limit.** The threshold depends on genome length, mutation spectrum, population structure, and fitness landscape. No numeric mutation rate should be copied. “Survival of the flattest” is not a justification for intentionally maximizing neutrality; it is a context-dependent population outcome that Genesis should test.

### [GUPTA16] Gupta, A., LaBar, T., Miyagi, M., and Adami, C. (2016). “Evolution of Genome Size in Asexual Digital Organisms.” *Scientific Reports* 6: 25786. doi:10.1038/srep25786.

**Contribution.** Tracks insertions, deletions, information content, and phenotypic traits in Avida across mutation-rate treatments. In that regime, lower point-mutation rates permitted genome expansion and more traits, while high mutation load favored smaller, denser genomes.

**Genesis relevance.** The study supports treating genome size as an evolved outcome of mutation pressure, insertion/deletion bias, coding density, and phenotype benefit. It motivates explicit genome-size and coding-density measurements instead of an arbitrary assumption that bloat is always neutral.

**Transfer limit.** The populations were asexual, well mixed, fixed in size, and governed by Avida’s instruction semantics. Results do not specify the correct Genesis size cost or indel distribution. They justify a factorial experiment over mutation rate, size cost, and duplication/deletion policy.

### [MISEVIC06] Misevic, D., Ofria, C., and Lenski, R. E. (2006). “Sexual Reproduction Reshapes the Genetic Architecture of Digital Organisms.” *Proceedings of the Royal Society B* 273(1585): 457–464. doi:10.1098/rspb.2005.3338.

**Contribution.** Compares 200 sexual and asexual Avida populations evolved for more than 10,000 generations. In the reported setting, sexual populations evolved genomes with greater modular organization and weaker overall epistasis.

**Genesis relevance.** This is direct evidence that recombination can change the architecture that evolves, not merely shuffle already fixed genes. It supports measuring functional overlap, locus clustering, and epistasis under crossover versus no-crossover treatments.

**Transfer limit.** The result does not establish that sex or crossover universally improves adaptation, diversity, or open-endedness. The recombination scheme, rewards, genome language, and population structure matter. Paired reproduction in Genesis should therefore be experimentally separable from crossover.

### [ANDERSON14] Anderson, C. J. R., and Harmon, L. J. (2014). “Ecological and Mutation-Order Speciation in Digital Organisms.” *The American Naturalist* 183(2): 257–268. doi:10.1086/674359.

**Contribution.** Uses digital organisms to examine divergence caused by ecological differences and by different mutation orders under controlled conditions. It demonstrates that reproductive isolation and lineage divergence can have distinct causal routes in an artificial system.

**Genesis relevance.** The study supports experiments that distinguish environment-dependent divergence, historical contingency, and explicit compatibility mechanisms. It also reinforces the need to preserve true ancestry when comparing convergent or divergent populations.

**Transfer limit.** The operational definition of reproductive isolation is tied to the study’s digital model. It does not justify installing a researcher-defined species label into organism behavior. In Genesis, compatibility must be a causal mechanism with explicit inputs; behavioral or genetic clusters remain observer analyses.

### [ELENA08] Elena, S. F., and Sanjuán, R. (2008). “The Effect of Genetic Robustness on Evolvability in Digital Organisms.” *BMC Evolutionary Biology* 8: 284. doi:10.1186/1471-2148-8-284.

**Contribution.** Compares digital genotypes with different levels of mutational robustness and evaluates adaptation after environmental change. In simpler environments, robustness could impede early adaptation yet support longer-term evolvability; results were less decisive in more complex conditions.

**Genesis relevance.** This supports separating short-horizon response, long-horizon adaptation, and mutational robustness in the experimental dashboard. A single “evolvability score” would erase the central temporal result.

**Transfer limit.** The robustness treatments and environmental changes are Avida-specific. The paper does not prove that neutral networks always improve adaptation. Genesis should preregister time horizons and report null or reversed effects rather than tuning the horizon until a desired result appears.

### [CLUNE08] Clune, J., Misevic, D., Ofria, C., Lenski, R. E., Elena, S. F., and Sanjuán, R. (2008). “Natural Selection Fails to Optimize Mutation Rates for Long-Term Adaptation on Rugged Fitness Landscapes.” *PLoS Computational Biology* 4(9): e1000187. doi:10.1371/journal.pcbi.1000187.

**Contribution.** Evolves mutation rates in digital organisms and compares them with empirically identified rates that maximize later adaptation. In rugged landscapes, evolved rates were consistently below the long-term adaptive optimum; the discrepancy diminished on a smoother landscape.

**Genesis relevance.** This is the central caution against assuming that heritable mutation rates self-tune for the experimenter’s long-term goals. It supports bounded mutation-rate genes, fixed-rate controls, direct measurement of long-term adaptation, and a distinction between organism-level selection and researcher objectives.

**Transfer limit.** The exact outcome depends on landscape structure and modifier linkage. The paper does not show that evolvable mutation rates are undesirable in every system. It shows that enabling them is a scientific intervention that requires controls, not a harmless realism feature.

### [LABAR17] LaBar, T., and Adami, C. (2017). “Evolution of Drift Robustness in Small Populations.” *Nature Communications* 8: 1012. doi:10.1038/s41467-017-01003-7.

**Contribution.** Uses a mathematical model and Avida experiments to show that small populations can occupy fitness peaks with fewer slightly deleterious mutational neighbors. The reported “drift robustness” is an outcome of which peaks small populations can maintain, rather than a straightforward adaptive optimization for robustness.

**Genesis relevance.** The study supports measuring the distribution of mutation effects, not just mean robustness, across population-size and bottleneck treatments. It also warns that population size can change the genetic architecture that persists.

**Transfer limit.** The finding requires an appropriate multi-peak landscape and does not imply that every small Genesis population will become robust. Effective population size, reproductive skew, spatial structure, and overlapping generations must be measured rather than equated with census size.

### [LANGDON19] Langdon, W. B., and Banzhaf, W. (2019). “Continuous Long-Term Evolution of Genetic Programming.” In *Proceedings of the 2019 Conference on Artificial Life*, pp. 388–395. doi:10.1162/isal_a_00191.

**Contribution.** Reports very long genetic-programming runs in which programs can grow to extremely large sizes while evolution continues. It is a strong demonstration that variable-length evolutionary systems can accumulate enormous neutral or weakly constrained structure.

**Genesis relevance.** The paper justifies hard compilation limits, explicit size telemetry, disabled-gene retention policies, and bloat ablations. It also shows why serialization success alone is not adequate genome-size control.

**Transfer limit.** Tree/program bloat in genetic programming is not identical to neural-graph or modular-genome growth. The paper establishes a failure possibility, not the expected Genesis trajectory. Size limits and costs must be calibrated against actual controller benefit and compute budget.

### [MORENO22] Moreno, M. A., and Ofria, C. (2022). “Exploring Evolved Multicellular Life Histories in an Open-Ended Digital Evolution System.” *Frontiers in Ecology and Evolution* 10: 750837. doi:10.3389/fevo.2022.750837.

**Contribution.** Reports evolved digital-cell groups with varied multicellular life histories and group-level traits, including division of labor, resource sharing, messaging-mediated behavior, and reproductive organization.

**Genesis relevance.** The study supports representing persistent group membership, signaling, and lineage at multiple levels if Genesis later permits nested reproductive individuals. It also illustrates the analytical value of exact life-history records.

**Transfer limit.** The system supplies specific group-formation and kin-recognition mechanisms. It does not show that such transitions will arise from the proposed Genesis baseline, nor that a genome should begin with a biological multicellularity apparatus. Multi-level inheritance should be added only when the world’s physics supplies a clear unit of collective persistence and reproduction.

### [WILLENSDORFER08] Willensdorfer, M. (2008). “Organism Size Promotes the Evolution of Specialized Cells in Multicellular Digital Organisms.” *Journal of Evolutionary Biology* 21(1): 104–110. doi:10.1111/j.1420-9101.2007.01466.x.

**Contribution.** In a digital multicellular model, larger undifferentiated organisms more readily evolved specialized cells because the temporary cost of an unsuccessful specialized component was diluted across the larger organism.

**Genesis relevance.** This supplies a mechanistic example of how redundancy or scale can open evolutionary paths by reducing the cost of intermediates. It supports experiments on module duplication, partial specialization, and component cost.

**Transfer limit.** The cell types and logic-function rewards were designed into the model. The result is not evidence that larger genomes or organisms are intrinsically more evolvable. Genesis should test whether duplicated modules actually preserve function long enough to diverge under its own physical and energetic costs.


### [CHAN23] Chan, B. W.-C. (2023). “Towards Large-Scale Simulations of Open-Ended Evolution in Continuous Cellular Automata.” *Proceedings of the Companion Conference on Genetic and Evolutionary Computation (GECCO ’23 Companion)*, pp. 127–130. doi:10.1145/3583133.3590670.

**Contribution.** Reports a large-scale JAX implementation of Lenia evolution with implicit reproduction, differential survival, localized genetic information, and genotype-to-phenotype maintenance. Runs exhibited an initial period of diversity and creativity but tended to converge toward fast-expanding patterns under the implemented environment.

**Genesis relevance.** The result is a direct warning that scale, implicit replication, and a flexible substrate do not guarantee open-ended evolutionary dynamics. It supports treating mass conservation, energy constraints, resource topology, and exploit-resistant ecology as first-class experimental variables independent of genome representation.

**Transfer limit.** This is a short conference paper about continuous cellular automata, not an explicit modular neural genome. Its observations do not identify a sufficient recipe for open-endedness, and the proposed environmental factors remain hypotheses to test rather than established requirements.

### [SHVARTZMAN24] Shvartzman, B., and Ram, Y. (2024). “Self-Replicating Artificial Neural Networks Give Rise to Universal Evolutionary Dynamics.” *PLoS Computational Biology* 20(3): e1012004. doi:10.1371/journal.pcbi.1012004.

**Contribution.** Introduces self-replicating artificial neural networks whose learned copying errors generate endogenous mutations. In a population of 1,000 individuals evolved for 6,000 generations, the model exhibited adaptation, clonal interference, epistasis, and evolution of mutation rate and the distribution of fitness effects.

**Genesis relevance.** The paper establishes endogenous mutation generation as a credible artificial-life alternative to externally sampled mutation kernels. It also illustrates how replication fidelity can become an evolvable tradeoff rather than a fixed scalar.

**Transfer limit.** The genotype is decoded into Python source code, invalid programs are removed by survival selection, and each phenotype is trained with a deep-learning stack. Those choices are expensive and expose behavior to library, interpreter, accelerator, and stochastic-training semantics. They conflict with the Genesis baseline’s requirement for explicit mutation provenance, bounded compilation, and exact cross-platform replay; the result should motivate a separate experiment, not replace named deterministic operators.

---

## 20.4 Recombination and variable-length genomes

### [MERLEVEDE19] Merlevede, A., Åhl, H., and Troein, C. (2019). “Homology and Linkage in Crossover for Linear Genomes of Variable Length.” *PLoS ONE* 14(1): e0209712. doi:10.1371/journal.pone.0209712.

**Contribution.** Defines separate measures for retaining homologous structure and reshuffling linked variation in variable-length crossover. It compares crossover methods and shows that good performance requires balancing both properties in the tested benchmarks.

**Genesis relevance.** The paper directly supports measuring homology preservation and linkage disruption instead of judging crossover only by offspring viability. It also supports alignment methods that keep historically corresponding loci together while permitting controlled reassortment.

**Transfer limit.** The studied genomes are linear benchmark representations, not typed modular graphs with developmental semantics. Genesis must extend the measures to modules, graph edges, and capability bindings. The paper does not identify one crossover operator that will be best in every ecology.

### [LANGDON00] Langdon, W. B. (2000). “Size Fair and Homologous Tree Crossovers for Tree Genetic Programming.” *Genetic Programming and Evolvable Machines* 1: 95–119. doi:10.1023/A:1010024515191.

**Contribution.** Develops crossover operators designed to reduce size bias and exchange structurally corresponding regions in tree genetic programming. It demonstrates that operator design can materially alter bloat and offspring structure.

**Genesis relevance.** The work motivates size-fair module exchange, homologous subtree or subgraph matching, and explicit reporting of offspring-size distributions. It is particularly relevant to grammar/tree alternatives and to module-level crossover.

**Transfer limit.** Tree position is not sufficient homology for a historical graph genome. Structural correspondence and descent identity must remain separate. Results from GP trees cannot determine the correct treatment of recurrent neural edges, shared capabilities, or duplicated module families.

### [POLI04] Poli, R., McPhee, N. F., and Rowe, J. E. (2004). “Exact Schema Theory and Markov Chain Models for Genetic Programming and Variable-Length Genetic Algorithms with Homologous Crossover.” *Genetic Programming and Evolvable Machines* 5(1): 31–70. doi:10.1023/B:GENP.0000017010.41337.a7.

**Contribution.** Provides formal analysis of homologous crossover in variable-length evolutionary representations and clarifies how operator structure changes schema transmission.

**Genesis relevance.** The formal perspective supports treating crossover as a defined transmission kernel with testable probabilities rather than an informal splice. It motivates golden statistical tests for inheritance frequencies and schema survival.

**Transfer limit.** Exact schema results rely on the paper’s representation and operator assumptions. They do not directly describe typed graph genomes, viability filtering, or ecological selection. Genesis should use the analytical discipline, not claim that the same closed-form results apply.

### [DOERR08] Doerr, B., Happ, E., and Klein, C. (2008). “Crossover Can Provably Be Useful in Evolutionary Computation.” In *Proceedings of GECCO 2008*, pp. 539–546. doi:10.1145/1389095.1389202.

**Contribution.** Gives a theoretical problem class in which crossover provides a provable advantage over mutation-only evolutionary search under stated assumptions.

**Genesis relevance.** The paper rebuts the blanket claim that crossover is never useful. More importantly, it shows that usefulness depends on exploitable problem structure and on the algorithm’s ability to preserve and combine partial solutions.

**Transfer limit.** A proof for a constructed optimization problem is not evidence that crossover improves open-ended artificial life. It cannot choose Genesis’s default inheritance mode. The appropriate translation is to run crossover and no-crossover treatments and measure whether useful modules are actually combined rather than disrupted.


### [QIAO23] Qiao, Y., and Gallagher, M. (2023). “Modularity Based Linkage Model for Neuroevolution.” *Proceedings of the Companion Conference on Genetic and Evolutionary Computation (GECCO ’23 Companion)*, pp. 675–678. doi:10.1145/3583133.3590648.

**Contribution.** Estimates dependencies among neural-network connection weights, detects modular communities in the resulting dependency graph, and uses those communities as crossover masks in an optimal-mixing evolutionary algorithm. A variant also mitigates neural-network permutation symmetry. Experiments on 8- and 10-bit parity problems reported more functionally coherent linkage and more successful crossover.

**Genesis relevance.** The work supports the proposition that useful recombination units can be functional linkage groups rather than arbitrary serialized intervals. It motivates an optional, versioned linkage-learning operator and diagnostics that compare inferred functional communities with explicit genome modules.

**Transfer limit.** The evidence comes from small parity benchmarks and a short conference paper. Dependency estimation adds substantial cost, may change with task and environment, and does not establish historical homology. Researcher-derived offline communities must not affect reproduction unless they are deliberately promoted into a new causal policy version and tested against fixed-module and no-crossover controls.

---

## 20.5 Neutrality, robustness, modularity, duplication, and error limits

### [VANNIM99] van Nimwegen, E., Crutchfield, J. P., and Huynen, M. (1999). “Neutral Evolution of Mutational Robustness.” *Proceedings of the National Academy of Sciences* 96(17): 9716–9720. doi:10.1073/pnas.96.17.9716.

**Contribution.** Analyzes evolution on neutral networks and shows how population dynamics can concentrate lineages in regions with more neutral neighbors, producing mutational robustness without direct selection for a robustness trait.

**Genesis relevance.** The result supports measuring the topology of neutral genotype networks and distinguishes robustness emerging from population occupancy from a directly encoded “robustness gene.” It also motivates population-size and mutation-rate controls.

**Transfer limit.** The model’s neutral-network assumptions are abstract. Genesis phenotypes may have graded effects, state-dependent fitness, and changing environments. Neutrality must be defined operationally over a measurement window and cannot be inferred merely from identical serialized phenotypes.

### [DRAGHI10] Draghi, J. A., Parsons, T. L., Wagner, G. P., and Plotkin, J. B. (2010). “Mutational Robustness Can Facilitate Adaptation.” *Nature* 463: 353–355. doi:10.1038/nature08694.

**Contribution.** Uses theoretical models to show conditions under which mutational robustness can increase access to adaptive innovations by allowing populations to explore broader neutral genotype neighborhoods.

**Genesis relevance.** The paper supplies a mechanism for why neutral variation might matter after environmental change. It supports measuring cryptic genotypic diversity, reachable novel phenotypes, and adaptation following shifts.

**Transfer limit.** Robustness does not universally accelerate adaptation; the effect depends on population size, mutation supply, neutral-network structure, and time horizon. The paper is not a reason to add nonfunctional genome content intentionally. Genesis should compare neutral-retention policies under controlled shifts.

### [EIGEN71] Eigen, M. (1971). “Selforganization of Matter and the Evolution of Biological Macromolecules.” *Naturwissenschaften* 58: 465–523. doi:10.1007/BF00623322.

**Contribution.** Foundational quasispecies theory relating replication fidelity, sequence length, selection, and the maintenance of hereditary information. It introduced the conceptual basis for an error threshold.

**Genesis relevance.** The work motivates mapping the joint space of genome length and mutation rate, measuring loss of high-fidelity hereditary structure, and distinguishing per-locus from per-genome mutation load.

**Transfer limit.** Molecular sequence assumptions and analytic thresholds do not map numerically onto a typed graph genome with viability filters, modular redundancy, or recombination. Genesis should use “error threshold” as a hypothesis to test, not as a borrowed constant or a guaranteed sharp transition.

### [CILIBERTI07] Ciliberti, S., Martin, O. C., and Wagner, A. (2007). “Innovation and Robustness in Complex Regulatory Gene Networks.” *Proceedings of the National Academy of Sciences* 104(34): 13591–13596. doi:10.1073/pnas.0705396104.

**Contribution.** Studies neutral networks of gene-regulatory-network genotypes and finds that phenotypically equivalent networks can form large connected sets from which diverse new phenotypes are accessible.

**Genesis relevance.** The study supports separating a regulatory network’s current phenotype from the diversity of phenotypes in its mutational neighborhood. It also motivates bounded regulatory encodings as an experimental route to neutral exploration.

**Transfer limit.** The regulatory model is abstract and has specific dynamics and phenotype definitions. Large neutral networks are not guaranteed in Genesis. A regulatory encoding also adds difficult cycle, convergence, and developmental-semantic requirements that must be measured against a direct baseline.

### [KASHTAN05] Kashtan, N., and Alon, U. (2005). “Spontaneous Evolution of Modularity and Network Motifs.” *Proceedings of the National Academy of Sciences* 102(39): 13773–13778. doi:10.1073/pnas.0503610102.

**Contribution.** Shows in an evolutionary network model that modularly varying goals can favor modular architectures and recurrent network motifs.

**Genesis relevance.** The result supplies a concrete environmental mechanism for modularity: repeated recombination of subproblems or ecological demands, rather than an arbitrary bonus for modules. It motivates environment-shift experiments with repeated substructure.

**Transfer limit.** The goals and network task decomposition are explicitly defined. A physical ecology may not present clean modularly varying objectives. The paper does not show that adding syntactic module boundaries causes functional modularity.

### [CLUNE13] Clune, J., Mouret, J.-B., and Lipson, H. (2013). “The Evolutionary Origins of Modularity.” *Proceedings of the Royal Society B* 280: 20122863. doi:10.1098/rspb.2012.2863.

**Contribution.** Demonstrates in evolved networks that a cost for connections can promote modularity and, in the reported tasks, improve evolvability and adaptation.

**Genesis relevance.** This supports testing physically grounded costs for long-range or numerous connections rather than directly rewarding a modularity metric. It also motivates measuring connection cost, modularity, performance, and adaptation jointly.

**Transfer limit.** A connection cost is still an imposed pressure and can distort behavior if it lacks a world-level physical interpretation. The result does not prove that every cost produces useful modularity or that modularity should be a direct fitness term. Genesis should implement costs only where controller complexity or communication has a causal resource cost.

### [ESPINOSA10] Espinosa-Soto, C., and Wagner, A. (2010). “Specialization Can Drive the Evolution of Modularity.” *PLoS Computational Biology* 6: e1000719. doi:10.1371/journal.pcbi.1000719.

**Contribution.** Uses regulatory-network models to show that selection for specialization across contexts can promote modular organization.

**Genesis relevance.** The paper supports ecological specialization and context-dependent task demands as potential sources of modularity. It suggests measuring whether different modules become associated with different sensory, behavioral, or environmental contexts.

**Transfer limit.** The modeled phenotypes and contexts are simplified regulatory outputs. A result about network specialization does not determine module syntax or crossover policy. The relevant Genesis claim must be tested with actual embodied behavior.

### [FORCE99] Force, A., Lynch, M., Pickett, F. B., Amores, A., Yan, Y.-L., and Postlethwait, J. (1999). “Preservation of Duplicate Genes by Complementary, Degenerative Mutations.” *Genetics* 151(4): 1531–1545. doi:10.1093/genetics/151.4.1531.

**Contribution.** Develops the duplication-degeneration-complementation model: duplicate genes can be retained when different subfunctions are lost from each copy, so that both copies together preserve the ancestral function.

**Genesis relevance.** The mechanism translates cleanly into duplicated modules with separable outputs, contexts, or interface bindings. It motivates tracking subfunction use and allowing duplicated modules to partition roles rather than requiring immediate new function.

**Transfer limit.** The biological model concerns genes with multiple regulatory or functional subcomponents. Genesis modules will only exhibit this mechanism if their interfaces and effects actually permit partial loss. A duplication operator alone does not create subfunctionalization.

### [LYNCH00] Lynch, M., and Conery, J. S. (2000). “The Evolutionary Fate and Consequences of Duplicate Genes.” *Science* 290(5494): 1151–1155. doi:10.1126/science.290.5494.1151.

**Contribution.** Reviews and analyzes the prevalence and evolutionary fates of gene duplicates, including loss, divergence, and retention.

**Genesis relevance.** The work supports recording duplication families and expecting most duplicates not to become durable innovations. It motivates metrics for duplicate survival time, divergence, deletion, subfunctionalization, and neofunctionalization.

**Transfer limit.** Biological duplication rates and retention times are not portable. Artificial modules may have very different costs and mutational neighborhoods. Genesis should not tune duplication to reproduce biological distributions unless that serves a specific research question.

### [POSADAS22] Posadas-García, Y. S., and Espinosa-Soto, C. (2022). “Early Effects of Gene Duplication on the Robustness and Phenotypic Variability of Gene Regulatory Networks.” *BMC Bioinformatics*. doi:10.1186/s12859-022-05067-1.

**Contribution.** Examines immediate effects of duplicating regulators in model gene-regulatory networks. Duplication often buffered mutations when the original phenotype survived, but effects depended strongly on network context, mutation type, and which genes were involved.

**Genesis relevance.** This is a direct warning against coding duplication as automatically neutral or beneficial. It supports recording the immediate phenotype delta of a duplicate, the affected interfaces, and subsequent divergence.

**Transfer limit.** The result uses a particular deterministic regulatory-network model. Its quantitative robustness changes do not transfer to neural modules. The applicable claim is only that duplication outcomes are context dependent and should be tested rather than assumed.

---

## 20.6 Learning, plasticity, signaling, mating, and kin cues

### [SASAKI99] Sasaki, T., and Tokoro, M. (1999). “Evolving Learnable Neural Networks Under Changing Environments with Various Rates of Inheritance of Acquired Characters: Comparison of Darwinian and Lamarckian Evolution.” *Artificial Life* 5(3): 203–223. doi:10.1162/106454699568746.

**Contribution.** Evolves learning neural networks while varying how much acquired weight change is inherited. In the reported changing environments, lower or zero inheritance of acquired characters produced more stable adaptability than stronger Lamarckian inheritance.

**Genesis relevance.** The paper supports a strict baseline separation between inherited initial parameters and lifetime-learned state. It also supplies an ablation template for any later Lamarckian experiment.

**Transfer limit.** The environment, learning rule, and inheritance interpolation are specific to the study. The result does not prove that inherited learned state is always harmful. It does show that exact learned-state persistence and germline inheritance are separate mechanisms and should never be conflated silently.

### [DOWNING04] Downing, K. L. (2004). “Development and the Baldwin Effect.” *Artificial Life* 10(1): 39–63. doi:10.1162/106454604322875904.

**Contribution.** Examines interactions among development, learning, and evolution in computational models related to the Baldwin effect, where learning changes selection on inherited traits without direct inheritance of acquired state.

**Genesis relevance.** The work supports analyzing how plastic behavior can smooth or redirect inherited evolution while preserving a Darwinian inheritance boundary. It motivates tracking learning cost, initial performance, learned performance, and eventual genetic assimilation.

**Transfer limit.** Baldwin-effect outcomes are sensitive to learning cost, environment stability, and developmental representation. The paper does not justify adding a generic assimilation operator. In Genesis, assimilation should be an observed population pattern unless a separate, explicitly versioned inheritance mechanism is under experiment.

### [AXELROD04] Axelrod, R., Hammond, R. A., and Grafen, A. (2004). “Altruism via Kin-Selection Strategies That Rely on Arbitrary Tags with Which They Coevolve.” *Evolution* 58(8): 1833–1838. doi:10.1111/j.0014-3820.2004.tb00465.x.

**Contribution.** Demonstrates in a computational model that coevolving arbitrary tags and tag-conditioned behavior can support assortative interaction and cooperation.

**Genesis relevance.** The paper supports heritable signaling traits whose value is not predefined by the engine. It also shows that a visible tag can serve as a behavioral cue without exposing exact pedigree or a privileged “kin” oracle.

**Transfer limit.** Tags can be exploited, lose correlation with ancestry, or generate green-beard dynamics. The model does not justify treating tag similarity as true relatedness. Genesis must store exact ancestry separately and let organisms perceive only physically instantiated signals.

### [SCOTT22] Scott, T. W., Grafen, A., and West, S. A. (2022). “Multiple Social Encounters Can Eliminate Crozier’s Paradox and Stabilise Genetic Kin Recognition.” *Nature Communications* 13: 3902. doi:10.1038/s41467-022-31545-4.

**Contribution.** Analyzes conditions under which genetically encoded recognition cues can remain informative despite selection that might otherwise erode cue diversity. Repeated social encounters alter the stability conditions.

**Genesis relevance.** The work warns that kin-recognition systems are evolutionary systems, not static distance checks. It supports testing cue evolution, false positives, cheating, encounter structure, and actual genealogical correlation.

**Transfer limit.** The analytical assumptions are biological and game-theoretic. The result does not prescribe a Genesis cue. A built-in exact-relatedness sensor would bypass the problem studied and would be an artificial privilege, not emergent recognition.

### [WEIGEL15] Weigel, E. G., Testa, N. D., Peer, A., and Garnett, S. C. (2015). “Context Matters: Sexual Signaling Loss in Digital Organisms.” *Ecology and Evolution* 5(17): 3725–3736. doi:10.1002/ece3.1631.

**Contribution.** Studies the loss or maintenance of sexual signaling in digital organisms under different ecological and reproductive contexts. The results emphasize that signaling traits persist only when their benefits and costs remain supported by the environment and mating system.

**Genesis relevance.** This supports heritable signal traits with physical production, perception, and opportunity costs, rather than a permanent “mate quality” field. It also motivates experiments in which signaling can disappear.

**Transfer limit.** The specific signaling implementation and digital mating ecology are model dependent. The paper does not imply that sexual selection will emerge merely because paired reproduction exists. Genesis must provide observable variation and behavioral mate choice without guaranteeing signal utility.

### [DECARA08] de Cara, M. A. R., Barton, N. H., and Kirkpatrick, M. (2008). “A Model for the Evolution of Assortative Mating.” *The American Naturalist* 171(5): 580–596. doi:10.1086/587062.

**Contribution.** Develops population-genetic models for the evolution of assortative mating and examines how preference, trait variation, recombination, and selection interact.

**Genesis relevance.** The work supports treating assortative mating as an evolvable behavior or compatibility response rather than an observer-imposed species partition. It also highlights the need to track preference and target traits separately.

**Transfer limit.** The analytic assumptions, diploid genetics, and mating structure are not direct prescriptions for a digital graph genome. Genesis should use the causal decomposition—cue, preference, encounter, cost, and offspring consequences—while measuring its own outcomes.

---

## 20.7 Exact ancestry, compressed genealogy, and distributed lineage inference

### [KELLEHER16] Kelleher, J., Etheridge, A. M., and McVean, G. (2016). “Efficient Coalescent Simulation and Genealogical Analysis for Large Sample Sizes.” *PLoS Computational Biology* 12: e1004842. doi:10.1371/journal.pcbi.1004842.

**Contribution.** Introduces efficient succinct tree-sequence methods for representing correlated genealogies across genomes and enables large-scale population-genetic simulation and analysis.

**Genesis relevance.** The central storage lesson is that ancestry contains extensive shared structure and should not be duplicated in every organism record. Content-addressed genomes, shared parent edges, interval or module contribution runs, and canonical simplification can provide similar savings.

**Transfer limit.** A recombining biological chromosome is an ordered sequence with interval inheritance; a modular graph genome is not automatically one. Tree-sequence concepts should be adapted at the module or locus-contribution level, while the exact reproduction event graph remains authoritative.

### [KELLEHER18] Kelleher, J., Thornton, K. R., Ashander, J., and Ralph, P. L. (2018). “Efficient Pedigree Recording for Fast Population Genetics Simulation.” *PLoS Computational Biology* 14(11): e1006581. doi:10.1371/journal.pcbi.1006581.

**Contribution.** Shows how forward-time simulations can record complete genetic ancestry and periodically simplify history relative to retained samples. The approach can reduce memory and runtime substantially while preserving relevant genealogical information.

**Genesis relevance.** This is the strongest precedent for separating live simulation state from a compact ancestry store and for pruning only under an explicit retention policy. It motivates exact parentage records, contribution maps, and offline simplification.

**Transfer limit.** Genesis also requires organism-level, mutation-event, policy-fork, artifact, and semantic-version provenance that biological tree sequences do not represent by default. Simplification must never remove records required for replay, audit, or user-selected historical samples. It is an archival policy, not a mutation of the causal past.

### [MORENO22HS] Moreno, M. A., Dolson, E., and Ofria, C. (2022). “Hereditary Stratigraphy: Genome Annotations to Enable Phylogenetic Inference over Distributed Populations.” *Proceedings of the 2022 Conference on Artificial Life*, pp. 418–428. doi:10.1162/isal_a_00550. A shorter companion version appears in *GECCO 2022 Companion*, pp. 65–66, doi:10.1145/3520304.3533937.

**Contribution.** Introduces heritable ancestry annotations that permit approximate phylogenetic reconstruction when exact centralized parentage recording is unavailable or impractical in distributed systems. The method exposes a tunable memory-versus-accuracy tradeoff.

**Genesis relevance.** Hereditary stratigraphy is useful for exported organisms, distributed shards, or worlds that cannot share an exact lineage database. It is also a valuable cross-check against exact lineage records.

**Transfer limit.** It is an inference method, not a replacement for exact parentage when exact recording is affordable. Genesis’s main single-world simulation should retain authoritative reproduction events. Approximate annotations must be visibly labeled and must not be used to overwrite true ancestry or determine mating unless the cue is physically available to organisms.


### [MORENO24] Moreno, M. A., Yang, C., Dolson, E., and Zaman, L. (2024). “Trackable Island-Model Genetic Algorithms at Wafer Scale.” *Proceedings of the Genetic and Evolutionary Computation Conference Companion (GECCO ’24 Companion)*, pp. 101–102. doi:10.1145/3638530.3664090.

**Contribution.** Presents a tracking-enabled asynchronous island-model genetic algorithm for wafer-scale hardware. Hereditary annotations supported approximate phylogenetic reconstruction while emulated and on-hardware benchmarks scaled to populations in the millions and distinguished adaptive from non-adaptive evolutionary regimes through phylometric signals.

**Genesis relevance.** The study demonstrates that very large distributed evolutionary runs can retain useful lineage observability without a centralized full pedigree. It supports hereditary stratigraphy for exported organisms, disconnected shards, extreme-scale experiments, and independent validation of exact records.

**Transfer limit.** The framework uses simple fixed-length genomes, specialized hardware, and approximate reconstruction. It does not solve exact ancestry, per-locus contribution, schema migration, or replay for variable modular graph genomes. Genesis should retain exact birth events whenever available and label inferred phylogenies as estimates.

---

## 20.8 Artificial chemistries and organizational transitions

### [FONTANA94] Fontana, W., and Buss, L. W. (1994). “‘The Arrival of the Fittest’: Toward a Theory of Biological Organization.” *Bulletin of Mathematical Biology* 56(1): 1–64. doi:10.1016/S0092-8240(05)80205-8.

**Contribution.** Develops the AlChemy framework, in which lambda-calculus expressions interact under artificial reaction rules and can form persistent, self-maintaining organizations. It explores organization emerging from reaction closure rather than from an externally named organism schema.

**Genesis relevance.** The transferable lesson is to distinguish authored interaction laws from observer-defined organizations. If Genesis later develops an artificial chemistry for materials or internal developmental signals, persistent reaction organizations should be detected from dynamics rather than hard-coded as technologies or species.

**Transfer limit.** AlChemy’s symbolic reactions are computational abstractions, not a ready genome representation for embodied neural organisms. Unconstrained expression interaction would complicate safety, termination, determinism, and debugging. Artificial chemistry is a research alternative for a narrowly scoped substrate, not the recommended successor genome.

---

## 20.9 Bibliographic synthesis

Across these sources, several conclusions are comparatively well supported:

- Historical tags can make variable-topology crossover tractable, but the allocator and reproductive policy must be redesigned for deterministic, world-local execution [NEAT02].
- Direct encodings remain easier to inspect and migrate; indirect encodings become compelling when phenotype regularity and geometry are real, stable parts of the problem [CPPN07; HYPERNEAT09; HORNBY03].
- Recombination can help or harm depending on homology, linkage, epistasis, and problem structure [MISEVIC06; MERLEVEDE19; DOERR08].
- Duplication is a source of possible redundancy, partitioning, and novelty, but most benefits are conditional and require an ecology that can retain intermediate copies [FORCE99; LYNCH00; POSADAS22; WILLENSDORFER08].
- Robustness, neutrality, and evolvability cannot be reduced to one monotonic objective [VANNIM99; ELENA08; DRAGHI10; LABAR17].
- Evolved mutation rates optimize short-term lineage success under the current landscape, not necessarily the researcher’s long-term objective [CLUNE08].
- Exact lineage, similarity, reproductive compatibility, and organism-visible recognition cues answer different questions and must remain separate [ANDERSON14; AXELROD04; SCOTT22; KELLEHER18].
- Long-running variable-length evolution requires explicit bloat controls, mutation-load experiments, and storage-aware lineage architecture [EIGEN71; GUPTA16; LANGDON19; KELLEHER16].

The literature does **not** establish that any one representation guarantees open-ended evolution, cumulative culture, technological progress, major evolutionary transitions, or continuously increasing complexity. Those remain empirical questions for the world physics, ecology, population process, controller architecture, and genome system together.

---

**End of review.**
