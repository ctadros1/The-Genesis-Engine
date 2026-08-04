# The Genesis Engine: Open-Ended Evolution, Experimental Methodology, and Scalable Deterministic Execution

**Deliverable:** Research and systems-methodology dossier  
**Date:** 2026-08-04  
**Encoding:** UTF-8  
**Status:** Research guidance and proposed standards; not an implementation specification  
**Primary scientific constraint:** Offline analysis may classify what occurred, but classification outputs must never become hidden behavioral inputs to the simulation.

---

## How to read this dossier

This dossier distinguishes three kinds of statement:

- **Established methodology** — methods with mature theoretical or empirical support, usually outside artificial life but directly relevant to simulation science.
- **Emerging practice** — promising methods or recent findings whose transfer to Genesis remains uncertain.
- **Genesis synthesis** — recommendations derived from the literature and the project's determinism contract. These are reasoned engineering judgments, not settled scientific consensus.

### Evidence-quality grades

| Grade | Meaning |
|---|---|
| **A** | Mature methodology, authoritative standard, or convergent primary evidence. |
| **B** | Strong primary evidence or widely used practice, but substantial transfer uncertainty remains. |
| **C** | Emerging, single-system, contested, or preprint-level evidence. |
| **S** | Genesis-specific synthesis grounded in cited evidence; must be validated in this engine. |

### Governing interpretation rule

A finite experiment cannot prove literal, mathematical unboundedness. Genesis can test whether novelty, adaptation, ecological differentiation, or complexity **fails to saturate within explicitly stated horizons and representations**. It cannot infer “unbounded evolution,” “civilization,” “language,” or a technological era from a visually compelling run.

### Contents

1. [Executive summary](#1-executive-summary)  
2. [State of open-ended-evolution research](#2-state-of-open-ended-evolution-research)  
3. [Definitions and measurement problems](#3-definitions-and-measurement-problems)  
4. [Recommended Genesis research claims](#4-recommended-genesis-research-claims)  
5. [Experimental-unit and replication guidance](#5-experimental-unit-and-replication-guidance)  
6. [Multi-seed experimental design](#6-multi-seed-experimental-design)  
7. [Statistical methods](#7-statistical-methods)  
8. [General phase-acceptance template](#8-general-phase-acceptance-template)  
9. [Worked acceptance-criterion examples](#9-worked-acceptance-criterion-examples)  
10. [Deterministic simulation architecture](#10-deterministic-simulation-architecture)  
11. [Parallel-world scheduling](#11-parallel-world-scheduling)  
12. [Headless execution and observer decoupling](#12-headless-execution-and-observer-decoupling)  
13. [Event logging and offline analysis](#13-event-logging-and-offline-analysis)  
14. [Mutable-state persistence](#14-mutable-state-persistence)  
15. [Save-format and migration strategy](#15-save-format-and-migration-strategy)  
16. [Performance and compute planning](#16-performance-and-compute-planning)  
17. [Null-result reporting](#17-null-result-reporting)  
18. [Reproducibility and determinism checklists](#18-reproducibility-and-determinism-checklists)  
19. [Risks and unsupported assumptions](#19-risks-and-unsupported-assumptions)  
20. [Open questions](#20-open-questions)  
21. [Annotated bibliography](#21-annotated-bibliography)

---

# 1. Executive summary

## 1.1 Principal findings

1. **The world is normally the replicate.** When a condition is assigned to a world, the independent experimental unit is the independently initialized world or seed block. Organisms, births, generations, encounters, ticks, and sliding windows are nested observations. Counting them as independent replicates is pseudoreplication and can produce arbitrarily small but invalid standard errors.

2. **No single metric establishes open-ended evolution.** Novelty, diversity, evolutionary activity, controller size, compression complexity, ecological-network statistics, and performance trends each capture different phenomena and each can be gamed. Genesis should use a preregistered metric panel, causal interventions, multiple representations, and explicit saturation tests.

3. **Continuous novelty is not the same as adaptive progress.** A population can generate endless neutral variation, random controller growth, noisy action sequences, or combinatorial object arrangements without improved function, ecological consequence, persistence, or cumulative dependence. Genesis should reserve *innovation* for novelty with demonstrated consequence and persistence.

4. **Literal unboundedness is not empirically demonstrable from finite runs.** For any exactly represented finite-state deterministic world, recurrence is guaranteed in principle, even if the recurrence time is astronomically large. The responsible claim is that specified metrics showed no detected ceiling within specified state, time, population, and compute horizons.

5. **No existing artificial-life system is a generally accepted demonstration of all strong forms of open-ended evolution.** Tierra, Avida, Geb, novelty search, POET, Genelife, and newer GPU platforms each demonstrate important partial properties. They do not jointly establish indefinitely expanding adaptive novelty, ecological organization, behavioral complexity, major transitions, and technological accumulation without substantial observer or task dependence.

6. **Culture requires causal evidence of social transmission.** A stable group difference is not enough. Genetic similarity, shared environment, common affordances, synchronized ecological response, and independent reinvention must be addressed. The strongest Genesis designs use randomized arbitrary variants, exposure records, social-channel ablations, yoked environmental controls, donor removal, and transmission-hazard models.

7. **Tool use requires causal mediation by an external object.** Object contact, carrying, or repeated action sequences are insufficient. The object must be controlled and must enable or materially improve an outcome, with object-removal or function-destroying ablations supporting the inference.

8. **Cumulative culture is a high-threshold claim.** It requires socially transmitted modifications, retained gains, descent-with-modification, and evidence that later performance causally depends on earlier innovations. A rising average outcome alone can be genetic adaptation, environmental change, or repeated independent invention.

9. **Acceptance must be conjunctive.** A phase passes only when its scientific primary endpoint, causal controls, determinism gates, log-completeness gates, and performance budget all pass. Secondary metrics do not rescue a failed primary endpoint.

10. **Independent-world parallelism should be the first scaling axis.** It is naturally isolated, statistically appropriate, and compatible with deterministic execution. Within-world parallelism should be added only after profiling and only through deterministic intent collection, canonical merging, and explicit conflict resolution.

11. **A keyed, counter-based random-number model is preferable to consumption-ordered mutable streams.** A draw should be a pure function of a versioned semantic key such as `(root_seed, world_id, tick, phase, system, subject_id, event_kind, draw_slot)`. This limits order sensitivity and makes paired-seed comparisons more meaningful.

12. **A successor persistence design should use canonical snapshots plus an append-only journal.** Periodic full snapshots provide bounded recovery and clear migration points. Journals provide short-interval recovery and auditability. Content-addressed region chunks can reduce storage for mutable terrain and structures, but event sourcing alone is not a suitable primary archival restore strategy.

13. **The live observer must be lossy; the scientific record must not be.** Rendering frames and UI telemetry may be sampled or dropped. Claim-critical simulation events must be complete, ordered, versioned, checksummed, and written under an explicit failure policy.

14. **GPU acceleration is workload contingent.** Evolving sparse topologies, dynamic object graphs, divergent control flow, irregular memory access, deterministic conflict resolution, and frequent synchronization map poorly to GPUs. Uniform batched kernels may benefit. Only end-to-end profiling, including data movement and determinism costs, can justify a GPU path.

15. **Null results are scientific results when bounded by a meaningful effect threshold.** Genesis should report the smallest effect of interest, confidence or credible intervals, equivalence/ROPE results, achieved exposure, extinction and censoring, and whether the result rules out a meaningful effect or remains underpowered.

## 1.2 Recommended research operating model

Genesis should separate work into four layers:

| Layer | Purpose | Allowed to affect simulation behavior? |
|---|---|---:|
| **Simulation kernel** | Defines physics, perception, action, learning, inheritance, resource dynamics, and exact state transitions. | Yes, by explicit versioned rules only. |
| **Experiment controller** | Assigns conditions, seeds, run budgets, interventions, checkpoints, and stopping rules. | Yes, but only as preregistered experimental treatment. |
| **Authoritative record** | Stores canonical events, snapshots, hashes, manifests, and failure state. | No feedback except explicit deterministic restart. |
| **Offline analysis** | Detects traditions, tool sequences, change points, eras, cultural lineages, and claims. | **Never.** Analysis-derived labels must not be returned to organisms or simulation systems. |

The minimum credible workflow is:

1. Freeze a versioned model, build, configuration schema, seed registry, event schema, detector configuration, primary estimand, smallest effect of interest, and stopping rule.
2. Run a pilot solely to estimate runtime, variance, extinction, event volume, and plausible effect distributions.
3. Lock a confirmatory seed registry that was not screened by outcomes.
4. Execute paired or blocked independent worlds under treatment and control.
5. Verify uninterrupted replay, checkpoint restoration, thread-count invariance where claimed, and event completeness.
6. Analyze at the world level or with a hierarchical model that preserves world-level clustering.
7. Report all worlds, failures, extinctions, censoring, deviations, sensitivity analyses, and null findings.
8. Promote only the claim supported by the design; preserve stronger labels as hypotheses.

## 1.3 Minimum set of deliverables for every confirmatory experiment

- Preregistration or immutable experiment plan.
- Exact source revision and binary digest.
- Rust toolchain and dependency lock.
- Versioned model/configuration manifest.
- Condition-to-seed assignment table.
- Full inclusion/exclusion and crash-recovery record.
- Per-world primary outcome table.
- Confidence or credible interval and prespecified effect threshold.
- Extinction/censoring table.
- Determinism verification report.
- Canonical snapshots and event-log hashes.
- Analysis code, detector configuration, and derived data provenance.
- Hardware, operating system, compiler flags, CPU/GPU features, and observed performance.
- Human-readable ODD-style model description and STRESS-style simulation-reporting supplement.

> **Major conclusion record 1 — The independently initialized world is normally the experimental unit.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A.** Classical experimental-unit and pseudoreplication theory; mature clustered-data methodology. |
> | **Applicability to Genesis** | Direct. Conditions such as learning enabled, social channel enabled, or artifacts persistent act on interacting populations within worlds. |
> | **Uncertainty** | An organism can be an experimental unit only for a genuinely randomized organism-level intervention with defensible interference assumptions; this is uncommon in Genesis. |
> | **Recommended action** | Register independent worlds or paired seed blocks as replicates. Treat organism/event observations as nested measurements. |
> | **Measurement method** | Report per-world outcomes; use hierarchical or cluster-robust models when retaining within-world detail. |
> | **Control or ablation** | Independently initialized control worlds using the same preregistered seed blocks and matched budgets. |
> | **Determinism implications** | Paired conditions require semantically keyed randomness so divergent event paths do not consume unrelated draws. |
> | **Compute implications** | Adequate power usually requires more independent worlds, not merely more organisms in one world. |

> **Major conclusion record 2 — Open-endedness requires a metric panel and claim ceiling.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **B/S.** OEE literature strongly documents definitional plurality and metric dependence; the exact Genesis panel is a synthesis. |
> | **Applicability to Genesis** | Central. Genesis can generate arbitrary variation that would inflate novelty or size measures without adaptive significance. |
> | **Uncertainty** | There is no consensus panel that is sufficient across all artificial-life systems. |
> | **Recommended action** | Preregister multiple novelty, activity, diversity, complexity, evolvability, ecological, and adaptation measures; report representation sensitivity and saturation. |
> | **Measurement method** | Multi-descriptor novelty production, adaptive-persistence tests, evolutionary activity, complexity panel, perturbation-based evolvability, and expanding-budget saturation analyses. |
> | **Control or ablation** | Neutral drift, frozen evolution, randomized behavior, bounded external-task positive control, and mechanism-specific ablations. |
> | **Determinism implications** | Metric extraction must be deterministic and versioned; offline detector labels cannot feed back into the world. |
> | **Compute implications** | Multi-metric analysis and longer horizon replication increase storage and compute; summaries must not replace primary events required by claims. |

---

# 2. State of open-ended-evolution research

## 2.1 Why the field lacks a single accepted definition

“Open-ended evolution” is used for several related but non-equivalent aspirations:

1. **Persistent novelty:** the system keeps producing phenotypes, behaviors, or organizations not previously observed.
2. **Adaptive innovation:** novelty repeatedly changes reproductive, ecological, or functional outcomes.
3. **Unbounded complexity growth:** some defensible complexity measure has no intrinsic ceiling and continues to increase.
4. **Expanding evolutionary activity:** new components or organizations enter and remain active rather than cycling through a fixed effective repertoire.
5. **Evolvability growth:** the capacity to generate viable and useful variants itself expands.
6. **Major transitions:** new levels of individuality or inheritance arise.
7. **Open-ended ecology:** niches, interactions, dependencies, and selection pressures are endogenously generated and expanded.
8. **Cumulative behavioral or technological evolution:** innovations are retained, recombined, and become prerequisites for later innovations.

Taylor et al. (2016), Banzhaf et al. (2016), Packard et al. (2019), and the 2024 *Artificial Life* special issue show that these emphases lead to different tests. A system can satisfy one while failing another. For example, an unbounded descriptor space can yield continuous novelty without adaptive innovation; a hand-authored task ladder can yield cumulative performance without endogenous ecological expansion.

The term should therefore be treated as an umbrella research program, not a binary property conferred by one score.

## 2.2 Three useful strengths of an OEE claim

Genesis should distinguish:

### Weak or exploratory OEE evidence

- Novel variants continue to appear over the tested horizon.
- Diversity does not visibly plateau.
- Some innovations persist or alter outcomes.
- Results may depend on a single descriptor, external task, or limited number of seeds.

This is valuable exploratory evidence, but it does not establish sustained adaptive progress.

### Intermediate or bounded open-endedness evidence

- Multiple preregistered novelty and complexity measures fail to saturate over expanding budgets.
- A substantial fraction of novelty is adaptive or ecologically consequential.
- Results replicate across independent worlds and survive representation changes and mechanism ablations.
- The system creates some new selection pressures or dependencies internally.

The responsible language remains “within the tested horizons and representations.”

### Strong OEE evidence

- Repeated, persistent adaptive novelty.
- Endogenous expansion of ecological and behavioral possibility.
- Sustained growth in evolvability or organization.
- Major transitions or new inheritance channels.
- No detected ceiling across successively expanded resources, durations, populations, and measurement representations.

Even this is finite evidence, not a proof of literal unboundedness.

## 2.3 What leading systems demonstrate—and what they do not

| System or line of work | Demonstrated contribution | Important limitation for Genesis interpretation |
|---|---|---|
| **Tierra** (Ray, 1991) | Self-replicating digital organisms, parasitism, ecological interactions, and evolutionary dynamics in executable code. | Long-run innovation and complexity often stagnate; instruction set and memory ecology strongly structure possibilities. |
| **Avida** (Ofria & Wilke, 2004; Lenski et al., 2003) | Rigorous digital-evolution experiments, lineage reconstruction, complex feature evolution, and repeatable adaptation. | Many celebrated gains rely on externally specified computational tasks and rewards; excellent evolutionary evidence does not equal unrestricted OEE. |
| **Geb** (Channon, 2019) | Evidence that maximum individual complexity can scale with world size and that several OEE tests can be met. | Findings are system- and metric-specific; broad ecological, cultural, and technological open-endedness is not established. |
| **Novelty search** (Lehman & Stanley, 2011) | Objective-free search can escape deceptive fitness landscapes by rewarding behavioral novelty. | Novelty depends on the observer-chosen behavior descriptor and archive policy; it can reward irrelevant or trivial variation. |
| **Quality diversity / MAP-Elites** | Illuminates diverse high-performing solutions across a chosen feature map. | Feature dimensions and quality function are externally authored; coverage is bounded by the archive representation. |
| **POET and Enhanced POET** (Wang et al., 2019, 2020) | Co-generates challenges and solutions, transfers solutions, and produces stepping stones not found by direct optimization. | Environment representation, minimal criterion, transfer policy, and progress measures are engineered; it is a strong open-ended-search result, not a complete artificial biosphere. |
| **Genelife** (Packard & McCaskill, 2024) | Reports continuing genetic and spatial innovation in a cellular-automaton evolutionary system and is analyzed against modern OEE tests. | The authors themselves distinguish observed innovation from the richer functional novelty of biology; functional and ecological interpretation remains limited. |
| **Coevolutionary Avida studies** (Zaman et al., 2014) | Show that ecological interactions can promote complexity and evolvability under controlled digital evolution. | Still bounded by the digital chemistry, tasks, and selected ecological setup; coevolution does not guarantee indefinite innovation. |
| **Microcosmos** (2026 preprint/platform paper) | Demonstrates a modern GPU-oriented platform direction for scaling artificial-life worlds and explicitly targets future OEE research. | A scalable platform is not itself evidence that open-ended evolution occurred. Its claims should be treated as emerging until peer-reviewed empirical results exist. |

### Bottom-line assessment

No system is broadly accepted as having demonstrated all of the following together: indefinitely sustained adaptive novelty, endogenous expansion of niches, growing behavioral and ecological organization, major transitions, cumulative material culture, and representation-robust evidence across extensive independent replications. The field has important partial demonstrations, not a solved recipe.

## 2.4 Why systems tend to stagnate

Recurring failure modes include:

- **Static niches:** agents exhaust the useful behaviors implied by a fixed environment.
- **Fixed interfaces:** unchanging sensors, actuators, and object semantics limit reachable phenotypes.
- **External objectives:** agents optimize the supplied task rather than generate new problems and dependencies.
- **Weak ecological feedback:** organisms do not persistently alter selection pressures for descendants.
- **Insufficient heredity:** innovations cannot be retained with enough fidelity.
- **Too much heredity:** high-fidelity copying without variation, recombination, or changing selection produces lock-in.
- **Limited developmental organization:** genome-to-phenotype mappings cannot express scalable modular structures.
- **Selection for short-term replication:** costly exploratory, social, or constructive behaviors are eliminated before their delayed benefits appear.
- **Population bottlenecks and extinction:** rare innovation lineages disappear before ecological establishment.
- **Metric ceilings:** the observer's descriptor saturates even when other changes continue—or continues indefinitely by counting noise.
- **Computational ceilings:** finite memory, population, object counts, map extent, precision, and run duration impose practical bounds.
- **Measurement blindness:** genuine ecological or behavioral novelty occurs outside the chosen representation.

These are hypotheses to test, not reasons to add scripted eras or goals. The correct response is to enlarge affordances and inheritance channels only when justified, then use ablations to determine whether they change evolutionary dynamics.

## 2.5 Finite-state and finite-evidence limits

An exactly represented Genesis build has a finite amount of state if all of the following are bounded: world size, object count, population, memory per organism, controller size, scheduler queue, integer widths, and storage. A deterministic finite-state system must eventually revisit a state and then repeat. This does not make the system scientifically uninteresting; the recurrence horizon may be vastly beyond feasible computation. It does mean that “mathematically unbounded” is the wrong empirical target unless the model can dynamically expand its state space without a fixed ceiling.

Even with a dynamically extensible state, a finite run can only show an observed trajectory. Formal results on unbounded evolution and undecidability emphasize that general detection can itself be fundamentally limited. Genesis should therefore separate:

- **Formal capacity:** whether the model specification permits state-space or representational expansion.
- **Empirical realization:** whether runs actually produce sustained adaptive novelty.
- **Measurement capacity:** whether the event and state representation can detect it.
- **Compute horizon:** how long and broadly the claim was tested.

> **Major conclusion record 3 — Existing systems provide partial, not conclusive, OEE demonstrations.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **B.** Multiple primary systems papers and field reviews agree on persistent definitional and empirical gaps. |
> | **Applicability to Genesis** | Direct warning against treating a visually rich run or one growing metric as proof. |
> | **Uncertainty** | OEE criteria remain contested; future work may change the assessment of particular systems. |
> | **Recommended action** | Frame Genesis as a platform for controlled tests of candidate mechanisms, not as a presumed route to civilization or unrestricted OEE. |
> | **Measurement method** | Replicated, preregistered metric panels; causal mechanism ablations; expanding-budget saturation tests; independent analysis replication. |
> | **Control or ablation** | Neutral/frozen baselines, external-task positive controls, ecological-feedback ablations, and descriptor alternatives. |
> | **Determinism implications** | Exact replay is a methodological advantage: detector disagreements and mechanism claims can be re-examined on identical histories. |
> | **Compute implications** | More compute extends the tested horizon but does not remove representational or mechanistic ceilings and does not, by itself, create OEE. |

> **Major conclusion record 4 — Literal unboundedness is not a finite-run acceptance criterion.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/B.** Finite-state recurrence is mathematical; empirical and formal OEE literature documents limits of finite observation and detection. |
> | **Applicability to Genesis** | Direct, especially while world, memory, controller, and object capacities are bounded. |
> | **Uncertainty** | Practical recurrence may be irrelevant at attainable timescales; dynamically growing representations complicate formal classification. |
> | **Recommended action** | Report “no detected saturation within X ticks, Y worlds, Z state limits, and these representations.” |
> | **Measurement method** | Model growth curves, breakpoint/saturation alternatives, rolling novelty rates, and replication across expanding horizons. |
> | **Control or ablation** | Synthetic saturating and non-saturating processes; frozen-evolution and random-noise baselines. |
> | **Determinism implications** | Exact state hashes can detect cycles or repeated macrostates, but a hash match must be verified against state identity and policy version. |
> | **Compute implications** | Expanding run budgets improves lower bounds on sustained activity; it never proves an infinite future. |

---

# 3. Definitions and measurement problems

## 3.1 General measurement doctrine

Every Genesis construct should specify:

1. **Object of measurement:** genome, phenotype, controller, behavior sequence, ecological role, artifact, structure, group, lineage, or world.
2. **Representation:** features, distances, temporal window, spatial scale, and normalization.
3. **Counterfactual:** what would have happened without the candidate mechanism or object.
4. **Persistence threshold:** how long or across how many turnovers the phenomenon must remain.
5. **Replication level:** worlds, not observations within one world.
6. **Sensitivity:** alternative representations, thresholds, and detector settings.
7. **Claim ceiling:** descriptive, associational, causal, mechanistic, or bounded-OEE.

A measurement is not neutral merely because it is computed after the run. Observer choices determine which patterns become visible. Offline status prevents behavioral feedback, but it does not eliminate observer bias.

## 3.2 Open-ended-evolution concepts and metric risks

| Construct | Operational meaning for Genesis | Candidate measures | Grounding | Main ways it can be gamed or misread | Observer/external-task dependence | Recommended status |
|---|---|---|---|---|---|---|
| **Open-ended evolution** | Sustained production of adaptive, persistent novelty without detected intrinsic saturation over tested horizons, with endogenous expansion of ecological or behavioral opportunities. | Joint panel of novelty, adaptive consequence, persistence, diversity, complexity, evolvability, ecological expansion, and saturation. | Theoretically motivated but no universal test. | One metric can rise because of noise, genome bloat, or descriptor expansion. | High unless multiple representations and causal controls are used. | Research program, not a binary phase flag. |
| **Novelty** | Difference from previous organisms, behaviors, artifacts, or world states under a declared representation. | Archive distance, nearest-neighbor distance, unseen categorical combinations, sequence distance, graph-edit distance. | Strong as a search/exploration concept. | Random behavior, irrelevant morphology, and arbitrary descriptor dimensions can inflate it. | Very high; distance and descriptor are observer-selected. | Use only with representation sensitivity and adaptive-consequence tests. |
| **Innovation** | A novel variant that changes a preregistered functional, ecological, reproductive, or informational outcome and persists or spreads. | Causal outcome difference; establishment probability; lineage persistence; changed interaction network. | Stronger and more useful than novelty alone. | Post hoc choice of function; transient lucky outcomes; hitchhiking. | Moderate to high. | Core measure; require intervention or strong quasi-experimental evidence. |
| **Complexity** | Organized, causally necessary structure or behavior, not mere size or randomness. | Functional module count; causal graph depth; minimum sufficient controller after pruning; action-sequence conditionality; multi-object dependency depth; logical/compression measures under fixed encoding. | Multiple legitimate theories; no universal scalar. | Genome/controller bloat; incompressible noise; arbitrary parsing; overfitted task scores. | High. | Use a panel and normalize by resources; never equate size with complexity. |
| **Diversity** | Variety and distribution of genotypes, phenotypes, behaviors, ecological roles, or artifacts within/between worlds. | Richness, Shannon/Simpson diversity, Hill numbers, disparity, phylogenetic diversity, occupancy, turnover. | Mature ecological statistics. | Arbitrary bins; neutral microvariation; sample-size effects; counting dead-end variants. | Moderate to high. | Useful supporting measure, not progress. |
| **Evolvability** | Capacity of a lineage or population to generate viable, heritable, and potentially adaptive variation under standardized perturbations. | Distribution of viable descendants; probability/magnitude of beneficial phenotypic change; accessible behavior/role coverage; robustness–innovation tradeoff. | Strong evolutionary grounding. | Depends on mutation operator, assay environment, time horizon, and fitness proxy. | Moderate; assay is designed by observer. | High-value but expensive standardized assay. |
| **Major transition** | Emergence of a new heritable higher-level unit whose components reproduce or function as an integrated collective with conflict mediation and collective-level persistence. | Collective reproduction, heritable collective traits, bottleneck, division of labor, fitness decoupling, suppression/alignment of lower-level conflict. | Strong evolutionary theory. | Temporary aggregation, spatial clustering, or cooperation can be mislabeled a transition. | Moderate. | Use only with strict conjunctive criteria. |
| **Ecological complexity** | Growth in persistent interaction types, niches, dependencies, feedback loops, and organism–environment construction. | Multiplex interaction-network statistics; niche occupancy; trophic/dependency depth; feedback motifs; response diversity; niche-construction persistence. | Strong concepts, weak universal scalar. | Dense random networks; event-volume effects; arbitrary edge thresholds. | High. | Panel with null networks and abundance controls. |
| **Behavioral complexity** | Rich, context-dependent, organized action that cannot be explained by random variation or controller size alone. | Predictive-state complexity; conditional sequence depth; repertoire under standardized contexts; transfer across contexts; causal lesion sensitivity. | Mixed but useful. | Larger controllers, noisy switching, and rare actions inflate counts. | Moderate to high. | Normalize by opportunity/controller resources; use ablations. |
| **Cumulative adaptation** | Retention and accumulation of heritable gains in endogenous survival, reproduction, resource transformation, or standardized challenge performance. | World-level performance frontier; lineage improvement; retention after perturbation; contribution of successive mutations/learned variants. | Mature in evolutionary experiments. | External task defines direction; environmental easing mimics improvement; mean rises while frontier stagnates. | Moderate to high. | Distinguish endogenous adaptation from bounded external-task performance. |
| **Unboundedness** | Absence of a fixed ceiling in the formal model, or empirically no detected saturation within a declared horizon. | Formal state-space analysis; expanding-budget growth curves; saturation-model comparison; cycle detection. | Strong formal basis; finite empirical proof impossible. | Extrapolating a short trend; expanding a meaningless descriptor; ignoring hard caps. | High for empirical measures. | Report lower bounds and tested horizons, never absolute proof. |
| **Adaptive radiation** | Diversification from common ancestry into multiple persistent ecological roles following opportunity or competitive release. | Lineage branching; role occupancy; trait–environment association; decreased within-role and increased between-role similarity; opportunity timing. | Strong biological grounding. | Genetic clusters without ecological differentiation; transient branching. | Moderate. | Require lineage, niche, persistence, and opportunity evidence. |
| **Evolutionary activity** | Continued introduction, persistence, and turnover of evolutionarily relevant components or organizations. | Bedau-style activity statistics, component persistence, adaptive activity, innovation rate. | Historically important OEE metric family. | Component definition and thresholds dominate; neutral churn can look active. | High. | Supporting panel measure with representation audit. |
| **Ratcheting** | Successive modifications are retained and later improvements depend on earlier socially transmitted changes. | Transmission-depth graph; monotonic or retained functional gains; predecessor-removal loss; inability of naive agents to recreate endpoint within matched effort. | Strong cumulative-culture concept. | Genetic adaptation, easier environments, repeated reinvention, or selective reporting of best chains. | Moderate. | High threshold; require causal dependency and social transmission. |

## 3.3 Recommended OEE metric panel

No metric below is sufficient alone. Genesis should preregister at least one measure from each relevant family and explain omissions.

### A. Novelty production

- Rate of first-seen phenotypic, behavioral, ecological, and artifact variants.
- Distance from a historical archive under at least three preregistered representations.
- Novelty after coarsening and after removal of low-consequence dimensions.
- Novelty establishment: fraction persisting for a minimum lineage or ecological duration.

### B. Adaptive consequence

- Change in survival, reproduction, energetic efficiency, resource access, risk reduction, or interaction outcomes.
- Causal effect of possessing/using an innovation in checkpoint-fork or matched trials.
- Spread probability and selection coefficient where identifiable.
- Persistence after the initiating environment or actor disappears.

### C. Diversity and turnover

- Hill-number profiles across multiple orders to distinguish rare-variant richness from dominant-type diversity.
- Phylogenetic or lineage diversity.
- Functional-role diversity.
- Between-world beta diversity and within-world temporal turnover.

### D. Organized complexity

- Minimal sufficient controller size after deterministic pruning or lesion tests.
- Number and depth of causally necessary modules.
- Context-conditioned behavior-sequence complexity relative to shuffled and random-controller baselines.
- Artifact or structure dependency depth.
- Interaction-network motif and hierarchy measures compared with degree-preserving nulls.

### E. Evolvability

At registered checkpoints, clone representative genomes into a standardized assay suite and apply a versioned mutation distribution. Measure:

- Viability fraction.
- Phenotypic dispersion.
- Fraction producing beneficial changes across multiple endogenous and standardized contexts.
- Robustness–innovation profile.
- Accessibility of new roles rather than just local performance gain.

These assays are expensive and observer-designed. They should be sampled from registered checkpoints, not continuously applied in-world.

### F. Ecological expansion

- New persistent interaction types and dependencies.
- New niches that are not mere spatial bins.
- Persistent environment modification that changes descendant selection.
- Coevolutionary response and reciprocal adaptation.
- Expansion of resource-transformation paths or artifact-mediated opportunities.

### G. Saturation and horizon analysis

For every headline metric:

- Plot per-world trajectories and uncertainty, not only pooled means.
- Fit prespecified saturating and non-saturating alternatives.
- Repeat at increasing time, population, map, and controller/object budgets.
- Check whether apparent growth is caused by increasing sample opportunity.
- Report ceilings imposed by integer widths, memory limits, schema limits, map extent, entity caps, and descriptor dimensions.

## 3.4 Representation audit

For every novelty, complexity, tradition, or era detector, publish:

- Input variables and units.
- Spatial and temporal scale.
- Binning or embedding method.
- Distance metric.
- Normalization.
- Missing-data behavior.
- Threshold selection.
- Training/fit data versus held-out data.
- Sensitivity to at least two plausible alternatives.
- Synthetic false-positive and false-negative tests.
- Whether condition labels were blinded during detector development.

A detector that only succeeds after inspecting treatment labels is exploratory. Confirmation requires locking the detector and applying it to new worlds.

## 3.5 Operational definitions for culture and technology

### Social transmission

**Definition:** The probability or hazard that a naive organism acquires a behavioral variant increases because of exposure to another organism's behavior, signal, result, or artifact, beyond what is explained by genotype, shared environment, individual experience, or independent reinvention.

**Required data:** exposure source, time, proximity or sensory availability, demonstrator behavior, learner prior state, learner acquisition time, genotype/lineage, environment, and opportunities for asocial discovery.

**Strong test:** Randomly assign arbitrary donor variants after birth, enable or disable the social-learning channel, match environmental consequences, and test whether adoption follows the donor-specific variant and exposure network.

### Tradition

**Definition:** A behavioral variant that is socially transmitted, shared by a bounded population or network, and persists beyond its originator for a preregistered duration or number of population turnovers.

Mere persistence is not enough; a genetic polymorphism or stable environmental response is not a cultural tradition.

### Cultural inheritance

**Definition:** A causal contribution of socially acquired information to descendant or later-cohort behavior, independent of genetic descent and shared environmental exposure.

This can occur without parent–offspring transmission. “Inheritance” refers to persistence through a social channel.

### Cultural lineage

**Definition:** A temporally ordered chain or network of behavior/artifact variants linked by supported transmission and descent-with-modification relationships.

Because horizontal transfer is expected, cultural lineages are often directed acyclic graphs or reticulate networks rather than trees.

### Ratcheting

**Definition:** Successive socially transmitted modifications retain prior functional gains, and later variants causally depend on earlier ones.

Required evidence includes predecessor ablation or naive reconstruction tests. A sequence of better outcomes is not enough.

### Tool use

**Definition:** An organism exerts control over an unattached or manipulable external object so that the object causally mediates achievement of a goal, changes another object/organism/environment, or obtains information, and the mediation enables or materially improves the outcome.

For Genesis, “goal” need not be symbolic or explicit; it may be inferred operationally from repeated outcome-directed behavior and counterfactual benefit.

### Compound and composite tools

Terminology varies across animal-cognition research. Genesis should declare its convention:

- **Compound tool:** two or more components physically joined into one jointly functional object.
- **Composite tool system:** two or more distinct objects used in coordinated sequence or combination, each contributing to the function.

A pile of objects or serial object contacts is not a composite tool without joint causal function.

### Persistent structure

**Definition:** An organism-caused spatial configuration of terrain or objects that remains after the builder leaves or dies and is later used, maintained, modified, or causally affects outcomes.

### External memory

**Definition:** Persistent environmental or artifact state created or selected by an organism that carries information across time and causally changes later decisions or performance, beyond internal memory and current sensory conditions.

### Technological dependency

**Definition:** A population's performance, survival, reproduction, or access to a resource becomes causally dependent on an artifact, structure, or production sequence such that removing or functionally degrading it causes a preregistered loss that is not rapidly recovered through existing asocial behavior.

### Group-specific behavior

**Definition:** A behavioral distribution differs between persistent social/contact groups after controlling for genetic relatedness, local environment, opportunity, and sampling, and the difference is stable for a preregistered period.

This is descriptive evidence of a practice difference. It becomes cultural evidence only when social transmission is supported.

### Era-like transition

**Definition:** A retrospective, statistically supported change in a multivariate world-level regime—such as interaction topology, artifact dependency, construction, resource transformation, or transmission depth—that persists and is robust to detector choice.

“Era” is an observer label. It must never become an in-world state, objective, reward, unlock, or hidden feature.

## 3.6 Distinguishing competing explanations

| Observed pattern | Competing explanation | Minimum discriminating design |
|---|---|---|
| Learners adopt a behavior after seeing others | Genetic inheritance | Cross-lineage demonstrators, genotype adjustment, randomized post-birth arbitrary variants, or foster-like exposure assignment. |
| Behavior spreads through a group | Shared environment | Yoked environmental controls, spatial covariates, environment shuffling, and exposure-network timing. |
| Similar behavior appears in separate groups | Independent reinvention | Track exact action/object sequences, exposure opportunities, latency, and donor-specific arbitrary variants. |
| Group difference persists | Drift | Replicate worlds, neutral-drift baselines, turnover duration, and social-channel ablation. |
| Object interactions increase | Genuine tool use | Object-removal/function-destroying ablation and demonstrated mediated outcome benefit. |
| Controller gets larger | Behavior becomes more complex | Size-normalized behavioral tests, deterministic pruning/lesions, random-size-matched controllers. |
| A rare run appears culturally rich | Robust mechanism | Prespecified multi-seed endpoint, all-world reporting, held-out detector, and ablation. |
| Performance rises across generations | Cumulative culture | Genetic controls, social-channel ablation, lineage transmission graph, predecessor dependence, and naive/asocial reconstruction assay. |
| Many entities cluster and fight | Coalitionary conflict | Third-party intervention, partner specificity, coordinated timing, common-target evidence, and identity/support ablation. |
| Stable occupancy appears around resources | Territoriality | Boundary-contingent defense/exclusion, site fidelity, intruder response, resource-shuffle control, and density matching. |

> **Major conclusion record 5 — Social transmission must be identified causally, not by geographic or group differences alone.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/B.** Mature animal-culture methodology, network-based diffusion analysis, and experimental social-learning designs. |
> | **Applicability to Genesis** | Direct; Genesis can record exposures, randomize arbitrary variants, and perform exact ablations more cleanly than field studies. |
> | **Uncertainty** | Interference, dynamic networks, and correlated environment can still bias transmission inference. |
> | **Recommended action** | Make randomized arbitrary-variant, social-channel, and yoked-environment experiments the acceptance standard. Use observational diffusion only as supporting evidence. |
> | **Measurement method** | Acquisition hazard or probability conditioned on exposure network, prior state, genotype, environment, and opportunity. |
> | **Control or ablation** | Social learning disabled; demonstrator absent; environment-only playback/yoking; genotype and location matching. |
> | **Determinism implications** | Social exposures must be collected from a common pre-update state and aggregated in canonical order to prevent traversal-order learning effects. |
> | **Compute implications** | Requires exposure-level logs and many independent worlds; dynamic-network models are analytically expensive. |

> **Major conclusion record 6 — Tool and technology claims require functional counterfactuals.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **B.** Tool-use definitions are mature, but mapping them to artificial agents requires explicit operationalization. |
> | **Applicability to Genesis** | Direct once manipulable objects and structures exist. |
> | **Uncertainty** | Inferring an organism's “goal” is observer-dependent; functional mediation is more defensible than mental-state attribution. |
> | **Recommended action** | Define tool use through controlled object mediation and outcome change; use removal, substitution, and function-destroying ablations. |
> | **Measurement method** | Matched opportunity trials, causal outcome difference, sequence reconstruction, reuse by nonbuilders, and dependency depth. |
> | **Control or ablation** | Remove object; preserve appearance but destroy function; provide unmodified alternative; randomize object placement. |
> | **Determinism implications** | Object identities, transformations, ownership/holding, collision outcomes, and causal parent events need stable IDs and canonical logs. |
> | **Compute implications** | Fine-grained object events can dominate log volume; claim-critical interactions must remain lossless. |

---

# 4. Recommended Genesis research claims

## 4.1 Evidence ladder

Genesis should attach every public or internal statement to an evidence level.

| Level | Required evidence | Responsible claim form | Claims not yet justified |
|---|---|---|---|
| **G0 — Implementation verification** | Unit/property tests, deterministic fixtures, exact replay/restore, invariant checks. | “The engine implements version X of mechanism Y and reproduces identical state hashes under the declared determinism tier.” | Any biological or cultural effect. |
| **G1 — Observational example** | One or a few inspected worlds; complete provenance. | “A candidate pattern consistent with X was observed in seed S.” | Prevalence, causality, robustness, culture, or OEE. |
| **G2 — Replicated association** | Preregistered metric across independent worlds; uncertainty reported. | “Under condition A, X occurred in 8/12 worlds versus 0/12 under B.” | Mechanism unless treatment isolates it; generality beyond tested seed distribution. |
| **G3 — Causal treatment effect** | Randomized/blocked treatment, control or ablation, world-level inference, determinism and log gates. | “Enabling mechanism M increased the probability/rate of X by Y within the tested model and horizon.” | Strong mechanism if alternative pathways remain; broad biological equivalence. |
| **G4 — Mechanism-robust result** | Multiple controls, dose response, alternative operationalizations, independent seed registry, checkpoint-fork tests, detector sensitivity. | “Evidence supports M as a contributing mechanism for X in model versions/configurations V.” | Universal necessity/sufficiency; literal open-endedness. |
| **G5 — Bounded open-ended dynamics** | Expanding budgets, multi-metric non-saturation, adaptive consequence, endogenous ecological expansion, independent replication. | “Genesis sustained adaptive novelty without detected saturation across X worlds up to Y ticks under limits Z.” | Infinite/unbounded future, civilization, language, or human-like technology. |
| **G6 — Extraordinary systems-level claim** | Multiple independent teams, cross-version robustness, formal capacity analysis, long horizons, broad representation robustness, transparent artifacts. | Possibly “strong evidence for a form of open-ended evolution under the declared definition.” | Proof of unrestricted evolution or inevitable civilization. |

## 4.2 Claim wording templates

### Descriptive

> In **N** independently initialized worlds under configuration **V**, detector **D** identified candidate persistent object-mediated behavior in **k** worlds over **T** ticks. This result is descriptive and does not establish social transmission or tool function.

### Causal mechanism

> In a preregistered paired-seed experiment with **N** seed blocks, enabling **M** changed the world-level primary outcome by **effect estimate** (95% interval **L–U**) relative to ablation **A**. The result passed restore/replay and event-completeness gates and applies to the tested model, seed distribution, and horizon.

### Null or bounded negative

> The experiment did not detect an effect of **M** larger than the prespecified smallest effect of interest **δ**. The 95% interval was **L–U**; equivalence was/was not supported. This rules out / does not rule out effects of practical importance under the tested conditions.

### Bounded OEE

> Across **N** worlds and horizons from **T1** to **T4**, preregistered adaptive-novelty, ecological-expansion, and organized-complexity metrics showed no detected saturation under representations **R1–Rk**. This is bounded evidence; it does not prove mathematical unboundedness or future continuation beyond the tested limits.

## 4.3 Claims Genesis should avoid

Avoid these unless evidence far exceeds the current project stage:

- “The organisms developed culture” from group differences alone.
- “They learned from each other” from temporal proximity alone.
- “They invented a tool” from carrying or contacting an object.
- “They built a structure” from accidental piles or terrain disturbance.
- “They formed nations/factions” from clustering.
- “They declared war” without an authored declaration mechanism—which Genesis intentionally should not have.
- “They coordinated” from simultaneous action absent evidence of partner-contingent causation.
- “They entered a new era” because a retrospective cluster changed.
- “Complexity increased” because genome/controller/object counts increased.
- “Open-ended evolution was achieved” because novelty remained positive for one duration.
- “More compute will yield civilization.”

## 4.4 Generalization boundaries

Every claim must identify:

- Engine and schema version.
- Configuration and allowed affordances.
- Seed distribution or exact registry.
- World-size, population, memory, controller, and object limits.
- Run horizon and stopping rule.
- Hardware and deterministic execution tier.
- Detector representation and thresholds.
- Treatment and ablation.
- Whether the effect replicated under an independent seed registry or model version.

A seed is not merely a nuisance parameter. It indexes a sampled initial condition and random history under a declared generator. Results generalize only to the seed-generating process and configuration that were actually sampled.

> **Major conclusion record 7 — Genesis needs an explicit claim ladder and should promote claims one level at a time.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Causal-inference and replication principles are mature; the Genesis ladder is a project-specific synthesis. |
> | **Applicability to Genesis** | Direct. Emergent systems invite anthropomorphic overstatement and anecdotal selection. |
> | **Uncertainty** | Boundaries between descriptive categories can remain contestable even with strong data. |
> | **Recommended action** | Tag every result G0–G6, use standardized wording, and require a new confirmatory design before promotion. |
> | **Measurement method** | Evidence audit against preregistration, replication, control, effect, robustness, determinism, and horizon requirements. |
> | **Control or ablation** | Claim-specific; no causal label without a mechanism-isolating comparison. |
> | **Determinism implications** | Exact replay enables rigorous reclassification, but replay alone does not convert description into causality. |
> | **Compute implications** | Higher claim levels require more independent worlds, longer horizons, and broader sensitivity—not just denser within-world observation. |


---

# 5. Experimental-unit and replication guidance

## 5.1 The unit follows treatment assignment

The experimental unit is the smallest unit independently assigned to a treatment and capable of producing an independent treatment contrast. For most Genesis experiments, the condition changes a world-level rule or affordance: plasticity is enabled, social observation is available, artifacts persist, terrain is mutable, or identity memory exists. Organisms within that world interact, share ancestry, alter one another's environment, and inherit consequences from prior generations. They are therefore not independent treatment replicates.

### Default hierarchy

```text
Experiment
└── Seed block / independently initialized world  ← usual replicate
    ├── Population and ecological history
    │   ├── Lineages / groups
    │   │   ├── Organisms
    │   │   │   ├── Lifetimes / episodes
    │   │   │   └── Events / ticks / observations
    │   └── Artifacts, structures, terrain regions, and networks
    └── Repeated checkpoints and analysis windows
```

Multiple organisms improve measurement precision **within a world**. They do not create additional independent worlds. Likewise, ten million events from one seed do not provide the inferential strength of many independently initialized worlds.

## 5.2 Why organism-level analysis is usually invalid as replication

Organisms share:

- Initial world and resource configuration.
- Common evolutionary history.
- Genealogical dependence.
- Environmental modifications made by earlier organisms.
- Epidemic, social, and ecological interactions.
- Shared artifacts and structures.
- Global shocks and density changes.
- Selection pressures generated by the population itself.

This violates simple IID assumptions. Treating each organism as independent usually underestimates uncertainty and converts one contingent history into an apparently decisive result.

### When an organism could be an experimental unit

An organism-level experiment is possible when all of the following are substantially true:

1. The intervention is randomized independently at organism level.
2. The outcome is defined over a bounded assay interval.
3. Other organisms' assignments do not materially affect the focal outcome, or interference is explicitly modeled.
4. The same shared world does not create an unaccounted common treatment history.
5. The analysis clusters by world and accounts for exposure spillovers.

A cloned-organism assay in isolated arenas can meet these conditions. A social-learning experiment in a shared ecology usually cannot.

## 5.3 World-level estimands

Define the quantity Genesis wants to estimate before choosing a model. Examples:

- Difference in the probability that a world develops a persistent tradition by tick **T**.
- Ratio of world-level rates of nonbuilder artifact reuse.
- Difference in median recovery time after within-lifetime environmental shifts.
- Difference in restricted mean time to extinction.
- Difference in the slope of adaptive-novelty accumulation over a registered interval.
- Probability that an innovation lineage survives at least **g** population turnovers.

The estimand should be meaningful at the world level even when calculated from organism events.

## 5.4 Repeated observations and hierarchical models

Repeated measurements can increase precision and describe dynamics, but the model must preserve dependence. Appropriate structures include:

- Random intercepts for worlds.
- Random slopes for treatment response over time.
- Seed-block effects for paired conditions.
- Lineage or group effects nested within worlds.
- Autocorrelation or state-space residual structure.
- Frailty terms for survival outcomes.
- Cluster bootstrap resampling of worlds or seed blocks.

A hierarchical model does not create independence that the design lacks. With very few worlds, complicated random-effects estimates can be unstable. In that case, preregister simple per-world summaries, exact or permutation methods at the seed-block level, and transparent plots of every world.

## 5.5 Seed blocking and paired worlds

A paired-seed design runs the same registered root seed under treatment and control. This can reduce variance when the two outcomes remain positively correlated. It is especially useful when initial geography, founder genomes, or resource patterns drive large between-seed variation.

However, “same seed” is not automatically a valid common-random-number design. A mutable stateful RNG consumed in call order will diverge as soon as one condition changes the number or order of draws. Subsequent draws then refer to different semantic events. Genesis should pair worlds through **event-keyed or counter-based random draws**, not merely through equal initial RNG state.

Paired designs should report:

- The pairing key and assignment.
- Correlation of paired outcomes.
- Paired effect and interval.
- An unpaired sensitivity analysis.
- Any seeds where one condition becomes undefined or terminates.

If pair correlation is negative, pairing can reduce precision. Do not assume benefit.

## 5.6 Extinction, censoring, and failed execution

These categories must not be merged.

| Event | Statistical status | Recommended treatment |
|---|---|---|
| **Natural extinction caused by simulation dynamics** | Scientific outcome or competing event. | Retain. Analyze directly, as a competing risk, or through a composite estimand. Do not exclude as “failed run.” |
| **Administrative end of planned horizon** | Right censoring for time-to-event outcomes. | Mark exact censoring tick and use survival methods. |
| **Resource-budget termination specified in protocol** | Administrative censoring or estimand-defining truncation. | Report and model as preregistered. |
| **Software crash with valid checkpoint recovery** | Execution incident, not scientific event. | Resume deterministically; document incident and verify hash equivalence. |
| **Software crash without recoverable state** | Missing experimental unit or invalid execution. | Do not silently rerun under a new seed. Apply preregistered retry/replacement policy and report. |
| **Corrupt log or snapshot** | Evidence failure. | Mark world invalid for affected claims unless lossless reconstruction is verified. |
| **Manual termination after seeing outcome** | Informative stopping and protocol violation. | Report prominently; exclude from confirmation or analyze only as exploratory. |

Natural extinction can also mediate treatment effects. For example, a learning mechanism might improve task performance among surviving worlds while increasing extinction risk. Both outcomes must be reported.

## 5.7 Checkpoint-fork experiments

Exact restoration gives Genesis an unusually strong causal tool. At a registered checkpoint, the engine can fork an identical world into interventions such as:

- Remove a candidate artifact.
- Disable a learned weight update.
- Replace a structure with a visually similar but nonfunctional object.
- Remove a demonstrator.
- Scramble social identities.
- Prevent third-party intervention.

The immediate fork comparison controls the complete prior history. It is therefore powerful for **proximal causal effects**. It does not create an independent world-level replicate. A robust claim requires the same fork intervention across many independently generated source worlds or checkpoints selected by a preregistered rule.

Checkpoint selection after visual inspection creates selection bias. Discovery forks may nominate a mechanism; confirmation must use a locked trigger rule on new worlds.

## 5.8 Interference is part of the phenomenon

Standard causal inference often assumes one unit's treatment does not affect another unit's outcome. Social and ecological Genesis experiments intentionally violate this. The solution is not to pretend interference is absent; it is to define treatment at the level at which interference is contained, usually the world, or to specify an exposure mapping.

Examples of valid exposure mappings:

- Number or duration of observed demonstrations before acquisition.
- Fraction of neighbors carrying a behavioral variant.
- Weighted network exposure to an artifact practice.
- Presence of an identity-known coalition partner.
- Local density of a structure-modifying behavior.

These mappings must be registered before confirmatory analysis and tested for sensitivity.

> **Major conclusion record 8 — Nested observations cannot substitute for independent worlds.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A.** Pseudoreplication, clustered data, and interference theory are established. |
> | **Applicability to Genesis** | Direct and pervasive. Organisms coevolve and alter shared state. |
> | **Uncertainty** | The efficient level of summarization depends on outcome and world count. |
> | **Recommended action** | Make world/seed-block outcomes the default; use hierarchical detail only with enough worlds and validated models. |
> | **Measurement method** | Per-world estimands, seed-block contrasts, cluster bootstrap, hierarchical models, and all-world plots. |
> | **Control or ablation** | Independent matched worlds; checkpoint forks only as repeated within-history interventions. |
> | **Determinism implications** | Exact forks strengthen proximal causality but must preserve complete scheduler, RNG, and event-cursor state. |
> | **Compute implications** | Statistical power is purchased mainly with independent worlds. Scaling architecture should optimize world throughput. |

---

# 6. Multi-seed experimental design

## 6.1 Separate exploration, calibration, and confirmation

A disciplined Genesis study has at least three stages:

### Stage A — Engineering and detector development

Purpose: find crashes, tune logging, test fixtures, explore candidate metrics, and build detectors. Seeds and outcomes may be inspected freely, but results are not confirmatory.

### Stage B — Pilot and power calibration

Purpose: estimate runtime, event volume, extinction, variance, pair correlation, rare-event frequency, and plausible effect distributions. The primary endpoint and detector should be selected by the end of this stage. Pilot worlds should not be reused as confirmatory evidence unless that reuse was explicitly planned with valid error control.

### Stage C — Locked confirmation

Purpose: test a preregistered hypothesis on a new, immutable seed registry. No outcome-based parameter tuning, detector revision, exclusion change, or stopping change is permitted without labeling the analysis exploratory.

For major claims, add:

### Stage D — Independent replication

Use a new seed registry, a separately built binary or machine, and preferably an independently implemented analysis pipeline. Cross-version replication tests whether the result survives reasonable implementation changes.

## 6.2 Seed registry and assignment

A seed registry should contain:

- Registry identifier and cryptographic digest.
- Seed-generation procedure and root entropy provenance.
- Ordered seed list.
- Condition assignment and blocking variables.
- Whether seeds were ever executed before lock.
- Replacement policy for unrecoverable execution failure.
- Maximum number of worlds and batches.

Recommended practice:

1. Generate more candidate seeds than required using a documented procedure.
2. Lock the candidate list before outcomes are observed.
3. If blocking on pre-treatment properties, compute only properties that cannot be influenced by treatment—for example, initial terrain summary or founder-genome summary.
4. Randomize condition order within blocks.
5. Keep the seed-to-condition table immutable.
6. Never remove “boring” or extinct seeds because they weaken the effect.

The seed distribution defines the population of initial conditions to which the inference applies. A handpicked dramatic seed has no population-level interpretation.

## 6.3 Paired-seed comparisons

Use paired seeds when:

- Initial conditions explain substantial variance.
- Treatment does not redefine the meaning of the initial state.
- Randomness is keyed to semantic events.
- The primary estimand is defined in both conditions.

Analyze the per-pair contrast. For binary outcomes, use an exact paired test or hierarchical paired logistic model. For continuous outcomes, use paired differences, robust intervals, or a hierarchical model with seed-block effects.

Do not pair by reusing an entire mutable random stream whose consumption diverges. That creates superficial seed equality rather than aligned random causes.

## 6.4 Ablations

An ablation should remove or disable the proposed mechanism while preserving other opportunities as much as possible.

Good ablations are:

- **Specific:** remove social weight updates, not all perception.
- **Matched:** preserve computational cost or sensory input when possible.
- **Versioned:** encoded as a configuration policy, not an ad hoc patch.
- **Interpretable:** define what is lost and what remains.
- **Checked:** verify through events that the ablated mechanism was actually inactive.

A broad ablation can show that a package matters but not which component caused the effect. Use a sequence:

1. Package ablation.
2. Component ablations.
3. Dose response.
4. Rescue or restoration condition when feasible.

## 6.5 Factorial designs

Factorial designs are valuable because mechanisms may interact. A 2×2 design with social transmission and artifact persistence can distinguish:

- Main effect of social transmission.
- Main effect of persistence.
- Interaction: persistence is useful only when information can spread, or social transmission is useful only when artifacts remain.

For **k** binary factors, a full `2^k` design becomes expensive. Use screening designs during exploration and reserve confirmatory worlds for a small set of theoretically important interactions. Do not interpret a missing main effect when a strong interaction is present without examining conditional effects.

## 6.6 Dose-response experiments

Binary on/off tests can conceal thresholds and nonlinearity. Candidate doses include:

- Learning rate or plasticity half-life.
- Observation range or fidelity.
- Artifact durability.
- Resource patch persistence.
- Memory capacity.
- Mutation rate.
- Identity-recognition noise.
- Construction cost.

Use at least three nonzero levels plus an ablation where feasible. Predefine whether the expected relationship is monotonic, saturating, U-shaped, or unknown. Analyze with contrasts or a model flexible enough to detect nonlinearity without uncontrolled researcher degrees of freedom.

A dose response supports mechanism but does not prove it; correlated side effects such as CPU budget, event count, or survival opportunity must be measured.

## 6.7 Control environments

### Negative controls

A negative control should be affected by the same biases but not by the proposed mechanism.

Examples:

- A behavior variant that cannot be observed by others.
- A visually similar object with no causal function.
- Exposure occurring after acquisition.
- A social network edge that is temporally impossible as a transmission route.
- A shuffled identity label that preserves density and encounter rate.
- A detector applied to time-reversed or phase-randomized data.

### Positive controls

A positive control verifies that the experiment and analysis can detect an effect when one is known to exist.

Examples:

- A deliberately simple within-lifetime contingency that a known learning controller can solve.
- A direct and explicit information channel in a test-only fixture.
- A seeded persistent artifact practice with known provenance.
- A synthetic change point inserted into derived analysis data.

Positive controls must not be confused with target phenomena. A scripted training fixture proves detector sensitivity, not emergent culture.

### Yoked controls

A yoked world or organism receives the same environmental outcomes or resource events as a treatment counterpart without the hypothesized information channel. This helps separate learning from shared reward or ecological change. Yoking must be defined carefully because exact cross-world event matching may become impossible after trajectories diverge; a standardized assay is often cleaner.

## 6.8 Preregistration content

A Genesis preregistration should lock:

- Claim and evidence level sought.
- Model/build/configuration versions.
- Hypothesis and causal diagram or mechanism statement.
- Treatment, control, and ablation.
- Experimental unit and seed registry.
- Primary endpoint and exact computation.
- Secondary and exploratory endpoints.
- Smallest effect size of interest (SESOI).
- Run duration and event exposure requirements.
- Extinction, censoring, crash, and replacement policy.
- Maximum worlds and batch sizes.
- Interim looks and stopping rule.
- Statistical model and diagnostic/sensitivity plan.
- Multiple-testing family and correction.
- Detector versions and thresholds.
- Determinism, restore, logging, and performance gates.
- Conditions that make the experiment invalid.

A preregistration is not useful if it says only “we will test whether culture emerges.”

## 6.9 Sequential testing and stopping

Long runs make sequential designs attractive, but repeatedly checking ordinary p-values inflates false positives. Valid options include:

- Group-sequential designs with O'Brien–Fleming-like conservative early boundaries.
- Lan–DeMets alpha-spending functions when analysis times vary.
- Anytime-valid confidence sequences or e-values for compatible endpoints.
- Bayesian sequential decisions with preregistered priors, utility/loss, and stopping thresholds.

Recommended Genesis policy:

- Interim decisions occur only after completing registered batches of independent worlds, not after arbitrary ticks from one world.
- Stop for efficacy only under a valid boundary and only if determinism/log/performance gates also pass.
- Stop for futility using a registered conditional-power or posterior-predictive rule.
- Preserve a minimum world count before any scientific stop so estimates are not dominated by early lucky seeds.
- A world can have an internal duration stopping condition, but that condition must be defined before the run and included in the estimand.

## 6.10 Adaptive experimentation

Adaptive search can efficiently identify promising environments or mechanism doses. It also biases ordinary estimates because conditions are selected based on observed performance.

Use two ledgers:

- **Discovery ledger:** adaptive condition proposals, all outcomes, and search policy.
- **Confirmation ledger:** untouched seed registry and locked conditions.

Do not report the best adaptively discovered condition's discovery performance as an unbiased confirmatory effect. Re-estimate it on new worlds.

> **Major conclusion record 9 — Discovery and confirmation must use separate seed evidence.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A.** Selective inference, adaptive experimentation, and preregistration principles are mature. |
> | **Applicability to Genesis** | Direct because model and detector tuning can easily overfit a finite seed set. |
> | **Uncertainty** | Valid adaptive-inference methods exist, but their complexity and assumptions may not fit every endpoint. |
> | **Recommended action** | Use open exploration and pilots, then lock a new seed registry for confirmation. Reconfirm adaptively selected conditions. |
> | **Measurement method** | Immutable experiment manifests, separate discovery/confirmation ledgers, and analysis of all attempted conditions. |
> | **Control or ablation** | Fresh seeds and fixed conditions; independent detector application. |
> | **Determinism implications** | Exact replay permits extensive exploration, increasing overfitting risk unless seed reuse is tracked. |
> | **Compute implications** | Confirmation duplicates some compute but is necessary to distinguish discovery from evidence. |

---

# 7. Statistical methods

## 7.1 Multi-seed statistical-analysis framework

The following is the recommended default framework.

### Step 1 — Define the estimand

Examples:

- `ΔP = P(tradition by T | social enabled) − P(tradition by T | social ablated)`.
- `log(IRR)` for nonbuilder artifact-reuse events per eligible artifact-time.
- Difference in restricted mean time to extinction through tick **T**.
- Difference in per-world median adaptation regret after contingency changes.

The estimand should state whether extinction is part of the outcome, a competing event, or an administrative endpoint.

### Step 2 — Define the SESOI

The smallest effect size of interest should be scientifically meaningful, not chosen after seeing variance. Examples:

- 15 percentage-point increase in worlds with a tradition.
- 20% reduction in recovery time.
- Rate ratio of 1.5 for socially exposed acquisition.
- At least four linked, retained cultural innovations and 20% performance gain over an asocial frontier.

### Step 3 — Lock independent worlds and blocking

Specify seed registry, pairing, blocking variables, condition assignment, maximum sample, batch sizes, and replacement rules.

### Step 4 — Validate outcome computation

Use synthetic fixtures, hand-audited traces, known positive/negative controls, and deterministic detector tests. The outcome calculator must be versioned independently of the simulation.

### Step 5 — Inspect only registered diagnostics

Diagnostics include event completeness, exposure opportunity, extinction, pair correlation, residual behavior, and model convergence. Diagnostics should not become a license to switch endpoints opportunistically.

### Step 6 — Estimate a world-level effect

Report the point estimate, interval, all-world distribution, and absolute scale. Relative effects without baseline rates can mislead.

### Step 7 — Account for clustering and time

Use seed-block contrasts, hierarchical models, cluster bootstrap, survival analysis, or time-series models as appropriate. Never use event count as the sample size.

### Step 8 — Apply multiplicity policy

One confirmatory primary endpoint is preferable. Use Holm control for a small confirmatory family and Benjamini–Hochberg false-discovery control for explicitly exploratory detector panels.

### Step 9 — Perform registered robustness checks

- Alternative reasonable outcome threshold.
- Unpaired analysis for paired design.
- Robust summary for heavy tails.
- Extinction handling alternatives.
- Alternative representation/detector.
- Exclusion of no-opportunity worlds only if an opportunity-conditioned estimand was preregistered; otherwise retain.
- Hardware/execution stratum.

### Step 10 — Apply science and engineering gates

A statistical effect does not pass if replay hashes disagree, the event log is incomplete, the implementation is invalid, or performance makes the intended experimental program infeasible.

### Step 11 — Classify the result

- Accepted at the prespecified claim level.
- Rejected because the effect is below the threshold.
- Inconclusive because uncertainty remains wide.
- Invalid because implementation/evidence gates failed.
- Exploratory because protocol changed.

### Step 12 — Publish the complete evidence package

Include per-world data, not only aggregated plots.

## 7.2 Effect sizes

Prefer effect measures tied to the estimand:

| Outcome | Preferred effect measures |
|---|---|
| Binary world event | Risk difference, risk ratio, odds ratio as secondary; number of worlds should always be shown. |
| Count or event rate | Rate difference and incidence-rate ratio with explicit exposure denominator. |
| Positive skewed continuous | Median difference, ratio of geometric means where valid, robust location shift, quantile effects. |
| Time to event | Survival probability difference at T, hazard ratio cautiously, and restricted mean survival-time difference. |
| Longitudinal trajectory | Difference in prespecified area under curve, slope over a registered interval, or time-to-threshold. |
| Network/complexity metric | Standardized and raw difference, plus null-network calibration and representation sensitivity. |
| Paired worlds | Distribution and mean/median of within-seed differences. |

Standardized effect sizes alone can obscure scientific meaning when between-seed variance changes. Always report raw units.

## 7.3 Confidence intervals and uncertainty

Use intervals that respect design:

- Exact or score intervals for small binary samples.
- Paired intervals for seed-block differences.
- Cluster bootstrap resampling worlds/blocks, not organisms or events.
- Profile-likelihood or robust intervals for count models.
- Bootstrap or Bayesian intervals for medians and heavy-tailed statistics.
- Simultaneous intervals when multiple confirmatory contrasts are interpreted together.

With small world counts, asymptotic cluster-robust standard errors can be unreliable. Favor exact randomization tests where assignment supports them, small-sample corrections, or transparent Bayesian hierarchical models with prior sensitivity.

## 7.4 Bayesian analysis

Bayesian methods are useful for rare events, hierarchical structure, and direct probability statements, but they do not eliminate design problems.

Recommended elements:

- Prior predictive simulation before observing confirmatory outcomes.
- Weakly informative or domain-informed priors at the world level.
- Hierarchical partial pooling across seed blocks, configurations, or versions only when exchangeability is defensible.
- Posterior probability that the effect exceeds the SESOI.
- Region of practical equivalence (ROPE) for bounded null claims.
- Posterior predictive checks for extinction, tails, zero inflation, and temporal structure.
- Sensitivity to plausible priors.

A possible decision rule is:

> Accept the scientific endpoint when `P(effect > SESOI | data) ≥ 0.95`, all registered robustness checks support the direction, and all engineering gates pass.

This threshold is a policy choice, not a universal theorem.

## 7.5 Survival and competing-event analysis

Use survival methods for:

- Time to extinction.
- Time to first innovation.
- Time to tradition establishment.
- Artifact or structure lifetime.
- Cultural-lineage persistence.
- Time to loss of a practice.

Recommended outputs:

- Kaplan–Meier curves by condition for descriptive display.
- Restricted mean survival time through a registered horizon.
- Cox or accelerated-failure-time models with seed-block terms when assumptions are adequate.
- Frailty or hierarchical survival models for nested entities.
- Competing-risk cumulative incidence when extinction prevents an innovation event.

Do not call natural extinction “censoring.” If a world goes extinct before a tradition can emerge, the interpretation depends on the estimand:

- For “probability a world develops a tradition by T,” extinction is a non-event outcome.
- For “time to tradition among worlds that remain viable,” conditioning on survival can induce selection bias and must be labeled.
- For mechanistic acquisition among eligible organisms, extinction can terminate exposure and should be handled in the exposure process.

## 7.6 Time-series and longitudinal analysis

Genesis trajectories are adaptive and nonstationary. Useful methods include:

- Prespecified area under a curve.
- Piecewise slopes over biologically meaningful intervals.
- Generalized additive mixed models with world-level random effects.
- State-space models for noisy latent trends.
- Autoregressive residual structures.
- Functional data summaries.
- Change-point analysis as a separate offline detector, not as an automatic causal endpoint.

Avoid fitting a simple regression to every tick and treating the number of ticks as the sample size. The uncertainty about a treatment effect is primarily between worlds.

## 7.7 Heavy-tailed outcomes

Open-ended and rare-innovation systems can produce highly skewed distributions: most worlds stagnate, a few explode in diversity or survive much longer. Means can then be unstable but scientifically important.

Report:

- Every world or a complete compact plot.
- Median and interquartile range.
- Mean and a robust interval.
- 10th/90th or 5th/95th quantiles when sample supports them.
- Tail probability above a prespecified meaningful threshold.
- Trimmed mean as a sensitivity analysis, not a way to delete genuine breakthroughs.
- Log-scale display where appropriate, with zeros handled explicitly.

A rare dramatic world should not be discarded automatically. It may be the phenomenon of interest. The design must distinguish “probability of breakthrough” from “average performance conditional on breakthrough.”

## 7.8 Rare-event analysis

For outcomes such as coalitionary conflict or cumulative culture:

- Use exact binomial or beta-binomial models for world-level occurrence.
- Model eligible exposure, not raw duration alone.
- Consider hurdle models: first whether an event occurs, then its frequency/strength.
- Use bias-reduced logistic regression for sparse covariate models.
- Prefer sequential batches with valid stopping over enormous fixed samples chosen by guesswork.
- Report the upper confidence bound when zero events occur. Zero of **N** does not mean impossible.

For zero events in **N** independent worlds, the rough “rule of three” gives an approximate 95% upper bound of `3/N` for the event probability under simple assumptions. Exact intervals are preferable.

## 7.9 Hierarchical models

A generic hierarchy may include:

- Seed-block effect.
- World effect.
- Lineage/group effect nested in world.
- Organism effect nested in lineage/world.
- Time or cohort effects.
- Configuration/version effect.

Use partial pooling to stabilize noisy world estimates, not to hide world heterogeneity. Report the between-world variance and posterior/predictive distribution for a new seed. A large average effect with large sign variation across seeds may be less useful than a smaller consistent effect.

## 7.10 Multiple hypotheses

Recommended hierarchy:

1. **One confirmatory primary endpoint.** No correction beyond the registered design if only one test determines acceptance.
2. **Small confirmatory family:** Holm's step-down family-wise error control.
3. **Exploratory high-dimensional detector family:** Benjamini–Hochberg false-discovery control, with all hypotheses and q-values reported.
4. **Adaptive detector search:** separate held-out confirmation; correction alone is not enough when the feature space was repeatedly revised.

Secondary endpoints should describe mechanism and tradeoffs. They should not substitute for a failed primary endpoint.

## 7.11 Sequential and adaptive inference

For group-sequential frequentist experiments, define:

- Maximum worlds.
- Number and timing of looks.
- Alpha-spending function.
- Efficacy and futility boundaries.
- Minimum sample before stopping.
- Handling of incomplete pairs.

For Bayesian sequential experiments, define:

- Prior.
- Utility or loss.
- Posterior efficacy threshold.
- Futility threshold.
- Minimum and maximum worlds.
- Simulation-calibrated frequentist operating characteristics.

Sequential decisions should occur at completed independent-world batches. Looking at within-world events can be useful for engineering monitoring but must not trigger scientific stopping unless the design explicitly models that process.

## 7.12 Power analysis

Closed-form power calculations are often inadequate for Genesis because of:

- Heavy tails.
- Extinction and competing events.
- Paired-seed correlation.
- Zero inflation.
- Nonlinear detector thresholds.
- Time-varying exposure.
- Hierarchical dependence.
- Adaptive population dynamics.

Use simulation-based power:

1. Estimate plausible null and alternative parameter ranges from pilots or synthetic generators—not from the confirmatory seeds.
2. Generate world-level outcomes preserving heavy tails, extinction, censoring, and paired correlation.
3. Run the complete planned analysis, including stopping and multiplicity.
4. Estimate power, false-positive rate, interval coverage, Type S error (wrong sign), and Type M error (magnitude exaggeration).
5. Repeat under model misspecification and weaker effects.
6. Choose a range of sample sizes and publish the power curve.

Power should target the SESOI. “80% power for any nonzero effect” is not a meaningful goal.

## 7.13 Null-result interpretation

A nonsignificant result can mean:

- Effect is practically absent.
- Effect exists but is smaller than the SESOI.
- Too few independent worlds.
- Exposure opportunity was insufficient.
- Extinction prevented expression.
- Detector was insensitive.
- Treatment was not actually delivered.
- Between-seed heterogeneity overwhelmed the average.
- Mechanism works only in an untested interaction or dose range.
- Implementation/evidence failure invalidated inference.

Use equivalence tests or Bayesian ROPE analysis to support “meaningfully small.” Otherwise say “not detected,” not “no effect.”

> **Major conclusion record 10 — Power and inference must be simulated at the world level.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Clustered, survival, sequential, and simulation-based power methods are mature; Genesis combination is a synthesis. |
> | **Applicability to Genesis** | Direct because outcomes are non-IID, heavy-tailed, censored, and often rare. |
> | **Uncertainty** | Power estimates depend on pilot assumptions; strong distribution shifts can invalidate them. |
> | **Recommended action** | Simulate the complete analysis under multiple plausible data-generating processes and publish operating characteristics. |
> | **Measurement method** | World- or block-level effect estimates, hierarchical variance, tail probabilities, survival estimands, and interval coverage. |
> | **Control or ablation** | Null generators, synthetic known-effect generators, and negative/positive control experiments. |
> | **Determinism implications** | Power simulation and analysis must use separate RNG domains and versioned code; confirmatory seed outcomes must not tune assumptions. |
> | **Compute implications** | Analysis calibration itself may require many synthetic runs, but these can often use surrogate world-level generators rather than full simulations. |

---

# 8. General phase-acceptance template

This template is intended to be copied into every Genesis phase proposal. Numeric values must be justified by pilot data and simulation-based power. The template is deliberately stricter than “feature works in a demo.”

## 8.1 Acceptance record

### A. Claim and hypothesis

- **Phase name and version:**
- **Requested evidence level (G0–G6):**
- **Primary scientific claim:**
- **Hypothesis:**
- **Causal mechanism or directed acyclic graph:**
- **Claims explicitly out of scope:**

### B. Treatment and comparison

- **Treatment condition:**
- **Control condition:**
- **Mechanism ablation:**
- **Negative control:**
- **Positive control:**
- **Dose levels or factorial cells:**
- **Rescue condition, if applicable:**

### C. Experimental unit and sampling

- **Experimental unit:** independently initialized world / seed block unless justified otherwise.
- **Seed registry ID and digest:**
- **Number of worlds:**
- **Blocking and pairing variables:**
- **Condition assignment procedure:**
- **Replacement policy:**
- **Generalization population:** seed generator, model version, and configuration space.

### D. Run exposure

- **Run duration:** ticks, turnovers, lifetimes, or event opportunities.
- **Minimum exposure requirement:** demonstrations, environmental shifts, eligible artifacts, conflicts, or other opportunities.
- **Administrative censoring rule:**
- **Extinction handling:**
- **Crash/restart policy:**
- **Maximum compute budget:**

### E. Outcomes

- **Primary metric and exact computation:**
- **Primary estimand:**
- **Secondary metrics:**
- **Exploratory metrics:**
- **Smallest effect size of interest:**
- **Detector version, representation, and thresholds:**
- **Opportunity denominator:**

### F. Statistical decision rule

Choose one and register it:

- **Frequentist default:** lower bound of a two-sided 95% confidence interval exceeds the SESOI, or a prespecified one-sided test rejects at the alpha-spending-adjusted level.
- **Bayesian default:** posterior probability that the effect exceeds the SESOI is at least 0.95, with prior-sensitivity checks.
- **Equivalence/null acceptance:** interval lies within prespecified equivalence bounds or posterior mass in the ROPE exceeds the registered threshold.

Also specify:

- Analysis model.
- Sequential looks and stopping.
- Multiplicity family and correction.
- Missing/invalid world handling.
- Robustness analyses required to preserve acceptance.

### G. Robustness checks

At minimum:

- Paired and unpaired analysis when paired seeds are used.
- Alternative reasonable outcome threshold or detector representation.
- Heavy-tail robust summary.
- Extinction/censoring sensitivity.
- All-world display.
- Negative and positive control results.
- Independent seed registry for G4+ claims.
- Checkpoint-fork causal assay when relevant.

### H. Failure interpretation

Predeclare categories:

- **Scientific rejection:** effect estimate is below the threshold with adequate precision.
- **Scientific inconclusive:** interval remains too wide.
- **Mechanism failure:** treatment delivered but predicted intermediate mechanism absent.
- **Opportunity failure:** worlds lacked prespecified opportunities; redesign environment, not interpretation.
- **Implementation invalid:** invariant, determinism, or treatment-delivery failure.
- **Evidence invalid:** required logs/snapshots missing or corrupt.
- **Performance invalid:** feature exceeds budget and prevents required replication.
- **Exploratory only:** protocol changed after outcomes.

### I. Performance budget

Use measured reference quantities rather than arbitrary absolute claims:

- **Reference build/configuration:**
- **Reference throughput (`T_ref`):** simulated ticks or agent-updates per wall-clock second.
- **Minimum accepted throughput:** e.g., at least `0.85 × T_ref` for the defined benchmark.
- **Reference peak RSS (`M_ref`):**
- **Maximum peak RSS:** e.g., at most `1.5 × M_ref`.
- **Checkpoint overhead:** e.g., no more than 10% wall-clock at registered interval.
- **Claim-critical logging overhead:** e.g., no more than 15% while preserving zero event loss.
- **Storage budget per world-hour/tick:**
- **Restart recovery objective:**

These values are illustrative placeholders until Genesis benchmarks establish realistic budgets.

### J. Determinism verification

- Same binary, same manifest, repeated run hash equality.
- Uninterrupted versus checkpoint/restore equality.
- Supported thread-count/scheduler variants.
- Supported hardware execution class.
- Event-stream hash equality.
- Snapshot logical-state hash equality.
- RNG-key audit.
- Canonical-order audit.
- Floating-point/fixed-point policy audit.
- Fail-closed corruption and unknown-version tests.

### K. Required artifacts and logs

- Preregistration.
- Experiment manifest.
- Source revision and binary digest.
- Toolchain/dependency manifest.
- Seed registry and assignment table.
- World status ledger.
- Canonical snapshots and event-log segment hashes.
- Per-world primary outcome table.
- Analysis code/environment.
- Detector config and provenance.
- Determinism report.
- Performance report.
- Deviations and incident report.
- Human-readable model description.

## 8.2 Default acceptance logic

A phase is **accepted** only when all of the following are true:

1. The primary scientific decision rule passes.
2. The minimum effect threshold is met, not merely `p < 0.05`.
3. Required causal control or ablation behaves as predicted.
4. Registered robustness analyses preserve the conclusion.
5. Positive and negative controls are valid.
6. Deterministic replay and restore gates pass at the declared tier.
7. Claim-critical event completeness is verified.
8. The performance budget passes.
9. No undisclosed protocol deviation affects interpretation.
10. The claim wording remains within the registered evidence level.

A phase is **not accepted** merely because one seed is compelling, the mean moves in the expected direction, or a secondary endpoint is significant.

> **Major conclusion record 11 — Phase acceptance must combine scientific, evidentiary, deterministic, and performance gates.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Reproducible simulation and confirmatory-design standards are established; the conjunctive Genesis gate is a synthesis. |
> | **Applicability to Genesis** | Direct because a scientific effect is unusable if it cannot be replayed, logged, restored, or scaled to adequate replication. |
> | **Uncertainty** | Exact performance thresholds require empirical benchmark data. |
> | **Recommended action** | Treat acceptance as a signed record with all required fields and no substitution of secondary outcomes. |
> | **Measurement method** | Primary effect/interval plus control checks, hash equality, event completeness, and benchmark results. |
> | **Control or ablation** | Phase-specific mechanism isolation and general positive/negative controls. |
> | **Determinism implications** | Determinism is a gating endpoint, not a descriptive footnote. |
> | **Compute implications** | A feature that prevents sufficient independent replication has not met the research objective even if it functions visually. |


---

# 9. Worked acceptance-criterion examples

## 9.1 How to use the examples

The sample sizes and horizons below are **illustrative planning defaults**, not claims that those numbers guarantee adequate power. Each must be replaced or confirmed by pilot-estimated variance, extinction, opportunity rate, paired-seed correlation, and simulation-based operating characteristics. The examples intentionally use demanding criteria because they are meant to prevent phase promotion from a visually attractive run.

### Shared engineering defaults used in examples

Unless overridden:

- Confirmatory seeds are new and immutable.
- Conditions are paired by semantic, counter-based RNG keys.
- All worlds have a maximum horizon and minimum opportunity requirement.
- Natural extinction is retained as an outcome.
- Primary analysis is world- or seed-block-level.
- `T_ref` and `M_ref` refer to a locked pre-feature benchmark on the same execution class.
- Claim-critical log loss must be zero.
- Uninterrupted and checkpoint-restored final logical-state and event-stream hashes must match.
- Any event-derived group, culture, tradition, or era label is offline only.

## 9.2 Worked example: lifetime learning

### Scientific purpose

Determine whether plasticity changes behavior within an organism's lifetime in response to experience, rather than merely changing inherited controller parameters across generations.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | Organisms with lifetime synaptic plasticity adapt more quickly to within-lifetime contingency changes than otherwise identical organisms with synaptic updates frozen. |
| **Treatment condition** | Versioned plasticity rule enabled, including learned weights and required eligibility/neuromodulatory traces. |
| **Control or ablation** | Plasticity update is frozen after birth while initial topology, initial weights, sensors, actuators, energy costs, and deterministic inference are preserved. Add a no-information negative control and a simple solvable contingency positive control. |
| **Experimental unit** | Independently initialized world/seed block for ecological confirmation. Isolated cloned-organism assays are supporting organism-level tests and must still cluster by source genome/world. |
| **Number of worlds** | Pilot: at least 12 paired seed blocks. Confirmation: provisionally 48 paired blocks = 96 worlds, recalibrated by simulated power. |
| **Run duration** | Long enough to produce at least 24 registered within-lifetime contingency shifts per eligible world, or a fixed maximum such as 1,000,000 ticks. Also require a minimum number of organisms surviving through both pre- and post-shift windows. |
| **Primary metric** | Per-world median normalized post-shift regret or recovery time among eligible organisms, computed from a locked standardized shift definition. Primary estimand: paired difference or ratio treatment versus frozen-plasticity control. |
| **Secondary metrics** | Pre-shift performance, retention, catastrophic interference, energy cost, extinction, learned-weight magnitude, generalization to held-out contexts, genetic compensation over generations, and population-level selection on plasticity. |
| **Minimum effect size** | Provisionally at least a 20% reduction in median recovery time or regret, while pre-shift performance is not worse by more than a registered noninferiority margin. |
| **Statistical decision rule** | Lower 95% confidence bound for the paired percent reduction exceeds 20%, or Bayesian `P(reduction > 20%) ≥ 0.95`. Positive control must be detected; no-information negative control must not show a comparable effect. |
| **Robustness checks** | Repeat with reproduction paused in an acute cloned-founder assay; test two shift schedules; use alternative robust outcome summaries; verify benefit after controller-size and lifespan adjustment; inspect all pair differences; run unpaired sensitivity. |
| **Failure interpretation** | No acute learning effect: plasticity mechanism or signal is ineffective. Acute effect but no ecological effect: opportunity, cost, or selection suppresses it. Ecological performance gain without within-life change: likely genetic or demographic pathway, not accepted as lifetime learning. |
| **Performance budget** | Plasticity-on throughput at least `0.85 × T_ref`; p95 peak RSS at most `1.5 × M_ref`; save/restore includes learned weights and traces; checkpoint plus logging overhead at most 15% in the registered benchmark. |
| **Determinism verification** | Learned weights, eligibility traces, modulatory state, and update counters are included in canonical state hashes. Social/environment observations are read from a common pre-update snapshot. Restore at mid-lifetime must reproduce every subsequent update and event hash. |
| **Required artifacts and logs** | Per-update or claim-sufficient plasticity events, contingency changes, observations, actions, rewards/modulators, initial and checkpointed learned state, genotype/controller IDs, eligibility criteria, opportunity counts, extinction, and full provenance. |

### Important identification rule

A treatment population outperforming the control after many generations does not itself demonstrate lifetime learning. Selection may favor different inherited genotypes. The acute within-lifetime assay and direct learned-state record are therefore required.

## 9.3 Worked example: social transmission

### Scientific purpose

Test whether behavior acquisition follows social exposure rather than genetic inheritance, shared environment, or independent discovery.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | Exposure to a demonstrator carrying an arbitrary, post-birth assigned behavioral variant increases learner acquisition of that specific variant through the social-information channel. |
| **Treatment condition** | Demonstrators receive one of two functionally matched arbitrary variants after birth. Learners can perceive the relevant demonstrator action/result, and the registered social-learning mechanism is enabled. |
| **Control or ablation** | Primary ablation: social learning disabled while perceptual load and environmental consequences are matched. Yoked environment control: learners receive matched resource/outcome changes without demonstrator information. Negative control: exposure timestamp after acquisition or demonstrator outside sensory range. |
| **Experimental unit** | Seed block containing all randomized conditions. Learners are nested observations; world is the replicate. |
| **Number of worlds** | Provisionally 48 seed blocks × 3 conditions = 144 worlds. Increase if acquisition is rare or extinction high. |
| **Run duration** | Until at least 30 registered naive learner cohorts or exposure episodes per viable world, or a fixed maximum such as 1,500,000 ticks. Require population turnover sufficient to include learners not genetically descended from demonstrators. |
| **Primary metric** | Donor-specific acquisition hazard or probability after exposure, adjusted for prior behavior, genotype/lineage, location, opportunity, and environmental state. World-level summary or hierarchical network-based diffusion model with seed-block effects. |
| **Secondary metrics** | Adoption latency, fidelity, variant matching, spread depth, persistence, exposure-dose response, payoff sensitivity, lineage crossing, and acquisition in unexposed organisms. |
| **Minimum effect size** | Provisionally social-exposure hazard ratio ≥1.5 and absolute adoption increase ≥15 percentage points versus social-channel ablation. Both thresholds prevent a large ratio on a trivial baseline from passing. |
| **Statistical decision rule** | 95% interval lower bound exceeds HR 1.5 and risk difference 0.15, or Bayesian posterior probability above both thresholds ≥0.95. Donor-specific matching must exceed chance and yoked-environment control. |
| **Robustness checks** | Swap variant labels; cross-lineage demonstrators; randomize demonstrator locations; alternative exposure-network weighting; held-out seeds; time-shuffled exposure negative control; no-payoff arbitrary variant condition; unpaired and per-world analyses. |
| **Failure interpretation** | Spread without donor-specific matching suggests shared environment. Variant matching without social-channel dependence suggests perceptual cue or direct environmental copying. Effect only within kin lineages leaves genetic confounding. Low exposure opportunity makes result inconclusive, not negative. |
| **Performance budget** | Social-exposure logging and learning must maintain at least `0.80 × T_ref`; claim-critical exposure events cannot be sampled; p95 memory at most `1.6 × M_ref`. |
| **Determinism verification** | Demonstrations and learner exposures are gathered from the same immutable tick snapshot, canonically sorted, and combined by an order-independent/versioned rule. No first-observed-wins behavior. Variant randomization uses a separate named RNG domain. |
| **Required artifacts and logs** | Demonstrator and learner stable IDs, genotype/lineage, variant assignment, all eligible exposures, sensory availability, learner prior state, acquisition event, location, environment, payoff, social-channel state, and causal parent links. |

### Interpretation ceiling

Passing establishes a causal social-transmission effect under the tested mechanism. It does not yet establish a stable tradition or cumulative culture.

## 9.4 Worked example: tradition persistence

### Scientific purpose

Determine whether a socially transmitted practice persists beyond its originator and through population turnover.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | A socially transmitted arbitrary practice persists after demonstrator removal and across multiple population turnovers when social transmission is enabled. |
| **Treatment condition** | Social transmission enabled; one or more originators carry a randomized arbitrary practice; originators are removed at a registered trigger after initial spread. |
| **Control or ablation** | Social transmission disabled with all other affordances maintained. Additional drift control uses a neutral inherited marker with matched initial prevalence. Environment-reset or spatial-shuffle control tests local cue persistence. |
| **Experimental unit** | Independent world/seed block. Practices and organisms are nested. |
| **Number of worlds** | Provisionally 48 paired seed blocks = 96 worlds after the social-transmission phase has independently passed. |
| **Run duration** | At least 10 population turnovers after originator removal or 2,000,000 ticks, whichever exposure criterion is met first; administrative maximum fixed in advance. |
| **Primary metric** | Indicator or duration that the practice remains above a registered prevalence threshold in eligible organisms for at least three turnovers after all originators are absent. Primary estimand: difference in world-level persistence probability. |
| **Secondary metrics** | Cultural lineage depth, prevalence trajectory, fidelity, between-group differentiation, reacquisition after local loss, turnover of carriers, and practice fitness effect. |
| **Minimum effect size** | At least a 20 percentage-point increase in persistent-tradition probability versus ablation, with at least 30% of treatment worlds meeting the absolute criterion. |
| **Statistical decision rule** | Lower 95% interval for the paired risk difference exceeds 0.20; social-transmission prerequisite remains satisfied; neutral inherited-marker and environment controls do not reproduce the same pattern. |
| **Robustness checks** | Two arbitrary practice labels; stricter and looser prevalence thresholds; exclude direct descendants in one analysis; reset local environmental marks; blinded detector; new seed registry; survival analysis of tradition loss. |
| **Failure interpretation** | Practice dies after originator removal: social acquisition may occur but inheritance is insufficient. Persistence only among descendants suggests genetic or parental confounding. Persistence after social ablation suggests environmental memory or independent reinvention. |
| **Performance budget** | At least `0.85 × T_ref`; cultural-lineage derivation is offline and may be expensive but must not slow the world; raw exposure/action logs remain within registered storage budget. |
| **Determinism verification** | Originator-removal trigger is based on canonical events and produces identical fork state. Carrier/prevalence calculation is offline, versioned, and deterministic. |
| **Required artifacts and logs** | Originator assignment/removal, exposure and acquisition histories, genealogy, environment state, carrier prevalence by cohort, practice variant, detector config, and all world outcomes. |

### Interpretation ceiling

Passing supports “persistent tradition” only for the operationalized practice and horizon. It does not imply symbolic culture, norms, teaching, or cumulative improvement.

## 9.5 Worked example: artifact reuse

### Scientific purpose

Test whether persistent artifacts mediate useful behavior across actors or generations, rather than merely attracting repeated contact.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | Durable artifacts created or modified by one organism are later functionally reused by noncreators, including after the creator is absent, at a rate and benefit exceeding matched decay/removal controls. |
| **Treatment condition** | Artifacts persist with registered physical properties and can be perceived/manipulated by later organisms. No named recipes or “tool” labels exist in-world. |
| **Control or ablation** | Rapid artifact decay or deterministic removal after creator departure, with raw material availability and resource distribution matched. Functional-destruction fork preserves appearance/location but removes the candidate causal property. |
| **Experimental unit** | Independent world/seed block. Artifact events are nested; checkpoint-fork interventions are repeated within independently generated source worlds. |
| **Number of worlds** | Provisionally 48 paired seed blocks, plus at least 24 independent source worlds contributing registered checkpoint-fork tests. |
| **Run duration** | 1–2 million ticks or until a registered minimum number of eligible created artifacts and noncreator encounters is reached. |
| **Primary metric** | Per-world proportion or rate of eligible artifacts functionally reused by a noncreator after creator absence, where functional reuse requires a causal outcome improvement versus object-disabled counterfactual. |
| **Secondary metrics** | Time to reuse, distance/lineage between creator and user, repeated reuse, maintenance/modification, artifact survival, energy/time benefit, spatial inheritance, and reuse network depth. |
| **Minimum effect size** | At least +15 percentage points in world-level functional-reuse probability or incidence-rate ratio ≥1.5, plus a positive median causal benefit in registered forks. |
| **Statistical decision rule** | Lower 95% interval exceeds the registered threshold; functional-destruction fork reduces the outcome by at least the SESOI; simple object-contact rate alone cannot satisfy the endpoint. |
| **Robustness checks** | Randomize artifact location; use appearance-preserving nonfunctional substitutes; control for resource hotspots and encounter opportunity; restrict to nonkin users; require creator death in a stricter analysis; alternative eligibility thresholds. |
| **Failure interpretation** | More contact without benefit is attraction or clutter, not reuse. Benefit only to creators shows individual tool use but not environmental inheritance. Reuse disappears after hotspot adjustment, indicating shared resource response. |
| **Performance budget** | At least `0.80 × T_ref`; object/event storage no more than registered bytes per object-tick; checkpoint-fork batch overhead budgeted separately; p95 RSS ≤`1.7 × M_ref`. |
| **Determinism verification** | Artifact creation, transformation, ownership/holding, location, collision, and destruction use stable IDs and total event order. Fork removal/substitution is a versioned intervention. |
| **Required artifacts and logs** | Artifact component IDs, material/properties, creator and modifiers, transformations, carries/placements, eligible encounters, use sequence, target outcome, counterfactual fork, creator status, and environment covariates. |

### Interpretation ceiling

Passing supports functional artifact reuse and environmental inheritance. Calling the artifact a “tool” additionally requires controlled causal mediation; calling it “technology” requires a broader production and dependency context.

## 9.6 Worked example: persistent structures

### Scientific purpose

Determine whether organisms create durable spatial configurations that persist, are recognized or reused, and causally affect later outcomes.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | When objects/terrain can be persistently configured, organisms produce structures that survive builder absence and are reused or maintained by later organisms with measurable functional effects. |
| **Treatment condition** | Placement, joining, terrain modification, and registered persistence mechanics enabled. |
| **Control or ablation** | Rapid decay/nonbinding condition preserving object availability and manipulation cost. Negative control uses randomized physics-generated piles with matched size/density. Fork ablation removes organization while preserving material quantity. |
| **Experimental unit** | Independent world/seed block. Candidate structures are nested observations. |
| **Number of worlds** | Provisionally 48 paired seed blocks; 24+ independent fork-source worlds for causal organization tests. |
| **Run duration** | At least 2,000,000 ticks or 10 population turnovers, with a minimum registered number of construction opportunities. |
| **Primary metric** | Probability that a world produces at least one configuration meeting all criteria: organism-caused; above a registered organization threshold; persists after all builders leave/die; reused or maintained by at least two nonbuilders; and has a positive causal effect in organization-destroying forks. |
| **Secondary metrics** | Structure lifetime, component count, maintenance, modification lineage, occupancy, shelter/resource/flow effect, reuse generations, construction sequence, and spatial distribution. |
| **Minimum effect size** | At least +15 percentage points in qualifying-world probability and median functional benefit above the registered SESOI; absolute treatment prevalence at least 20%. |
| **Statistical decision rule** | Risk-difference interval exceeds 0.15, and randomized piles/nonbinding controls do not meet the conjunctive criterion at comparable frequency. |
| **Robustness checks** | Material-matched disorganization; location randomization; builder removal; nonkin/later-generation reuse; detector threshold sensitivity; manual blinded audit of a registered sample; held-out worlds. |
| **Failure interpretation** | Persistent piles without causal function are environmental debris. Functional configurations used only by builders are individual construction, not environmental inheritance. High control prevalence means the detector confuses physics with construction. |
| **Performance budget** | At least `0.75 × T_ref` if mutable terrain is a major phase change, with an explicit justification; region snapshot and event journal overhead ≤20%; storage scaling measured against active modified regions, not total theoretical map. |
| **Determinism verification** | Simultaneous placement/terrain conflicts are resolved by total-order policy or keyed lottery. Region updates, joins, splits, and component IDs are canonical. Snapshot restore preserves exact topology and scheduler state. |
| **Required artifacts and logs** | Builder action sequences, object/terrain deltas, component graph, structure detector inputs, users/maintainers, builder presence, organization-destroying forks, outcomes, region hashes, and migration version. |

### Interpretation ceiling

Passing supports persistent functional structures. It does not establish architecture, settlement, ownership, planning, or a civilization stage.

## 9.7 Worked example: territoriality

### Scientific purpose

Distinguish persistent, site-contingent defense and exclusion from simple aggregation around valuable resources.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | Stable, defendable resource patches produce repeated site-specific occupancy and boundary-contingent defense/exclusion by persistent residents, beyond density and resource-attraction effects. |
| **Treatment condition** | Spatially clustered resources with temporal persistence and feasible exclusion/defense affordances; no group or territory state is authored. |
| **Control or ablation** | Spatially shuffled resources matched in abundance and renewal; unstable resource locations; identity-memory ablation where relevant; density-matched no-exclusion control. |
| **Experimental unit** | Independent world/seed block. Encounters, residents, and patches are nested. |
| **Number of worlds** | Provisionally 60 paired seed blocks = 120 worlds because territorial outcomes may be heterogeneous and extinction-sensitive. |
| **Run duration** | At least 20 population turnovers or a registered minimum number of resident–intruder encounters at stable patches, with a fixed maximum horizon. |
| **Primary metric** | World-level territoriality score requiring: stable site occupancy; identifiable resident set from prior presence rather than analyst-imposed faction; increased resident aggression/exclusion toward intruders near a persistent boundary; and reduced intruder access relative to matched nonboundary encounters. |
| **Secondary metrics** | Home-range/site fidelity, boundary stability, scent/marking or external-memory use, ownership turnover, defense cost, kin structure, resource intake, and displacement. |
| **Minimum effect size** | Resident–intruder boundary defense/exclusion rate ratio ≥1.5 versus matched encounters and at least +20 percentage points in worlds satisfying the full conjunctive criterion relative to shuffled-resource control. |
| **Statistical decision rule** | Both within-world boundary contrast and between-condition world-level prevalence thresholds pass with 95% intervals; density and resource value adjustment cannot explain the effect. |
| **Robustness checks** | Resource-location shuffle; boundary-placebo regions; density matching; resident-identity scrambling; removal of stable environmental marks; alternative home-range detector; held-out seed registry. |
| **Failure interpretation** | Clustering without exclusion is aggregation. Aggression everywhere is not territorial defense. Boundary effect caused by resource density rather than resident status is resource competition. Identity ablation effect may indicate individual recognition, not necessarily group territory. |
| **Performance budget** | At least `0.80 × T_ref`; spatial contact summaries may be derived online only if exact claim-critical encounter events remain available; p95 RSS ≤`1.6 × M_ref`. |
| **Determinism verification** | Resident/intruder labels are offline derivations. In-world agents only perceive authorized cues. Simultaneous movement/attack/access conflicts use canonical resolution. Spatial index traversal cannot determine outcomes. |
| **Required artifacts and logs** | Positions, resource fields, encounters, actions, outcomes, prior occupancy, stable IDs, kin/genetic data, local density, environmental marks, access attempts, and detector version. |

### Interpretation ceiling

Passing supports territorial behavior under the operational definition. It does not justify factions, sovereignty, borders as symbolic institutions, or political organization.

## 9.8 Worked example: coalitionary conflict

### Scientific purpose

Distinguish coordinated third-party support and partner-contingent joint aggression from coincident attacks caused by crowding or a common resource.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | Identity memory and observation of social interactions permit repeated partner-contingent third-party intervention in conflicts, producing joint effects greater than expected from independent aggression. |
| **Treatment condition** | Stable identity cues, social memory, observation of interactions, and a mechanically available intervention/attack action; no coalition, group-goal, alliance, or war state is authored. |
| **Control or ablation** | Identity scrambling between encounters; third-party-support action disabled while individual aggression remains; social-history memory ablated; crowd density and resource distribution matched. |
| **Experimental unit** | Independent world/seed block. Conflict episodes and organisms are nested. |
| **Number of worlds** | Provisionally 80 paired seed blocks = 160 worlds because the event may be rare and heavy-tailed. Sequential batches are allowed under a registered alpha-spending or Bayesian rule. |
| **Run duration** | At least 30 population turnovers or a minimum number of eligible dyadic conflicts observed by potential third parties; fixed maximum horizon. |
| **Primary metric** | Rate of temporally coordinated third-party intervention supporting a prior interaction partner against a common target, with intervention probability conditioned on relationship history and with joint outcome exceeding an independence/crowding null. |
| **Secondary metrics** | Partner specificity, reciprocity, coalition stability, target selection, kin bias, cost to helper, repeat support, conflict escalation, mortality, territory/resource context, and between-cluster asymmetry. |
| **Minimum effect size** | Intervention incidence-rate ratio ≥1.5 with identity/social memory enabled, plus a preregistered supra-additive joint-outcome effect and partner-history coefficient above the SESOI. |
| **Statistical decision rule** | All three components—third-party intervention, partner contingency, and joint causal effect—must pass. A significant rise in raw attacks alone fails. World-level or hierarchical rare-event model with seed-block effect and exposure denominator is used. |
| **Robustness checks** | Density-matched event-time shuffle; identity-label permutation; support-action ablation; kin-only exclusion; resource-conflict adjustment; checkpoint fork preventing helper intervention; alternative coordination time window; independent seed registry. |
| **Failure interpretation** | Simultaneous attacks without partner specificity are crowd aggression. Partner support without joint effect is social preference, not effective coalition. Identity dependence may show recognition but not stable coalitions. Extremely low eligible exposure yields inconclusive evidence. |
| **Performance budget** | At least `0.80 × T_ref`; detailed conflict/contact logs must remain lossless; rare-event indexing and offline network analysis budgeted separately. |
| **Determinism verification** | Attack and intervention intents derive from the same pre-resolution state. Conflict resolution cannot depend on thread arrival or atomic first-writer order. Identity assignment and any keyed lottery are versioned and replayable. |
| **Required artifacts and logs** | Full eligible conflict set, observers, relationship history inputs, intents, timing, targets, outcomes, costs, kin/genetic data, density/resources, identity-policy version, fork interventions, and null-model configuration. |

### Interpretation ceiling

Passing supports coalitionary behavior under the operational definition. “Organized inter-group conflict” additionally requires persistent group structure, repeated between-group targeting, coordination, and controls against local network density. It still does not imply war declarations, political goals, or symbolic group identity.

## 9.9 Worked example: cumulative cultural improvement

### Scientific purpose

Test the strongest near-term culture claim: whether socially transmitted practices and persistent artifacts support retained, dependent improvements across cultural lineages beyond genetic adaptation and asocial reinvention.

| Required field | Proposed criterion |
|---|---|
| **Hypothesis** | Social transmission and artifact persistence interact to produce cultural lineages with successive retained functional improvements that exceed matched asocial and genetic baselines and depend causally on predecessor innovations. |
| **Treatment condition** | 2×2 factorial: social transmission enabled/disabled × artifact persistence enabled/rapid decay. Genetic evolution remains enabled in all ecological cells. Separate standardized naive/asocial reconstruction assays are run from registered checkpoints. |
| **Control or ablation** | Social-channel ablation; artifact-decay ablation; both ablated; checkpoint forks removing predecessor artifacts/practices; genotype-matched naive agents without cultural exposure; optional frozen-genome assay to isolate cultural accumulation over a bounded interval. |
| **Experimental unit** | Seed block across four factorial worlds. Cultural variants, lineages, artifacts, and organisms are nested. Checkpoint assays are repeated across independent source worlds. |
| **Number of worlds** | Provisionally 80 seed blocks × 4 cells = 320 ecological worlds, plus registered checkpoint assays. This high threshold reflects rarity, interaction testing, and heavy-tailed outcomes. Sequential expansion is permitted only under a valid plan. |
| **Run duration** | At least 20 population turnovers and 5,000,000 ticks, or a registered minimum number of innovation/transmission opportunities; expanding-horizon replication is required before any bounded-OEE wording. |
| **Primary metric** | World-level cumulative-cultural score requiring all of: supported social-transmission edges; at least four linked retained modifications; monotonic or retained functional gain after each accepted step; later endpoint ≥20% above the same-seed asocial/genotype-matched frontier; and predecessor ablation causing a registered performance loss. |
| **Secondary metrics** | Cultural lineage depth/branching, fidelity, recombination, artifact dependency chain, naive reconstruction time, genetic contribution, population turnover, group specificity, transmission bottlenecks, and performance frontier over time. |
| **Minimum effect size** | Social×persistence interaction above a prespecified scale, at least +15 percentage points in worlds satisfying the conjunctive endpoint, endpoint performance ≥20% above asocial frontier, and ≥4 causally linked retained steps. |
| **Statistical decision rule** | The factorial interaction and full conjunctive endpoint must pass with 95% interval or posterior threshold. Evidence from the single best lineage is insufficient; world-level prevalence and uncertainty are primary. All predecessor-ablation and naive-reconstruction criteria must pass in a registered subset. |
| **Robustness checks** | Freeze genetic evolution in bounded assays; cross-foster genotypes into other cultural histories; independent seed registry; alternative but locked functional metric; random cultural-edge rewiring; artifact-function destruction; detector blinding; time-reversed/phase-randomized negative controls; expanding duration/population budgets. |
| **Failure interpretation** | Improvement without transmission is genetic/asocial adaptation. Transmission without retained gains is culture but not cumulative culture. Gains without predecessor dependence are serial reinvention. A rare chain with no replication is a candidate case, not phase acceptance. No effect may reflect insufficient affordances or opportunity, not proof that cumulative culture is impossible. |
| **Performance budget** | Ecological worlds at least `0.70–0.80 × T_ref` depending on feature scope; checkpoint assay throughput reported separately; claim-critical log volume and storage must fit the full 320-world design with at least 25% reserve. |
| **Determinism verification** | Cultural lineage is derived offline. All raw transmission, artifact, genome, and outcome events are canonical. Checkpoint forks and naive assays use exact source-state hashes and separate RNG domains. Cross-condition pairing is semantic, not consumption ordered. |
| **Required artifacts and logs** | Full genetic and cultural ancestry, exposure/transmission records, artifact/component histories, function assays, predecessor forks, genotype-matched naive trials, world-level outcome table, detector code/configuration, all failed candidate chains, and expanding-horizon manifests. |

### Interpretation ceiling

Passing supports cumulative cultural improvement under a narrow operational definition and tested affordances. It does not support “technological civilization,” human-like cumulative culture, language, intentional invention, or future unbounded progress.

## 9.10 Optional extension: organized inter-group conflict

Because coalitionary conflict and territoriality are prerequisites rather than synonyms for organized inter-group conflict, a later phase should require all of the following:

- Persistent contact-network or co-residence groups detected offline without in-world labels.
- Greater conflict rate across than within those groups after distance, resource, kinship, and encounter-opportunity controls.
- Repeated common-target coordination involving multiple members.
- Partner or group-history dependence.
- Persistence across membership turnover.
- Group-structure or social-memory ablations that reduce the effect.
- No authored faction, diplomacy, war-state, or group objective.

The experimental unit remains the world. The claim should be “repeated coordinated between-group conflict under detector D,” not “war,” unless a separately justified operational definition is met.

> **Major conclusion record 12 — Cumulative cultural improvement requires a conjunctive, high-threshold endpoint.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **B/S.** Cumulative-culture theory supports ratcheting, retention, and social transmission; the exact Genesis endpoint is a synthesis. |
> | **Applicability to Genesis** | Direct for preventing genetic adaptation or serial reinvention from being mislabeled culture. |
> | **Uncertainty** | Functional metrics and predecessor definitions remain representation-dependent; true effects may be extremely rare. |
> | **Recommended action** | Require transmission, retained gains, lineage depth, asocial/genetic counterfactuals, and predecessor dependence together. |
> | **Measurement method** | Cultural transmission graph, function assays, checkpoint forks, genotype-matched naive reconstruction, and world-level prevalence. |
> | **Control or ablation** | Social channel off, persistence off, genotype frozen/matched, predecessor removed, function destroyed, and edge-rewired nulls. |
> | **Determinism implications** | Exact forks are unusually valuable, but cultural graph construction must remain offline and versioned. |
> | **Compute implications** | This is likely a rare-event, large-N, long-horizon program. It should not be an early feature gate. |


---

# 10. Deterministic simulation architecture

## 10.1 Determinism is a scientific contract, not merely a debugging feature

Genesis determinism should mean:

> Given the same versioned executable behavior, experiment manifest, initial state, seed/key policy, and declared execution class, the simulation produces the same ordered scientific events and canonical state hashes.

That definition has four parts:

1. **Input identity:** build, configuration, initial state, policies, and seed registry are immutable and hashed.
2. **Transition identity:** every state transition has explicit ordering and arithmetic semantics.
3. **Record identity:** the scientific event sequence is canonical and complete.
4. **Execution-class scope:** the project states exactly which machines, thread counts, compiler/toolchain versions, CPU features, or GPU architectures are covered.

Determinism does not imply scientific validity. A deterministic bug is reproducible but still wrong. Conversely, a statistically valid stochastic model can be implemented deterministically by treating the pseudorandom key space as part of its inputs.

## 10.2 Recommended determinism tiers

| Tier | Reproducibility claim |
|---|---|
| **D0 — Local replay** | Same process/build/configuration can replay a fixture or checkpoint and match hashes. |
| **D1 — Same host** | Separate runs on one host match across process restarts. |
| **D2 — Same execution class** | Different hosts with the same architecture, OS/toolchain policy, and CPU feature set match. |
| **D3 — Scheduler/thread invariant** | Supported thread counts and scheduler perturbations produce identical outputs. |
| **D4 — Cross-host independent verification** | Independently provisioned machines in the same declared class match. |
| **D5 — Cross-architecture/platform bit identity** | Architectures or platforms with different floating-point, SIMD, or library behavior match exactly. This is difficult and should be claimed only after explicit testing. |

Most scientific work can be strong at D2–D4. D5 may require fixed-point arithmetic, software-defined transcendental functions, and strict compiler/target controls.

## 10.3 State-transition discipline

The safest world model is a sequence of explicit phases operating on immutable or logically frozen input state:

1. **Sense:** construct each organism's authorized observations from the state at the beginning of the phase.
2. **Decide:** compute organism intents without mutating shared world state.
3. **Collect:** append intents to local buffers using stable actor and intent identifiers.
4. **Canonicalize:** sort or bucket intents under a complete total-order key.
5. **Resolve:** apply explicit, versioned conflict policies.
6. **Commit:** write the next world state.
7. **Derive events/hashes:** emit canonical events and subsystem hashes.
8. **Checkpoint boundary:** if scheduled, snapshot a coherent committed state.

This avoids hidden “who was visited first” semantics. A strict phase model does not require every physical process to be simultaneous, but the ordering must be part of the model rather than an accident of storage or threading.

## 10.4 Named, counter-based random-number streams

Counter-based RNGs such as Philox or Threefry treat a random draw as a stateless mapping from key/counter to output. The design is well suited to parallel simulations because the nth value need not be reached by consuming the previous `n−1` values (Salmon et al., 2011).

### Recommended semantic key

A Genesis draw should be derived from a versioned tuple conceptually equivalent to:

```text
(root_seed,
 experiment_id,
 world_id,
 tick_or_event_time,
 phase_id,
 system_id,
 stable_subject_id,
 stable_counterparty_or_object_id,
 event_kind,
 draw_purpose,
 draw_slot,
 rng_policy_version)
```

Not every field must be used for every draw, but domain separation must prevent unrelated systems from sharing a key space.

### Properties of a good draw key

- **Semantic:** identifies why the draw exists, not where a function happened to call the RNG.
- **Stable:** refactoring unrelated loops does not change it.
- **Unique:** no accidental key reuse.
- **Versioned:** changing the mapping creates an explicit policy version.
- **Canonical:** tuple encoding has fixed widths/endianness or a canonical tagged format.
- **Auditable:** debug mode can report key and result for a sampled set of draws.

### Named domains

Examples:

- Terrain initialization.
- Founder initialization.
- Mutation operator and mutation locus.
- Recombination choice.
- Learning exploration/noise.
- Environmental stochasticity.
- Conflict lottery.
- Object fracture or transformation.
- Experiment assignment.
- Offline power simulation and detector nulls—strictly separate from world RNG.

### Draw slots

A semantic event that needs multiple numbers should reserve named slots such as `angle`, `magnitude`, `mutation_count`, and `locus_choice`. Do not let adding one preliminary draw shift all later values.

### Rejection sampling and variable draw counts

Rejection samplers and variable-length mutation assignment can reintroduce consumption dependence. Options include:

- Derive each attempt from `(event_key, attempt_index)`.
- Use bounded algorithms with an explicit maximum and fail policy.
- Use direct sampling algorithms with stable arithmetic.
- Include the accepted-attempt index in debug/audit events.

## 10.5 Stable identities and lifecycle rules

Every semantically addressable entity should have a stable identity:

- World.
- Organism.
- Genome and controller schema instance.
- Lineage node.
- Artifact and component.
- Structure or region component where explicitly represented.
- Scheduled event.
- Experiment intervention.

Avoid reusing raw entity IDs after deletion. If storage slots are reused, expose `(slot, generation)` or a globally monotonic/canonically derived identity. Otherwise, an RNG or event key referring to ID 42 can silently change meaning after deletion.

### Parallel creation

Do not allocate IDs with a first-arriving atomic counter when creation order can vary by thread. Instead:

1. Collect creation requests.
2. Assign each a canonical request key such as `(tick, phase, creator_id, request_kind, local_ordinal)`.
3. Sort requests.
4. Allocate IDs in that order or derive IDs from the request key with collision handling defined canonically.

## 10.6 Canonical iteration and data structures

Rust's standard `HashMap` is randomly seeded and does not promise a semantic iteration order. Even a fixed hasher would not make dependence on internal hash-table layout a sound simulation rule. Recommended policy:

- Unordered maps and sets may be used for lookup.
- Their iteration order must never directly determine state transitions, event order, ID assignment, reduction order, or random keys.
- Before semantic traversal, collect keys and sort by a complete canonical key, or use an ordered structure such as `BTreeMap` when appropriate.
- Do not rely on serialization order of an unordered container.
- Do not assume insertion order unless the type and policy explicitly guarantee it and the insertion order is itself deterministic.

### Sorting

An unstable sort can be deterministic when all keys are unique and totally ordered, but ties are dangerous. Every semantic sort key must include a final stable tie-breaker. “Sort by score” is incomplete when scores can tie. A total key might be:

```text
(priority, fixed_point_score, actor_id, target_id, event_kind, local_ordinal)
```

## 10.7 Simultaneous updates and conflict resolution

Common conflicts include:

- Multiple organisms move into one location.
- Multiple actors pick up or modify the same object.
- Reproduction claims the same space/resource.
- Several attacks or interventions target the same organism.
- Concurrent placement joins structures.
- Multiple social demonstrations update one learner.

Valid deterministic policies include:

- **All fail** under contention.
- **Capacity-limited selection** by a total priority key.
- **Proportional sharing** using fixed-point arithmetic and deterministic remainder assignment.
- **Keyed lottery** where each candidate receives a random priority derived from the conflict key and candidate ID.
- **Simultaneous physical resolution** under a versioned solver with deterministic ordering and arithmetic.

The policy is part of the artificial physics. It should be documented and experimentally examined because tie-breaking can create selection pressure.

### Social-learning traversal order

A particularly subtle failure occurs when a learner updates once per demonstrator and the final weight depends on the order of neighbor traversal. Recommended alternatives:

- Aggregate exposure features using deterministic/reproducible reduction, then perform one update.
- Sort demonstrations by a complete semantic key and define order as part of the model.
- Use an explicitly commutative update when scientifically appropriate.

“First demonstration wins” is unacceptable unless firstness is defined by simulated time and tie-breaking, not container or thread order.

## 10.8 Deterministic reductions

Integer sums are order-independent only if overflow behavior is explicitly handled. Floating-point sums are generally order-dependent because addition is not associative.

Recommended hierarchy:

1. Use exact integer/fixed-point representation for state-critical quantities where range and precision are known.
2. Use checked, saturating, or wrapping integer arithmetic explicitly; do not rely on build-profile overflow differences.
3. For floating-point diagnostics that do not affect state, allow bounded numerical tolerance only if the deterministic claim says so.
4. For floating-point values that affect state, use a fixed reduction tree, reproducible summation method, or exact accumulator.
5. Record compiler target features, FMA policy, denormal handling, rounding assumptions, and math-library versions.

Parallel tree reductions must have a fixed topology independent of worker count if D3 is claimed. Otherwise different thread counts change grouping and results.

## 10.9 Fixed-point and floating-point policy

Genesis already benefits from fixed-point state. Continue to classify quantities:

| Class | Recommended policy |
|---|---|
| Position, energy, resource quantity, damage, physical coefficients affecting branching | Fixed-point or explicitly specified integer arithmetic. |
| Neural weights and plasticity state affecting future actions | Prefer fixed-point or a versioned deterministic numeric layer; if floats are used, constrain execution class and operations. |
| Offline statistical analysis | Floating point is acceptable with pinned library versions and numerical-tolerance validation; it does not affect simulation state. |
| Rendering | May use nondeterministic floats because it is nonauthoritative, provided it cannot feed back. |

A “versioned floating-point policy” should state:

- IEEE format and rounding mode.
- Treatment of NaNs and signed zero.
- FMA enabled/disabled.
- Denormal/flush-to-zero behavior.
- SIMD target features.
- Compiler optimization and fast-math prohibition.
- Transcendental-function implementation.
- GPU library and deterministic-algorithm setting.
- Supported architecture/driver range.

NVIDIA documents that some library routines are reproducible only within particular versions/architectures and that atomics can introduce nondeterministic rounding. Therefore, GPU determinism must be demonstrated per kernel and execution class, not assumed from identical inputs.

## 10.10 Canonical event ordering

Every authoritative event should have a complete order key. A useful logical form is:

```text
(world_id,
 tick,
 phase,
 subphase,
 event_priority,
 actor_id,
 target_or_object_id,
 event_type,
 local_sequence)
```

Rules:

- The key's semantics and comparison are versioned.
- Events with the same scientific meaning must receive the same key across supported thread counts.
- Wall-clock timestamps are metadata, never simulation order.
- Distributed worker arrival order is not event order.
- Event IDs should be stable and may incorporate segment ID and canonical ordinal.
- Parent/causal references must point backward in canonical order or use a separately validated dependency graph.

## 10.11 Hashing and checksums

Use distinct hashes for distinct purposes:

- **Logical-state hash:** canonical uncompressed semantic state, independent of physical chunk compression or file placement.
- **Serialized-chunk hash:** exact bytes of a chunk.
- **Event-segment hash:** header plus ordered event bytes.
- **Experiment-manifest hash:** configuration, policies, seed assignment, build and schema identifiers.
- **Artifact package root:** Merkle or manifest root over all required files.

A fast cryptographic hash such as BLAKE3 is suitable for content identity and parallel hashing; SHA-256 may be retained where interoperability or conservative external tooling matters. The algorithm is a versioned policy. CRC32C can provide fast accidental-corruption detection but should not be the sole content-identity mechanism.

Hash checkpoints at multiple granularities:

- Whole world.
- Terrain/region state.
- Organism state.
- Artifact/structure graph.
- Scheduler.
- Learned/plastic state.
- Event cursor.

Subsystem hashes make divergence localization practical.

## 10.12 Snapshot consistency and restore verification

A snapshot is valid only at a declared consistency boundary. It must include all state that can affect the future:

- World tick and phase boundary.
- Mutable terrain and region metadata.
- Organisms, genomes, controllers, learned weights, plasticity traces, and memory.
- Artifacts, structures, component graphs, and ownership/holding state.
- Scheduled events and deterministic queue order.
- Pending intents only if snapshots can occur mid-phase; simpler policy is committed-boundary snapshots only.
- RNG policy and any explicit counters not reconstructible from semantic keys.
- ID allocation state.
- Event-log segment and cursor.
- Configuration and numeric-policy versions.

Restore verification should include:

1. Parse and validate fail-closed.
2. Recompute all chunk and logical hashes.
3. Validate references, uniqueness, bounds, graph invariants, and scheduler order.
4. Compare restored logical-state hash to the recorded hash.
5. Replay a registered number of ticks and compare expected event and state hashes.
6. For migration, compare semantic invariants and golden fixtures.

## 10.13 Fail-closed parsing

Reject rather than guess when encountering:

- Unknown required schema or policy version.
- Duplicate stable ID.
- Dangling reference.
- Invalid enum/tag.
- Length overflow or allocation beyond registered bounds.
- Unsorted supposedly canonical data.
- Hash/checksum mismatch.
- Noncanonical encoding.
- Scheduler event in the past.
- Impossible ownership/component graph.
- Missing learned state for a controller that requires it.
- Unknown RNG or floating-point policy.
- Event cursor inconsistent with log segment.

Best-effort recovery may exist as a separate forensic tool, but it must never silently produce a scientific continuation.

## 10.14 Common determinism failures and mitigations

| Failure source | How it changes science | Required mitigation |
|---|---|---|
| Hash-map/set iteration | Changes action, update, serialization, or exposure order. | Sort complete keys or use ordered structures for semantic traversal. |
| Thread scheduling/work stealing | Changes first-writer, atomic increment, or reduction order. | Intent buffers, canonical merge, fixed reduction tree, no arrival-order semantics. |
| Atomic counters | Changes IDs/event ordinals. | Canonical creation request ordering or key-derived IDs. |
| Incomplete/unstable sorting | Ties resolve through incidental input order. | Complete total-order tie-breaker. |
| Floating-point reduction order | Alters thresholds and branches. | Fixed-point, exact/reproducible accumulator, fixed tree, constrained execution class. |
| SIMD/FMA/fast math | Changes rounding or exceptional-value behavior. | Versioned target and compiler policy; no fast math for authoritative state. |
| GPU atomics/algorithm selection | Can vary across runs, drivers, or architectures. | Deterministic kernels only, fixed algorithm selection, empirical hash tests. |
| Mutable RNG consumption order | Refactors or divergent conditions shift every later draw. | Counter-based semantic keys and named slots. |
| Entity deletion/reuse | Old ID keys refer to a new entity. | Never-reused IDs or generation counters. |
| Mutable global registry | Initialization or plugin order affects behavior. | Immutable versioned registry sorted by stable identifiers. |
| Parallel mutation assignment | Mutation IDs/loci depend on worker completion. | Per-offspring semantic keys and canonical structural-mutation requests. |
| Social-learning traversal | Neighbor order changes learned state. | Aggregate commutatively or sort by canonical exposure key. |
| Logging backpressure | World timing/state changes because observer or disk is slow. | Logical simulation independent of wall clock; explicit stop-on-record-loss policy. |

## 10.15 Rust-specific review points

- Commit `Cargo.lock` for the research executable and build with locked dependencies.
- Pin `rust-toolchain.toml`, target triple, target CPU/features, and relevant linker settings.
- Record compiler version and full build flags.
- Avoid `HashMap`/`HashSet` semantic traversal.
- Treat `usize` as platform-sized and therefore unsuitable for canonical file formats or cross-architecture state unless converted explicitly.
- Specify integer overflow semantics; use explicit checked/wrapping/saturating operations.
- Do not serialize Rust memory layouts directly; layout is not a stable wire format.
- Avoid deriving canonical order from pointer addresses, allocation order, or thread IDs.
- Audit dependencies that may spawn threads, choose algorithms dynamically, use platform math, or incorporate entropy.
- Disable or isolate incremental/build metadata when pursuing bit-reproducible binaries; reproducible build and deterministic run are separate goals.
- Maintain golden fixtures for every versioned state/event schema.

> **Major conclusion record 13 — Semantic, counter-based randomness is the preferred determinism foundation.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Counter-based RNG theory and parallel use are established; the semantic Genesis key is a synthesis. |
> | **Applicability to Genesis** | Direct for mutation, learning, conflict, world generation, and paired-seed studies. |
> | **Uncertainty** | Key design errors and accidental reuse remain possible; statistical quality and domain separation require audit. |
> | **Recommended action** | Make every authoritative draw a pure function of a versioned semantic key and named slot. Ban mutable global RNG state. |
> | **Measurement method** | Collision/key-reuse tests, sampled key trace, statistical test suite, paired-condition alignment audit, and replay hashes. |
> | **Control or ablation** | Scheduler/thread perturbation; refactor tests that add unrelated draws; same-seed treatment divergence tests. |
> | **Determinism implications** | Removes random-consumption-order dependence and supports D3 parallel invariance when combined with canonical state updates. |
> | **Compute implications** | Counter-based generation parallelizes well and removes stream-state synchronization; hashing/key construction has measurable but usually bounded cost. |

> **Major conclusion record 14 — Cross-platform bit identity requires an explicit numeric execution class.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A.** Floating-point nonassociativity, FMA/SIMD differences, and nondeterministic GPU atomics are established. |
> | **Applicability to Genesis** | Direct if neural learning or physics uses floating point. |
> | **Uncertainty** | Exact behavior depends on chosen compiler, libraries, and hardware; some kernels may be reproducible within narrower classes. |
> | **Recommended action** | Prefer fixed-point for authoritative state; otherwise publish numeric policy and determinism tier, and test every supported execution class. |
> | **Measurement method** | Cross-host/architecture hash matrix, threshold-near fixtures, reduction stress tests, NaN/denormal/FMA tests. |
> | **Control or ablation** | Scalar versus SIMD, thread counts, CPU features, compiler versions, GPU algorithms. |
> | **Determinism implications** | Never infer D5 from D1. A tolerant numerical match is not exact replay and should be labeled separately. |
> | **Compute implications** | Reproducible arithmetic may cost throughput; measure the end-to-end cost and preserve a research-correct reference path. |

---

# 11. Parallel-world scheduling

## 11.1 Scaling priority

The recommended scaling order is:

1. Optimize and profile one headless world.
2. Run many independent worlds concurrently.
3. Improve storage, checkpointing, and scheduling throughput.
4. Parallelize selected within-world kernels with deterministic merge semantics.
5. Consider distributed or GPU execution only after evidence identifies a bottleneck and a compatible workload.

Independent worlds are an “embarrassingly parallel” workload: they need no causal communication during normal execution. This aligns the computational unit with the statistical replicate.

## 11.2 Immutable experiment manifest

Each task should reference an immutable manifest containing:

- Experiment and protocol IDs.
- Build and source digests.
- Model/configuration/schema/policy versions.
- Condition ID.
- Root seed and world ID.
- Maximum tick/event/turnover and resource budgets.
- Checkpoint policy.
- Logging tier.
- Determinism execution class.
- Expected output schema and artifact list.
- Scientific stopping/interim policy.

A task identity should be idempotent, for example:

```text
(experiment_id, protocol_version, condition_id, seed_id, attempt_policy_version)
```

The same task may be retried, but it must not silently become a new scientific replicate.

## 11.3 Coordinator and work queue

Recommended components:

- **Experiment planner:** validates protocol and creates immutable task manifests.
- **Coordinator:** places tasks in a durable queue, manages leases, and records status.
- **Worker:** validates compatibility, executes one or more isolated worlds, checkpoints, and writes outputs to a temporary location.
- **Artifact validator:** verifies hashes, completeness, determinism gates, and schema.
- **Atomic publisher:** promotes a validated output package to immutable completed status.
- **Analysis catalog:** indexes completed worlds and their provenance; never changes simulation behavior.

### Task states

- Planned.
- Eligible.
- Leased/running.
- Checkpointed.
- Completed pending validation.
- Validated and published.
- Recoverable failure.
- Invalid execution.
- Scientifically terminated (extinction or registered stopping).

Retries must be distinguishable from independent worlds. A recovered task retains its original scientific identity.

## 11.4 Process versus thread isolation

### Process-per-world or small process group

Advantages:

- Crash and memory corruption containment.
- Cleaner accounting for CPU, RSS, file descriptors, and logs.
- No accidental shared global RNG or registry.
- Easier version and compatibility enforcement.
- Simpler restart and artifact boundaries.

Costs:

- Higher startup and memory overhead.
- Less efficient sharing of immutable data.
- More operating-system scheduling overhead at very small worlds.

### Many worlds in one process with threads

Advantages:

- Lower startup cost.
- Shared immutable assets and code pages.
- Efficient for small worlds.

Risks:

- One crash can invalidate many worlds.
- Shared allocators, registries, caches, and logging can create coupling.
- Resource accounting is harder.
- Accidental cross-world state or RNG contamination.

### Recommendation

Use process isolation as the correctness reference and default for long/expensive worlds. A batched multi-world process may be introduced for throughput only after tests prove:

- No shared mutable semantic state.
- Stable per-world resource limits.
- One world's failure cannot corrupt another's outputs.
- Scheduling changes do not alter hashes.
- The same world matches the process-isolated reference.

## 11.5 Work allocation and reproducibility

Worker assignment may vary without changing scientific output. Therefore:

- World IDs and RNG keys never depend on worker ID, process ID, hostname, queue position, or start time.
- Wall-clock start/end times are metadata only.
- Workers validate build and execution-class compatibility before starting.
- Outputs include worker/hardware metadata for audit but not for semantic identity.
- Duplicate successful executions of the same task should have identical canonical hashes; disagreement quarantines both results.

## 11.6 Crash recovery and deterministic restart

On lease expiry or process failure:

1. Find the latest fully committed and validated checkpoint.
2. Verify snapshot and journal hashes.
3. Restore into a compatible worker.
4. Replay the registered verification window if present.
5. Continue from the exact committed boundary.
6. Publish a restart incident record.
7. Compare final hashes with a duplicate uninterrupted run on a sampled basis.

Never restart from an unvalidated partial snapshot. Temporary files should be ignored unless a commit marker and root manifest validate.

## 11.7 Checkpoint interval selection

Checkpoint frequency trades write cost against lost work. Let:

- `C` = measured checkpoint duration.
- `M` = measured mean time between relevant failures.
- `R` = recovery cost.

Young's first-order approximation suggests an interval on the order of `sqrt(2CM)`, while Daly refines the estimate. These formulas are starting points, not universal policies. Genesis should measure:

- Failure rate by execution environment.
- Checkpoint duration distribution and shared-storage contention.
- Restore/replay duration.
- World value and maximum tolerable lost work.
- Log-journal durability between snapshots.

A journal can reduce lost work, permitting less frequent full snapshots, but only if replay cost and schema stability remain bounded.

## 11.8 Distributed execution

A distributed worker pool should enforce:

- Content-addressed build or container image.
- Exact manifest and dependency lock.
- Execution-class label.
- Local scratch staging with atomic upload.
- Durable queue lease and heartbeat.
- Idempotent task identity.
- Server-side artifact validation.
- No direct worker mutation of experiment definitions.
- Clock independence for simulation semantics.
- Central catalog of incidents, duplicates, and hashes.

The scheduler may prioritize jobs by operational policy, but adaptive scientific prioritization must be recorded as part of the experiment. A hidden scheduler that gives promising conditions more worlds creates selection bias.

## 11.9 Within-world parallelism

Within-world parallelism is harder because state transitions interact. Candidates include:

- Per-organism perception from an immutable spatial snapshot.
- Same-topology batched neural inference.
- Local physics calculations that emit intents.
- Region-local environmental updates with explicit halo/boundary exchange.
- Compression and hashing of already committed chunks.
- Offline statistics and event conversion.

### Deterministic pattern

1. Freeze read state.
2. Partition work by a stable function of entity/region ID, not worker count when possible.
3. Produce thread-local intents and partial results.
4. Canonically merge.
5. Resolve cross-partition conflicts under a complete policy.
6. Commit once.
7. Hash and compare against the single-thread reference.

### Conservative versus optimistic PDES

Parallel discrete-event simulation offers:

- **Conservative methods:** process only events known to be safe from earlier causal arrivals.
- **Optimistic methods/Time Warp:** process speculatively and roll back when causality violations appear.

For Genesis, optimistic PDES is unlikely to be the first choice because it requires rollback state, anti-events, fossil collection, deterministic rollback/re-execution, and careful integration with learning, mutation, and event logging. It may be useful only if profiling shows sparse cross-region interaction and substantial exploitable event parallelism.

A synchronous phased/tick model with deterministic region intents is usually simpler, easier to verify, and more compatible with exact checkpointing.

## 11.10 Deterministic reductions across worlds

Worlds are independent, so aggregate reporting can be computed offline. Do not let concurrent completion order determine:

- Sequential-test decisions without batch rules.
- Seed inclusion.
- Floating-point aggregate order.
- Adaptive condition selection.
- File naming or event ordering.

Sort completed tasks by immutable task key before deterministic aggregation, or use reproducible statistical accumulation.

> **Major conclusion record 15 — Independent worlds are the primary scaling axis.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Independent replications are statistically required and computationally embarrassingly parallel; the process architecture is a synthesis. |
> | **Applicability to Genesis** | Direct. Most treatment contrasts require many worlds, and worlds need no causal communication. |
> | **Uncertainty** | Very small worlds may benefit from batched threads; extremely large worlds may eventually require within-world distribution. |
> | **Recommended action** | Build durable idempotent world tasks, process-isolated reference execution, and atomic validated artifact publication before complex PDES. |
> | **Measurement method** | Worlds/hour, cost per valid replicate, crash/recovery rate, duplicate hash agreement, queue utilization, and scientific power per compute unit. |
> | **Control or ablation** | Compare isolated versus batched execution and thread counts against the single-world hash reference. |
> | **Determinism implications** | Worker assignment and completion order must be semantically irrelevant. |
> | **Compute implications** | Scales nearly linearly until CPU, memory bandwidth, storage, or queue overhead becomes limiting; directly increases effective sample size. |

---

# 12. Headless execution and observer decoupling

## 12.1 The observer is not part of the world

The live observer should be treated as an external consumer. It must not influence:

- Tick pacing in logical simulation time.
- Random-number keys or consumption.
- Entity update order.
- Resource availability.
- Checkpoint boundaries unless explicitly requested by a versioned experiment command.
- Scientific event emission.
- Any organism observation or behavior.

The same world must produce identical canonical hashes with:

- No observer connected.
- One observer connected.
- Observer paused.
- Observer sampling at different rates.
- Slow or disconnected network.
- Rendering disabled.

## 12.2 Separate rates

Genesis should distinguish:

- **Simulation rate:** logical ticks or events per wall-clock second.
- **Scientific record rate:** authoritative events and checkpoints written per wall-clock second.
- **Observer publication rate:** sampled snapshots or deltas per wall-clock second.
- **Rendering rate:** frames per second on the client.

A headless run should advance as fast as CPU and scientific I/O permit. The observer can receive, for example, one state snapshot every **K** ticks or at a bounded wall-clock frequency. Dropping observer frames is acceptable. Dropping claim-critical scientific events is not.

## 12.3 Observer data products

Possible observer products:

- Periodic read-only state snapshots.
- Region tiles at a requested level of detail.
- Entity summaries.
- Recent event excerpts.
- Precomputed nonauthoritative display aggregates.
- Checkpoint thumbnails or videos generated offline.

Every observer product should state whether it is:

- Authoritative and exact.
- Deterministically derived but sampled.
- Nonauthoritative visualization.

The UI should not present offline group or era labels as if organisms possess those labels. Clear wording such as “analysis cluster 4” is preferable to “nation 4.”

## 12.4 Bounded channels and backpressure

Use separate queues:

### Scientific event channel

- Lossless for registered claim-critical events.
- Canonical sequence and segment commitment.
- If durable recording cannot keep up, apply the registered policy: block logical execution at a phase boundary, rotate storage, or terminate the world as an evidence failure.
- Never silently drop.

### Observer channel

- Bounded and lossy.
- Latest-state or coalescing semantics.
- Dropped frames counted as telemetry.
- Cannot block or reorder world state transitions.

### Debug trace channel

- Optional and explicitly nonauthoritative unless registered.
- May be sampled deterministically.
- Must not change compiler behavior or world state; debug/release equivalence should be tested for scientific builds.

## 12.5 Headless benchmark modes

Maintain at least four benchmark profiles:

1. **Kernel-only:** no scientific logging beyond hashes; identifies compute ceiling.
2. **Research minimum:** manifest, checkpoints, primary events, hashes.
3. **Full claim-critical:** all events required by the richest planned claims.
4. **Observer stress:** full scientific record plus maximum supported observer rate.

Report throughput and memory for all four. A fast kernel-only number is not representative if the research workload is I/O-bound.

## 12.6 Observer-triggered inspection

A human may request a checkpoint or high-resolution trace. To preserve rigor:

- The request is an external experiment intervention with an event and manifest entry.
- Inspection-only snapshots occur at committed boundaries.
- Request timing cannot alter organism behavior or RNG.
- A manually selected checkpoint is exploratory unless the selection rule was registered.
- Any branch/fork from a visually chosen moment is a candidate mechanism study, not independent confirmatory evidence.

> **Major conclusion record 16 — Observer output may be sampled or dropped; scientific evidence may not.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Separation of model execution, instrumentation, and visualization is established systems practice; Genesis policy is a synthesis. |
> | **Applicability to Genesis** | Direct because live visualization can otherwise become an unmeasured performance and determinism input. |
> | **Uncertainty** | Some interactive experiment modes may intentionally intervene; these must be separately versioned. |
> | **Recommended action** | Use independent bounded channels, lossy latest-state observer delivery, and fail-explicit lossless scientific logs. |
> | **Measurement method** | Hash equality across observer configurations, dropped-frame counters, scientific-log completeness, and throughput profiles. |
> | **Control or ablation** | Observer disconnected, slow, high-rate, and multiple-client stress tests. |
> | **Determinism implications** | Wall-clock and UI behavior must not enter simulation keys or ordering. |
> | **Compute implications** | Decoupling enables accelerated headless runs while retaining low-rate live monitoring. |


---

# 13. Event logging and offline analysis

## 13.1 Two data products with different purposes

Genesis should not force one format to serve both replay and analytics.

### Authoritative simulation record

Purpose:

- Exact ordered evidence.
- Restore/replay support.
- Audit of treatment delivery and causal sequence.
- Corruption detection.

Properties:

- Canonical binary encoding.
- Append-only segmented journal.
- Complete for claim-critical event families.
- Versioned event schema and ordering policy.
- Segment hashes and root manifest.
- Written by the simulation or a logically synchronous recorder with an explicit failure policy.

### Analytical derivative

Purpose:

- Fast scans, filtering, joins, aggregation, sequence mining, network analysis, and statistics.

Properties:

- Columnar format such as Parquet, with Arrow IPC useful for local interchange or streaming analytical batches.
- Partitioned by experiment, condition, world, event family, and time range.
- Reproducibly generated from authoritative segments.
- Carries source segment hashes, conversion-tool digest, schema mapping, and row-count reconciliation.
- Disposable and rebuildable; it is not the authoritative restore source.

## 13.2 Canonical event envelope

Every authoritative event should contain or inherit from its segment:

| Field family | Required content |
|---|---|
| **Experiment provenance** | Experiment/protocol ID, world ID, condition ID, seed ID, build/config/model/event-schema/policy hashes. |
| **Logical order** | Tick or event time, phase/subphase, canonical ordinal, event ID. |
| **Actors and objects** | Stable actor, target, object, component, region, and lineage/genome/controller IDs as applicable. |
| **Type and schema** | Event-family tag, event type, version, required/optional field map. |
| **Location/context** | Position/region, local environment summary or references, opportunity state. |
| **Precondition and intent** | Relevant prior state, action/intent, authorization and eligibility. |
| **Resolution and outcome** | Accepted/rejected, conflict resolution, state delta, costs, benefits, damage, resource change. |
| **Causal provenance** | Parent event IDs, source observation/demonstration, input artifact/component IDs, predecessor transformation. |
| **Integrity** | Segment ID, byte/row ordinal, segment hash, optional per-event checksum if justified. |

Not every event should duplicate every field. Segment headers and dictionary tables can reduce redundancy, but the logical record must be self-describing under a versioned schema.

## 13.3 Event families

### A. Lifecycle and demography

- Birth, activation, maturation, death, extinction.
- Parentage and reproductive contribution.
- Population membership and eligibility intervals.

### B. Genome and controller

- Recombination and mutation requests/results.
- Structural mutation, innovation/homology identifiers.
- Genome/controller activation and schema version.
- Deterministic lesion or assay intervention.

### C. Learning and memory

- Observation/exposure.
- Plasticity/neuromodulatory update sufficient to reconstruct learned state.
- Memory write, decay, retrieval, or summary if memory is explicit.
- Learning-rule and numeric-policy version.

### D. Action and physical interaction

- Movement intent/result.
- Hold, carry, place, strike, combine, split, modify, consume, emit signal.
- Collision and conflict resolution.

### E. Artifact and structure

- Creation, transformation, join/split, ownership/holding, placement, decay, destruction.
- Component graph and material/property changes.
- Builder/modifier/user links.
- Structure candidate inputs remain raw physical events; the “structure” label is offline unless structure is merely a physical connected component with no behavioral meaning.

### F. Terrain and environment

- Region mutation, resource change, field update, weather/shock if present.
- Organism-caused versus exogenous cause.
- Persistent marks and environmental memory.

### G. Social and conflict

- Encounter and sensory opportunity.
- Demonstration and learner exposure.
- Signal send/receive.
- Attack, intervention, assistance, sharing, transfer.
- Relationship-memory input if explicitly represented.

### H. Scheduler and experiment

- Scheduled event insertion/cancellation/execution where needed for restore.
- Treatment assignment and intervention.
- Checkpoint start/commit.
- Stop, extinction, crash recovery, and invalidation.

## 13.4 Raw versus derived labels

The authoritative simulation may record physical and internal state that actually exists in the model. It should not insert observer interpretations such as:

- Culture.
- Tradition.
- Tribe, faction, nation, civilization.
- Era.
- Tool, technology, weapon, home, border.
- Coalition or war.
- Innovation significance.

Derived tables may contain:

- `analysis_group_id`.
- `candidate_tradition_id`.
- `cultural_variant_id`.
- `era_segment_id`.
- `tool_use_episode_id`.
- `innovation_network_id`.

Every derived row must include detector version, parameters, input hashes, and uncertainty or score. Those IDs must never return to the simulation.

## 13.5 Causal provenance fields

Many cultural and technological claims become much easier if raw events preserve causal candidates:

- Which organisms could perceive a demonstration.
- Which demonstrator action and result were visible.
- Which artifact components were inputs to a transformation.
- Which event created a terrain change.
- Which resource/outcome followed an object-mediated action.
- Which actors were present during a conflict.
- Which prior interaction records were available to an organism.

These links need not assert high-level causality. They preserve the mechanistic event graph from which causal tests can be constructed.

## 13.6 Logging tiers

Recommended tier policy:

| Tier | Contents | Sampling policy |
|---|---|---|
| **L0 — Provenance** | Manifests, build/config/schema hashes, seed assignment, status, final hashes. | Never sampled. |
| **L1 — Claim-critical events** | All events required to compute registered primary/secondary outcomes and treatment delivery. | Never sampled; zero loss. |
| **L2 — Periodic state summaries** | Population, resource, diversity, memory, region and performance summaries. | Fixed deterministic interval. |
| **L3 — Deterministically sampled telemetry** | High-volume events not currently claim-critical, selected by stable hash/key. | Sampling policy and inclusion probability versioned. |
| **L4 — Debug/audit traces** | Sampled RNG keys, internal solver details, subsystem states. | Enabled for fixtures or registered subset; not assumed complete. |

A future claim may require an event that earlier runs sampled. Those earlier runs cannot be retroactively treated as complete evidence. Preserve a conservative L1 schema for foreseeable culture, tool, conflict, and persistence claims.

## 13.7 Deterministic sampling

When sampling is scientifically acceptable:

- Sample by a stable hash of `(world_id, event_id, sampling_policy_version)`, not by arrival order.
- Record inclusion probability or deterministic selection predicate.
- Stratify rare event families so important categories are not missed.
- Never sample events that define a primary outcome, treatment delivery, exposure denominator, extinction, lineage, artifact transformation, or restore state.
- Validate estimates against unsampled pilot logs.

Reservoir sampling based on processing order is unsafe unless order is canonical and the algorithm/key policy is fixed.

## 13.8 Compression and storage organization

Useful techniques:

- Delta encode ticks and monotonically increasing IDs.
- Dictionary encode event types, policy IDs, material types, and repeated tags.
- Use fixed-width integers where range is known.
- Separate frequent fixed fields from optional payloads.
- Compress independent segments with Zstandard under a pinned format/level policy.
- Partition analytical Parquet files by world and coarse time/event family.
- Maintain row-count, min/max tick, and stable-ID range metadata.
- Avoid giant monolithic files that make corruption and parallel analysis expensive.

Compression settings affect physical bytes but should not affect logical-state/event hashes. Hash the canonical uncompressed record for semantic identity and the stored bytes for corruption/transport identity.

## 13.9 Event-volume planning

Estimate before scaling:

```text
raw bytes per world
≈ ticks
  × mean active organisms
  × claim-critical events per organism-tick
  × encoded bytes per event
  + environment/artifact events
  + checkpoints
  + indexes/manifests
```

Because means hide bursts, measure:

- p50, p95, and p99 events per tick.
- Maximum burst during conflict, reproduction, construction, or environmental change.
- Compression ratio by event family.
- Write amplification.
- Checkpoint size and modified-region fraction.
- Analytics conversion expansion.

Capacity planning should include at least 25–50% reserve for schema growth and unexpected event density.

## 13.10 Online summaries versus raw evidence

Online summaries can reduce analytical cost but are dangerous if they replace reconstructable evidence. Recommended rules:

- Use online summaries for operational dashboards and predeclared low-level metrics.
- Keep raw events needed for causal exposure, lineage, artifact sequence, conflict, and primary outcomes.
- Version every summary accumulator and reduction order.
- Validate summaries against offline recomputation on sampled worlds.
- Never let an online “tradition” or “era” detector change simulation behavior.

## 13.11 Offline detection methods

### Change-point detection

**Use:** abrupt or gradual shifts in world-level metrics, candidate era boundaries, innovation-rate changes, or ecological regime shifts.

**Methods:** PELT, binary segmentation, Bayesian online change-point detection, kernel methods, or multivariate state-space approaches.

**Strengths:** explicit timing and uncertainty; scalable for well-chosen summaries.

**Risks:** penalty, hazard prior, window, and variable selection determine results; a change point is not a mechanism or an era. Autocorrelation and variance changes can trigger false boundaries.

**Genesis policy:** preregister input variables and penalties for confirmation; use synthetic null and injected-change fixtures; require persistence and cross-method sensitivity.

### Hidden Markov models and latent-state models

**Use:** recurring behavioral/ecological regimes and probabilistic era-like segmentation.

**Strengths:** handles noisy observations and recurrence.

**Risks:** number of states, emissions, and Markov assumptions are observer-authored; states may be mathematically useful but semantically empty.

**Genesis policy:** label states descriptively by measured features after fitting; never treat state number as an in-world era.

### Clustering

**Use:** exploratory behavioral repertoires, artifact forms, ecological roles, or group-specific practice distributions.

**Strengths:** flexible and useful for hypothesis generation.

**Risks:** almost completely dependent on representation, scale, distance, and cluster count; every dataset can be partitioned.

**Genesis policy:** stability under resampling and alternative embeddings; held-out assignment; synthetic nulls; avoid naming clusters anthropomorphically.

### Graph community detection

**Use:** contact groups, interaction communities, cultural diffusion networks, innovation networks.

**Methods:** Leiden or other quality-function methods; dynamic/multilayer variants where justified.

**Strengths:** captures relational organization not visible in spatial clusters.

**Risks:** resolution limit, edge-definition choices, density effects, and temporal aggregation can manufacture groups.

**Genesis policy:** compare multiple time windows/resolution parameters and degree-preserving nulls; group IDs remain offline.

### Sequence mining

**Use:** repeated object-action sequences, construction pathways, candidate composite-tool procedures, innovation chains.

**Methods:** PrefixSpan-like frequent subsequence mining, episode mining, process mining, grammar induction, or probabilistic sequence models.

**Strengths:** can discover repeated procedures without named recipes.

**Risks:** combinatorial multiple testing, common-action confounding, temporal-window flexibility, and post hoc selection of successful sequences.

**Genesis policy:** require held-out replication, opportunity-adjusted frequency, outcome association, and object/function ablation before “tool sequence” claims.

### Causal diffusion and network-based diffusion analysis

**Use:** whether acquisition follows social-network exposure faster than asocial discovery.

**Strengths:** directly targets social transmission and uses event timing/network structure.

**Risks:** homophily, shared environment, network measurement error, and changing networks can mimic diffusion.

**Genesis policy:** combine with randomized arbitrary variants, social-channel ablations, and temporal negative controls. Observational NBDA is supporting evidence, not the highest causal tier.

### Phylogenetic comparative approaches

**Use:** lineage-specific innovation rates, correlated trait evolution, vertical inheritance, adaptive radiation, and divergence.

**Strengths:** leverages exact genealogy unavailable in most biological systems.

**Risks:** horizontal cultural transfer violates tree assumptions; extinction and incomplete sampling bias reconstruction; genome similarity is not phenotype identity.

**Genesis policy:** use exact genetic phylogeny for genetic questions and reticulate networks for culture. Compare innovations to lineage opportunity/exposure.

### Network-based transmission inference

**Use:** infer candidate who-copied-whom edges from exposures, timing, variant similarity, and acquisition.

**Strengths:** produces cultural-lineage networks and diffusion pathways.

**Risks:** edges are probabilistic; identical reinvention and unobserved exposure remain possible; overconfident single-parent trees are misleading.

**Genesis policy:** store edge probabilities or support scores; preserve alternative parents; validate on synthetic known-transmission fixtures.

## 13.12 Detection targets and minimum evidence

| Target | Recommended detector package | Minimum evidence beyond detector output |
|---|---|---|
| **Behavioral tradition** | Variant classifier + exposure network + persistence/turnover analysis. | Accepted social-transmission effect and originator-independent persistence. |
| **Change point** | PELT/Bayesian/multivariate method. | Synthetic calibration, persistence, alternative penalty/window, no causal wording. |
| **Era-like period** | Multivariate change points or HMM plus stable descriptive feature profile. | Robustness across methods/representations and held-out worlds; retrospective label only. |
| **Lineage-specific innovation** | Exact genealogy + first-seen/adaptive consequence detector. | Opportunity adjustment and comparison with sister/control lineages. |
| **Group-specific practice** | Dynamic community detection + behavioral distribution model. | Genetics/environment/contact controls; social transmission for cultural claim. |
| **Tool-use sequence** | Sequence mining + causal object mediation. | Function-destroying or object-removal effect and matched opportunity. |
| **Cultural diffusion** | NBDA/hazard model or transmission network. | Randomized variant or strong environmental/genetic controls. |
| **Innovation network** | Artifact/behavior descent graph with function changes. | Stable input-output provenance and uncertainty for inferred edges. |
| **Dependency chain** | Directed artifact/practice/function graph + ablation. | Predecessor removal reduces later performance and naive agents cannot rapidly bypass it. |

## 13.13 Observer-bias and false-pattern controls

- Blind detector developers to treatment labels where feasible.
- Separate detector-development worlds from confirmation worlds.
- Use synthetic datasets with known nulls and known planted patterns.
- Use time-shuffled, phase-randomized, edge-rewired, spatially permuted, and degree-preserving surrogate data.
- Register representations and thresholds.
- Report all detector families tried.
- Correct multiple testing.
- Publish negative detections and false-positive rates.
- Require manual audit only on a registered sample; do not select the most cinematic examples.
- Distinguish detector confidence from effect uncertainty across worlds.
- Confirm offline-nominated mechanisms through new-world interventions.

## 13.14 Proposed Genesis event-data strategy

1. Define a compact canonical event envelope and append-only segments.
2. Keep all lifecycle, lineage, treatment, exposure, action-resolution, artifact-transformation, terrain-change, conflict, checkpoint, and failure events lossless at L1.
3. Use stable IDs and causal parent references.
4. Store periodic canonical state hashes and subsystem hashes.
5. Store full snapshots separately; journal links to snapshot base and cursor.
6. Convert validated segments into versioned Parquet analytical partitions.
7. Create derived tables for organisms, lineages, exposures, artifacts, structures, contacts, conflicts, candidate traditions, and detector outputs.
8. Track source hashes from every derived row group to authoritative segments.
9. Run offline detectors in containers/environments with pinned dependencies and deterministic seeds.
10. Prevent any analysis output from entering simulation configuration except through a new, explicitly authored and reviewed experiment version.

> **Major conclusion record 17 — Raw physical and exposure events must remain separate from cultural/era interpretations.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Provenance and reproducible-analysis principles are mature; the raw/derived boundary is a Genesis synthesis. |
> | **Applicability to Genesis** | Central to “observe rather than instruct.” |
> | **Uncertainty** | Some low-level physical components may legitimately have in-world types; the boundary must be documented per schema. |
> | **Recommended action** | Record stable physical events and causal candidates; compute culture, group, tool, and era labels in versioned derived tables only. |
> | **Measurement method** | Source-hash lineage, detector manifests, synthetic validation, and confirmation on held-out worlds. |
> | **Control or ablation** | Shuffled/rewired/null data and new-world mechanism interventions. |
> | **Determinism implications** | Analytical derivations should be deterministic for audit, but their labels never alter world state or RNG. |
> | **Compute implications** | Maintaining raw claim-critical events increases storage but allows future reanalysis without rerunning every world. |

---

# 14. Mutable-state persistence

## 14.1 Persistence requirements created by future Genesis features

A successor format must preserve more than regenerated terrain and inherited genomes. Future-relevant state includes:

- Mutable terrain cells, regions, fields, and organism-caused marks.
- Resources and material quantities.
- Artifacts, components, joins, fractures, modifications, and positions.
- Persistent structures and connectivity.
- Organisms and exact lifecycle state.
- Genome, controller topology, initial parameters, and active controller state.
- Learned weights, plasticity traces, neuromodulatory state, and adaptation counters.
- Organism memory and social memory if explicit.
- Signals or external memory still present in the environment.
- Scheduler events, priorities, and cancellation state.
- ID-allocation state or derivation policy.
- RNG-relevant counters not reconstructible from semantic keys.
- Event-log segment, cursor, and commit status.
- Numeric, physics, learning, mutation, conflict, and ordering policy versions.

A snapshot that omits any future-affecting state is a visualization export, not a scientific checkpoint.

## 14.2 Design comparison

| Design | Advantages | Risks/limitations | Genesis fit |
|---|---|---|---|
| **Full snapshot** | Simple restore; bounded dependency; easy integrity verification and migration. | Large writes; repeated unchanged terrain/artifact bytes; pause or copy cost. | Required periodic anchor and archival baseline. |
| **Incremental snapshot** | Writes only changes; efficient when modification fraction is small. | Long dependency chains; one corrupt base/delta can invalidate later state; complex deletion semantics. | Useful between full anchors with bounded chain length. |
| **Copy-on-write snapshot** | Short pause; snapshot can be written while simulation continues; unchanged pages/chunks shared. | Runtime complexity, memory spikes, platform/filesystem coupling; page layout is not a stable archival schema. | Useful implementation technique, not the external format itself. |
| **Pure event sourcing** | Complete audit trail; state can be rebuilt; natural causal history. | Replay cost grows; event semantics/migrations become difficult; logs may omit derived or scheduler state; one historical bug contaminates replay. | Not recommended as the sole long-term restore mechanism. |
| **Snapshot + event journal** | Bounded restore from recent snapshot; short-interval crash recovery; auditability. | Requires exact snapshot/log cursor consistency and event-version policy. | Recommended core architecture. |
| **Content-addressed chunks** | Deduplication, corruption localization, immutable sharing, parallel transfer, Merkle roots. | Garbage collection and reference management; canonical chunking required; small chunks add overhead. | Strong fit for region/object graph data and immutable experiment archives. |
| **Region-based storage** | Localizes mutable terrain and structures; supports incremental writes and parallel I/O. | Cross-region entities/transactions and moving objects complicate consistency. | Recommended physical organization with explicit global tables and atomic snapshot cuts. |

## 14.3 Recommended hybrid

Use:

- Periodic canonical full logical snapshots.
- Append-only journal segments between snapshots.
- Optional content-addressed region/object chunks to avoid rewriting unchanged data.
- Bounded incremental chain, after which a new full logical anchor is required.
- Separate authoritative event history and analytical derivatives.

“Full logical snapshot” does not require one monolithic file. It means the manifest root references a complete set of chunks sufficient to restore the world without older snapshot roots.

## 14.4 Snapshot cut and phase boundary

The simplest valid snapshot occurs only after a committed simulation phase when:

- No pending intents remain.
- All conflicts have resolved.
- Scheduler updates are committed.
- Event segment is committed through a declared cursor.
- Learned state and memories correspond to the same tick.
- Region/object ownership transfers are complete.

If asynchronous writers are used, the engine must freeze or version the logical state. Chandy–Lamport-style distributed snapshot principles are relevant if a world is truly distributed across communicating processes, but a single-world engine should prefer a simpler synchronous committed boundary where possible.

## 14.5 Region-based mutable terrain

Recommended region principles:

- Fixed canonical region coordinates independent of storage layout.
- Stable cell/object encoding within each region.
- Explicit halo or cross-region references.
- Moving object belongs to exactly one authoritative region at a committed boundary.
- Cross-region placement, joins, or conflicts are represented as transactions resolved before snapshot commit.
- Region chunk hash covers canonical semantic content, not allocator layout.
- Unmodified regions may reference content shared with earlier snapshots.

Global state—scheduler, ID policy, world configuration, lineage tables, and cross-region index—must be included in the snapshot root.

## 14.6 Artifact and composite-object graphs

Artifacts may become graphs rather than flat entities. Save:

- Stable artifact and component IDs.
- Material/properties and local state.
- Directed/undirected connection type and parameters.
- Parent transformation or assembly provenance if part of authoritative state.
- Position/orientation and owning/holding organism.
- Constraints, damage, wear, and pending interactions.
- Canonical component ordering.

Validate:

- No duplicate component membership unless explicitly allowed.
- No dangling connection.
- Graph constraints and maximum bounds.
- Ownership consistency.
- Region placement consistency.
- Canonical connected-component representation.

## 14.7 Learned and memory state

For exact continuation, store:

- Current learned synaptic values.
- Plasticity eligibility traces.
- Neuromodulator traces or buffers.
- Recurrent activations or internal controller state if carried across ticks.
- Lifetime learning counters and decay clocks.
- Organism episodic/social memory contents, timestamps, capacities, and eviction order.
- Any normalization statistics that affect future inference.
- Numeric-policy version.

Reconstructing learned state from events may be possible, but it is too expensive and fragile as the sole recovery path. The snapshot should contain it directly; logs audit it.

## 14.8 Scheduler and RNG state

A deterministic restart must preserve:

- Next logical tick/phase.
- Scheduled event set and total order.
- Cancellation/tombstone state.
- Stable event IDs and local ordinals.
- ID allocation state.
- RNG policy version.
- Stateful counters only where semantic keys are insufficient.
- Experiment interventions already delivered and pending.

If RNG is fully event-keyed, most stream state disappears. However, any algorithm using sequential attempts, generated local ordinals, or persistent noise processes must save the relevant indices/state.

## 14.9 Event-log cursor consistency

The snapshot manifest should identify:

- Last fully committed event segment.
- Canonical event ordinal/tick included in the snapshot.
- Journal segment expected next.
- Hash chain or Merkle root through the included event record.

On restore, reject:

- A journal that starts before/after the expected cursor without an explicit branch manifest.
- Duplicate segments.
- Missing segment in a required chain.
- Segment whose declared base snapshot differs.

Branching from a checkpoint should create a new branch/world identifier and preserve the parent snapshot hash and intervention record.

## 14.10 Corruption detection and recovery

Use layers:

- File/chunk length and bounds.
- Fast checksum for accidental corruption.
- Cryptographic content hash.
- Root manifest/Merkle hash.
- Canonical logical-state hash.
- Referential and semantic invariant validation.
- Short replay verification.

Keep at least one prior validated snapshot until the new snapshot and journal chain are fully committed and restored in a test path. A successful write call is not proof of a valid checkpoint.

## 14.11 Event sourcing limitations

Pure replay from genesis is attractive because it seems conceptually complete. It fails operationally when:

- Billions of events make recovery too slow.
- Event schemas change meaning.
- Old events depended on implementation bugs or numeric policies.
- Events are insufficient to reconstruct hidden caches or scheduler details.
- A corruption early in the chain destroys the whole future.
- Migration requires reinterpreting every historical event.

Use event history as evidence and short journal recovery, not as the only materialized state.

> **Major conclusion record 18 — Snapshot plus journal is the recommended persistence core.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Checkpoint/journal and snapshot principles are mature; exact Genesis chunking is a synthesis. |
> | **Applicability to Genesis** | Direct once terrain, artifacts, learning, memory, and scheduler state become mutable. |
> | **Uncertainty** | Optimal region/chunk size and full-snapshot cadence depend on measured mutation locality, failure rate, and storage. |
> | **Recommended action** | Periodic complete logical anchors plus append-only journal, with bounded incremental/content-addressed region chunks. |
> | **Measurement method** | Checkpoint size/time, modified fraction, restore time, replay length, deduplication, corruption localization, and hash equivalence. |
> | **Control or ablation** | Uninterrupted versus restored run; corrupted/truncated chunk fixtures; old-version migration fixtures. |
> | **Determinism implications** | Snapshot must contain every future-affecting state and exact event cursor; restore hash equality is mandatory. |
> | **Compute implications** | More complex than monolithic snapshots but reduces write volume and recovery loss; requires garbage collection and manifest management. |

---

# 15. Save-format and migration strategy

## 15.1 Research recommendation

Adopt a **versioned, canonical, chunked snapshot container** with:

- A small fixed-format bootstrap header.
- An immutable manifest.
- Typed content-addressed chunks.
- A root logical-state hash.
- A linked append-only journal.
- Explicit policy-version references.
- Fail-closed restore.
- Pure one-version-at-a-time migration tools.

Do not serialize live Rust structs or memory pages as the archival format. Runtime copy-on-write may accelerate capture, but the exported representation must be schema-defined and platform-neutral within the declared compatibility goal.

## 15.2 Conceptual container layout

### Bootstrap header

- Magic bytes.
- Container-format major/minor version.
- Endianness declaration for fixed fields.
- Manifest offset/length.
- Manifest stored-byte hash.
- Required decoder capability flags.

### Manifest

- World and experiment identity.
- Parent snapshot/branch hash.
- Logical tick/phase.
- Build/source/config/model/schema/policy hashes.
- Determinism tier/execution class.
- Canonical serialization version.
- Numeric/RNG/conflict/order policy versions.
- Chunk table.
- Event-log cursor and segment root.
- Root logical-state hash.
- Creation metadata that is explicitly nonsemantic.

### Typed chunks

Possible chunk types:

- World/global state.
- Region terrain/resource state.
- Organism table.
- Genome/controller table.
- Learned/plastic state.
- Organism/social memory.
- Artifact/component graph.
- Structure/connectivity state.
- Scheduler queue.
- Stable-ID/allocation state.
- String/type dictionaries.
- Optional indexes.

Each chunk descriptor should contain:

- Type and schema version.
- Canonical coordinate/ID range.
- Uncompressed logical length.
- Compression codec/version/parameters.
- Stored length.
- Fast checksum.
- Cryptographic stored-byte hash.
- Canonical logical-content hash.
- Required/optional flag.

## 15.3 Canonical serialization rules

Define once and test heavily:

- Fixed-width integer types in the file schema.
- Explicit endianness.
- Canonical boolean and enum encoding.
- Length-prefix bounds.
- Canonical ordering of entities, edges, fields, and map entries.
- Duplicate-key rejection.
- Canonical representation of absent/optional values.
- Fixed-point scale and range.
- Float bit representation, NaN canonicalization, and signed-zero policy if floats are persisted.
- Canonical string normalization policy where strings affect semantics; preferably stable numeric identifiers for semantic types.
- Unknown optional field behavior and unknown required field rejection.

Canonicality allows two logically identical states to produce the same logical hash even if physical chunk placement or compression differs.

## 15.4 Full, incremental, and content-addressed roots

Maintain two concepts:

- **Complete root:** references all chunks needed for independent restore.
- **Incremental root:** references a base root plus changed/tombstone chunks.

Policy:

- Every **K** checkpoints or maximum chain depth, emit a complete root.
- A complete logical root may reuse immutable content-addressed chunks from earlier roots but must list the full dependency set.
- Garbage collection occurs only when no retained root references a chunk.
- Scientific archival packages should be self-contained or include an explicit verified content bundle, not depend on an untracked local chunk store.

## 15.5 Atomic write and commit protocol

1. Write chunks to temporary names or content-addressed staging.
2. Flush and verify stored hashes.
3. Write manifest referencing only verified chunks.
4. Re-read and validate the manifest and a registered sample/all chunks according to policy.
5. Optionally perform an immediate restore-and-hash verification.
6. Atomically publish the manifest/root commit marker.
7. Only then mark the checkpoint usable and allow prior checkpoint retirement.

On distributed/object storage, “atomic rename” may not exist with local-filesystem semantics. Use immutable objects plus a final small commit object whose creation is atomic under the storage service's guarantees.

## 15.6 Schema evolution

Classify changes:

- **Additive optional:** older readers can ignore; newer readers supply defaults only when semantics are truly unchanged.
- **Additive required:** new major capability; older readers fail closed.
- **Renamed/retyped field:** migration required.
- **Changed units or numeric policy:** semantic migration with explicit conversion and potential loss analysis.
- **Changed model rule:** generally not a save-format migration. Continuing under a new model version is an experimental branch and must be labeled.
- **Deleted field:** reserve field/tag identifier permanently; do not reuse.

A format version does not fully describe behavior. Preserve separate model, physics, RNG, arithmetic, event-ordering, learning, and mutation policy versions.

## 15.7 Migration design

Migrations should be:

- **Pure:** source snapshot is not modified.
- **One-step:** version `n → n+1`; compose steps for older saves.
- **Deterministic:** same source and migration tool produce the same canonical destination.
- **Fail-closed:** unknown/corrupt/ambiguous data is rejected.
- **Auditable:** output manifest records source root, migration tool/build, rule set, warnings, and destination root.
- **Tested:** golden fixtures, property tests, boundary values, corruption fixtures, and semantic invariants.
- **Reversible when feasible:** or explicitly documented as lossy.

Do not silently invent missing learned weights, memory ordering, scheduler state, or artifact properties. A default is acceptable only if the older schema unambiguously implied that value.

## 15.8 Migration versus scientific continuation

Two distinct operations must not be confused:

1. **Format migration:** re-encodes the same logical model state under a new storage schema.
2. **Model-version continuation:** starts a branch whose future transitions follow new physics, learning, mutation, or numeric rules.

A migrated snapshot should preserve its logical-state hash under a version-independent canonical semantics where possible. A model-version continuation must record the parent root and a model-change intervention; results before and after are not one homogeneous experimental condition.

## 15.9 Restoration procedure

A production restore should:

1. Validate bootstrap bounds and manifest hash.
2. Verify required decoder/policy support.
3. Validate every required chunk descriptor and stored hash.
4. Decompress under bounded resource limits.
5. Parse canonical data, rejecting duplicates/noncanonical order/unknown required fields.
6. Validate IDs, references, graph constraints, ranges, ownership, scheduler order, and event cursor.
7. Recompute per-chunk logical hashes and root logical-state hash.
8. Instantiate runtime state without relying on file order beyond the canonical schema.
9. Re-emit or compare a deterministic restored-state summary.
10. Run a short verification replay when expected hashes are available.
11. Only then mark the world resumable.

## 15.10 Branching and provenance

Every checkpoint fork should record:

- Parent snapshot root.
- Parent world and experiment.
- Fork tick/phase.
- Intervention manifest.
- New world/branch ID.
- New condition ID if scientific.
- RNG domain policy for post-fork draws.
- Whether the fork is exploratory or preregistered.

Never overwrite the parent history. A checkpoint-fork comparison is paired causal evidence, not a second independent seed.

## 15.11 Save-format acceptance tests

- Golden read/write fixtures for every supported version.
- Round-trip logical hash equality.
- Canonical byte/logical hash equality across insertion orders.
- Unknown required field/version rejection.
- Duplicate ID and dangling-reference rejection.
- Truncation at every byte boundary for small fixtures.
- Bit corruption in header, manifest, and chunks.
- Decompression bomb and malicious length defenses.
- Cross-region object/structure fixtures.
- Learned-state and scheduler continuity fixtures.
- Uninterrupted versus restored execution through multiple checkpoints.
- Migration chain and direct semantic equivalence.
- Crash during each commit stage.
- Garbage-collection reachability safety.

> **Major conclusion record 19 — Save schema, model policy, and scientific branch identity must be separate.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Schema-evolution and provenance principles are mature; the policy separation is a Genesis synthesis. |
> | **Applicability to Genesis** | Direct because future changes affect terrain, neural state, event meaning, and exact replay. |
> | **Uncertainty** | Some semantic migrations may be impossible without loss; this should result in unsupported continuation, not guessed state. |
> | **Recommended action** | Version container, state schema, event schema, numeric/RNG/order policies, and model behavior independently. Treat rule changes as branches. |
> | **Measurement method** | Golden migrations, logical hash/invariant checks, provenance graph, and uninterrupted-versus-restored continuation. |
> | **Control or ablation** | Corrupt/unknown-version fixtures; migration from every supported predecessor; cross-build restore matrix. |
> | **Determinism implications** | A parseable file is not enough; future transition semantics must be identified exactly. |
> | **Compute implications** | Chunking and migrations add tooling cost but prevent irreproducible long-run archives and unbounded replay. |


---

# 16. Performance and compute planning

## 16.1 Performance is measured against the scientific workload

A benchmark is useful only if it resembles the experiment it is intended to enable. Genesis should report at least:

- Simulated ticks per second.
- Agent-updates per second.
- Eligible interaction opportunities per second for the relevant phase.
- Valid independent worlds completed per day.
- Compute-hours per accepted experimental unit.
- Peak and sustained memory.
- Checkpoint write and restore time.
- Authoritative event bytes per simulated tick and per world-hour.
- Compression throughput and ratio.
- Analysis conversion and detector cost.
- Energy or cloud cost where relevant.

The most decision-relevant quantity is often **cost per valid independent replicate**, not peak tick rate.

## 16.2 Reference benchmark suite

Maintain versioned fixtures representing:

1. **Baseline ecology:** current immutable world, fixed-topology controller, no learning.
2. **Large population:** stresses perception, collision, and reproduction.
3. **Variable topology:** representative controller-size distribution, not only the smallest network.
4. **Plasticity:** learned-weight and trace updates.
5. **Social exposure:** dense interaction and memory logging.
6. **Mutable terrain/artifacts:** active construction and region writes.
7. **Conflict burst:** high contention and canonical resolution.
8. **Persistence stress:** maximum practical checkpoint and restore state.
9. **Full research logging:** all planned L1 events.
10. **Long soak:** memory growth, journal rotation, and checkpoint garbage collection.

Each benchmark should fix model configuration, seed, duration, execution class, logging tier, and expected hashes.

## 16.3 Performance measurement protocol

- Warm up caches and code paths under a declared rule.
- Report median and tail latency/throughput over repeated trials.
- Pin or record CPU frequency policy and contention.
- Record CPU model, cores, NUMA topology, memory, storage, OS/kernel, compiler, target features, GPU/driver if used.
- Separate kernel compute, logging, compression, checkpointing, and observer costs.
- Profile representative long runs; microbenchmarks can misidentify the dominant cost.
- Compare against the single-thread deterministic reference.
- Include validation and recovery overhead in end-to-end numbers.
- Publish flame graphs or equivalent profiles as artifacts where licensing/security permits.

## 16.4 CPU parallelism and vectorization

### Likely CPU strengths

- Branch-heavy organism logic.
- Dynamic sparse controller traversal.
- Object and graph manipulation.
- Complex conflict resolution.
- Region-local work with irregular but cacheable structures.
- Compression, hashing, and analytical conversion.

### Vectorization candidates

- Same-topology batched neural layers.
- Uniform sensor transforms.
- Fixed-radius grid/field kernels.
- Fixed-point arithmetic over contiguous arrays.
- Hashing/compression libraries with deterministic output policies.

Vectorization can alter floating-point operation grouping or choose target-specific instructions. A speedup is acceptable only within the declared numeric/determinism policy.

## 16.5 GPU execution: evidence-based scope

GPU agent-based frameworks such as FLAME GPU show that large speedups are possible when agent functions expose massive parallelism and data layouts suit GPU execution. That evidence does **not** imply that an irregular evolving Genesis world will benefit as a whole.

### Likely poor GPU mapping

- Variable neural topology with many small sparse networks.
- Dynamic graph mutation and memory allocation.
- Highly divergent organism actions and controller paths.
- Pointer-rich artifact/structure graphs.
- Irregular neighborhood sizes and random memory access.
- Frequent global conflict resolution.
- Fine-grained entity creation/deletion.
- Exact ordered reductions or atomic updates.
- Branch-heavy social memory and sequence processing.
- Continuous CPU–GPU transfers for observer, logs, or checkpoints.

Irregular-program studies consistently identify control divergence, memory indirection, low locality, and synchronization as GPU limitations. Deterministic GPU execution can also require disabling faster algorithms or atomics.

### Plausible GPU candidates

Only after profiling:

- Dense batched inference for organisms sharing a topology and tensor shape.
- Regular convolutional/local perception over a grid.
- Uniform environmental diffusion or field updates.
- Batched standardized genome/controller assays.
- Offline embeddings, clustering, sequence models, or compression if deterministic exact replay is not required for simulation state.
- GPU-native worlds deliberately designed around structure-of-arrays and bounded agent functions—provided that design does not distort the scientific model merely to fit hardware.

### GPU acceptance test

A GPU path should be accepted only if:

- End-to-end wall-clock improves materially at the intended world size and batch size.
- Data transfer, packing, synchronization, checkpoint, and logging costs are included.
- Output matches the declared deterministic reference or a separately justified numerical-equivalence policy.
- Speedup persists across representative controller and object heterogeneity.
- Memory capacity and failure recovery remain acceptable.
- The CPU path remains available as a research-correct reference.

Kernel-only speedup is insufficient.

## 16.6 Amdahl and bottleneck migration

If a fraction `p` of runtime is accelerated by factor `s`, overall speedup is limited by:

```text
overall speedup = 1 / ((1 - p) + p/s)
```

Even a 100× neural-inference kernel produces little benefit if inference is 5% of end-to-end time. As features evolve, bottlenecks will move from controller evaluation to spatial queries, object graphs, logging, memory bandwidth, or persistence. Reprofile every major phase.

## 16.7 Memory planning

Track memory by semantic owner:

- Terrain base and mutable deltas.
- Spatial index.
- Organisms and phenotype state.
- Genomes and controller topologies.
- Learned weights/traces.
- Memories.
- Artifacts/structures and connectivity.
- Scheduler.
- Intent buffers.
- Event buffers/compression.
- Snapshot copy-on-write or staging.
- Observer products.
- Analysis conversion.

Report bytes per active organism, connection, artifact component, modified cell, scheduled event, and retained event. These unit costs support extrapolation better than one total RSS number.

Long-soak tests should detect:

- Unbounded caches.
- Tombstones never reclaimed.
- Lineage/event indexes growing faster than policy.
- Snapshot/chunk reference leaks.
- Fragmentation.
- Memory spikes during checkpoint and compression.

## 16.8 Storage planning

For each experiment, produce a storage bill of materials:

- Number of worlds.
- Maximum ticks/turnovers.
- L1 event rate and size distribution.
- L2/L3 policy.
- Snapshot interval and expected modified fraction.
- Compression ratio uncertainty.
- Temporary write amplification.
- Analytical derivative size.
- Replication/backup factor.
- Retention tiers.

Suggested retention classes:

- **Permanent:** manifest, per-world outcome, final/root hashes, accepted-claim L1 logs, key snapshots, analysis code.
- **Long-term:** complete authoritative logs for confirmatory and unusual worlds.
- **Regenerable:** Parquet derivatives and observer media.
- **Temporary:** full debug traces, staging, failed upload fragments.

Do not delete “failed” or null worlds from confirmatory studies; they are part of the evidence.

## 16.9 Checkpoint and journal cost planning

Measure:

- Snapshot pause/copy latency.
- Background write duration.
- Journal flush latency.
- Restore and verification time.
- Shared-storage congestion across concurrent worlds.
- Deduplication hit rate.
- Incremental-chain depth effect.

Stagger checkpoints deterministically by world ID or scheduler policy to avoid all workers writing simultaneously. The stagger may depend on operational metadata, but each world's logical checkpoint ticks must remain those in its manifest.

## 16.10 Reproducible build environments

A reproducible experiment environment should include:

- Source revision and clean/dirty state.
- Exact Rust toolchain.
- `Cargo.lock` and feature set.
- Build script and environment variables.
- Target triple and CPU features.
- Linker and native dependency versions.
- Container or image digest where used.
- OS/kernel and relevant runtime libraries.
- GPU driver/runtime/library versions where used.
- Binary digest and embedded manifest.

A container improves environment capture but does not automatically make CPU instruction behavior, kernel scheduling, filesystem durability, or GPU algorithms identical.

## 16.11 Capacity planning for an experiment

A planning worksheet should estimate:

```text
world compute hours
= planned worlds
  × expected wall-clock hours per world
  × retry/recovery factor

storage
= planned worlds
  × (event logs + snapshots + derivatives)
  × replication factor

calendar duration
≈ world compute hours / effective concurrent workers
  + queue, validation, and analysis overhead
```

Add uncertainty bands based on p50/p95 runtimes and extinction. Early extinction reduces some compute but is an outcome, not a scheduling assumption to exploit selectively.

## 16.12 Performance budgets by phase

A feature may legitimately cost substantial throughput if it creates essential scientific capacity. The decision should compare alternatives:

- Scientific value and claim enabled.
- Effective replicate count under available compute.
- Opportunity rate per wall-clock hour.
- Event/storage cost.
- Determinism complexity.
- Simpler approximation or assay alternative.

Reject a fixed “no more than 10% slowdown” rule across all phases. Instead, set a phase-specific budget that still permits the powered experiment.

> **Major conclusion record 20 — GPU use must be justified by end-to-end, workload-specific evidence.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/B.** GPU ABM speedups and irregular-workload limitations are supported; Genesis mapping remains unmeasured. |
> | **Applicability to Genesis** | Direct caution because variable topologies, sparse objects, divergence, and exact conflicts are likely. |
> | **Uncertainty** | Future data layouts and batched kernels may create substantial opportunities. |
> | **Recommended action** | Profile first; retain CPU reference; accelerate only uniform high-cost kernels; benchmark complete worlds. |
> | **Measurement method** | End-to-end speedup, transfer/synchronization share, utilization, determinism hash matrix, and heterogeneity sensitivity. |
> | **Control or ablation** | CPU scalar/reference, CPU SIMD, GPU deterministic and fastest modes, varying batch/world/controller distributions. |
> | **Determinism implications** | GPU architecture, driver, library algorithm, atomics, and floating policy must enter the execution-class claim. |
> | **Compute implications** | A useful kernel can lower cost per world; a poor mapping can increase complexity, transfers, and validation cost with no net gain. |

> **Major conclusion record 21 — Optimize cost per valid independent world, not headline tick rate.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** End-to-end benchmarking is established; the scientific-unit objective is a Genesis synthesis. |
> | **Applicability to Genesis** | Direct because inferential power depends on independent worlds and valid evidence packages. |
> | **Uncertainty** | Some phases may be opportunity-limited rather than tick-limited. |
> | **Recommended action** | Benchmark worlds/day, eligible events/day, valid replicate cost, storage, and recovery—not kernel-only throughput. |
> | **Measurement method** | Complete task lifecycle metrics with p50/p95, invalid/retry rate, and power-per-compute simulations. |
> | **Control or ablation** | Logging tiers, checkpoint intervals, process/thread modes, observer load, and feature profiles. |
> | **Determinism implications** | Faster mode is acceptable only if it passes the declared hash/evidence gates. |
> | **Compute implications** | Directly aligns engineering priorities with the research program. |

---

# 17. Null-result reporting

## 17.1 Null results are not one category

Genesis should classify a non-acceptance as:

1. **Informative negative:** the interval excludes the SESOI or equivalence/ROPE supports a practically small effect.
2. **Inconclusive:** uncertainty includes both meaningful benefit and no/practically harmful effect.
3. **Opportunity-limited:** the mechanism was implemented but worlds rarely encountered the conditions needed to express it.
4. **Mechanism-delivery failure:** treatment did not alter the intended intermediate state.
5. **Detector-limited:** positive control or synthetic validation failed.
6. **Implementation-invalid:** invariants, deterministic replay, or restore failed.
7. **Evidence-invalid:** claim-critical records are missing/corrupt.
8. **Performance-invalid:** adequate replication cannot be achieved within the phase budget.
9. **Exploratory:** the protocol, detector, exclusions, or stopping changed after outcomes.

Only the first category supports a strong statement that a meaningful effect is unlikely under the tested conditions.

## 17.2 Required null report

### Claim

- State the exact hypothesis and claim level sought.
- Use “did not detect” unless equivalence or ROPE supports practical absence.

### Design and exposure

- Treatment/control, independent worlds, seed registry, duration.
- Actual opportunities per world.
- Extinctions, censoring, crashes, and invalid worlds.
- Treatment-delivery/intermediate-mechanism check.

### Effect and precision

- Point estimate in raw units.
- 95% confidence/credible interval.
- SESOI/equivalence bounds.
- Tail or rare-event probability where relevant.
- World-level plot/table.

### Decision

- Did the interval exclude the SESOI?
- Did an equivalence test pass?
- What is the posterior probability above the SESOI or within the ROPE?
- Was the maximum sample reached or was there a valid futility stop?

### Diagnostics and robustness

- Positive/negative controls.
- Detector sensitivity.
- Heavy-tail analysis.
- Extinction/censoring alternatives.
- Paired/unpaired analysis.
- Representation sensitivity.
- Protocol deviations.

### Engineering validity

- Replay/restore result.
- Event completeness.
- Performance budget.
- Incidents and recovery.

### Interpretation

- What effect sizes are ruled out.
- What effects remain plausible.
- Whether a different environment/dose/affordance is a new hypothesis rather than an explanation inferred from the same data.

## 17.3 Example null wording

### Informative negative

> Across 48 paired seed blocks, enabling social plasticity changed the probability of persistent tradition by 3 percentage points (95% CI −6 to 11). The preregistered SESOI was +20 percentage points, and the interval lay within the equivalence bounds of ±12 points. The study therefore found no effect of practical importance under this model, exposure level, and 2-million-tick horizon. It does not rule out smaller effects or effects under different environmental regimes.

### Inconclusive

> The estimated effect was +18 percentage points (95% CI −5 to 41) relative to a +20-point SESOI. The data are compatible with no effect and with a meaningful effect; the result is inconclusive. Only 21 of 48 worlds reached the minimum demonstration exposure because of early extinction.

### Zero rare events

> No qualifying coalitionary-conflict events occurred in 40 treatment worlds or 40 controls. Under a simple independent-world binomial model, the upper 95% bound remains nontrivial; the study cannot conclude that the event is impossible. The environment generated fewer than the preregistered eligible observed conflicts in most worlds, so this is opportunity-limited rather than an informative negative.

## 17.4 Extinction in null reports

Always report:

- Extinction probability by condition.
- Time to extinction.
- Whether primary outcome counts extinction as failure/non-event.
- Outcome among all worlds and, only as secondary, among survivors.
- Competing-risk analysis where appropriate.

A treatment that produces impressive culture among a small surviving subset while causing most worlds to go extinct may not be scientifically or engineering-wise successful.

## 17.5 Nulls in open-endedness tests

A plateau or lack of innovation can mean:

- A true model ceiling.
- Insufficient duration or population.
- Descriptor saturation.
- Detector insensitivity.
- Extinction/bottleneck.
- Compute cap.
- Low mutation/learning opportunity.
- Ecological lock-in.

Report the strongest bounded statement supported:

- “Metric M saturated under representation R by horizon T.”
- “No adaptive innovations meeting threshold δ were detected in N worlds.”
- “The experiment cannot distinguish a true ceiling from a horizon longer than T.”

Do not infer that the mechanism is fundamentally incapable of open-endedness from one budget.

## 17.6 Publication and retention policy

- Keep all confirmatory worlds, including nulls and extinctions.
- Publish outcome tables and intervals.
- Register follow-up hypotheses before rerunning.
- Do not replace a null primary result with a favorable secondary detector.
- Include null experiments in model-version history and phase decisions.
- Track repeated attempts so a later success is not presented without the earlier failures.

> **Major conclusion record 22 — “No significant effect” is not evidence of no meaningful effect.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A.** Equivalence testing, interval interpretation, and Bayesian ROPE principles are mature. |
> | **Applicability to Genesis** | Direct because expensive simulations often have wide uncertainty and rare opportunities. |
> | **Uncertainty** | SESOI choice remains a scientific judgment and should be sensitivity-tested. |
> | **Recommended action** | Report interval versus SESOI, equivalence/ROPE, achieved opportunity, and null classification. |
> | **Measurement method** | World-level effect intervals, equivalence tests, posterior probabilities, power and opportunity diagnostics. |
> | **Control or ablation** | Positive-control detector sensitivity and treatment-delivery checks. |
> | **Determinism implications** | Invalid replay or incomplete evidence converts a null into an invalid experiment, not a negative scientific result. |
> | **Compute implications** | More worlds are warranted only when the current interval leaves meaningful effects unresolved and the opportunity mechanism is valid. |

---

# 18. Reproducibility and determinism checklists

## 18.1 Genesis Minimum Experiment Reporting Standard (GMERS)

A report is GMERS-complete only if every applicable item is supplied.

### 1. Research question and claim

- [ ] Exact hypothesis and proposed mechanism.
- [ ] Evidence level G0–G6 sought and achieved.
- [ ] Claims explicitly not supported.
- [ ] Primary estimand and SESOI.
- [ ] Preregistration/immutable protocol identifier and timestamp.

### 2. Model description

- [ ] Human-readable ODD-style Overview, Design concepts, and Details.
- [ ] Agent, environment, artifact, learning, inheritance, and scheduler rules.
- [ ] Initial conditions and seed-generation process.
- [ ] Interaction/conflict order and randomness policy.
- [ ] State limits: population, map, memory, topology, object count, precision.
- [ ] Differences from prior model version.

### 3. Software and build

- [ ] Repository and source revision.
- [ ] Clean/dirty state and patch bundle if needed.
- [ ] Binary digest.
- [ ] Rust toolchain, target, profile, flags, and features.
- [ ] `Cargo.lock` and native dependency versions.
- [ ] Container/image digest or build environment manifest.
- [ ] License and access instructions.

### 4. Experiment design

- [ ] Treatment, control, ablation, positive and negative controls.
- [ ] Experimental unit.
- [ ] Number of worlds and power rationale.
- [ ] Seed registry and condition assignment.
- [ ] Blocking/pairing and common-random-number policy.
- [ ] Duration, exposure requirement, maximum budget, and stopping.
- [ ] Extinction, censoring, crash, retry, and exclusion policy.
- [ ] Discovery/pilot/confirmation status.

### 5. Outcomes and analysis

- [ ] Exact primary metric computation and detector version.
- [ ] Secondary/exploratory metrics labeled.
- [ ] Statistical model, effect scale, interval, and decision rule.
- [ ] Multiplicity and sequential policy.
- [ ] Heavy-tail, rare-event, and censoring handling.
- [ ] All-world outcome table/plot.
- [ ] Robustness and representation sensitivity.
- [ ] Deviations from preregistration.

### 6. Determinism and validation

- [ ] Declared determinism tier.
- [ ] Same-run/restart/thread/hardware test matrix.
- [ ] Uninterrupted versus restore hashes.
- [ ] Event completeness and segment hashes.
- [ ] Treatment-delivery and invariant tests.
- [ ] Unknown/corrupt save failure tests.
- [ ] Divergence incident record.

### 7. Performance and resources

- [ ] Hardware and OS/kernel.
- [ ] Throughput, worlds/day, peak RSS, storage, checkpoint, and restore metrics.
- [ ] Logging/observer overhead.
- [ ] Failure/retry rate.
- [ ] Performance budget pass/fail.
- [ ] Total compute and storage consumed.

### 8. Artifacts

- [ ] Experiment manifest and seed assignment.
- [ ] Source/build environment.
- [ ] Snapshots and authoritative event roots.
- [ ] Derived analytical data and source-hash map.
- [ ] Analysis code/notebook and environment lock.
- [ ] Detector configs/models.
- [ ] Readme with reproduction path.
- [ ] Artifact integrity manifest.

GMERS should be used alongside ODD for model description and STRESS/TRACE-style reporting for simulation execution, validation, and traceability.

## 18.2 Reproducibility checklist

### Planning

- [ ] Separate exploratory and confirmatory seed registries.
- [ ] Freeze primary endpoint, SESOI, stopping rule, and exclusions.
- [ ] Document model purpose and claim ceiling.
- [ ] Register positive and negative controls.
- [ ] Simulate power and error rates under realistic world-level distributions.

### Code and environment

- [ ] Archive exact source and build scripts.
- [ ] Pin compiler/toolchain/dependencies.
- [ ] Record target CPU/GPU features and native libraries.
- [ ] Produce and hash the executable or image.
- [ ] Make external data/configuration content-addressed.
- [ ] Ensure no network-fetched mutable dependency is required during reproduction.

### Data provenance

- [ ] Hash experiment manifest, seed registry, snapshots, event segments, and derived files.
- [ ] Preserve authoritative-to-derived lineage.
- [ ] Record every conversion and detector tool version.
- [ ] Include all worlds, statuses, failures, and extinctions.
- [ ] Maintain units, schema, and data dictionary.

### Analysis

- [ ] Provide executable analysis with pinned environment.
- [ ] Recompute primary outcomes from authoritative or reconciled derivative data.
- [ ] Validate row/event counts and world coverage.
- [ ] Set and record analysis RNG seeds/domains.
- [ ] Avoid dependence on unordered file discovery or parallel reduction order.
- [ ] Produce machine-readable result tables, not only figures.

### Independent verification

- [ ] Rebuild on a clean machine.
- [ ] Reproduce a fixture and at least one complete world.
- [ ] Recompute primary analysis independently.
- [ ] Compare hashes or declared numerical tolerances.
- [ ] Record differences rather than manually reconciling them.

### Archival

- [ ] Publish a root integrity manifest.
- [ ] Use open, documented formats for analytical derivatives.
- [ ] Preserve decoder/migration tools for canonical binary data.
- [ ] Include license and citation metadata.
- [ ] Define retention and deprecation policy.

## 18.3 Determinism review checklist

### Randomness

- [ ] Counter-based or otherwise partitionable RNG chosen and statistically tested.
- [ ] Every draw has a named semantic domain and versioned key.
- [ ] Draw slots do not shift when unrelated draws are added.
- [ ] Rejection attempts use explicit attempt indices.
- [ ] No mutable global RNG.
- [ ] No worker/thread/hostname/wall-clock values in keys.
- [ ] Key-collision/reuse audit exists.

### Identity and containers

- [ ] Stable IDs are never ambiguously reused.
- [ ] Parallel creation requests are canonically ordered.
- [ ] Hash containers are never semantically traversed without sorting.
- [ ] Serialization order is canonical.
- [ ] Every sort has a complete tie-breaker.
- [ ] No pointer/allocation address enters behavior.

### Updates and conflicts

- [ ] Sense/decide uses a coherent pre-update state.
- [ ] Intents are buffered before shared mutation.
- [ ] Conflict policy is explicit and versioned.
- [ ] First-writer/arrival-order atomics do not choose outcomes.
- [ ] Social exposures are aggregated or canonically ordered.
- [ ] Cross-region transfers are atomic at commit boundaries.

### Arithmetic

- [ ] State-critical values use fixed-point or a documented numeric policy.
- [ ] Overflow behavior is explicit.
- [ ] Floating reductions have fixed order/reproducible accumulator.
- [ ] FMA, denormal, rounding, NaN, signed-zero, and transcendental policies are declared.
- [ ] Fast math is prohibited for authoritative state unless its exact semantics are part of a constrained execution class.
- [ ] SIMD/GPU variants are hash-tested.

### Parallel execution

- [ ] Single-thread reference exists.
- [ ] Supported thread counts match reference.
- [ ] Work stealing/scheduler perturbation tests pass.
- [ ] World output does not depend on completion order.
- [ ] Duplicate task execution matches.
- [ ] Reductions/aggregates are deterministic.

### Events and persistence

- [ ] Event order key is total and versioned.
- [ ] Event segments are hash-chained or root-hashed.
- [ ] Snapshot occurs at a coherent boundary.
- [ ] Every future-affecting state is persisted.
- [ ] Event cursor matches snapshot.
- [ ] Restore and uninterrupted runs match.
- [ ] Parser rejects corruption, ambiguity, and unknown required policies.

### Builds and platforms

- [ ] Toolchain, lockfile, target, and native libraries pinned.
- [ ] Determinism tier and execution class stated.
- [ ] Cross-host matrix tested for claimed tier.
- [ ] Reproducible binary build tested separately from runtime determinism.
- [ ] Dependency upgrades trigger fixture and cross-version review.

### Instrumentation

- [ ] Observer presence/rate cannot affect hashes.
- [ ] Scientific logging failure is explicit.
- [ ] Debug traces do not change state.
- [ ] Performance counters and wall clock are nonsemantic.
- [ ] Offline detector output cannot feed back.

## 18.4 Required artifacts bundle

A completed experiment directory or content-addressed package should contain:

```text
README.md
protocol/
  preregistration.md
  experiment_manifest.*
  seed_registry.*
  condition_assignment.*
software/
  source_revision.txt
  build_manifest.*
  dependency_lock/
  binary_digest.txt
worlds/
  world_status.*
  per_world_primary_outcomes.*
  snapshot_roots.*
  event_segment_roots.*
analysis/
  environment_lock.*
  scripts_or_notebooks/
  detector_manifests/
  derived_data_manifest.*
  statistical_results.*
validation/
  determinism_report.md
  restore_report.md
  event_reconciliation.md
  invariant_report.md
performance/
  benchmark_manifest.*
  resource_results.*
reports/
  GMERS_report.md
  deviations_and_incidents.md
integrity/
  artifact_root_manifest.*
```

The exact format can differ, but every object needs a stable identifier and provenance.

## 18.5 Compact phase-acceptance checklist

- [ ] Hypothesis and mechanism are falsifiable.
- [ ] Control/ablation isolates the mechanism.
- [ ] World/seed block is the experimental unit unless justified.
- [ ] Independent worlds and duration are power-calibrated.
- [ ] One primary metric and SESOI are locked.
- [ ] Statistical rule and multiplicity are locked.
- [ ] Extinction, censoring, rare events, and heavy tails are addressed.
- [ ] Positive/negative controls pass.
- [ ] Robustness and representation sensitivity pass.
- [ ] Replay, restore, event completeness, and fail-closed tests pass.
- [ ] Performance budget permits the planned replication.
- [ ] Required artifacts are complete.
- [ ] Public wording does not exceed evidence level.

> **Major conclusion record 23 — Reproducibility requires model, execution, evidence, and analysis provenance together.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/S.** Reproducible-computing, FAIR, ODD, STRESS, and artifact-evaluation guidance are established; GMERS is a synthesis. |
> | **Applicability to Genesis** | Direct because exact simulation alone does not reproduce an experiment without seeds, conditions, logs, and analysis. |
> | **Uncertainty** | Long-term archival formats and external repository limits may constrain what can be retained. |
> | **Recommended action** | Require GMERS and a content-integrity artifact bundle for every confirmatory result. |
> | **Measurement method** | Clean-machine rebuild, world replay, outcome recomputation, artifact hash validation, and independent analysis check. |
> | **Control or ablation** | Remove one provenance component in a dry run to verify that missing dependencies are detected rather than silently substituted. |
> | **Determinism implications** | Reproducible build, deterministic run, deterministic restore, and reproducible analysis are separate verified properties. |
> | **Compute implications** | Archiving and verification add cost but prevent expensive long runs from becoming unauditable anecdotes. |


---

# 19. Risks and unsupported assumptions

## 19.1 Scientific risks

### Risk: more compute is treated as the missing mechanism

Longer and larger runs can reveal rare events and delay apparent saturation. They cannot create missing affordances, inheritance channels, ecological feedback, or evolvability. Compute extends a test horizon; it does not guarantee culture, technology, language, major transitions, or open-ended evolution.

**Mitigation:** pair every scale-up with a mechanistic hypothesis, exposure model, and saturation analysis. Compare additional compute to targeted affordance or environment ablations.

### Risk: novelty is mistaken for innovation

Random behavior, genome bloat, controller growth, object clutter, and neutral combinations can all increase novelty.

**Mitigation:** require adaptive/ecological consequence, persistence, causal ablation, and representation sensitivity.

### Risk: complexity metrics reward noise or size

Compression complexity can count randomness; node/edge counts reward bloat; sequence diversity can reward indecision.

**Mitigation:** use functional pruning/lesions, size-matched random controls, causal module tests, context dependence, and multiple encodings.

### Risk: external tasks define the apparent direction of evolution

A reward for computing functions or solving a benchmark can produce strong cumulative adaptation but does not show endogenous open-ended evolution.

**Mitigation:** label external-task results as bounded adaptation or positive controls; separately measure endogenous ecological consequence.

### Risk: cultural labels are inferred from stable environmental differences

Spatially persistent resources can generate group-specific behavior with no social learning.

**Mitigation:** randomized arbitrary variants, yoked environments, spatial shuffles, social-channel ablations, and exposure-timing analysis.

### Risk: serial reinvention is labeled cumulative culture

The same affordance may independently elicit the same action. Later high performance may not inherit earlier information.

**Mitigation:** exact transmission graph, arbitrary variant labels, naive reconstruction, predecessor ablation, and lineage depth.

### Risk: rare cinematic cases dominate interpretation

A long simulation can produce extraordinary-looking sequences by chance. Human observers preferentially retain them.

**Mitigation:** preregister primary outcomes, retain all worlds, blind detector development, report prevalence and intervals, and use held-out confirmation.

### Risk: detector proliferation creates false patterns

Trying many embeddings, thresholds, windows, cluster methods, and sequence definitions guarantees apparent structure.

**Mitigation:** detector registry, exploratory/confirmatory separation, multiplicity control, surrogate nulls, and full disclosure of attempted analyses.

### Risk: formal labels imply mental states

Terms such as “goal,” “intention,” “teaching,” “ownership,” or “war” can exceed observable mechanics.

**Mitigation:** prefer operational descriptions: outcome-directed repeated action, information transfer, site-contingent exclusion, third-party intervention, or artifact dependence.

## 19.2 Experimental risks

### Risk: pseudoreplication

Millions of organisms or events from a few worlds produce false precision.

**Mitigation:** world-level replication and hierarchical/cluster-aware inference.

### Risk: seed cherry-picking

Reusing seeds that produced interesting behavior overfits the experiment.

**Mitigation:** immutable registries, discovery/confirmation separation, complete attempt ledger.

### Risk: treatment changes opportunity as well as mechanism

For example, artifact persistence may increase object density, encounter rate, and survival time, any of which can increase reuse.

**Mitigation:** matched controls, opportunity denominators, mediation analysis, function-destroying forks, and factorial designs.

### Risk: conditioning on survival

Analyzing only surviving worlds can reverse or inflate treatment effects.

**Mitigation:** all-world estimands, extinction as outcome/competing event, survivor analysis secondary.

### Risk: common random numbers are misaligned

Same mutable RNG seed across diverging conditions does not preserve corresponding randomness.

**Mitigation:** semantic counter-based keys and paired/unpaired sensitivity.

### Risk: sequential peeking

Long experiments invite stopping when the result looks favorable.

**Mitigation:** group-sequential/anytime-valid design at independent-world batches, or fixed stopping.

### Risk: positive control is too close to the target

A scripted demonstration can prove a detector works but can also leak the desired structure into the experiment.

**Mitigation:** keep fixtures separate and describe their authored information. Do not include scripted positive-control worlds in target prevalence estimates.

## 19.3 Determinism risks

### Risk: deterministic implementation hides semantic order bias

A stable “lowest ID wins” policy is deterministic but can create lifetime advantage for early IDs.

**Mitigation:** treat conflict policy as physics; test alternative policies, keyed lotteries, and selection effects.

### Risk: exact replay is claimed too broadly

Same-host replay may fail across architecture, compiler, SIMD, GPU, or library versions.

**Mitigation:** determinism tiers and execution-class matrix.

### Risk: logging or observer changes behavior

Blocking I/O, allocation pressure, or thread scheduling can alter a poorly isolated engine.

**Mitigation:** logical time separation, dedicated channels, hash equality across instrumentation profiles.

### Risk: hidden nondeterminism in dependencies

Native libraries may use dynamic algorithms, thread pools, platform math, atomics, or entropy.

**Mitigation:** dependency audit, pinned versions, deterministic fixtures, and reference implementations.

### Risk: hash equality is overtrusted

A hash collision is extremely unlikely with a strong digest, but equal final state can conceal differing histories, and a bug may omit state from the hash.

**Mitigation:** event-stream hashes, subsystem hashes, canonical-state coverage tests, and semantic invariants.

## 19.4 Persistence and data risks

### Risk: snapshot omits future-affecting state

Learned traces, scheduler events, ID generation, or event cursors are common omissions.

**Mitigation:** explicit state inventory, differential restore tests, and state-coverage review for every feature.

### Risk: event sourcing becomes unreplayable

Historical semantics and replay cost grow without bound.

**Mitigation:** periodic full logical anchors, bounded journals, versioned migrations, and decoder retention.

### Risk: incremental chain corruption

One missing base invalidates many snapshots.

**Mitigation:** bounded chain depth, content hashes, self-contained archival roots, and prior checkpoint retention.

### Risk: analytical derivative becomes de facto authority

Parquet conversion can omit or reinterpret events.

**Mitigation:** authoritative segment roots, reconciliation counts, source-hash provenance, and rebuild tests.

### Risk: storage pressure causes selective deletion

Interesting worlds may be retained while nulls are dropped.

**Mitigation:** preregister retention by experiment class, not outcome; permanent minimum evidence package for every confirmatory world.

## 19.5 Performance risks

### Risk: optimizing a microbenchmark distorts architecture

A fast controller kernel may not improve full-world throughput.

**Mitigation:** representative end-to-end benchmark suite and cost per valid world.

### Risk: within-world parallelism consumes engineering effort before independent-world scheduling

Complex deterministic merges can be expensive and fragile.

**Mitigation:** scale independent processes first and profile the remaining bottleneck.

### Risk: GPU architecture pressures the scientific model

Constraining agents to uniform dense networks or fixed action phases solely for GPU efficiency can remove the heterogeneity being studied.

**Mitigation:** preserve research requirements; use GPU only where compatible rather than redesigning science around hardware without justification.

### Risk: event volume becomes the dominant limit

Rich social and object provenance can exceed compute cost.

**Mitigation:** compact canonical schemas, tier policy, deterministic sampling only for noncritical events, compression, and pilot volume measurement.

## 19.6 Explicit unsupported assumptions

Genesis should not assume:

- Open-ended evolution will occur.
- Variable topology will produce greater behavioral complexity.
- Lifetime learning will be selected for.
- Social learning will create traditions.
- Persistent artifacts will be reused.
- Tool use will lead to construction.
- Construction will lead to cumulative technology.
- Group differences imply culture.
- Territoriality implies groups or politics.
- Damage implies organized conflict.
- Larger populations necessarily increase innovation.
- Longer runs necessarily escape stagnation.
- Mutable terrain necessarily creates niche construction.
- High mutation necessarily increases evolvability.
- More memory necessarily improves social behavior.
- High-fidelity copying is sufficient for ratcheting.
- A complex controller is using its complexity.
- A learned behavior is socially learned.
- A detected era is a real causal regime.
- GPU acceleration is beneficial.
- Same seed means paired randomness.
- Same final hash proves the same causal history.
- A parseable old save can be scientifically continued under new rules without an explicit branch.

> **Major conclusion record 24 — Mechanisms and compute create possibilities, not guaranteed emergent outcomes.**
>
> | Required field | Assessment |
> |---|---|
> | **Evidence quality** | **A/B.** OEE history and evolutionary theory show recurring stagnation and context dependence. |
> | **Applicability to Genesis** | Foundational. The project's credibility depends on falsifiable mechanism tests rather than destiny claims. |
> | **Uncertainty** | Rare combinations may produce unexpectedly rich dynamics; absence in one configuration does not establish impossibility. |
> | **Recommended action** | Phrase every feature as a hypothesis-enabling affordance and require powered ablations before scientific promotion. |
> | **Measurement method** | Multi-seed effects, opportunity/exposure, saturation, causal forks, and null classification. |
> | **Control or ablation** | Remove or vary each candidate mechanism while preserving matched opportunity. |
> | **Determinism implications** | Exact replay improves diagnosis but does not make an emergent interpretation true. |
> | **Compute implications** | Scale only after identifying whether the limitation is statistical rarity, mechanistic absence, detector weakness, or raw performance. |

---

# 20. Open questions

## 20.1 Highest-priority scientific questions

### 1. What is Genesis's primary endogenous adaptation measure?

Survival and reproduction are obvious but can reward trivial replication. Resource transformation, resilience, niche creation, and delayed descendant benefit may be relevant. The project needs a small registered set that does not become an authored objective.

**Research path:** compare candidate measures under neutral, bounded-task, coevolutionary, and niche-construction controls; test whether they reward known pathologies.

### 2. Which behavioral representation is least misleading?

Action counts miss sequence and context; raw trajectories overcount noise; learned embeddings are opaque and condition-sensitive.

**Research path:** maintain three complementary representations—symbolic event sequence, context-conditioned state/action summaries, and causal outcome features—and require conclusions to survive at least two.

### 3. How should functional complexity be measured without external tasks?

Potential approaches include deterministic lesions, minimal sufficient controller reconstruction, causal graph depth, transfer across endogenous contexts, and artifact dependency chains.

**Research path:** validate measures on synthetic controllers with known modularity/noise/bloat and on evolved controllers under matched size.

### 4. What environmental features create sustained endogenous selection pressure?

Possibilities include resource renewal patterns, predator/prey or competitive feedback, organism-created niches, material heterogeneity, persistent artifacts, and environmental inheritance.

**Research path:** factorial screening followed by held-out confirmation; measure ecological feedback rather than visual richness.

### 5. How can evolvability be assayed fairly across variable topologies?

Mutation distributions can favor some encodings. Viability and novelty depend on assay environments.

**Research path:** standardized versioned perturbation batteries, multiple mutation operators, genotype-size normalization, and cross-environment descendant distributions.

### 6. What is the minimum causal social-learning experiment supported by the controller architecture?

The design requires arbitrary demonstrator variants, learner-naive status, matched environmental consequences, and exposure logs.

**Research path:** build an isolated positive-control assay before ecological deployment; confirm social channel delivery without introducing a named cultural state.

### 7. How should cultural variants be represented?

Exact action sequences are too brittle; coarse labels can merge independent behaviors. Artifact variants may have graph structure and functional equivalence.

**Research path:** hierarchical variant representation with raw sequence, structural form, and function; report conclusions at multiple coarsenings.

### 8. How can independent reinvention be distinguished when affordances strongly canalize behavior?

Identical physics may make the same solution likely.

**Research path:** arbitrary conventions with equal payoff, exposure timing, donor-specific labels, and naive reconstruction distributions.

### 9. What constitutes a persistent group without feeding group identity into behavior?

Spatial or contact communities fluctuate. Kin groups and resource clusters can be mistaken for social groups.

**Research path:** dynamic multiplex contact networks, persistence/turnover criteria, genetic/environment controls, and offline-only IDs.

### 10. What physical affordances permit composite tools without encoding recipes?

Joining, carrying, support, striking, containment, leverage, and persistent placement are candidate primitives.

**Research path:** affordance audit and minimal physics fixtures; verify that no named sequence or hidden recipe reward exists.

## 20.2 Highest-priority methodological questions

### 11. Which primary outcomes are sufficiently frequent for powered phase gates?

Rare high-level outcomes may require hundreds of worlds. Earlier phases should gate on proximal mechanisms—learning, transmission, reuse—before demanding civilization-like outcomes.

### 12. How should opportunity be defined?

A world cannot express social transmission without demonstrator–learner exposure or tool use without manipulable object/target opportunities. Conditioning on opportunity can bias results if treatment changes opportunity.

**Research path:** define both intention-to-treat world outcomes and secondary opportunity-conditioned estimands.

### 13. How should seed distributions be constructed?

Uniform integer seeds are easy but do not guarantee balanced initial worlds. Blocking on initial terrain/founders can improve efficiency but risks selecting a narrow world population.

**Research path:** publish generator and pre-treatment blocking variables; replicate across at least one broader registry.

### 14. What is the correct horizon unit?

Ticks are implementation-level; generations fail with overlapping lifetimes; population turnovers and event opportunities may be more interpretable.

**Research path:** report all relevant units and preregister a primary horizon tied to mechanism exposure.

### 15. How should model-version effects be analyzed?

Long development means mechanisms and optimizations will change.

**Research path:** bridge experiments across versions using shared seed registries, compatibility fixtures, and hierarchical version effects; do not pool automatically.

### 16. Can bounded formal tests complement empirical OEE metrics?

Formal state-space capacity, dynamic memory limits, or undecidability-related properties may clarify what is possible, but do not show what evolves.

**Research path:** separate formal-capacity dossier from empirical realization and detector performance.

## 20.3 Highest-priority systems questions

### 17. What is the smallest complete authoritative event schema?

Logging every internal operation is infeasible; logging only outcomes may prevent cultural causal inference.

**Research path:** claim-to-field traceability matrix: for every planned claim, list required raw fields and test reconstruction on fixtures.

### 18. What checkpoint granularity minimizes total research cost?

Optimal interval depends on failure rate, snapshot cost, journal cost, and recovery. Mutable terrain locality is unknown.

**Research path:** instrument pilots and evaluate full, incremental, and content-addressed region strategies under injected failures.

### 19. What determinism tier is scientifically necessary?

Cross-architecture exactness is valuable but may impose large numeric and performance costs.

**Research path:** define a fixed-point reference tier and compare constrained floating/GPU tiers; decide whether exact cross-platform identity or independently validated statistical equivalence is required for each use.

### 20. Which within-world kernels dominate after planned features?

Current profiling may not predict variable topology, social memory, or construction.

**Research path:** benchmark each phase with representative heterogeneity before choosing vectorization/GPU/PDES.

### 21. How should region ownership and cross-region structures work?

Large persistent structures and moving composite artifacts can span partitions.

**Research path:** define canonical authoritative region, cross-region transaction, and snapshot rules; stress with adversarial boundary fixtures.

### 22. How long can event schemas remain replayable?

Historical journals may span years of development.

**Research path:** maintain decoders and semantic migration policy; periodically materialize full canonical anchors under supported versions; preserve old binaries/images.

### 23. Can analysis remain reproducible at scale?

Parallel Parquet scans, graph algorithms, clustering, and GPU ML can themselves be nondeterministic.

**Research path:** version analytical determinism separately, retain machine-readable primary tables, and treat stochastic detector uncertainty explicitly.

## 20.4 Recommended research sequence

1. Finalize GMERS, determinism tiers, experiment manifests, and seed registry.
2. Implement headless independent-world scheduling and lossless claim-critical logging.
3. Establish snapshot+journal successor format with restore equivalence.
4. Validate lifetime learning in acute and ecological paired-world assays.
5. Validate social transmission with randomized arbitrary variants.
6. Add persistent objects and test functional artifact reuse.
7. Add mutable structures and environmental inheritance.
8. Test tradition persistence only after social transmission passes.
9. Test territoriality and coalitionary support as separate mechanisms.
10. Treat cumulative cultural improvement and bounded OEE as long-horizon research programs, not near-term feature acceptance.

This ordering minimizes interpretive ambiguity. Each later claim depends on mechanisms validated earlier.


## 20.5 Required-recommendation crosswalk

| Required recommendation | Location in this dossier |
|---|---|
| 1. Minimum experiment-reporting standard | [Section 18.1: GMERS](#181-genesis-minimum-experiment-reporting-standard-gmers) |
| 2. Phase-acceptance template | [Section 8](#8-general-phase-acceptance-template) |
| 3. Reproducibility checklist | [Section 18.2](#182-reproducibility-checklist) |
| 4. Determinism review checklist | [Section 18.3](#183-determinism-review-checklist) |
| 5. Multi-seed statistical-analysis framework | [Section 7.1](#71-multi-seed-statistical-analysis-framework) |
| 6. Proposed event-data strategy | [Section 13.14](#1314-proposed-genesis-event-data-strategy) |
| 7. Scalable independent-world execution architecture | [Section 11](#11-parallel-world-scheduling) |
| 8. Save-format research recommendation | [Section 15](#15-save-format-and-migration-strategy) |
| 9. Framework for reporting null results | [Section 17](#17-null-result-reporting) |
| 10. List of claims Genesis could responsibly make at different evidence levels | [Section 4](#4-recommended-genesis-research-claims) |

---

# 21. Annotated bibliography

**Source policy:** Primary papers, journal standards, official documentation, and authoritative specifications are prioritized. Reviews are included where they synthesize contested definitions. Preprints and platform announcements are explicitly labeled. Access dates for mutable technical documentation should be recorded in an experiment's own manifest; this dossier reflects sources available through 2026-08-04.

## 21.1 Open-ended evolution: definitions, theory, and tests

### Taylor, T., Bedau, M. A., Channon, A., Ackley, D., Banzhaf, W., Beslon, G., et al. (2016). *Open-Ended Evolution: Perspectives from the OEE Workshop in York*. Artificial Life, 22(3), 408–423. https://doi.org/10.1162/ARTL_a_00210

A field-defining workshop synthesis that documents the plurality of OEE concepts, including adaptive novelty, new kinds of entities, major transitions, evolvability, and complexity growth. It supports Genesis's refusal to use a single litmus test.

### Banzhaf, W., Baumgaertner, B., Beslon, G., Doursat, R., Foster, J. A., McMullin, B., et al. (2016). *Defining and Simulating Open-Ended Novelty: Requirements, Guidelines, and Challenges*. Theory in Biosciences, 135, 131–161. https://doi.org/10.1007/s12064-016-0229-7

Develops requirements and conceptual distinctions for open-ended novelty in natural and artificial systems. Particularly relevant to the difference between a system that can generate variation and one that sustains meaningful innovation.

### Packard, N., Bedau, M. A., Channon, A., Ikegami, T., Rasmussen, S., Stanley, K. O., & Taylor, T. (2019). *Open-Ended Evolution and Open-Endedness: Editorial Introduction to the Open-Ended Evolution I Special Issue*. Artificial Life, 25(1), 1–3. https://doi.org/10.1162/artl_e_00282

Concise taxonomy of different kinds of open-endedness. Useful for claim scoping and for explaining why adaptive novelty, complexity growth, evolvability, and major transitions should not be collapsed into one binary label.

### Packard, N., Bedau, M. A., Channon, A., Ikegami, T., Rasmussen, S., Stanley, K. O., & Taylor, T. (2019). *An Overview of Open-Ended Evolution: Editorial Introduction to the Open-Ended Evolution II Special Issue*. Artificial Life, 25(2), 93–103. https://doi.org/10.1162/artl_a_00291

Surveys the two 2019 special issues and summarizes current OEE categories and evidence. This is an authoritative overview rather than proof that any one system meets all categories.

### Dolson, E. L., Vostinar, A. E., Wiser, M. J., & Ofria, C. (2019). *The MODES Toolbox: Measurements of Open-Ended Dynamics in Evolving Systems*. Artificial Life, 25(1), 50–73. https://doi.org/10.1162/artl_a_00280

Provides a practical suite of metrics for change, novelty, ecology, and complexity. It is valuable as a measurement toolbox, but its metrics remain representation- and threshold-dependent; Genesis should use them as a panel, not a verdict.

### Taylor, T. (2019). *Evolutionary Innovations and Where to Find Them: Routes to Open-Ended Evolution in Natural and Artificial Systems*. Artificial Life, 25(2), 207–224. https://doi.org/10.1162/artl_a_00290

Examines routes by which genuinely new entities and interactions can arise. Supports attention to state-space expansion, new levels of organization, and endogenous environmental feedback rather than continuous variation within a fixed repertoire.

### Channon, A. (2019). *Maximum Individual Complexity Is Indefinitely Scalable in Geb*. Artificial Life, 25(2), 134–144. https://doi.org/10.1162/artl_a_00285

Presents a concrete OEE-related result in Geb: maximum individual complexity scales with world size under the paper's measure. It is important positive evidence, while also illustrating that conclusions depend on the system and chosen complexity statistic.

### Pattee, H. H., & Sayama, H. (2019). *Evolved Open-Endedness, Not Open-Ended Evolution*. Artificial Life, 25(1), 4–8. https://doi.org/10.1162/artl_a_00276

Argues that open-endedness can itself be an evolutionary outcome rather than merely an externally supplied environmental property. This motivates measuring whether evolvability and representational capacity change, while avoiding the circular instruction to “make the environment open-ended.”

### Hintze, A. (2019). *Open-Endedness for the Sake of Open-Endedness*. Artificial Life, 25(2), 198–206. https://doi.org/10.1162/artl_a_00289

Challenges overly instrumental views of OEE and clarifies why ongoing creative dynamics can be a scientific target in their own right. It is conceptual, not an operational measurement standard.

### Stanley, K. O. (2019). *Why Open-Endedness Matters*. Artificial Life, 25(3), 232–235. https://doi.org/10.1162/artl_a_00294

Connects open-endedness to discovery and search beyond fixed objectives. Relevant to Genesis's “author physics and affordances, not progress” principle, with the caveat that objective-free search still requires observer-chosen representations.

### Adams, A. M., Zenil, H., Davies, P. C. W., & Walker, S. I. (2017). *Formal Definitions of Unbounded Evolution and Innovation Reveal Universal Mechanisms for Open-Ended Evolution in Dynamical Systems*. Scientific Reports, 7, 997. https://doi.org/10.1038/s41598-017-00810-8

Develops formal notions of unbounded evolution and innovation in dynamical systems. Useful for separating formal capacity from finite empirical evidence and for motivating explicit state-space and recurrence analysis.

### Hernández-Orozco, S., Hernández-Quiroz, F., & Zenil, H. (2018). *Undecidability and Irreducibility Conditions for Open-Ended Evolution and Emergence*. Artificial Life, 24(1), 56–70. https://doi.org/10.1162/ARTL_a_00254

Links OEE detection to computational irreducibility and undecidability. It supports caution that no general finite detector can certify all future open-ended behavior.

### Corominas-Murtra, B., Seoane, L. F., & Solé, R. V. (2018). *Zipf's Law, Unbounded Complexity and Open-Ended Evolution*. Journal of the Royal Society Interface, 15, 20180395. https://doi.org/10.1098/rsif.2018.0395

Proposes information-theoretic relationships between Zipf-like statistics and unbounded complexity. Genesis may use such statistics as exploratory signatures, not as self-sufficient OEE evidence.

### Stepney, S., & Hickinbotham, S. (2024). *On the Open-Endedness of Detecting Open-Endedness*. Artificial Life, 30(3), 390–416. https://doi.org/10.1162/artl_a_00399

Analyzes the difficulty and observer dependence of OEE detection. Directly supports representation audits, multiple detectors, and bounded wording.

### Channon, A. (2024). *A Procedure for Testing for Tokyo Type 1 Open-Ended Evolution*. Artificial Life, 30(3), 345–355. https://doi.org/10.1162/artl_a_00430

Combines several analyses into a procedure targeting ongoing adaptive novelty and complexity growth. It is one of the clearest recent attempts at an operational test, but still applies a specific OEE taxonomy and metric set.

### Packard, N., & McCaskill, J. S. (2024). *Open-Endedness in Genelife*. Artificial Life, 30(3), 356–389. https://doi.org/10.1162/artl_a_00426

Evaluates continuing genetic and spatial innovation in Genelife. Important contemporary evidence for partial open-ended dynamics; the paper does not establish the full functional and ecological innovation seen in biology.

### Channon, A., Bedau, M. A., Packard, N., & Taylor, T. (2024). *Editorial Introduction to the 2024 Special Issue on Open-Ended Evolution*. Artificial Life, 30(3), 300–304. https://doi.org/10.1162/artl_e_00445

Summarizes recent OEE methods and systems. Useful for locating the 2024 evidence frontier and the continuing absence of a universal test.

### Wong, M. L., Cleland, C. E., Arend, D., Bartlett, S., Cleaves, H. J., Demarest, H., et al. (2023). *On the Roles of Function and Selection in Evolving Systems*. Proceedings of the National Academy of Sciences, 120, e2310223120. https://doi.org/10.1073/pnas.2310223120

Develops a selection-related framework for functional information across evolving systems. Relevant to distinguishing adaptive/functional organization from random complexity, though operational choices remain necessary in Genesis.

### López-Díaz, A. J., & Gershenson, C. (2025). *Closing the Loop: How Semantic Closure Enables Open-Ended Evolution?* Journal of the Royal Society Interface, 22, 20250784. https://doi.org/10.1098/rsif.2025.0784

Proposes semantic closure and self-referential construction as a route to OEE. This is emerging theoretical work and should inform long-term architecture questions rather than near-term acceptance criteria.

### Tensen, M., Regan, C., Chan, B. W.-C., Oka, M., Stanley, K. O., & Szep, G. (2026). *Microcosmos: Reimagining Artificial Life for the GPU Era*. arXiv:2607.02954; accepted at ALIFE 2026. https://doi.org/10.48550/arXiv.2607.02954

**Preprint/platform paper.** Presents a GPU-era artificial-life platform direction and associated demonstrations. It is evidence about platform design and scalability, not evidence that simply using GPUs produces open-ended evolution.

## 21.2 Digital evolution, novelty search, and evolutionary systems

### Ray, T. S. (1991). *An Approach to the Synthesis of Life*. In C. G. Langton et al. (Eds.), Artificial Life II, Santa Fe Institute Studies in the Sciences of Complexity, Vol. X, 371–408.

Introduces Tierra and its self-replicating machine-code ecology. A foundational demonstration of digital organisms and ecological interactions, also historically important for understanding long-run stagnation.

### Ofria, C., & Wilke, C. O. (2004). *Avida: A Software Platform for Research in Computational Evolutionary Biology*. Artificial Life, 10(2), 191–229. https://doi.org/10.1162/106454604773563612

Describes Avida's architecture and experimental use. A key model for Genesis's emphasis on exact lineage, controlled treatments, and reproducible digital evolution.

### Lenski, R. E., Ofria, C., Pennock, R. T., & Adami, C. (2003). *The Evolutionary Origin of Complex Features*. Nature, 423, 139–144. https://doi.org/10.1038/nature01568

Shows a complex digital feature arising through a traceable mutational pathway. Strong evidence for cumulative genetic adaptation in a defined task ecology, not a general demonstration of OEE.

### Zaman, L., Meyer, J. R., Devangam, S., Bryson, D. M., Lenski, R. E., & Ofria, C. (2014). *Coevolution Drives the Emergence of Complex Traits and Promotes Evolvability*. PLOS Biology, 12(12), e1002023. https://doi.org/10.1371/journal.pbio.1002023

Demonstrates that antagonistic coevolution can promote complex traits and evolvability in digital organisms. Supports ecological feedback as a candidate mechanism, while not guaranteeing indefinite innovation.

### LaBar, T., & Adami, C. (2016). *Different Evolutionary Paths to Complexity for Small and Large Populations of Digital Organisms*. PLOS Computational Biology, 12(12), e1005066. https://doi.org/10.1371/journal.pcbi.1005066

Shows that population size can change evolutionary routes and complexity outcomes. Relevant to Genesis compute planning because “larger population” changes the scientific regime, not merely speed or sample count.

### Lehman, J., & Stanley, K. O. (2011). *Abandoning Objectives: Evolution Through the Search for Novelty Alone*. Evolutionary Computation, 19(2), 189–223. https://doi.org/10.1162/EVCO_a_00025

Establishes novelty search as an alternative to objective optimization in deceptive spaces. It also exemplifies the decisive role of the behavior descriptor, archive, and novelty threshold.

### Mouret, J.-B., & Clune, J. (2015). *Illuminating Search Spaces by Mapping Elites*. arXiv:1504.04909. https://doi.org/10.48550/arXiv.1504.04909

Introduces MAP-Elites, which maps high-performing solutions across observer-chosen feature dimensions. Useful for offline exploration and assay design, but not evidence that the underlying world is open-ended.

### Pugh, J. K., Soros, L. B., & Stanley, K. O. (2016). *Quality Diversity: A New Frontier for Evolutionary Computation*. Frontiers in Robotics and AI, 3, 40. https://doi.org/10.3389/frobt.2016.00040

Synthesizes quality-diversity methods. Relevant to diversity–performance tradeoffs and repertoire measurement, with externally defined behavior and quality dimensions.

### Wang, R., Lehman, J., Clune, J., & Stanley, K. O. (2019). *Paired Open-Ended Trailblazer (POET): Endlessly Generating Increasingly Complex and Diverse Learning Environments and Their Solutions*. arXiv:1901.01753. https://doi.org/10.48550/arXiv.1901.01753

Introduces POET's co-generation of environments and solutions. Demonstrates stepping-stone discovery but retains designed environment representations, minimal criteria, and transfer rules.

### Wang, R., Lehman, J., Rawal, A., Zhi, J., Li, Y., Clune, J., & Stanley, K. O. (2020). *Enhanced POET: Open-Ended Reinforcement Learning through Unbounded Invention of Learning Challenges and their Solutions*. Proceedings of ICML, PMLR 119, 9940–9951. https://proceedings.mlr.press/v119/wang20e.html

Extends POET and demonstrates more diverse/challenging environments and solution transfer. Strong open-ended-search evidence within a designed domain; not a biological or cultural OEE result.

### Ratcliff, W. C., Fankhauser, J. D., Rogers, D. W., Greig, D., & Travisano, M. (2015). *Origins of Multicellular Evolvability in Snowflake Yeast*. Nature Communications, 6, 6102. https://doi.org/10.1038/ncomms7102

Empirical evolution study showing how multicellular organization can alter evolvability. Useful for strict major-transition criteria and the distinction between aggregation and a heritable higher-level unit.

### Michod, R. E., & Roze, D. (2001). *Cooperation and Conflict in the Evolution of Multicellularity*. Heredity, 86, 1–7. https://doi.org/10.1046/j.1365-2540.2001.00808.x

Develops conflict/cooperation considerations in transitions to multicellularity. Supports requiring conflict mediation and collective-level heredity rather than labeling any cluster a transition.

## 21.3 Social learning, culture, tools, and cumulative technology

### Franz, M., & Nunn, C. L. (2009). *Network-Based Diffusion Analysis: A New Method for Detecting Social Learning*. Proceedings of the Royal Society B, 276, 1829–1836. https://doi.org/10.1098/rspb.2008.1824

Introduces NBDA, which tests whether acquisition order/timing follows a social network. Directly relevant to Genesis exposure logs, while still requiring controls for environment and homophily.

### Hoppitt, W., Boogert, N. J., & Laland, K. N. (2010). *Detecting Social Transmission in Networks*. Journal of Theoretical Biology, 263(4), 544–555. https://doi.org/10.1016/j.jtbi.2010.01.004

Extends network-based social-transmission inference. Useful for modeling acquisition hazards and alternative asocial pathways.

### Hoppitt, W., & Franz, M. (2017). *The Conceptual Foundations of Network-Based Diffusion Analysis: Choosing Networks and Interpreting Results*. Philosophical Transactions of the Royal Society B, 372, 20160418. https://doi.org/10.1098/rstb.2016.0418

Clarifies what NBDA can and cannot infer and why network construction matters. Supports Genesis's requirement to preregister exposure mappings and retain environmental controls.

### Whiten, A., & Rutz, C. (2025). *The Growing Methodological Toolkit for Identifying and Studying Social Learning and Culture in Non-Human Animals*. Philosophical Transactions of the Royal Society B, 380, 20240140. https://doi.org/10.1098/rstb.2024.0140

Authoritative recent synthesis of multiple methods for identifying social learning and culture. Particularly useful for avoiding reliance on the geographic “method of exclusion” alone.

### Mesoudi, A., & Thornton, A. (2018). *What Is Cumulative Cultural Evolution?* Proceedings of the Royal Society B, 285, 20180712. https://doi.org/10.1098/rspb.2018.0712

Clarifies competing definitions of cumulative culture and emphasizes repeated improvement, retention, and lineage processes. Central to the worked cumulative-culture criterion.

### Tennie, C., Call, J., & Tomasello, M. (2009). *Ratcheting Up the Ratchet: On the Evolution of Cumulative Culture*. Philosophical Transactions of the Royal Society B, 364, 2405–2415. https://doi.org/10.1098/rstb.2009.0052

Develops the ratchet concept and the distinction between socially retained innovations and behaviors likely to be independently reinvented. Supports naive-reconstruction and predecessor-dependence tests.

### Caldwell, C. A., & Millen, A. E. (2008). *Experimental Models for Testing Hypotheses about Cumulative Cultural Evolution*. Evolution and Human Behavior, 29(3), 165–171. https://doi.org/10.1016/j.evolhumbehav.2007.12.001

Presents laboratory transmission-chain methods for cumulative culture. Genesis can implement analogous controlled chains with more complete event and lineage records.

### Dean, L. G., Kendal, R. L., Schapiro, S. J., Thierry, B., & Laland, K. N. (2012). *Identification of the Social and Cognitive Processes Underlying Human Cumulative Culture*. Science, 335, 1114–1118. https://doi.org/10.1126/science.1213969

Experimental evidence connecting social/cognitive processes to cumulative performance. Useful as biological context, not as a requirement that Genesis reproduce human cognition.

### Acerbi, A., Tennie, C., & Mesoudi, A. (2016). *Social Learning Solves the Problem of Narrow-Peaked Search Landscapes: Experimental Evidence in Humans*. Royal Society Open Science, 3, 160215. https://doi.org/10.1098/rsos.160215

Shows how social learning can affect search and cumulative performance in a controlled task. Relevant to positive-control and bounded-task assays, while remaining externally structured.

### St Amant, R., & Horton, T. E. (2008). *Revisiting the Definition of Animal Tool Use*. Animal Behaviour, 75, 1199–1208. https://doi.org/10.1016/j.anbehav.2007.09.028

Provides a carefully reasoned tool-use definition based on controlled external objects and mediated effects. It supports Genesis's refusal to equate object contact with tool use.

### von Bayern, A. M. P., Danel, S., Auersperg, A. M. I., Mioduszewska, B., & Kacelnik, A. (2018). *Compound Tool Construction by New Caledonian Crows*. Scientific Reports, 8, 15676. https://doi.org/10.1038/s41598-018-33458-z

Empirical evidence for combining components into a functional tool. Useful for distinguishing physically joined compound tools from arbitrary multi-object sequences.

### Osuna-Mascaró, A. J., Mundry, R., Tebbich, S., Beck, S. R., & Auersperg, A. M. I. (2022). *Innovative Composite Tool Use by Goffin’s Cockatoos (Cacatua goffiniana)*. Scientific Reports, 12, 1510. https://doi.org/10.1038/s41598-022-05529-9

Demonstrates coordinated use of multiple objects and highlights terminology around composite tool systems. Relevant to Genesis's need for explicit component and causal-function records.

### Kendal, R. L., Boogert, N. J., Rendell, L., Laland, K. N., Webster, M., & Jones, P. L. (2018). *Social Learning Strategies: Bridge-Building between Fields*. Trends in Cognitive Sciences, 22(7), 651–665. https://doi.org/10.1016/j.tics.2018.04.003

Synthesizes when and whom organisms copy. Useful for secondary analyses of payoff bias, conformity, kin bias, and uncertainty, after basic social transmission is established.


## 21.4 Experimental design, causal inference, and simulation reporting

### Hurlbert, S. H. (1984). *Pseudoreplication and the Design of Ecological Field Experiments*. Ecological Monographs, 54(2), 187–211. https://doi.org/10.2307/1942661

The canonical discussion of pseudoreplication. Its central principle maps directly to Genesis: observations inside one treated world do not become independent replicates.

### Hudgens, M. G., & Halloran, M. E. (2008). *Toward Causal Inference with Interference*. Journal of the American Statistical Association, 103(482), 832–842. https://doi.org/10.1198/016214508000000292

Formalizes treatment effects when one unit's treatment can affect another's outcome. Relevant because social and ecological interaction deliberately violates no-interference assumptions.

### Lipsitch, M., Tchetgen Tchetgen, E., & Cohen, T. (2010). *Negative Controls: A Tool for Detecting Confounding and Bias in Observational Studies*. Epidemiology, 21(3), 383–388. https://doi.org/10.1097/EDE.0b013e3181d61eeb

Provides the modern negative-control framework. Genesis can construct unusually strong negative controls, such as temporally impossible exposure edges or appearance-matched nonfunctional objects.

### Grimm, V., Railsback, S. F., Vincenot, C. E., Berger, U., Gallagher, C., DeAngelis, D. L., et al. (2020). *The ODD Protocol for Describing Agent-Based and Other Simulation Models: A Second Update to Improve Clarity, Replication, and Structural Realism*. Journal of Artificial Societies and Social Simulation, 23(2), 7. https://doi.org/10.18564/jasss.4259

The authoritative current ODD model-description protocol. GMERS should supplement rather than replace ODD's model-level transparency.

### Monks, T., Currie, C. S. M., Onggo, B. S. S., Robinson, S., Kunc, M., & Taylor, S. J. E. (2019). *Strengthening the Reporting of Empirical Simulation Studies: Introducing the STRESS Guidelines*. Journal of Simulation, 13(1), 55–67. https://doi.org/10.1080/17477778.2018.1442155

Provides structured reporting guidance for simulation studies. Relevant to execution, inputs, experimentation, implementation, and output reporting beyond the model description itself.

### Grimm, V., Augusiak, J., Focks, A., Frank, B. M., Gabsi, F., Johnston, A. S. A., et al. (2014). *Towards Better Modelling and Decision Support: Documenting Model Development, Testing, and Analysis Using TRACE*. Ecological Modelling, 280, 129–139. https://doi.org/10.1016/j.ecolmodel.2014.01.018

Introduces TRACE for documenting the full model-development and evaluation process. Supports a long-lived Genesis evidence trail across model versions and failed hypotheses.

### Williams, C., Yang, Y., Lagisz, M., Morrison, K., Ricolfi, L., Warton, D. I., & Nakagawa, S. (2024). *Transparent Reporting Items for Simulation Studies Evaluating Statistical Methods: Foundations for Reproducibility and Reliability*. Methods in Ecology and Evolution, 15, 1926–1939. https://doi.org/10.1111/2041-210X.14415

Recent reporting guidance emphasizing planning, coding, analysis, and Monte Carlo uncertainty. Although focused on statistical-method simulations, its transparency requirements transfer well to Genesis power studies and synthetic detector validation.

### Morris, T. P., White, I. R., & Crowther, M. J. (2019). *Using Simulation Studies to Evaluate Statistical Methods*. Statistics in Medicine, 38(11), 2074–2102. https://doi.org/10.1002/sim.8086

Presents the ADEMP framework—Aims, Data-generating mechanisms, Estimands, Methods, and Performance measures. Particularly useful for Genesis's simulation-based power and detector-operating-characteristic work.

### Sandve, G. K., Nekrutenko, A., Taylor, J., & Hovig, E. (2013). *Ten Simple Rules for Reproducible Computational Research*. PLOS Computational Biology, 9(10), e1003285. https://doi.org/10.1371/journal.pcbi.1003285

Compact, practical reproducibility rules: record every result's provenance, avoid manual data manipulation, archive exact programs, and retain intermediate data where needed. These principles underlie the artifact bundle.

### Wilson, G., Bryan, J., Cranston, K., Kitzes, J., Nederbragt, L., & Teal, T. K. (2017). *Good Enough Practices in Scientific Computing*. PLOS Computational Biology, 13(6), e1005510. https://doi.org/10.1371/journal.pcbi.1005510

Practical guidance for data, software, collaboration, project organization, and manuscripts. Useful for sustaining reproducibility without requiring heavyweight infrastructure for every exploratory task.

### Wilkinson, M. D., Dumontier, M., Aalbersberg, I. J., Appleton, G., Axton, M., Baak, A., et al. (2016). *The FAIR Guiding Principles for Scientific Data Management and Stewardship*. Scientific Data, 3, 160018. https://doi.org/10.1038/sdata.2016.18

Defines findable, accessible, interoperable, and reusable data principles. Genesis's manifests, data dictionaries, content hashes, and open analytical derivatives should implement these where feasible.

### Nosek, B. A., Ebersole, C. R., DeHaven, A. C., & Mellor, D. T. (2018). *The Preregistration Revolution*. Proceedings of the National Academy of Sciences, 115(11), 2600–2606. https://doi.org/10.1073/pnas.1708274114

Explains how preregistration separates prediction from post hoc explanation. Directly relevant to a project where replay and detector iteration make overfitting unusually easy.

### Lakens, D. (2017). *Equivalence Tests: A Practical Primer for t Tests, Correlations, and Meta-Analyses*. Social Psychological and Personality Science, 8(4), 355–362. https://doi.org/10.1177/1948550617697177

Accessible introduction to testing whether an effect is small enough to be practically equivalent. Supports informative Genesis null claims tied to a SESOI.

### Gelman, A., & Carlin, J. (2014). *Beyond Power Calculations: Assessing Type S (Sign) and Type M (Magnitude) Errors*. Perspectives on Psychological Science, 9(6), 641–651. https://doi.org/10.1177/1745691614551642

Shows that low-powered studies can produce wrong-sign or exaggerated estimates even when “significant.” Particularly relevant to rare, heavy-tailed emergent outcomes.

### Lan, K. K. G., & DeMets, D. L. (1983). *Discrete Sequential Boundaries for Clinical Trials*. Biometrika, 70(3), 659–663. https://doi.org/10.1093/biomet/70.3.659

Introduces alpha-spending for flexible interim analysis times. Genesis can adapt this logic to completed batches of independent worlds.

### O'Brien, P. C., & Fleming, T. R. (1979). *A Multiple Testing Procedure for Clinical Trials*. Biometrics, 35(3), 549–556. https://doi.org/10.2307/2530245

Classic conservative-early group-sequential design. Useful when stopping an expensive study early should require unusually strong evidence.

### Johari, R., Koomen, P., Pekelis, L., & Walsh, D. (2022). *Always Valid Inference: Continuous Monitoring of A/B Tests*. Operations Research, 70(3), 1806–1821. https://doi.org/10.1287/opre.2021.2135

Develops inference valid under continuous monitoring for suitable designs. Relevant to adaptive experiment infrastructure, though Genesis endpoints and clustering require careful adaptation.

### Holm, S. (1979). *A Simple Sequentially Rejective Multiple Test Procedure*. Scandinavian Journal of Statistics, 6(2), 65–70.

Provides strong family-wise error control with more power than simple Bonferroni correction. Recommended for small confirmatory endpoint families.

### Benjamini, Y., & Hochberg, Y. (1995). *Controlling the False Discovery Rate: A Practical and Powerful Approach to Multiple Testing*. Journal of the Royal Statistical Society B, 57(1), 289–300. https://doi.org/10.1111/j.2517-6161.1995.tb02031.x

Foundational FDR method. Appropriate for clearly labeled exploratory detector panels, not as a substitute for held-out confirmation after extensive feature engineering.

### Cameron, A. C., Gelbach, J. B., & Miller, D. L. (2008). *Bootstrap-Based Improvements for Inference with Clustered Errors*. Review of Economics and Statistics, 90(3), 414–427. https://doi.org/10.1162/rest.90.3.414

Develops cluster-aware bootstrap inference, including settings with limited clusters. Relevant when retaining organism/event detail while resampling at world or seed-block level.

### Deen, M., & de Rooij, M. (2020). *ClusterBootstrap: An R Package for the Analysis of Hierarchical Data Using Generalized Linear Models with the Cluster Bootstrap*. Behavior Research Methods, 52, 572–590. https://doi.org/10.3758/s13428-019-01252-y

Practical cluster-bootstrap methodology. The software itself need not be adopted, but the resampling level and logic are directly relevant.

### Firth, D. (1993). *Bias Reduction of Maximum Likelihood Estimates*. Biometrika, 80(1), 27–38. https://doi.org/10.1093/biomet/80.1.27

Provides penalized likelihood that reduces small-sample/separation bias. Useful for sparse world-level binary outcomes with carefully limited covariates.

### King, G., & Zeng, L. (2001). *Logistic Regression in Rare Events Data*. Political Analysis, 9(2), 137–163. https://doi.org/10.1093/oxfordjournals.pan.a004868

Discusses rare-event logistic bias and correction. Relevant to coalitionary conflict or cumulative-culture occurrence, although exact/hierarchical models may be preferable for very small numbers of worlds.

### Fine, J. P., & Gray, R. J. (1999). *A Proportional Hazards Model for the Subdistribution of a Competing Risk*. Journal of the American Statistical Association, 94(446), 496–509. https://doi.org/10.1080/01621459.1999.10474144

Foundational competing-risk method. Useful when extinction prevents later innovation/tradition events and the cumulative incidence itself is of interest.

## 21.5 Deterministic simulation, parallel execution, and checkpointing

### Salmon, J. K., Moraes, M. A., Dror, R. O., & Shaw, D. E. (2011). *Parallel Random Numbers: As Easy as 1, 2, 3*. Proceedings of SC11. https://doi.org/10.1145/2063384.2063405

Introduces counter-based RNG families including Philox and Threefry. The stateless nth-draw property is the technical foundation for Genesis's semantic draw-key recommendation.

### Demmel, J., & Nguyen, H. D. (2013). *Fast Reproducible Floating-Point Summation*. 21st IEEE Symposium on Computer Arithmetic, 163–172. https://doi.org/10.1109/ARITH.2013.9

Shows methods for reproducible summation despite parallel ordering. Relevant when floating-point reductions cannot be removed from authoritative state.

### ReproBLAS Project. *Reproducible Basic Linear Algebra Subprograms*. https://bebop.cs.berkeley.edu/reproblas/

Authoritative project implementing reproducible reductions and BLAS-like operations. Useful as evidence that reproducibility is achievable but may require specialized accumulation and performance tradeoffs.

### The Rust Project. *`std::collections::HashMap` Documentation*. https://doc.rust-lang.org/std/collections/struct.HashMap.html

Official documentation notes randomized seeding; iteration order is not a semantic contract. Supports banning direct `HashMap` traversal from state-transition logic.

### The Rust Project. *`std::collections::BTreeMap` Documentation*. https://doc.rust-lang.org/std/collections/struct.BTreeMap.html

Official ordered-map semantics. Useful when key order should be explicit, though canonical serialization still requires stable key types and schema rules.

### The Cargo Project. *Cargo Book: Cargo.lock and Reproducible Dependency Resolution*. https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html

Official guidance on the role of `Cargo.lock`. Necessary for exact dependency selection, but not sufficient for bit-reproducible builds or deterministic runtime behavior.

### Reproducible Builds Project. *What Is a Reproducible Build?* https://reproducible-builds.org/docs/definition/

Authoritative definition of build reproducibility. Genesis should verify this separately from simulation replay and analysis reproducibility.

### Fujimoto, R. M. (1990). *Parallel Discrete Event Simulation*. Communications of the ACM, 33(10), 30–53. https://doi.org/10.1145/84537.84545

Classic survey of conservative and optimistic PDES. Supports treating within-world distribution as a causality-management problem, not a routine thread-pool optimization.

### Jefferson, D. R. (1985). *Virtual Time*. ACM Transactions on Programming Languages and Systems, 7(3), 404–425. https://doi.org/10.1145/3916.3988

Introduces Time Warp optimistic simulation and rollback. Important background for why optimistic PDES would impose major state, rollback, and anti-event complexity on Genesis.

### Chandy, K. M., & Lamport, L. (1985). *Distributed Snapshots: Determining Global States of Distributed Systems*. ACM Transactions on Computer Systems, 3(1), 63–75. https://doi.org/10.1145/214451.214456

Foundational coherent-snapshot algorithm. Relevant if a single world is distributed across asynchronous processes; a synchronous committed boundary remains simpler where possible.

### Young, J. W. (1974). *A First Order Approximation to the Optimum Checkpoint Interval*. Communications of the ACM, 17(9), 530–531. https://doi.org/10.1145/361147.361115

Derives a classic approximation balancing checkpoint cost and lost work. Useful as an initial planning formula, not a fixed operational law.

### Daly, J. T. (2006). *A Higher Order Estimate of the Optimum Checkpoint Interval for Restart Dumps*. Future Generation Computer Systems, 22(3), 303–312. https://doi.org/10.1016/j.future.2004.11.016

Refines checkpoint-interval modeling. Genesis should still substitute measured failure, write, restore, and journal costs.

### Richmond, P., Chisholm, R., Heywood, P., Kabiri Chimeh, M., & Leach, M. (2023). *FLAME GPU 2: A Framework for Flexible and Performant Agent-Based Simulation on GPUs*. Software: Practice and Experience, 53(8), 1659–1680. https://doi.org/10.1002/spe.3207

Primary evidence that GPU agent-based simulation can be highly effective with suitable data and agent-function structure. It justifies profiling candidate kernels, not assuming whole-world acceleration.

### Burtscher, M., Nasre, R., & Pingali, K. (2012). *A Quantitative Study of Irregular Programs on GPUs*. IEEE International Symposium on Workload Characterization.

Empirically characterizes limitations of GPUs on irregular workloads, including divergence and memory behavior. Directly relevant to variable topology and dynamic object graphs.

### NVIDIA. *cuDNN Backend Documentation: Reproducibility (Determinism)*. https://docs.nvidia.com/deeplearning/cudnn/backend/latest/developer/misc.html

Official documentation states that some routines are reproducible only under specified conditions and that atomic-based routines can be nondeterministic. Supports per-kernel, per-architecture verification.

### NVIDIA. *Floating Point and IEEE 754 Compliance for NVIDIA GPUs*. https://docs.nvidia.com/cuda/floating-point/

Authoritative explanation of floating-point behavior, FMA, and CPU/GPU differences. Relevant to defining Genesis's numeric execution classes.

## 21.6 Persistence, data formats, compression, and artifact evaluation

### Apache Parquet Project. *Parquet Format and Documentation*. https://parquet.apache.org/docs/

Authoritative specification for columnar analytical storage. Appropriate for derived event tables, not for canonical live-state restore without a separate Genesis schema.

### Apache Arrow Project. *Arrow Columnar Format and IPC Documentation*. https://arrow.apache.org/docs/format/

Defines cross-language columnar memory and interchange formats. Useful for analytical pipelines and data exchange; version and endianness policies still require recording.

### Protocol Buffers. *Language Guide and Schema Best Practices*. https://protobuf.dev/programming-guides/dos-donts/

Official schema-evolution guidance, including not reusing deleted field numbers. The principles transfer to Genesis even if another binary encoding is chosen.

### Collet, Y., & Kucherawy, M. (Eds.). (2021). *Zstandard Compression and the application/zstd Media Type*. RFC 8878. https://doi.org/10.17487/RFC8878

Stable specification of Zstandard framing and decoding. Suitable for independently compressed event/snapshot chunks under a pinned policy.

### O'Connor, J., Aumasson, J.-P., Neves, S., & Wilcox-O'Hearn, Z. *BLAKE3 Specification and Reference Implementation*. https://github.com/BLAKE3-team/BLAKE3-specs

Primary technical specification for BLAKE3. Useful for fast content identity and Merkle-style integrity, with the algorithm/version recorded in manifests.

### Association for Computing Machinery. *Artifact Review and Badging*. https://www.acm.org/publications/policies/artifact-review-and-badging-current

Authoritative artifact-availability, functionality, and reproducibility vocabulary. GMERS can align external releases with these established badges and criteria.

## 21.7 Offline detection, sequence analysis, networks, and phylogenies

### Adams, R. P., & MacKay, D. J. C. (2007). *Bayesian Online Changepoint Detection*. arXiv:0710.3742. https://doi.org/10.48550/arXiv.0710.3742

Introduces a widely used Bayesian method for online change-point inference. Useful for candidate regime shifts, with hazard priors and input representations explicitly audited.

### Killick, R., Fearnhead, P., & Eckley, I. A. (2012). *Optimal Detection of Changepoints with a Linear Computational Cost*. Journal of the American Statistical Association, 107(500), 1590–1598. https://doi.org/10.1080/01621459.2012.737745

Introduces PELT, an efficient exact segmentation method under specified cost/penalty conditions. Appropriate for large Genesis summary series, but penalty sensitivity must be reported.

### Rabiner, L. R. (1989). *A Tutorial on Hidden Markov Models and Selected Applications in Speech Recognition*. Proceedings of the IEEE, 77(2), 257–286. https://doi.org/10.1109/5.18626

Classic HMM tutorial. Supports latent regime modeling while illustrating how state count and emission assumptions are authored rather than discovered facts.

### Traag, V. A., Waltman, L., & van Eck, N. J. (2019). *From Louvain to Leiden: Guaranteeing Well-Connected Communities*. Scientific Reports, 9, 5233. https://doi.org/10.1038/s41598-019-41695-z

Introduces Leiden community detection with improved connectivity guarantees. Useful for offline contact/community candidates, subject to resolution and edge-definition sensitivity.

### Pei, J., Han, J., Mortazavi-Asl, B., Wang, J., Pinto, H., Chen, Q., Dayal, U., & Hsu, M.-C. (2004). *Mining Sequential Patterns by Pattern-Growth: The PrefixSpan Approach*. IEEE Transactions on Knowledge and Data Engineering, 16(11), 1424–1440. https://doi.org/10.1109/TKDE.2004.77

Foundational frequent subsequence-mining algorithm. Relevant to discovering repeated object-action sequences, with strong multiple-testing and opportunity controls required.

### Felsenstein, J. (1985). *Phylogenies and the Comparative Method*. The American Naturalist, 125(1), 1–15. https://doi.org/10.1086/284325

Foundational phylogenetic comparative method. Genesis has exact genetic ancestry, but horizontal cultural transfer means cultural history often needs networks rather than trees.

### Moreno, M. A., Dolson, E., Rodriguez-Papa, S., & Ofria, C. (2023). *hstrat: A Python Package for Phylogenetic Inference on Distributed Digital Evolution Populations*. Journal of Open Source Software, 8(82), 4866. https://doi.org/10.21105/joss.04866

Provides tools and concepts for efficient lineage tracking in distributed digital evolution. Relevant to scaling ancestry reconstruction, though Genesis's exact stable-ID lineage record may support stronger direct provenance.

## 21.8 Recommended source use by evidence level

| Evidence task | Preferred sources |
|---|---|
| Define OEE claim | Taylor et al. (2016); Banzhaf et al. (2016); Packard et al. (2019); Stepney & Hickinbotham (2024). |
| Select OEE measures | Dolson et al. (2019); Channon (2024); Adams et al. (2017), with representation audits. |
| Compare current systems | Ofria & Wilke (2004); Lenski et al. (2003); Channon (2019); Packard & McCaskill (2024); Wang et al. (2019, 2020); Tensen et al. (2026, preprint). |
| Establish social transmission | Franz & Nunn (2009); Hoppitt et al. (2010, 2017); Whiten & Rutz (2025). |
| Establish cumulative culture | Tennie et al. (2009); Mesoudi & Thornton (2018); Caldwell & Millen (2008); Dean et al. (2012). |
| Define tool use | St Amant & Horton (2008); von Bayern et al. (2018); Osuna-Mascaró et al. (2022). |
| Avoid pseudoreplication/interference | Hurlbert (1984); Hudgens & Halloran (2008). |
| Report simulation research | ODD (Grimm et al., 2020); STRESS (Monks et al., 2019); TRACE (Grimm et al., 2014); GMERS in this dossier. |
| Design sequential/null inference | Lan & DeMets (1983); O'Brien & Fleming (1979); Johari et al. (2022); Lakens (2017); Gelman & Carlin (2014). |
| Build deterministic RNG/order | Salmon et al. (2011); Rust official docs; Demmel & Nguyen (2013); NVIDIA official numerical documentation. |
| Schedule/checkpoint at scale | Fujimoto (1990); Jefferson (1985); Chandy & Lamport (1985); Young (1974); Daly (2006). |
| Build analytical event pipeline | Parquet/Arrow official specifications; Zstandard RFC; BLAKE3 specification. |
| Detect offline regimes/networks/sequences | Adams & MacKay (2007); Killick et al. (2012); Traag et al. (2019); Pei et al. (2004); Franz & Nunn (2009). |

---

## 21.9 Closing research position

The Genesis Engine's strongest near-term contribution is not a promise of artificial civilization. It is a deterministic experimental platform capable of turning claims about learning, transmission, environmental inheritance, cooperation, conflict, and complexity into repeatable, falsifiable, multi-seed tests.

The responsible long-term OEE claim is conditional and bounded:

> Under a declared model, seed distribution, measurement panel, and compute horizon, Genesis may provide evidence that adaptive novelty, ecological organization, or cumulative cultural processes continued without detected saturation and depended on identified mechanisms.

That statement remains scientifically meaningful precisely because it does not promise that additional compute alone will produce open-ended evolution.
