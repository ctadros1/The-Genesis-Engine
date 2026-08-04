# Cumulative Culture and Open-Ended Technology in The Genesis Engine

**A source-backed research synthesis and dependency-ordered experimental program for a deterministic artificial-life simulation**

**Prepared:** 4 August 2026  
**Encoding:** UTF-8  
**Primary design rule:** **Author physics and affordances, not progress.**

---

## How to read this report

This report uses the following evidence labels.

- **[Peer-reviewed experiment]** Controlled empirical work with biological or human participants.
- **[Peer-reviewed model]** A formal, computational, agent-based, artificial-life, or robotics study that underwent peer review.
- **[Peer-reviewed review]** A review or synthesis in a peer-reviewed venue.
- **[Conference paper]** Peer-reviewed conference proceedings, where review standards may differ from journals.
- **[Book]** A canonical scholarly monograph or edited volume.
- **[Preprint]** Public manuscript not treated here as equivalent to peer-reviewed evidence.
- **[Demonstration]** A proof of concept whose evidential scope is narrower than a controlled study.
- **[Inference for Genesis]** A design conclusion derived from the literature rather than directly established by it.

Three claim levels are kept separate throughout:

1. **Physical possibility:** mechanisms that would allow cultural transmission or cumulative change to occur.
2. **Research objective:** outcomes that would be scientifically interesting to observe.
3. **Empirical prediction:** outcomes for which existing evidence gives a defensible expectation.

The literature supports several mechanisms that make traditions and bounded cumulative improvement possible. It does **not** justify predicting that a sufficiently rich world will spontaneously produce open-ended technological civilization, organized warfare, or an indefinite sequence of technological transitions.

---

# 1. Executive summary

## 1.1 Bottom line

The most defensible route to cultural evolution in The Genesis Engine is not a technology system. It is a causal ecology in which organisms can:

1. perceive some of what other organisms do and what changes in the world result;
2. learn within a lifetime;
3. retain bounded memories;
4. explore alternatives rather than only copy;
5. interact repeatedly in structured populations with overlapping generations;
6. leave persistent, manipulable traces in the environment; and
7. experience ecological consequences that make some socially acquired variants reproduce more successfully than others.

Those conditions are **plausibly sufficient for traditions** and may be sufficient for **bounded cumulative improvement**. They are not evidence that open-ended technological evolution will follow.

A stable tradition is a socially caused population-level recurrence that persists through time. It does not require imitation, language, intention reading, or teaching. Local enhancement, stimulus enhancement, emulation, response facilitation, observational conditioning, and persistent environmental traces can all support traditions under some conditions. The great-tit diffusion experiments are particularly strong evidence that arbitrary behavioral variants can spread through real social networks and survive population turnover without high-level cognition ([Aplin et al., 2015](https://doi.org/10.1038/nature13998)).

Cumulative culture is more demanding. A useful operational definition requires: **variation**, **social transmission**, **performance improvement**, and **repetition of that cycle so that improvements are retained and built upon** ([Mesoudi & Thornton, 2018](https://doi.org/10.1098/rspb.2018.0712)). A single behavior spreading, a group converging on a convention, a policy becoming more complex during optimization, or a structure being assembled by fixed local rules is not by itself cumulative culture.

The “cultural ratchet” is not one mental module. It is the population-level prevention of systematic slippage: useful modifications must persist long enough to be copied, tested, recombined, or improved. High-fidelity transmission helps, but there is no universal fidelity threshold. The frequently cited trait-loss threshold in Lewis and Laland's model is a result of one stylized model, not a parameter that should be copied into Genesis ([Lewis & Laland, 2012](https://doi.org/10.1098/rstb.2012.0119)). Required fidelity depends on trait complexity, redundancy, selection, innovation rate, population structure, environmental volatility, and how errors affect function.

Learners do not necessarily need to infer intentions. Human transmission-chain experiments show cumulative improvement through access to products, observation, or reinforcement without explicit teaching, language, or causal understanding in some tasks ([Caldwell & Millen, 2009](https://doi.org/10.1111/j.1467-9280.2009.02469.x); [Zwirner & Thornton, 2015](https://doi.org/10.1038/srep16781); [Derex et al., 2019](https://doi.org/10.1038/s41562-019-0567-9)). In more causally opaque and technically difficult tasks, teaching and language substantially improve transmission ([Morgan et al., 2015](https://doi.org/10.1038/ncomms7029)). The correct conclusion is task dependence, not that language is either universally required or irrelevant.

Recent animal studies materially weaken the claim that only humans can socially acquire a skill that individuals fail to invent alone. Bumblebees and chimpanzees acquired experimentally seeded multi-step skills after prolonged failure without demonstrations ([Bridges et al., 2024](https://doi.org/10.1038/s41586-024-07126-4); [van Leeuwen et al., 2024](https://doi.org/10.1038/s41562-024-01836-5)). These studies establish an important **capacity for copying-dependent know-how**. They do not show repeated intergenerational improvement, an expanding technological repertoire, or open-ended cumulative culture.

Among artificial systems, the strongest bounded cases are narrow:

- Artificial navigators with route memory, goal direction, social attraction, and generational turnover reproduced cumulative route improvement under controlled conditions ([Dalmaijer, 2024](https://doi.org/10.1371/journal.pbio.3002644)). The destination, movement model, and direct-route optimum were authored.
- Embodied agents in a classification task improved across selected demonstrator generations when social and individual learning were combined ([Acerbi & Nolfi, 2007](https://doi.org/10.1109/ALIFE.2007.367814)). The architecture, task decomposition, reward, sole-demonstrator selection, and privileged access to demonstrator outputs strongly scaffolded the result.
- Reinforcement-learning systems have shown few-shot imitation and finite-task accumulation across artificial “generations” ([Bhoopchand et al., 2023](https://doi.org/10.1038/s41467-023-42875-2); [Cook et al., 2024](https://doi.org/10.52202/079017-1907)). Their experts, curricula, tasks, generation boundaries, and selection channels were authored. They are useful mechanism studies, not evidence of open-ended technological culture.
- Robot imitation experiments produced evolving movement memes and behavioral traditions, but not demonstrated functional ratcheting ([Winfield & Erbas, 2011](https://doi.org/10.1007/s12293-011-0063-x)).
- Hide-and-seek agents used movable boxes and ramps in increasingly elaborate strategies, but shared-policy reinforcement learning and an authored competitive reward do not constitute cultural inheritance ([Baker et al., 2020](https://arxiv.org/abs/1909.07528)).
- Avida, POET, Tierra, PolyWorld, TERMES, EVOC, Axelrod's culture model, and naming games each illuminate genetic accumulation, open-ended search, construction, conventions, or group differentiation. None demonstrates the full target phenomenon under an unscripted embodied cultural ecology.

## 1.2 Recommended minimum viable cultural-transmission system

The initial Genesis implementation should contain the following and no direct cultural state transfer:

1. **Visible action and outcome perception.** An observer receives ordinary, local sensory information about a demonstrator's body motion, contact events, object motion, and resulting environmental state—not action labels, rewards, intentions, neural activations, or policy parameters.
2. **Bounded event memory.** Organisms retain a finite sequence or compressed trace of perceived state–action–outcome events. Memory capacity and decay are evolvable within strict deterministic bounds.
3. **Within-lifetime plasticity.** A learner can change action values, policy parameters, associations, or a small world model from its own experience and observed events.
4. **Independent exploration.** Social information biases search but does not replace it. This permits both retention and modification.
5. **Overlapping generations and natural demonstration opportunities.** No global “teacher” role or generation reset is required. Knowledgeable organisms must sometimes coexist with naïve ones.
6. **Persistent, generic artifacts.** Objects and structures survive their maker for a tunable period and can be perceived, moved, modified, damaged, and reused. Persistence must arise from material properties, not a “technology” flag.
7. **Spatially limited social networks.** Contact, attention, migration, and group topology emerge from location and behavior. Full mixing should not be the default.
8. **Offline cultural provenance.** The engine logs who observed whom, which artifacts were encountered, and which behavioral variants followed. These records are never exposed to organisms and never alter simulation state.

Signals with evolvable production and reception can be added after non-symbolic transmission is established. Functional teaching can be allowed to evolve through tolerance, attention-directing behavior, opportunity provisioning, or costly demonstration; it should not initially be inserted as a named cognitive faculty.

## 1.3 What social learning should initially copy

| Candidate transmission target | Initial recommendation | Scientific rationale | Main risk |
|---|---:|---|---|
| Visible actions/body trajectories | **Yes, through ordinary perception** | Directly tests action-form learning and correspondence under embodiment | A privileged “action ID” would script the channel |
| Visible outcomes/object-state transitions | **Yes** | Supports emulation and artifact-mediated learning; often sufficient for traditions and some cumulative tasks | If outcomes are supplied as abstract goals, the engine authors meaning |
| Action tendencies or abstract skills | **Not directly** | These may emerge through learner generalization | An authored skill ontology becomes a hidden recipe system |
| Demonstrator trajectories | **Only as raw sensory history** | Learners may memorize paths or sequences | Exact replay buffers can bypass perception and embodiment |
| Signals associated with behavior | **Later, with evolvable semantics** | Can improve attention and coordination without fixed language | A signal dictionary would script semantics |
| Internal neural activity | **No baseline transfer** | Not normally observable; bypasses the correspondence problem | Privileged state leakage can manufacture high fidelity |
| Learned synaptic changes or neural weights | **No** | Conflates cultural transmission with direct policy cloning | Eliminates the scientific distinction between observation, learning, and inheritance |
| Demonstrator reward or fitness value | **No** | Organisms should evaluate outcomes through their own drives and sensors | Exposes the experimenter's objective function |

Direct neural-weight copying can be retained as an **engineering upper-bound control**: it answers how much accumulation is possible when the transmission bottleneck is removed. It should not be treated as a scientifically desirable cultural mechanism.

## 1.4 Dependency-ordered experimental strategy

The research program should proceed only when each dependency has been demonstrated under preregistered quantitative criteria:

1. Instrument deterministic provenance and null models.
2. Demonstrate diffusion of an arbitrary behavior variant.
3. Demonstrate persistence through inventor death and complete cohort turnover.
4. Demonstrate payoff-sensitive competition between transmitted variants.
5. Demonstrate retention of one socially acquired improvement.
6. Demonstrate at least two causally dependent improvements across successive turnovers.
7. Demonstrate group divergence under controlled ecology and genetics.
8. Demonstrate learning from artifacts after all knowledgeable organisms are gone.
9. Demonstrate persistent structures functioning as external memory.
10. Only then run long-horizon, weakly specified ecological experiments for expanding behavioral and technological repertoires.

A failure at an early stage should block interpretation of later visual complexity as culture. A visually elaborate settlement generated by genetically fixed building rules, for example, is construction but not evidence of a learned tradition.

## 1.5 Highest-confidence design conclusions

- **Necessary for cultural inheritance:** some socially mediated causal pathway from prior behavior or products of behavior to later learning.
- **Necessary for cumulative culture:** retention plus variation plus repeated performance improvement.
- **Strongly helpful:** bounded memory, overlapping generations, repeated demonstrations, persistent artifacts, and a balance of social learning with independent exploration.
- **Context dependent:** imitation, teaching, language, large population size, and high connectivity.
- **Speculative but important:** artifact systems that open new affordance classes, evolvable signaling semantics, division of labor, and institutions that stabilize knowledge.
- **Unsupported:** a claim that generic physics plus neural agents will predictably yield open-ended technology, civilization stages, or organized war.

---

# 2. Scope and definitions

## 2.1 The target phenomenon

The target is not “agents becoming more competent.” It is a historically dependent population process in which learned behavioral or material variants are transmitted, retained, modified, and sometimes recombined so that later organisms can exploit achievements that would otherwise disappear with their inventors.

This report focuses on five nested phenomena:

1. **Social influence:** another organism changes where, when, or how an individual acts.
2. **Social learning:** social exposure causes a lasting change in behavior or learned state.
3. **Tradition:** a behavior or product is socially transmitted and persists in a group over time.
4. **Cumulative culture:** socially transmitted modifications repeatedly produce retained performance improvement.
5. **Open-ended technological culture:** cultural lineages continue to generate new dependencies, functions, combinations, niches, or affordance classes rather than converging on one finite optimum.

Each level requires evidence not supplied by the lower level. Communication can occur without learning. Learning can occur without a tradition. A tradition can persist without improving. Improvement can accumulate toward a single authored optimum without becoming open-ended.

## 2.2 Operational definitions

| Term | Operational meaning for Genesis | What it does **not** establish |
|---|---|---|
| **Social learning** | A durable change in an organism's learned behavior caused by observing or interacting with another organism or a product of its behavior | That the copied variant is adaptive, group specific, persistent, or cumulative |
| **Observational learning** | Learning from witnessed actions, events, outcomes, or contingencies without necessarily reproducing the observed motor pattern | Imitation specifically |
| **Imitation** | Increased reproduction of the demonstrator's action form or action sequence, conditional on observing that form | Understanding the demonstrator's goal or causal model |
| **Emulation** | Learning about an outcome, object relation, or environmental transformation and reaching it through the learner's own action sequence | Matching the demonstrator's movements |
| **Local enhancement** | A social stimulus increases attention or exploration at a location | Learning an action rule |
| **Stimulus enhancement** | A social stimulus increases attention or exploration toward an object or feature | Copying a motor sequence |
| **Response facilitation** | Seeing an action raises the probability of an already available action | Acquisition of a genuinely novel action |
| **Observational conditioning** | The learner acquires a value or association by witnessing another's interaction or outcome | High-fidelity procedural copying |
| **Teaching** | A knowledgeable individual changes behavior in a learner's presence, incurs a cost or no immediate benefit, and makes learning faster or more likely | Necessarily having a theory of mind or explicit intent representation |
| **Signaling** | A produced cue has evolved or been learned because of its effect on receivers; receiver response may be immediate or learned | Cultural transmission unless the signal causes lasting learning |
| **Copying** | A generic description that must be completed by naming what is copied: location, stimulus, action form, outcome, sequence, signal, artifact feature, or hidden state | A mechanistically precise claim by itself |
| **Tradition** | A group-level variant whose recurrence and persistence are causally dependent on social transmission | Functional improvement |
| **Cultural lineage** | A chain or graph of causal transmission events connecting an innovation to later users, variants, or artifacts | Genetic descent |
| **Cultural ratchet** | Population processes that preserve useful modifications sufficiently well for later modification, recombination, or reuse | A single “ratchet gene” or cognitive module |
| **Cumulative culture** | Repeated cycles of variation, social transmission, retained performance improvement, and further modification | Mere novelty, complexity, convergence, or spread |
| **Stigmergy** | Coordination mediated by persistent changes to the environment, such as deposits, trails, partial structures, or object arrangements ([Theraulaz & Bonabeau, 1999](https://doi.org/10.1162/106454699568700)) | Learning or culture unless organisms acquire behavior from those traces |
| **External memory** | Persistent environmental state that allows later organisms to recover useful information, constraints, or opportunities without the original producer being present | Symbolic representation or intentional record keeping |
| **Open-endedness** | Continued production of adaptive novelty, new dependencies, niches, or kinds of organization without a known fixed endpoint or rapid saturation | A long but finite curriculum or expanding scalar score |

These definitions follow the broad social-learning framework developed in comparative psychology and cultural evolution, especially [Heyes (1994)](https://doi.org/10.1111/j.1469-185X.1994.tb01506.x), [Caro and Hauser (1992)](https://doi.org/10.1086/417553), [Hoppitt and Laland (2013)](https://press.princeton.edu/books/hardcover/9780691150710/social-learning), and [Mesoudi and Thornton (2018)](https://doi.org/10.1098/rspb.2018.0712).

## 2.3 Core and extended criteria for cumulative culture

Mesoudi and Thornton's four core criteria provide the best current operational baseline:

1. a behavior or product changes;
2. the new or modified variant is transmitted through social learning;
3. the transmitted change improves a performance measure relevant to organisms; and
4. the process repeats, yielding sequential improvement over time.

They distinguish these core criteria from extended features common in human cultural evolution: diversification into multiple lineages, recombination of lineages, exaptation to new uses, and niche construction that changes later selection and learning environments ([Mesoudi & Thornton, 2018](https://doi.org/10.1098/rspb.2018.0712)).

For Genesis, the distinction should be explicit:

- A route becoming shorter over five replacement events can meet the **core** criteria.
- A family of tools diversifying, being recombined, and opening previously unavailable resource transformations approaches the **extended** target.
- A settlement becoming visually larger is neither unless its construction depends on socially transmitted improvements.

## 2.4 Traditions versus cumulative culture

A socially learned arbitrary preference can form a tradition even when neither variant is better. Wild great tits experimentally seeded with alternative feeder-opening variants developed population-specific traditions and conformity across turnover ([Aplin et al., 2015](https://doi.org/10.1038/nature13998)). Axelrod's artificial agents converge locally and polarize globally through homophily and trait copying ([Axelrod, 1997](https://doi.org/10.1177/0022002797041002001)). Naming games create shared lexicons through repeated interaction ([Steels & McIntyre, 1998](https://doi.org/10.1142/S021952599800020X)). These are relevant models of tradition or convention, but not cumulative technological improvement.

Cumulative culture requires an additional causal claim: later performance must depend on retained changes introduced earlier. That claim should be demonstrated by intervention, not inferred from chronological correlation.

## 2.5 Type I and Type II accumulation

A useful distinction is between:

- **Type I accumulation:** optimization within a known phenomenon or affordance class—for example, progressively shortening a route, sharpening an edge, or improving a known extraction method.
- **Type II accumulation:** exploiting a previously unused natural phenomenon or combining mechanisms so that a new class of function becomes available—for example, moving from striking to leverage, binding, containment, insulation, controlled combustion, or indirect energy storage.

Derex argues that much human technological expansion depends on repeated exploitation of new natural phenomena, not only refinement within existing ones ([Derex, 2022](https://doi.org/10.1098/rstb.2020.0311)). Most laboratory and artificial-agent demonstrations are Type I. Genesis should measure the two separately. A system can show legitimate cumulative culture while remaining bounded to one affordance and one optimum.

## 2.6 Scope exclusions

The following are outside the permitted implementation strategy:

- technology trees, eras, ages, civilization stages, research points, or unlocks;
- fixed recipes exposed to organisms;
- an external classifier that labels a behavior “tool use” and rewards it;
- globally assigned group objectives such as “build shelter” or “develop agriculture”;
- direct injection of offline archaeological or cultural labels into organism controllers;
- an LLM as the organism controller;
- claims of culture based only on increasing neural-network size, policy entropy, object count, or visual novelty.

Offline analysis may name inferred traditions, lineages, artifact classes, or periods after the run. Those classifications must remain observational and versioned separately from the authoritative simulation state.

---

# 3. State of the evidence

## 3.1 Evidence hierarchy used here

The strength of a claim depends on whether the study establishes the relevant causal chain.

| Evidence tier | Typical design | What it can establish |
|---|---|---|
| **A: Causal cumulative evidence** | Multiple transmission/replacement events; asocial and social controls; measured improvement; intervention on transmission; replicated seeds/groups | Bounded cumulative culture for the tested trait and environment |
| **B: Causal transmission evidence** | Demonstrator seeding, diffusion analysis, ghost/end-state/no-model controls, or demonstrator removal | Social learning, copying mechanism, or stable tradition |
| **C: Comparative or historical inference** | Cross-population, network, genetic, archaeological, or longitudinal association | Plausibility of cultural history, with alternative explanations requiring explicit treatment |
| **D: Mechanism proof of concept** | Artificial model or robot demonstration with authored task and channel | Computational sufficiency of the included mechanism in that model |
| **E: Anecdote or visual emergence** | One run, no null, no ablation, retrospective interpretation | A phenomenon worth measuring, not a cultural claim |

## 3.2 Findings with strong support

### Social learning does not require imitation

Low-level attentional and associative mechanisms can create lasting behavioral changes. Stimulus and local enhancement can direct exploration; emulation can transmit outcomes; observational conditioning can transmit values; imitation can transmit action form. These mechanisms are neither interchangeable nor arranged as a simple ladder of intelligence ([Heyes, 1994](https://doi.org/10.1111/j.1469-185X.1994.tb01506.x); [Lind, Ghirlanda & Enquist, 2019](https://doi.org/10.1098/rsos.181777)).

### Traditions can persist with modest cognition

Experimental diffusion work in birds, primates, fish, and insects demonstrates socially caused, group-specific variants. Persistence depends on repeated exposure, network structure, conformity or frequency effects, and turnover—not necessarily mental-state attribution or symbolic communication. The causal evidence for traditions is substantially stronger than the evidence for cumulative technological culture.

### Cumulative improvement can occur without language or intention understanding on bounded tasks

Human microsociety experiments, pigeon route experiments, artificial navigator models, and some reinforcement-learning systems show that retained improvement can arise through combinations of observation, product access, memory, trial-and-error modification, and turnover. The mechanism required depends strongly on the task. Language and teaching become more important as action sequences become opaque, causally difficult, or hard to infer from products.

### Population structure matters, but “larger population equals more culture” is not a law

Effective social-learning population, access to diverse models, connectivity, migration, specialization, and turnover interact. Larger or more connected groups can retain rare complex skills and recombine innovations, but complete mixing can erase local diversity and reduce parallel exploration. Empirical and modeling critiques reject population size as a sufficient explanation in isolation ([Henrich, 2004](https://doi.org/10.2307/4128416); [Derex et al., 2013](https://doi.org/10.1038/nature12774); [Vaesen et al., 2016](https://doi.org/10.1073/pnas.1520288113); [Migliano et al., 2020](https://doi.org/10.1126/sciadv.aax5913)).

### Persistent environmental products can transmit information without the producer

Social learning can occur from products of behavior, not only from live demonstrations. Stigmergy shows how persistent traces coordinate collective behavior; archaeology and distributed cognition show how artifacts can constrain, cue, and preserve action sequences. Whether a trace counts as culture depends on whether later behavior is learned from it rather than genetically fixed.

## 3.3 Findings with moderate or task-limited support

### High transmission fidelity supports ratcheting

Formal models and experiments support the importance of fidelity, but “fidelity” is multidimensional. Action-form fidelity, outcome fidelity, causal fidelity, sequence fidelity, and functional fidelity can diverge. Redundant social models, reconstructive learning, error correction, and selection can compensate for low copying accuracy. Exact reproduction is not always desirable: excessive fidelity can suppress innovation or preserve maladaptive detail.

### Teaching and explicit communication can widen the set of transmissible skills

Teaching and language strongly improve difficult stone-tool transmission and many human tasks, but other tasks accumulate without them. The evidence supports treating teaching as an amplifier and error-correction mechanism, not as a universal prerequisite.

### Nonhuman animals can cross an “individual innovation” barrier through social learning

The 2024 bumblebee and chimpanzee studies show acquisition of complex sequential skills after prolonged no-demonstration failure. This narrows an earlier proposed human uniqueness boundary. Because the demonstrations were experimentally trained and improvements were not iterated through multiple learner generations, the studies do not establish an open-ended ratchet.

### Some nonhuman cumulative culture exists in a narrow sense

Pigeon route chains meet the four core criteria for a single bounded trait ([Sasaki & Biro, 2017](https://doi.org/10.1038/ncomms15049)). Chimpanzee population connectivity correlates with the distribution of complex toolsets in a pattern consistent with cumulative transmission ([Gunasekaram et al., 2024](https://doi.org/10.1126/science.adk3381)). The latter is a powerful historical inference but not a direct observation of sequential invention and transmission.

## 3.4 Findings that remain weak, disputed, or open

- No artificial-life system has robustly demonstrated indefinite, unscripted technological expansion comparable to human cumulative technology.
- No known fidelity number can be installed as a general “culture threshold.”
- It is unsettled which combination of cognition, social structure, ecology, and material affordances explains the open-ended scope of human culture.
- Archaeological estimates of when hominins became dependent on cumulative culture are model-based inferences. Procedural-unit analysis points to a change around 600,000 years ago, but the threshold depends on coding choices and the chosen non-cumulative baseline ([Paige & Perreault, 2024](https://doi.org/10.1073/pnas.2319175121)).
- Increasing task score across generations in an artificial system does not establish cultural evolution unless transmission, modification, and dependency are causally isolated.
- Open-ended evolution itself remains an unsolved systems problem: diversity collapse, complexity saturation, and failure to create new levels of individuality recur across artificial-life research ([Taylor et al., 2016](https://doi.org/10.1162/ARTL_a_00210); [Packard et al., 2019](https://doi.org/10.1162/artl_a_00291); [Guttenberg et al., 2019](https://doi.org/10.1162/artl_a_00286)).

## 3.5 Recent changes to the evidence base

Research published in 2024–2025 changes older summaries in three important ways.

First, the ability to socially acquire a behavior that individuals fail to innovate is no longer defensibly described as demonstrated only in humans. Bumblebees and chimpanzees now provide controlled examples, although both involve experimenter-trained demonstrators and no repeated ratchet.

Second, chimpanzee tool-complexity evidence increasingly supports a historical role for inter-group connectivity. Gunasekaram and colleagues linked complex toolsets more strongly than simple behaviors to population-connectivity patterns. Because migration and culture are inferred from genetic and behavioral distributions, the result should be described as evidence **consistent with** incipient cumulative culture rather than direct observation of it.

Third, the uniqueness debate is shifting from “any cumulative change” toward **open-ended scope**. Morgan and Feldman argue that cumulative change and high-fidelity transmission are not uniquely human; the more defensible candidate distinction is the breadth and continuing expansion of culturally heritable variation ([Morgan & Feldman, 2025](https://doi.org/10.1038/s41562-024-02035-y)). This is a perspective and synthesis, not a settled empirical verdict, but it maps closely to the real Genesis research target.


---

# 4. Mechanisms of social learning

## 4.1 Social learning is an outcome class, not one algorithm

The comparative literature increasingly treats social learning as learning implemented by ordinary or specialized learning processes operating on socially structured input. A learner may acquire information because another organism changes the learner's attention, exposes a hidden affordance, moves an object, produces a signal, receives a consequence, leaves an artifact, or performs a reproducible action sequence. Different observable outcomes can be generated by different internal mechanisms, so mechanism claims require interventions rather than surface resemblance.

For Genesis, this means there should be no monolithic `social_learning()` operation. Social exposure should alter the learner through explicit perceptual and learning pathways whose contributions can be separately disabled.

## 4.2 Local enhancement

**Definition.** The presence or behavior of a demonstrator increases a learner's attention to, or interaction with, a location.

**Example causal sequence.** An experienced organism repeatedly visits a rock outcrop. A naïve organism approaches because conspecific activity is salient, then discovers that striking at that location exposes food.

**Sufficiency for traditions.** Yes. If repeated visits draw future learners to the same place and the behavior persists after turnover, a location-based tradition can form.

**Sufficiency for cumulative culture.** Not by itself. It can provide the social dependency needed for discovery, but retention of improved action or artifact variants requires additional memory and selection.

**Genesis implementation.** Conspecifics may be perceptually salient; attention or gaze allocation changes the sampling density of nearby spatial sectors. No “interesting location” label is provided.

**Critical control.** Replay demonstrator location while masking action and outcome. If learners still acquire the variant, local enhancement may explain transmission.

## 4.3 Stimulus enhancement

**Definition.** Social exposure raises attention to a particular object, object class, color, shape, material, or other stimulus.

**Example causal sequence.** Observing another organism manipulate a branch increases subsequent branch exploration, even if the observer does not copy the movement.

**Sufficiency for traditions.** Yes, especially for object-choice traditions.

**Sufficiency for cumulative culture.** It can maintain a material tradition and bias innovation toward an existing artifact lineage, but cannot alone preserve a multi-step technique.

**Genesis implementation.** Perceived conspecific contact temporarily or learnedly changes the attentional value of ordinary object features. Avoid special “tool salience” fields.

**Critical control.** Make demonstrator contact visible while hiding object transformation; compare with a condition showing transformation without the demonstrator.

## 4.4 Response facilitation

**Definition.** Observing an action increases the probability of performing a response already present in the observer's repertoire.

**Importance.** Response facilitation can look like imitation when the action is common or genetically predisposed. It is therefore insufficient evidence for transmission of a novel procedure.

**Genesis implementation.** An observed motor pattern can transiently activate homologous action tendencies, but the effect should be bounded and separated from durable learning.

**Critical control.** Test whether the response occurs in genetically or developmentally naïve organisms that had no prior opportunity to express it. Compare novel and common actions.

## 4.5 Observational conditioning and socially acquired value

**Definition.** The observer learns a predictive or value association by witnessing another organism interact with a stimulus or experience an outcome.

A learner need not receive the demonstrator's internal reward. It can observe visible consequences: food acquisition, injury, object breakage, escape, persistence, or social reaction. The learner then updates its own expectations according to its own drives and learning rule.

**Sufficiency for traditions.** Yes. Food preferences, danger avoidance, and habitat choices can persist socially.

**Sufficiency for cumulative culture.** Potentially, when value learning directs repeated exploration and selection among variants. It does not provide action details on its own.

**Genesis implementation.** Permit learning from `(perceived pre-state, demonstrator/object event, perceived post-state)` tuples. Do not expose demonstrator reward, fitness, or “success” labels.

**Critical control.** Hold demonstrator movement constant while swapping visible outcomes. If learners follow the outcome rather than the action, the pathway is outcome learning or emulation.

## 4.6 Emulation

**Definition.** The learner acquires information about an environmental result, object relation, or affordance and independently produces a means to recreate it.

Emulation can produce high functional fidelity with low motor fidelity. In artifact tasks it may be especially powerful because the transformed object reveals constraints and intermediate states. Product-focused transmission-chain experiments show that cumulative improvement can occur when learners inspect prior products without seeing their production, although the effect depends on how much causal information the product retains.

**Sufficiency for traditions.** Yes.

**Sufficiency for cumulative culture.** Yes for some bounded tasks. [Caldwell and Millen (2009)](https://doi.org/10.1111/j.1467-9280.2009.02469.x) found independent accumulation in paper-airplane microsocieties under conditions allowing imitation, emulation/product information, or teaching. [Derex et al. (2019)](https://doi.org/10.1038/s41562-019-0567-9) showed technological improvement without commensurate causal understanding. Neither result means emulation is sufficient for all technologies.

**Genesis implementation.** A learner observes raw object-state transitions and final configurations. It must generate its own action sequence using its own controller.

**Critical controls.** Ghost display, end-state-only display, and action-visible/outcome-masked conditions ([Hopper, 2010](https://doi.org/10.1111/j.1469-185X.2010.00120.x)).

## 4.7 Imitation

**Definition.** Observation increases reproduction of the demonstrator's action form or action sequence, beyond what can be explained by local/stimulus enhancement, outcome learning, or response facilitation.

Imitation is valuable when intermediate actions are causally opaque or when the artifact does not preserve enough information to reconstruct manufacture. It also creates a correspondence problem: the observer must map another body, viewpoint, morphology, and timing into its own motor space.

**Sufficiency for traditions.** Yes.

**Required for cumulative culture.** No universal requirement. Some cumulative tasks show accumulation without it; difficult stone-tool transmission benefits greatly from action teaching and language ([Morgan et al., 2015](https://doi.org/10.1038/ncomms7029)).

**Genesis implementation.** The observer sees body pose, relative motion, contacts, and forces within normal sensory limits. A learned forward or inverse mapping may cause similar action. The engine should not emit an abstract “demonstrator chose action 7” symbol.

**Critical controls.** Two-action designs, morphology mismatch, viewpoint rotation, ghost motion, and demonstrator trajectory time-shuffling.

## 4.8 Teaching and opportunity provisioning

Caro and Hauser's functional criteria define teaching without requiring a demonstrator to represent the learner's mental state: the demonstrator modifies behavior in the learner's presence, pays a cost or gains no immediate benefit, and enables faster or more effective learning ([Caro & Hauser, 1992](https://doi.org/10.1086/417553)). This is suitable for artificial life because it identifies teaching behaviorally.

Possible low-level forms include:

- increased tolerance near resources or tools;
- slowing or repeating actions in the learner's presence;
- leaving partially processed objects;
- transferring objects;
- disabling a hazard or reducing task difficulty;
- directing attention with an evolvable signal;
- provisioning learners with easier versions of a task.

**Sufficiency for traditions.** Teaching can greatly increase transmission, but is not necessary.

**Required for cumulative culture.** Task dependent. Human children in the three-stage puzzle-box study combined imitation, teaching, and prosociality in ways not seen in chimpanzees or capuchins, but the study does not prove that this package is universally necessary ([Dean et al., 2012](https://doi.org/10.1126/science.1213969)).

**Genesis implementation.** Do not create a `teach` action with a built-in knowledge payload. Permit ordinary actions whose costs and effects make them satisfy the functional criteria when measured offline.

**Critical control.** Remove learners from demonstrator perception while leaving demonstrations visible through recorded replay. A difference isolates contingent demonstrator adjustment.

## 4.9 Signaling and communication

A signal can guide immediate coordination without creating lasting learning. Conversely, repeated signals associated with actions or outcomes can become part of a transmission system. Evolutionary robotics shows that simple signaling channels can coevolve with receiver behavior, including cooperative and deceptive regimes ([Floreano et al., 2007](https://doi.org/10.1016/j.cub.2007.01.058)). Those experiments evolved communication genetically; they did not by themselves demonstrate cultural inheritance.

For Genesis, a signal system should have:

- generic production dimensions, such as intensity, duration, orientation, rhythm, or spectral channel;
- energetic or opportunity costs;
- local propagation and interference;
- receiver perception through normal sensors;
- no fixed semantic table;
- meaning grounded in predictive relationships and receiver learning;
- possible deception and signal exploitation.

Signals become culturally relevant only if acquired signal–behavior mappings persist through learning and transmission rather than being fully specified genetically.

## 4.10 Copying products and artifact-mediated learning

A learner may acquire behavior from a product even when no demonstrator is present. Relevant information can exist in:

- final geometry;
- wear patterns;
- residue;
- partial construction;
- object placement;
- paths and trails;
- resource distributions created by prior action;
- reusable tools;
- damage or repair history.

This is a critical route for knowledge to outlive individuals. It also reduces the requirement that generations overlap. A persistent artifact can act as a delayed demonstration.

Artifact-mediated learning is not automatically culture. Termite-like construction can be genetically fixed stigmergy. The cultural claim requires showing that naïve organisms change learned behavior because of the artifact and that the resulting variant is not explained by genotype or direct environmental affordance alone.

## 4.11 Must a learner understand intention?

No. High-fidelity behavior can be copied without explicit intention understanding, and learners can improve artifacts without a correct causal theory. [Derex et al. (2019)](https://doi.org/10.1038/s41562-019-0567-9) found cumulative improvement in a physical system while participants' causal explanations remained poor; transmitting causal theories did not necessarily accelerate improvement and could constrain exploration. [Kirby et al. (2008)](https://doi.org/10.1073/pnas.0707835105) showed artificial languages becoming more structured and learnable through iterated transmission, without designers intending the final structures.

However, intention and causal understanding can:

- improve selective copying;
- make actions interpretable across context changes;
- support efficient teaching;
- enable repair when surface copying fails;
- make recombination and transfer to new problems more reliable.

Therefore, Genesis should not require intention reading for initial culture, but should leave room for internal predictive models to evolve or develop if they are useful.

## 4.12 Can reinforcement and observation support cumulative culture without symbols?

Yes, for bounded domains. Reinforcement can select among observed and self-generated variants; observation can reduce the search space; memory can preserve a demonstrated sequence; turnover can introduce fresh exploration; artifacts can preserve intermediate results. Pigeon route chains and artificial navigators provide clear examples of cumulative optimization with no symbolic communication ([Sasaki & Biro, 2017](https://doi.org/10.1038/ncomms15049); [Dalmaijer, 2024](https://doi.org/10.1371/journal.pbio.3002644)).

The evidence is not sufficient to predict open-ended technological culture without symbolic communication. Symbols may become especially useful when knowledge is displaced from immediate context, compositional, counterfactual, normative, or too expensive to demonstrate directly. That should be treated as a later empirical question, not an initial hard-coded requirement.

## 4.13 Which mechanisms are sufficient for what?

| Mechanism | Can spread a learned behavior? | Can support a stable tradition? | Can support bounded cumulative improvement? | Likely role in open-ended technology |
|---|---:|---:|---:|---|
| Local enhancement | Yes | Yes | Indirectly | Directs discovery to places and social hubs |
| Stimulus enhancement | Yes | Yes | Indirectly | Stabilizes material lineages |
| Response facilitation | Sometimes | Yes | Weak alone | Speeds common actions; low novelty capacity |
| Observational conditioning | Yes | Yes | With exploration and retention | Transmits values and hazard/resource knowledge |
| Emulation/outcome learning | Yes | Yes | Yes on some tasks | Supports functional reconstruction and transfer |
| Imitation/action-form copying | Yes | Yes | Yes; task dependent | Preserves opaque procedures and sequences |
| Teaching/tolerance | Yes | Yes | Strong amplifier | Error correction, specialization, difficult skills |
| Signals with learned meaning | Yes | Yes | Potentially | Coordination, displaced attention, compositionality |
| Artifact-mediated learning | Yes | Yes | Yes | External memory, delayed transmission, path dependence |
| Direct weight/state copying | Yes by construction | Yes | Yes by construction | Scientifically confounded; not recommended baseline |

---

# 5. Requirements for cumulative culture

## 5.1 The minimal causal chain

A claim of cumulative culture requires evidence for the following complete chain:

1. **Variant generation:** a behavior or artifact differs from predecessors.
2. **Social dependency:** later acquisition is causally influenced by social observation or a product of prior behavior.
3. **Retention:** the variant survives beyond the originating episode or individual.
4. **Functional effect:** it changes a performance measure meaningful under organism-level ecology.
5. **Further modification:** a later organism changes the retained variant.
6. **Dependency:** the later improvement depends on the earlier one rather than being an independent rediscovery.
7. **Repetition:** this occurs over multiple transmission events.

A chronological increase in performance satisfies none of these causal claims by itself.

## 5.2 Smallest plausible mechanisms for seven target outcomes

### 5.2.1 A learned behavior spreads

**Minimum plausible set:**

- the behavior or its environmental consequences are perceptible;
- at least one learner has within-lifetime plasticity;
- social exposure changes action probability or exploration;
- learners encounter demonstrators or their products often enough;
- the behavior can be executed by learners.

A two-action diffusion design is the cleanest assay. Seed one arbitrary variant in matched populations and test whether the seeded variant becomes overrepresented relative to no-demonstrator and opposite-demonstrator controls.

### 5.2.2 A behavior survives the death of its inventor

**Additional requirements:**

- transmission occurs before death, or an artifact persists after death;
- at least one learner retains the variant;
- the variant is expressed often enough to become available to later learners;
- turnover does not erase all knowledgeable individuals simultaneously unless artifact memory is present.

The minimum test removes the inventor after one learning opportunity and observes at least one full cohort replacement.

### 5.2.3 Variants compete

**Additional requirements:**

- more than one transmissible variant can solve the same or related problem;
- organisms can choose between variants;
- payoffs, costs, risks, or social frequencies differ;
- copying is not so strong that the first variant always fixes regardless of performance;
- variants are identifiable offline without becoming engine-level categories.

Neutral variants should drift or form group traditions; payoff differences should predict selection only when learners can detect relevant outcomes.

### 5.2.4 An improvement is retained

**Additional requirements:**

- the improvement produces a detectable outcome or product difference;
- at least one social channel transmits enough information to reconstruct it;
- learning and memory do not overwrite it before expression;
- it is not genetically assimilated before cultural dependence can be measured;
- selection is strong enough to retain it but not so elitist that variation disappears.

### 5.2.5 Multiple improvements accumulate

**Additional requirements:**

- the trait space permits modification without forcing one fixed recipe;
- earlier modifications remain accessible as scaffolds;
- at least some later improvements are conditionally useful only after earlier ones;
- learners can combine retention with exploration;
- the population persists for enough cultural generations;
- metrics reconstruct dependency, not merely scalar improvement.

The most convincing assay removes an early component from a late artifact or behavior and shows loss of the later function.

### 5.2.6 Traditions diverge between groups

**Additional requirements:**

- multiple groups or spatial clusters have partially independent social networks;
- equivalent or locally contingent variants exist;
- within-group contact exceeds between-group contact;
- migration is neither zero nor overwhelming if recombination is of interest;
- ecology and genotype are controlled when claiming cultural causation.

Axelrod-type local convergence is easy to generate, but it demonstrates only convention unless traits have learned behavioral consequences.

### 5.2.7 Artifact-mediated learning and external memory

**Additional requirements:**

- environmental products persist longer than the action that made them;
- naïve organisms can perceive features causally related to use or manufacture;
- objects can be reused or modified;
- maker absence does not erase all relevant information;
- artifact reset reduces acquisition or performance.

The strongest test removes all knowledgeable organisms, introduces naïve organisms, and compares intact-artifact, scrambled-artifact, and reset-environment conditions.

## 5.3 The cultural ratchet

The ratchet succeeds when the expected cultural lifetime of useful information is long enough for additional modification and retransmission. It fails when loss, distortion, turnover, or environmental change erase improvements faster than new improvements are generated and stabilized.

A useful conceptual decomposition is:

- `q_f`: probability that functionally important information survives one transmission;
- `q_a`: action-form fidelity;
- `q_o`: outcome fidelity;
- `q_c`: causal/dependency fidelity;
- `s`: probability the acquired variant is expressed and available to others;
- `n`: number and diversity of potential models;
- `m`: memory survival through a learner's lifetime;
- `p`: persistence of environmental products;
- `u`: useful modification rate;
- `d`: deleterious modification rate;
- `v`: environmental validity of inherited information.

A crude cultural reproduction number for variant `i` can be defined offline as:

\[
R_{c,i} = \sum_j P(\text{learner }j\text{ adopts a descendant of }i \mid \text{exposure})
\]

A variant is likely to expand when `R_c > 1`, but cumulative culture additionally requires descendant modifications with positive functional effects and causal dependence. This is an analytical framing, not an engine rule.

## 5.4 How much fidelity is required?

There is no context-free answer. Lewis and Laland's model produced cumulative culture when trait-loss rate was at or below 0.5, with a marked change between 0.5 and 0.4 ([Lewis & Laland, 2012](https://doi.org/10.1098/rstb.2012.0119)). That threshold arose from their representation, rates of invention/modification/combination, and utility structure. It should not be converted into “Genesis needs 60% copying accuracy.”

Required fidelity falls when:

- multiple demonstrators provide redundancy;
- artifacts preserve key information;
- errors are mostly neutral;
- learners reconstruct function from outcomes;
- selection quickly rejects harmful variants;
- skills are modular;
- teaching or repeated demonstration corrects errors.

Required fidelity rises when:

- a long sequence contains fragile dependencies;
- intermediate steps are unrewarded or opaque;
- one error destroys function;
- knowledgeable models are rare;
- memory decays quickly;
- environments change enough to make feedback noisy;
- morphology makes action correspondence difficult.

Genesis should therefore sweep fidelity-producing mechanisms rather than set a target. Measure action, outcome, and functional fidelity separately.

## 5.5 Innovation–imitation balance

A pure copier population eventually depends on information generated elsewhere. Rogers' paradox and later social-learning-strategy models show that indiscriminate social learning can fail to improve average adaptation in changing environments because copiers may exploit outdated or low-quality information. Conditional strategies—copy when uncertain, copy successful or prestigious models when cues are reliable, explore after failure—perform better in many settings ([Rogers, 1988](https://www.jstor.org/stable/678986); [Rendell et al., 2010](https://doi.org/10.1126/science.1184719)).

For Genesis:

- social and individual learning should compete for attention, time, and energy;
- copying should not be free or automatically correct;
- model choice should be based on perceptible cues, not true fitness;
- exploration should remain possible after observing a model;
- environments should include enough recurrence for social information to be useful, but enough variation to prevent pure imitation from dominating trivially.

## 5.6 Memory requirements

The minimum useful memory is not a symbolic skill store. It is a bounded trace that allows the learner to connect temporally separated observations, actions, and consequences.

Possible levels, in dependency order:

1. short eligibility traces for observed state transitions;
2. episodic buffers of recent sensory events;
3. learned values for object features, locations, individuals, and signals;
4. sequence memory sufficient for multi-step behavior;
5. compact predictive models of object transformations;
6. meta-memory or confidence used to choose when to observe versus explore.

Each additional level increases both scientific capability and the risk of importing authored cognitive structure. Start with levels 1–3 and add sequence capacity only after simple diffusion is verified.

## 5.7 Demonstration opportunities and tolerance

Knowledge cannot spread if knowledgeable and naïve organisms rarely coexist, if learners cannot remain close enough to observe, or if resource competition makes demonstrators exclude observers. Opportunity is often as important as raw learning ability.

Relevant variables include:

- generation overlap;
- juvenile or naïve period length;
- spatial co-residence;
- resource aggregation;
- group size and density;
- observation range and field of view;
- demonstrator speed;
- tolerance or aggression;
- repeated task performance;
- whether unsuccessful attempts remain visible;
- object transfer and partial products.

Genesis should log missed opportunities: a failed culture may reflect no exposure rather than no learning capacity.

## 5.8 Social-network topology

Network structure creates a retention–innovation tradeoff.

- **Complete mixing** spreads successful variants quickly but homogenizes exploration and can amplify early accidents.
- **Isolated clusters** preserve diversity but can lose rare skills and cannot recombine discoveries.
- **Partially connected modular networks** permit local exploration with occasional diffusion and are repeatedly identified as favorable for cumulative dynamics in models and human experiments.
- **Turnover hubs** or highly central individuals can accelerate diffusion but create single points of cultural failure.

The relevant quantity is effective access to diverse models, not census population alone. Genesis should generate networks from spatial contact and behavior, then analyze topology offline.

## 5.9 Population turnover

Turnover can both destroy and generate culture. It removes knowledge carriers, but naïve entrants also explore differently and may improve inherited solutions. The pigeon route and artificial navigator studies found cumulative route improvement specifically under replacement, not in fixed pairs ([Sasaki & Biro, 2017](https://doi.org/10.1038/ncomms15049); [Dalmaijer, 2024](https://doi.org/10.1371/journal.pbio.3002644)).

Key dimensions are:

- rate of death and replacement;
- overlap between experienced and naïve organisms;
- whether replacements begin with genetic predispositions but no learned state;
- whether artifacts bridge non-overlapping generations;
- whether turnover is synchronized or continuous;
- age-biased model choice.

Synchronized generation resets are convenient experimentally but less ecologically neutral. Genesis should prefer continuous lifecycles and create cohort assays only as explicit experiments.

## 5.10 Group size, migration, and demography

The literature does not support a single optimal group size. Larger effective populations can:

- reduce stochastic loss;
- provide more innovators;
- offer more models;
- maintain specialization;
- support recombination.

They can also:

- increase competition and observation noise;
- make coordination harder;
- reduce repeated dyadic exposure;
- increase homogenization if connectivity is dense.

Migration likewise has a non-monotonic prediction: too little prevents exchange; too much erases group divergence and may spread maladaptation. A migration sweep is more informative than selecting one value from anthropological literature.

## 5.11 Environmental persistence and external memory

A persistent world changes the cultural problem from “copy before the inventor dies” to “recover information from traces.” This can lower social overlap requirements and preserve partial solutions. It also creates path dependence: old structures constrain what later organisms perceive and can make.

External memory can be minimal and non-symbolic:

- a cleared path lowers movement cost;
- stacked objects remain climbable;
- a sharpened object retains geometry;
- a container retains contents;
- a wall redirects movement or airflow;
- residue predicts prior food processing;
- a partial structure exposes an assembly order;
- wear indicates effective contact surfaces.

Persistence must have physical costs and decay. Infinite, indestructible artifacts create unbounded state growth and can turn early accidents into permanent authored-like constraints.

## 5.12 Selection pressures

Cumulative culture needs selection among variants, but selection need not be an explicit fitness function attached to a named behavior. It can arise from:

- energy gained or saved;
- injury avoided;
- time freed for reproduction;
- resource access;
- material durability;
- thermal or spatial protection;
- social acceptance or punishment;
- mating or alliance consequences;
- reduced uncertainty.

Selection that is too strong can erase exploratory diversity and make one early solution dominate. Selection that is too weak permits drift but does not retain improvements. A robust program varies payoff gradients and measures the resulting balance of diversity and adaptation.

## 5.13 Necessary, helpful, and speculative mechanisms

| Mechanism | Classification | Reason |
|---|---|---|
| Perception of social actions or products | **Necessary for social transmission** | Without a social causal channel, learned variants cannot be culturally inherited |
| Within-lifetime plasticity | **Necessary for learned culture** | Genetically fixed response is not cultural learning |
| Memory beyond the observation instant | **Necessary for delayed expression; helpful otherwise** | Required when action cannot be copied simultaneously |
| Innovation/exploration | **Necessary for accumulation** | Copying alone cannot generate modifications |
| Retention through turnover | **Necessary for intergenerational culture** | Otherwise behavior dies with carriers |
| Performance consequences | **Necessary for adaptive cumulative culture** | Distinguishes ratcheting from neutral drift |
| Persistent artifacts | **Helpful; potentially necessary for advanced technology** | Extend memory and expose material dependencies |
| Partial network structure | **Helpful** | Preserves diversity while enabling diffusion |
| Teaching/tolerance | **Helpful, task dependent** | Increases opportunity and fidelity for difficult skills |
| Evolvable signals | **Helpful; speculative for open-ended expansion** | Can coordinate attention and transmit displaced information |
| Explicit intention understanding | **Not necessary for initial accumulation; speculative amplifier** | Bounded CCE occurs without it |
| Symbolic language | **Not necessary for bounded CCE; likely helpful for human-like scope** | Evidence supports strong amplification on opaque tasks, not universal necessity |
| Direct neural state transfer | **Unnecessary and scientifically confounding** | Bypasses the phenomenon under investigation |


---

# 6. Evidence from existing simulations

## 6.1 Assessment standard

Artificial systems are often described using cultural language when they demonstrate only one component of culture. The following audit asks, for each system:

1. What changed through evolution or learning?
2. What task, representation, reward, transmission channel, or selection rule was authored?
3. Was there repeated social transmission with retained functional improvement?
4. Did the environment contain a known solution or fixed optimum?
5. Did behavior transfer beyond training conditions?
6. How many independent repetitions or seeds were reported?
7. Were controls and ablations sufficient to isolate transmission?
8. Was the phenomenon robust or primarily demonstrative?
9. What bounded further development?

“Cumulative” is used below only when the four core criteria are reasonably satisfied for the measured trait.

## 6.2 Comparative overview

| System | Main phenomenon | Cultural transmission? | Repeated retained improvement? | Open-ended technology? | Primary evidential value |
|---|---|---:|---:|---:|---|
| Dalmaijer artificial navigators | Route memory, social attraction, turnover | Yes, behaviorally | **Yes, bounded route efficiency** | No | Minimal components can generate a narrow ratchet |
| Acerbi–Nolfi embodied agents | Social plus individual learning across selected demonstrators | Yes, privileged | **Yes, bounded task score** | No | Innovation plus transmission can outperform either alone |
| Artificial Generational Intelligence | In-context and in-weight accumulation | Authored generational channel | **Yes on finite tasks** | No | Controlled RL mechanism for retaining cross-generation improvements |
| MEDAL-ADR / GoalCycle3D | Few-shot imitation and recall | Yes, from hard-coded expert | No multi-generation ratchet | No | Memory, demonstrator dropout, attention, and task diversity support social learning |
| Copybots | Embodied imitation of movement memes | Yes | Variation and inheritance, but functional ratchet not established | No | Physical embodiment can support noisy meme lineages |
| Chisausky et al. reconstructive-learning model | Evolved neural learners reconstruct social information | Yes, modelled | No retained multi-generation ratchet | No | Social input is filtered by genotype and learning history; environmental stability changes which learning mode evolves |
| EVOC | Invention and neighbor imitation of action patterns | Yes by model rule | Score can rise; “open-ended” result is representation-dependent | No defensible open-ended ecology | Studies exploration–imitation and network effects |
| Axelrod culture model | Local convergence and global polarization | Trait copying by rule | No adaptive improvement | No | Group-specific conventions and topology |
| Naming games / Talking Heads | Emergent shared lexicons | Yes by interaction/update rules | Convention refinement, not technological ratchet | No | Self-organized signaling conventions |
| Floreano communication robots | Genetically evolved cooperative/deceptive signaling | Primarily genetic, not cultural | No | No | Coevolution of sender and receiver; path dependence |
| Hide-and-seek autocurriculum | Tool use, construction-like strategies, counter-strategies | No individual cultural inheritance | No | No | Rich affordances plus competition create strategy cascades |
| POET | Coevolving environments and policies; stepping stones | Policy transfer, not social culture | Cumulative optimization across task ecology | Not demonstrated | Open-ended search architecture and transfer |
| Avida | Genetic evolution of complex logic functions | No cultural learning | Genetic cumulative adaptation | No | Historical contingency and stepping stones |
| TERMES | Decentralized stigmergic construction | No learning in baseline | No | No | Generic local interactions can assemble complex structures |
| Sugarscape | Cultural tags, trade, migration, conflict | Trait copying by rule | No technological ratchet | No | Macro-patterns from simple social rules |
| Tierra / PolyWorld | Digital genetic evolution and emergent behavior | No learned social inheritance | Genetic evolution only | Limited/saturating | Negative comparator for culture |

## 6.3 Dalmaijer: artificial navigators and cumulative route improvement

**Source:** [Dalmaijer, 2024, PLOS Biology](https://doi.org/10.1371/journal.pbio.3002644). **[Peer-reviewed model]**

**What was evolved or learned.** Agents accumulated and followed route memories while being influenced by goal direction, movement continuity, and social proximity. No neural controller or symbolic communication was required.

**What researchers designed.** The destination, landscape, movement equations, memory representation, social attraction mechanism, replacement schedule, and route-efficiency metric were specified. The optimal direct path was implicit in geometry.

**Was it genuinely cumulative?** Yes, under the four core criteria, for one bounded trait. In turnover pairs, new members introduced deviations while experienced members retained route structure; route efficiency improved over replacement events. Fixed pairs and solitary agents did not show the same pattern.

**Authored solution?** The exact path was not supplied, but the goal and single geometric optimum were authored. This is Type I optimization, not discovery of new functions.

**Generalization.** The paper swept 610 parameter combinations and compared simulated patterns with empirical pigeon data. The qualitative turnover effect was broad in parameter space, but the best-fitting model did not reproduce all empirical optima, limiting explanatory completeness.

**Repetitions.** The main simulation design used **50 repetitions per condition and parameter combination**, with **60 journeys** and partner turnover every **12 journeys**. The empirical pigeon study used **10 independent repetitions** of each condition.

**Controls and ablations.** Solo, stable-pair, and turnover-pair controls; component lesions for goal direction, social proximity, route memory, and continuity; precision analyses.

**Robustness.** Strong for the reported narrow effect. The lesion results make it one of the cleanest computational demonstrations relevant to Genesis.

**What prevented further development.** One destination, one performance dimension, finite route memory, no material environment, no artifact production, and a hard geometric asymptote.

**Genesis lesson.** Turnover plus memory plus social coupling can produce a ratchet without imitation, language, or sophisticated planning. This supports a minimal early assay but should not be generalized to technology.

**Code and data:** [model repository](https://github.com/esdalmaijer/artificial_navigators), [data archive](https://doi.org/10.5281/zenodo.6944185), and [code archive](https://doi.org/10.5281/zenodo.10997495).

## 6.4 Acerbi and Nolfi: embodied agents across cultural generations

**Source:** [Acerbi & Nolfi, 2007](https://doi.org/10.1109/ALIFE.2007.367814). **[Conference paper]**

**What was evolved or learned.** Embodied agents learned to approach two edible item types and avoid two poisonous item types. Individual learning modified behavior; social learners observed an expert's sensory context and outputs. Across cultural generations, a selected expert transmitted performance to later learners.

**What researchers designed.** The task categories, modular neural architecture, sensory encoding, arithmetic reward, social-learning access, learning schedule, and selection of the best individual as the sole next-generation demonstrator were authored. The social learner was effectively placed on the expert's “shoulder” and received privileged access to motor commands and categorization output, bypassing much of the correspondence problem.

**Was it genuinely cumulative?** The combination of social and individual learning produced later generations that integrated prior performance with new learning and improved over the initial expert. That satisfies a bounded cumulative-performance interpretation. It is not an ecologically emergent culture because the transmission and selection pipeline is imposed.

**Authored solution?** Strongly scaffolded. The correct approach/avoid mapping, module boundaries, and reward were explicit.

**Generalization.** Tested within the authored item-classification environment, not across new ecologies or morphologies.

**Repetitions.** The reported experiments used **10 independent replications**, **100,000 simulated-annealing cycles** for individual development, populations of **10**, and **10 cultural generations** in the cumulative treatment.

**Controls and ablations.** Individual-only, social-only, and combined social-plus-individual conditions. These isolate the value of combining retention and innovation, although they do not remove privileged neural/output access.

**Robustness.** Replicated within the task. The strong researcher-authored transmission channel limits inference to a proof of computational sufficiency.

**What prevented further development.** Finite classification task, sole elite demonstrator, no horizontal network, no persistent artifacts, no ecological niche construction, and no new action ontology.

**Genesis lesson.** Social learning should seed rather than replace individual learning. Genesis should reproduce the retention–innovation comparison while removing privileged state access and external elite selection.

## 6.5 Artificial Generational Intelligence

**Source:** [Cook et al., 2024, NeurIPS](https://doi.org/10.52202/079017-1907). **[Conference paper]**

**What was evolved or learned.** Reinforcement-learning agents accumulated knowledge through two artificial generational schemes: episodic generations preserving in-context hidden state and train-time generations preserving learned weights. Tasks included memory sequences, goal sequences, and a partially observable traveling-salesperson problem.

**What researchers designed.** Task distributions, generation boundaries, visibility schedule, policy architecture, oracle/noisy demonstrator, training objective, and selection of a prior generation's best behavior or policy state were authored. During training, privileged expert visibility was annealed. Evaluation explicitly uses the best previous-generation solution as the seed for the next generation.

**Was it genuinely cumulative?** Agents improved across artificial generations and outperformed single-lifetime baselines given comparable cumulative experience. This is legitimate finite-task accumulation through an engineered inheritance channel. It is not a population culture emerging from embodied social observation.

**Authored solution?** The tasks have finite authored objectives, and an oracle supplies solution-relevant behavior during training.

**Generalization.** The paper tests multiple task families and compares accumulation modes, but not transfer to an open-ended material ecology.

**Repetitions.** Primary results were averaged over **10 random seeds**.

**Controls and ablations.** Same-cumulative-experience single-lifetime baselines, in-context versus in-weights modes, and variations in generational setup.

**Robustness.** Convincing for its formal algorithmic claim. Cultural interpretation must remain narrow because the inheritance channel is designed and high-level.

**What prevented further development.** Finite task spaces, external generation construction, privileged demonstration, external best-of-generation selection, and no autonomous population lifecycle.

**Genesis lesson.** Use it as an upper-bound and experimental-design reference. Do not implement hidden-state or weight inheritance as the initial cultural pathway.

**Code:** [FLAIR cultural-accumulation repository](https://github.com/FLAIROx/cultural-accumulation).

## 6.6 MEDAL-ADR: few-shot imitation as cultural transmission

**Source:** [Bhoopchand et al., 2023, Nature Communications](https://doi.org/10.1038/s41467-023-42875-2). **[Peer-reviewed model]**

**What was learned.** A deep-RL agent learned to follow and remember an expert's ordered route through procedurally generated 3D goal environments, including held-out configurations and human demonstrators.

**What researchers designed.** The GoalCycle3D task, ordered goals, reward, hard-coded expert bot, expert dropout schedule, auxiliary attention loss predicting the expert's position, automatic domain randomization, and training architecture were authored.

**Was it genuinely cumulative?** No multi-generational improvement was tested. The work demonstrates robust few-shot social learning, within-episode recall, fidelity, and generalization. An agent sometimes improved on a non-optimal expert through reinforcement learning, but this improvement was not transmitted repeatedly through a cultural lineage.

**Authored solution?** Yes. Expert trajectories reveal the correct goal order; an auxiliary loss explicitly encourages attention to the expert.

**Generalization.** Strong within the procedurally generated task family, including out-of-distribution parameters and human trajectories.

**Repetitions.** Main ablations report means across **10 initialization/procedural-generation seeds**. Some generalization panels use **50** or **20** evaluation initializations, as specified by the authors.

**Controls and ablations.** Memory, expert presence, expert dropout, attention loss, automatic curriculum, and domain-randomization components were individually ablated. A two-action-style causal fidelity test was included.

**Robustness.** Strong for few-shot imitation. The “minimal starter kit” is minimal only relative to the paper's architecture and task, not a universal biological minimum.

**What prevented further development.** No reproduction, no population turnover, no learner-to-learner transmission chain, no artifact creation, and no endogenous objectives.

**Genesis lesson.** Memory, intermittent demonstrator availability, and attention are likely important. Genesis should test them through ordinary sensory affordances rather than an expert-prediction auxiliary loss.

## 6.7 Copybots and embodied memetic evolution

**Sources:** [Winfield & Erbas, 2011](https://doi.org/10.1007/s12293-011-0063-x); [Erbas, Winfield & Bull, 2014](https://doi.org/10.1177/1059712313500503). **[Peer-reviewed model / robot experiments]**

**What was learned.** Physical e-puck robots observed and imitated movement patterns. Sensor noise, robot heterogeneity, and imperfect imitation generated heritable variation across multiple imitation cycles. Later work used imitation to accelerate reinforcement learning while observing behavior rather than internal controller state.

**What researchers designed.** Demonstration protocol, movement representation, imitation metric, robot hardware, selection opportunities, and task environment were authored.

**Was it genuinely cumulative?** The experiments demonstrate variation, selection, and inheritance of movement memes and the possibility of behavioral traditions. They do not establish repeated functional improvement across generations.

**Authored solution?** Movement behaviors were constrained by the demonstration and imitation architecture; no open material problem was solved.

**Generalization.** Embodied transfer is a strength, but the behavioral space and sample size were limited.

**Repetitions.** The 2011 article reports experimental trials rather than a modern multi-seed simulation benchmark; a single comparable seed count is not stated in the accessible report. The results should be treated as initial proof of concept, not population-level robustness evidence.

**Controls and ablations.** Comparisons of imitation and learning conditions exist, but the central 2011 claim is demonstrative rather than a complete cumulative-culture ablation suite.

**What prevented further development.** Limited behavioral representation, short imitation chains, no ecological payoffs that favor continuing functional modification, and small physical-robot populations.

**Genesis lesson.** Imperfect embodied copying can itself generate cultural variation. Genesis should preserve the morphology and viewpoint correspondence problem rather than transferring action codes.

## 6.8 Chisausky and colleagues: reconstructive social learning in evolving neural networks

**Source:** [Chisausky et al., 2025](https://doi.org/10.1038/s41598-025-97492-4). **[Peer-reviewed evolutionary model]**

**What evolved or was learned.** Individuals inherited small neural networks and heritable learning parameters, while a delta-rule-like process modified part of each network during life. Depending on environmental stability, evolution favored inborn behavior, individual learning, socially guided learning, or mixtures of individual and social learning. Social input was reconstructed through each learner's inherited network and prior learning state rather than copied as a discrete cultural token.

**What researchers designed.** The cue-to-food-quality problem, neural architecture, feedback rule, environmental-change schedule, parental demonstrator channel, definitions of social guidance and social instruction, success bias, and reproductive mapping were authored.

**Was it genuinely cumulative?** No. The model studies the evolution of social-learning modes and gene–learning interactions, not repeated retained functional improvement along cultural lineages. There are no persistent artifacts or multi-step traditions whose improvements must survive turnover.

**Authored solution?** Strongly bounded. Food quality, cues, feedback, and the performance objective are explicit. Social instruction can expose a demonstrator's predicted target value, which is more privileged than ordinary embodied observation.

**Generalization.** The study varies environmental frequency and magnitude of change, learning schedules, learning speed, demonstrator bias, and social-learning type. This supports conclusions about regime dependence, not transfer to an open material ecology.

**Repetitions.** Principal parameter cells commonly used **60 replicate simulations**, each run for **20,000 generations**, with outcomes averaged over the final 2,000 generations. Replicates sometimes converged to qualitatively different evolutionary attractors.

**Controls and ablations.** No-learning, individual-learning, socially guided, socially instructed, mixed-learning, success-biased demonstrator selection, learning-order, and environmental-stability comparisons.

**Robustness.** Strong for the claim that learning mode depends on environmental regime and that stochastic evolutionary histories can reach alternative outcomes. It provides no evidence for cumulative culture itself.

**What prevented further development.** A single cue–quality task, parental generation channel, no organism-to-organism action observation, no persistent material state, no cultural lineage reconstruction, and no novel function space.

**Genesis lesson.** Treat social learning as learner-side reconstruction. The same visible action or outcome should update organisms differently because of genotype, morphology, attention, memory, and prior experience. Do not transfer learned weights. Also expect mixed individual–social learning to occupy a bounded ecological regime: extremely stable worlds favor assimilation or fixed behavior, while rapidly changing worlds devalue inherited social information.

## 6.9 EVOC and recursive chaining

**Sources:** [Gabora, 2013](https://arxiv.org/abs/1310.0522); [Gabora, Chia & Firouzi, 2013](https://arxiv.org/abs/1310.4086); [Gabora & Saberi, 2013](https://arxiv.org/abs/1310.3781). **[Preprints / agent-based demonstrations]**

**What was learned.** Agents invent discrete body-action patterns and imitate higher-scoring neighboring actions. Variants of EVOC add recursive chaining and contextual shifts between divergent and convergent invention.

**What researchers designed.** The body-action representation, invention operator, imitation rule, neighborhood, fitness function, and chaining score were authored. The reported absence of a score ceiling under chaining follows partly because longer chains can continue to receive additional fitness by construction.

**Was it genuinely cumulative?** Mean score can rise and action sequences can lengthen through invention plus imitation. This is a computational cultural-evolution model, but “open-ended novelty” should not be equated with open-ended technology: the action ontology and objective remain fixed, and functional dependencies are shallow.

**Authored solution?** Strongly. The fitness landscape defines cultural success directly.

**Generalization.** Parameter experiments examine population density, borders, leadership, creativity ratio, and environmental change; no embodied transfer or new affordance class is shown.

**Repetitions.** Some reported comparisons use **500 runs** for aggregate curves. Exact replication counts vary by publication and condition.

**Controls and ablations.** Chaining/no-chaining, contextual-focus/no-contextual-focus, leadership, border, density, and creator–imitator sweeps.

**Robustness.** Useful for qualitative theory. Claims about open-ended culture are not robust evidence for an unscripted physical ecology because the scoring rule is structurally extensible.

**What prevented further development.** Fixed action primitives, fixed fitness evaluator, direct neighbor imitation, no material world, no organism lifecycle, and no independent causal definition of progress.

**Genesis lesson.** Do not award complexity simply for longer action sequences. Require retained function, causal dependence, and performance under ecological controls.

## 6.10 Axelrod's dissemination-of-culture model

**Source:** [Axelrod, 1997](https://doi.org/10.1177/0022002797041002001). **[Peer-reviewed model]**

**What changed.** Agents copied discrete feature values from culturally similar neighbors. Local convergence produced homogeneous regions while global differences could remain.

**What researchers designed.** Every cultural feature, trait range, homophily rule, grid, and asynchronous update rule was authored.

**Was it cumulative?** No. There is no performance measure, innovation, or repeated functional improvement.

**Authored solution?** There is no task solution; the absorbing-state structure is created by the interaction rule.

**Generalization.** The paper explores territory size, number of features and traits, and interaction range. The result became a canonical model of polarization and cultural regions.

**Repetitions.** Repeated simulation averages are reported across parameter conditions, but the paper predates standardized seed reporting; no single universal seed count describes all analyses.

**Controls and ablations.** Parameter sweeps test the model's mechanisms, but there are no learning or ecological controls because the rule itself defines transmission.

**Robustness.** Strong as a formal result about local convergence and global polarization. Irrelevant as evidence of technological ratcheting.

**What prevented further development.** No learning process, no fitness consequences, no artifact environment, no invention beyond initial traits.

**Genesis lesson.** Partial connectivity can preserve group-specific traditions. Cultural divergence metrics must not be mislabeled cumulative progress.

## 6.11 Naming games and the Talking Heads experiments

**Sources:** [Steels & McIntyre, 1998](https://doi.org/10.1142/S021952599800020X); [Steels, 1999](https://mitpress.mit.edu/9780262692325/the-talking-heads-experiment/). **[Peer-reviewed model / book / robotic demonstration]**

**What was learned.** Agents negotiated object names through repeated speaker–hearer games, updating lexicons after success and failure. Shared vocabularies, regional variation, and language change emerged.

**What researchers designed.** Roles, communicative game, success criterion, perceptual categories or scene framing, lexical update rule, and interaction schedule.

**Was it cumulative?** Conventions emerge and can change; some language-game research studies compositional structure. The basic naming game is not repeated functional technological improvement.

**Authored solution?** Meanings are grounded in researcher-selected scenes and interaction games; labels are not preassigned, but the communicative problem is.

**Generalization.** Physical situated experiments provide stronger grounding than abstract symbol models. Scope remains bounded to the designed game.

**Repetitions.** Naming-game publications report repeated interactions and population simulations, but seed reporting varies and is not consistently presented in modern benchmark form. Quantitative reuse should consult each experiment's methods.

**Controls and ablations.** Spatial versus well-mixed contact, population contact changes, and update-rule variants.

**Robustness.** Strong evidence that conventions can self-organize from local interaction rules; not evidence that symbolic language will emerge without a designed communicative ecology.

**Genesis lesson.** Evolve signal production and learned receiver associations only after ecological reasons to coordinate exist. Do not insert speaker/hearer roles or a success oracle.

## 6.12 Floreano and colleagues: emergent communication in robots

**Source:** [Floreano et al., 2007](https://doi.org/10.1016/j.cub.2007.01.058). **[Peer-reviewed model with physical transfer]**

**What evolved.** Neural controllers evolved production and reception of blue-light signals in a foraging arena containing food and poison. Relatedness and colony-level selection favored cooperative signals; unrelated individual selection could favor deception.

**What researchers designed.** Arena, food/poison sources, visual channel, robot sensors, neural architecture, evolutionary algorithm, selection level, and fitness consequences.

**Was it cultural?** Primarily no. Signal systems were genetically evolved across controller generations rather than learned socially within lifetimes and culturally retransmitted.

**Authored solution?** Signal meaning was not preassigned, but the channel and ecological problem were authored.

**Generalization.** Evolved controllers were transferred from realistic simulation to physical robots, strengthening embodiment validity.

**Repetitions.** The paper reports repeated evolutionary trials across relatedness and selection regimes. A single exact seed count is not stated in the accessible article summary; replication is sufficient for the qualitative evolutionary claim but should be rechecked from the full methods before numerical comparison.

**Controls and ablations.** Kin versus unrelated colonies, colony versus individual selection, signal-capable conditions, and comparisons of evolved communication systems.

**Robustness.** Multiple signaling equilibria arose, and established systems constrained later movement to more efficient systems. This path dependence is relevant to cultural lock-in even though inheritance was genetic.

**What prevented further development.** One-bit-like signal channel, fixed foraging ecology, genetic rather than learned semantics, and finite task.

**Genesis lesson.** Evolvable signals may produce cooperation, exploitation, or deception. Their cultural status must be tested separately from their genetic emergence.

## 6.13 Hide-and-seek multi-agent autocurricula

**Source:** [Baker et al., 2020](https://arxiv.org/abs/1909.07528), ICLR 2020. **[Conference paper]**

**What was learned.** Teams learned competitive strategies including blocking doors, building box shelters, using ramps, and countering opponents' tool use. Six qualitative strategy phases were identified.

**What researchers designed.** Hide-and-seek reward, team identities, shared team policies, movable boxes and ramps, object locking, world randomization, centralized training, action capabilities, and episode resets.

**Was it cumulative culture?** No. The sequence is an autocurriculum in policy optimization. Agents do not form individual cultural lineages, observe and copy predecessors, reproduce, or transmit learned variants through turnover.

**Authored solution?** No exact tactic was specified, but all relevant tools and competitive objective were authored. Several phases exploit simulator mechanics, illustrating specification gaming as well as adaptive tool use.

**Generalization.** Transfer and fine-tuning tests compared hide-and-seek pretraining with intrinsic-motivation and random baselines on downstream skills.

**Repetitions.** The paper reports large distributed training runs and benchmark comparisons, but a single standardized seed count for the six-phase narrative is not the central unit of analysis. Phase timing varies and some behaviors are run-dependent.

**Controls and ablations.** Intrinsic motivation, random initialization, domain randomization, team configurations, and targeted transfer tasks.

**Robustness.** Tool-use families recur at scale, but specific phase sequences and exploits are not guaranteed. The visual result is stronger than the cultural inference.

**What prevented further development.** Shared policy within teams, no death/naïve replacement, finite object types, fixed adversarial reward, and finite physics affordances.

**Genesis lesson.** Rich generic manipulation and multi-agent pressure can create non-scripted strategies. Cultural instrumentation is still required to determine whether later organisms inherit rather than independently relearn them.

## 6.14 POET and open-ended search

**Source:** [Wang et al., 2019](https://arxiv.org/abs/1901.01753). **[Preprint / influential open-ended-learning system]**

**What changed.** POET jointly generated biped-walking environments and optimized policies, periodically transferring policies between environments. Transfer enabled solutions that direct optimization and a simple curriculum control did not find.

**What researchers designed.** Environment mutation vocabulary, minimal criterion, novelty/selection logic, biped morphology, locomotion objective, transfer schedule, and policy optimizer.

**Was it cumulative culture?** No. Policies transfer between authored algorithmic niches; there are no organism-level learners, social observation, or cultural transmission.

**Authored solution?** Exact courses were generated, but challenge dimensions and locomotion success were predefined.

**Generalization.** Diverse environments and stepping-stone transfer are strengths. Later POET variants expand environment complexity, but remain bounded by representation.

**Repetitions.** POET studies use multiple independent runs and comparisons, but seed counts vary by experiment. The original paper's central claim is distributional performance across generated environments rather than a fixed cultural-replication design.

**Controls and ablations.** Direct optimization, direct-path curriculum, and transfer-disabled comparisons establish the importance of stepping stones and cross-environment transfer.

**Robustness.** Strong evidence for search benefits from divergent niches and transfer. Not evidence for culture.

**What prevented further development.** Fixed challenge generator and objective, no new morphology or action ontology, and no cultural lifecycle.

**Genesis lesson.** Parallel ecological niches and occasional transfer may prevent premature convergence. This is an engine-level experimental strategy, not an organism-level culture mechanism.

## 6.15 Avida and cumulative genetic adaptation

**Source:** [Lenski et al., 2003](https://doi.org/10.1038/nature01568). **[Peer-reviewed digital-evolution experiment]**

**What evolved.** Self-replicating programs genetically evolved the complex EQU logic function through historical intermediates.

**What researchers designed.** Instruction set, mutation system, CPU ecology, energetic rewards for nine logic functions, and environment.

**Was it cumulative culture?** No. The accumulation is genetic. There is no within-lifetime learning or social transmission.

**Authored solution?** The exact genome was not supplied, but the rewarded logic operations and computational substrate were authored.

**Generalization.** The result demonstrates historical contingency, stepping stones, and the role of prior simpler functions.

**Repetitions.** **50 replicate populations**, each founded by the same ancestor, were evolved for more than **15,000 ancestral generations**; EQU evolved in 23 populations under the principal treatment.

**Controls and ablations.** Lineage reconstruction, removal/reintroduction of intermediate mutations, and alternative reward environments.

**Robustness.** Canonical evidence for cumulative genetic complexity in a digital system.

**What prevented cultural interpretation.** All inherited adaptive information was genomic.

**Genesis lesson.** Use genetic-freeze and common-garden controls, because cumulative genetic adaptation can otherwise be mistaken for culture.

## 6.16 TERMES and stigmergic construction

**Source:** [Werfel, Petersen & Nagpal, 2014](https://doi.org/10.1126/science.1245842). **[Peer-reviewed robotics experiment]**

**What occurred.** Three autonomous climbing robots built user-specified structures using local sensing and environmental state, without centralized control or direct communication.

**What researchers designed.** Desired structure, compiler/planner that generated local construction rules, block geometry, robot controller, and legal-placement constraints.

**Was it cumulative culture?** No. The robots did not learn or transmit construction methods.

**Authored solution?** Strongly. The blueprint and local control policy were generated from a target structure.

**Generalization.** The system handled multiple structures and physical hardware, showing robust decentralized construction.

**Repetitions.** Physical construction trials were reported for representative structures; the system is an engineering validation rather than a seed-based evolutionary study.

**Controls and ablations.** Decentralized algorithm validation, fault tolerance, and structure-specific tests; no cultural controls.

**What prevented further development.** Fixed blocks, fixed local rules, external blueprint, no learning, no organism reproduction.

**Genesis lesson.** Construction and stigmergy can create settlement-like output without culture. Demonstrate learned dependency before calling structures traditions.

## 6.17 Sugarscape, Tierra, and PolyWorld

### Sugarscape

[Epstein and Axtell's *Growing Artificial Societies* (1996)](https://www.brookings.edu/books/growing-artificial-societies/) models trade, migration, disease, mating, conflict, and copied cultural tags. Cultural markers affect interaction, but there is no ratcheting of learned technical performance. Replication is through scenario runs and parameter experiments rather than modern seed protocols.

### Tierra

[Ray's Tierra](https://tomray.me/pubs/tierra/) generated digital ecological interactions among self-replicating programs. It is foundational artificial life, but adaptive inheritance is genomic/programmatic and tends toward bounded ecological motifs rather than learned cultural technology.

### PolyWorld

[Yaeger's PolyWorld](https://shinyverse.org/larryy/polyworld.html), documented in the original *Artificial Life III* chapter and later analyses such as [Williams & Yaeger (2017)](https://doi.org/10.3390/geosciences7030049), evolved embodied agents with neural behavior in a virtual ecology. It demonstrates emergent behavior and evolutionary ecology, not socially transmitted learned lineages.

**Genesis lesson.** Rich population behavior, conflict, and visually complex ecology do not establish cultural causation. The distinction between inherited controller structure and learned behavior must be explicit in persistence and replay.

## 6.18 Why no reviewed system solves the Genesis target

The reviewed systems usually author at least one of the following:

- the task solution or finite optimum;
- a direct demonstrator channel;
- a privileged expert;
- an externally selected cultural parent;
- the generation boundary;
- an abstract trait or skill representation;
- a fixed communication game;
- the artifact blueprint;
- a scalar complexity reward;
- a finite environment mutation vocabulary;
- shared policy parameters that eliminate individual transmission.

This does not make the studies invalid. It makes them mechanism studies. Their components can inform Genesis, but combining them does not guarantee an open-ended system. The scientific opportunity is to retain their controls while replacing authored cultural objects with local perception, organism learning, material persistence, and causal measurement.


---

# 7. Evidence from animal and human culture

## 7.1 Animal traditions: strong evidence, usually non-cumulative

### Great-tit feeder traditions

Aplin and colleagues seeded one of two equivalent feeder-opening methods in wild great-tit subpopulations. The variants diffused through measured social networks, became locally dominant, and persisted across turnover, with birds tending to conform to the locally common method ([Aplin et al., 2015](https://doi.org/10.1038/nature13998)). **[Peer-reviewed field experiment]**

This is high-quality evidence for:

- causal social transmission;
- arbitrary group-specific traditions;
- network-dependent diffusion;
- persistence through population turnover;
- conformity-like frequency effects.

It is not evidence for cumulative culture because neither variant was repeatedly improved.

**Genesis implication.** The first cultural assay should use equivalent variants. Adaptive differences are unnecessary for proving transmission and can obscure it by allowing independent payoff learning.

### Vervet-monkey foraging traditions

Wild vervet groups experimentally developed preferences for differently colored foods, and immigrants often adopted local preferences ([van de Waal et al., 2013](https://doi.org/10.1126/science.1232769)). **[Peer-reviewed field experiment]** The work supports conformity and group tradition, although color preferences remain a narrow convention rather than a ratchet.

**Genesis implication.** Migration can transmit culture in both directions: immigrants may bring innovations, but may also conform and erase their own variants. Migration rate and migrant social influence should be measured separately.

### Chimpanzee traditions and tool-use norms

Cross-site comparisons identify regional differences in chimpanzee behavior not readily explained by ecology alone ([Whiten et al., 1999](https://doi.org/10.1038/21415)). Controlled diffusion experiments show conformity to seeded tool-use methods ([Whiten et al., 2005](https://doi.org/10.1038/nature04047)). Tool transfers in some wild communities satisfy functional teaching criteria by giving naïve individuals access to otherwise unavailable tools or opportunities.

The evidence strongly supports chimpanzee culture and tool traditions. Historical claims of cumulative refinement are harder because researchers rarely observe the origin and successive modification of a tool lineage, and ecological or genetic confounds remain possible.

### Dolphin sponge-tool traditions

Shark Bay dolphins use marine sponges as foraging tools, with evidence of matrilineal social transmission and ecological specialization ([Krützen et al., 2005](https://doi.org/10.1073/pnas.0500232102)). This is a strong culturally transmitted foraging specialization, not a demonstrated sequence of cumulative improvements.

### Birdsong and cultural regularization

Zebra-finch isolate song can change over several tutor generations toward species-typical structure ([Fehér et al., 2009](https://doi.org/10.1038/nature07994)). The result demonstrates iterated cultural transformation and interaction with species-typical learning biases. Because “improvement” is defined partly by convergence toward a biologically constrained song form, it should not be treated as technological ratcheting.

Bird and whale songs can display long-lived traditions, drift, recombination, and rapid population-wide replacement. These are important for cultural lineage and network metrics, but performance improvement is often undefined or contested.

## 7.2 Animal evidence for bounded cumulative change

### Pigeon route chains

Sasaki and Biro compared solitary pigeons, stable pairs, and turnover pairs in which the more experienced bird was repeatedly replaced by a naïve partner. Turnover chains improved route efficiency across five replacement “generations” and ultimately outperformed controls ([Sasaki & Biro, 2017](https://doi.org/10.1038/ncomms15049)). **[Peer-reviewed experiment]**

This meets the four core CCE criteria:

1. route variants changed;
2. route information passed socially through paired flight;
3. route efficiency improved;
4. replacement and improvement repeated.

The result remains bounded because the route has a direct-path optimum, can in principle be discovered by one individual, and does not diversify into new functions.

### Guinea-baboon transmission chains

Some baboon experiments show increasing structure or performance when one individual's output becomes the next individual's input. However, if subjects do not observe another agent and receive only transformed task stimuli, the process is iterated learning but not necessarily social learning under the strict core definition. Genesis should retain this distinction when artifacts act as inputs: an environmental product can be social only if it was produced by an organism and causally mediates later learning.

### Bumblebees learning a copying-dependent sequence

Bridges and colleagues trained demonstrator bumblebees to solve a two-step puzzle in which the first step was unrewarded. Naïve bees failed to solve the task despite 36 or 72 hours of no-demonstrator exposure, while 5 of 15 observers acquired the complete behavior from trained demonstrators; two became proficient under the authors' criterion ([Bridges et al., 2024](https://doi.org/10.1038/s41586-024-07126-4)). **[Peer-reviewed experiment]**

The study shows:

- social learning can cross a substantial individual-discovery barrier in an invertebrate;
- following and action continuity may convert a temporally separated sequence into an associatively learnable unit;
- high-level imitation or symbolic teaching is not required for copying-dependent know-how.

It does not show:

- learner-to-learner transmission;
- repeated improvement;
- survival through biological generations;
- a natural bumblebee tradition.

### Chimpanzees learning a complex sequential skill

Van Leeuwen and colleagues exposed two chimpanzee groups, totaling 66 individuals, to a sequential puzzle for three months without success. After one chimpanzee per group was trained, 14 naïve chimpanzees acquired the skill, with network-based diffusion analysis linking acquisition to observations ([van Leeuwen et al., 2024](https://doi.org/10.1038/s41562-024-01836-5)). **[Peer-reviewed experiment]**

This supports socially acquired know-how beyond the measured individual innovation baseline. As with the bee study, an experimenter supplied the first expert and no repeated improvement chain was observed.

### Chimpanzee population connectivity and complex toolsets

Gunasekaram and colleagues compared networks of genetic connectivity with distributions of behavioral traits across chimpanzee populations. Complex toolsets showed stronger historical connectivity patterns than simple behaviors, and the distribution of simpler and more complex variants was consistent with stepwise elaboration and migration-mediated transmission ([Gunasekaram et al., 2024](https://doi.org/10.1126/science.adk3381)). **[Peer-reviewed comparative study]**

This materially strengthens the case for incipient chimpanzee cumulative culture, but its causal status differs from a transmission-chain experiment. Genetic connectivity is a proxy for historical migration; behavior is sampled at current or recent sites; innovations were not observed as they occurred. The result should be treated as a well-supported historical inference rather than direct lineage reconstruction.

## 7.3 What animal evidence predicts for Genesis

Animal research predicts that Genesis may plausibly produce:

- arbitrary group traditions;
- socially biased food, object, location, and route preferences;
- conformity and local norms;
- copying-dependent acquisition of a bounded sequence;
- cumulative optimization of a route or other continuous behavior;
- artifact or tool traditions when affordances are accessible;
- migration-mediated diffusion and divergence.

It does not provide a defensible prediction of:

- repeated emergence of new tool classes;
- cumulative construction beyond one functional domain;
- compositional signaling or language;
- institutions, explicit norms, or teaching systems;
- organized warfare;
- open-ended technological dependence.

## 7.4 Human cumulative culture in the laboratory

### Paper-airplane microsocieties

Caldwell and Millen tested 700 participants in 70 microsocieties. Groups improved paper-airplane performance across transmission chains under conditions emphasizing imitation, emulation/product access, or teaching. Each information route independently supported cumulative improvement on this task ([Caldwell & Millen, 2009](https://doi.org/10.1111/j.1467-9280.2009.02469.x)). **[Peer-reviewed experiment]**

**Interpretation.** No one high-level mechanism is universally necessary. The artifact retains substantial performance-relevant information, and the task gives rapid feedback, making emulation and reinforcement effective.

### Basket construction without teaching

Zwirner and Thornton used a basket-making task to examine whether teaching was necessary. Teaching aided transmission but was not essential for cumulative performance ([Zwirner & Thornton, 2015](https://doi.org/10.1038/srep16781)). **[Peer-reviewed experiment]**

**Interpretation.** Product structure and individual reconstruction can support a ratchet. Genesis should include artifact-only conditions before adding explicit social signals.

### Physical-system improvement without causal understanding

Derex and colleagues studied cumulative improvement in a physical system and participants' causal theories. Performance improved over generations while explicit causal understanding remained incomplete. Transmitted theories did not necessarily improve performance and could restrict exploration ([Derex et al., 2019](https://doi.org/10.1038/s41562-019-0567-9)). **[Peer-reviewed experiment]**

**Interpretation.** A population can exploit effects that no individual fully understands. This supports “author physics, not progress”: organisms may ratchet reliable actions against natural regularities without symbolic causal models.

**Limitation.** Human participants brought extensive pretrained cognition, object concepts, and motor skills. The experiment does not show that such capabilities emerge from a minimal controller.

### Oldowan stone-tool transmission

Morgan and colleagues tested 184 participants in transmission chains with five information conditions ranging from reverse engineering and imitation to basic and gestural teaching and verbal teaching. Teaching—especially language—substantially improved Oldowan flake production; emulation and imitation alone were weak for this difficult, opaque task ([Morgan et al., 2015](https://doi.org/10.1038/ncomms7029)). **[Peer-reviewed experiment]**

**Interpretation.** When success depends on force, angle, grip, sequencing, and hidden fracture mechanics, product inspection may not reveal enough information. Teaching and communication can become necessary for reliable accumulation at a given task difficulty even if they are not universally required.

### The three-stage puzzle box

Dean and colleagues compared capuchins, chimpanzees, and children on a progressively rewarding three-stage task. Children reached higher stages and displayed teaching, imitation, and prosocial support; nonhuman groups did not show equivalent cumulative progression ([Dean et al., 2012](https://doi.org/10.1126/science.1213969)). **[Peer-reviewed experiment]**

**Interpretation.** A package of social tolerance, attention, imitation, and active support can unlock difficult cumulative tasks. The study does not isolate one necessary component, and the task contains authored sequential stages, so it is a capacity assay rather than a model for Genesis world design.

## 7.5 Iterated learning and structure without explicit design

Kirby, Cornish, and Smith passed artificial languages through ten-person transmission chains. Across four chains, languages became easier to learn and more structured as the bottleneck selected for transmissible regularities ([Kirby et al., 2008](https://doi.org/10.1073/pnas.0707835105)). **[Peer-reviewed experiment]**

This demonstrates cumulative adaptation of a communication system to learner biases without participants designing the final grammar. It also illustrates a major caution: the transmission bottleneck and communicative task strongly shape what emerges. A structured signal system can be an adaptation to its inheritance channel rather than a direct adaptation to the external ecology.

Genesis implication:

- signals should face finite perception and memory bottlenecks;
- regularity and compositionality should be measured offline;
- a signal's structure should not be rewarded directly;
- communication must improve organism-relevant outcomes or transmission, not an external language score.

## 7.6 Demography, model access, and the collective brain

Henrich's model of Tasmanian technological loss proposed that effective population size and access to skilled models can determine whether complex skills are maintained ([Henrich, 2004](https://doi.org/10.2307/4128416)). Powell, Shennan, and Thomas similarly linked demographic change to Upper Paleolithic cultural complexity ([Powell et al., 2009](https://doi.org/10.1126/science.1170165)). Human group experiments report benefits from larger groups or access to more models ([Derex et al., 2013](https://doi.org/10.1038/nature12774); [Muthukrishna et al., 2014](https://doi.org/10.1098/rspb.2013.2511)).

The demographic turn has substantial criticism. Reanalyses and empirical comparisons find that census population is not consistently associated with toolkit complexity, and ecological risk, mobility, contact structure, and measurement choices matter ([Andersson & Read, 2014](https://doi.org/10.1038/nature13411); [Vaesen et al., 2016](https://doi.org/10.1073/pnas.1520288113); [Collard et al., 2016](https://doi.org/10.1098/rstb.2015.0242)).

The most defensible synthesis is:

- more potential models reduce stochastic loss only if learners can access them;
- diverse model access can improve recombination and preserve rare skills;
- partially connected multilevel networks can preserve parallel exploration while allowing exchange;
- social organization and mobility can matter more than raw census size;
- population size interacts with task difficulty and transmission fidelity.

Genesis should vary census size, realized contact network, and model diversity independently.

## 7.7 Archaeology and technological evolution

Archaeology supplies long time depth but sparse behavioral mechanism. Artifact sequences can show elaboration, standardization, diversification, and recombination, but they do not directly reveal who learned from whom.

Paige and Perreault coded stone-tool manufacturing sequences into procedural units and compared archaeological complexity over 3.3 million years with baselines from nonhuman-primate technology and naïve modern-human knapping. They inferred that toolmaking exceeded the non-cumulative baseline reliably after roughly 600,000 years ago ([Paige & Perreault, 2024](https://doi.org/10.1073/pnas.2319175121)). **[Peer-reviewed archaeological analysis]**

The result is relevant because it operationalizes dependency depth rather than visual sophistication. It remains sensitive to:

- how procedural units are defined;
- preservation bias;
- whether naïve modern humans are an appropriate non-cumulative baseline;
- missing organic technologies;
- the assumption that sequence length maps monotonically to cultural dependence.

Genesis can improve on archaeological limitations by logging complete causal histories. It should still borrow the idea of **dependency depth**: count how many prior modifications are required for a late behavior or artifact to function, not merely how many steps are visible.

## 7.8 Human uniqueness: cumulative or open-ended?

Older accounts often treated cumulative culture as a human/nonhuman binary. Current evidence supports a more graded view:

- nonhuman traditions are widespread;
- bounded cumulative improvement is experimentally demonstrated in pigeons;
- copying-dependent know-how is demonstrated in bumblebees and chimpanzees;
- human culture differs most clearly in scale, diversification, recombination, exaptation, institutional stabilization, and continued expansion into new domains.

Morgan and Feldman propose that human culture is uniquely **open-ended** rather than uniquely cumulative ([Morgan & Feldman, 2025](https://doi.org/10.1038/s41562-024-02035-y)). **[Peer-reviewed perspective]** This is a productive hypothesis for Genesis: the hard target is not the first ratchet but sustained creation of new culturally heritable possibilities.

---

# 8. Failure modes

## 8.1 Why accumulation commonly fails

Cultural systems fail when one or more links in the causal chain—innovation, transmission, retention, selection, or further modification—has an expected lifetime shorter than the time needed for the next link. Artificial systems add a second class of failure: the experimenter may accidentally manufacture apparent accumulation through an authored task, hidden state transfer, or metric.

## 8.2 Failure-mode diagnostic table

| Failure mode | Mechanism of failure | Observable signature | Genesis diagnostic or ablation | Likely intervention |
|---|---|---|---|---|
| **Low copying fidelity** | Functionally important details are lost faster than improvements arise | Variant lineages shorten; performance regresses after transmission | Sweep sensory resolution, observation time, memory, and morphology mismatch; separate action and outcome fidelity | Repeated models, artifact cues, error correction, modular tasks—not direct weight copying |
| **Catastrophic forgetting** | New learning overwrites prior acquired behavior | Newly acquired variant works briefly, then disappears after unrelated learning | Probe old behaviors throughout life; compare replay/dual-memory/no-replay learners | Bounded replay, modular plasticity, slower consolidation, environmental rehearsal |
| **Insufficient lifetime memory** | Delayed steps or outcomes cannot be associated | Failure grows with temporal gap; observers follow but cannot reproduce later | Vary observation–execution delay and sequence length | Eligibility traces, episodic buffer, predictive state compression |
| **Excessive genetic assimilation** | Recurrent learned behavior becomes genetically predisposed, eliminating dependence on social transmission | Behavior persists in social-isolation/common-garden controls | Freeze genotype; reintroduce naïve ancestral genotypes; cross-foster | Maintain changing detail, cultural neutrality, or costs that preserve learning flexibility |
| **Innovation dies with inventor** | No learner observes or expresses the variant before death | High invention count, low cultural reproductive number | Inventor-removal and opportunity logging | Longer overlap, persistent artifacts, repeated public performance, tolerance |
| **Homogeneous population** | Identical policies and dense mixing reduce independent search | High within-group similarity, low novelty and lineage branching | Compare modular networks, mutation/plasticity variation, and local isolation | Preserve subgroups, heterogeneous experience, partial connectivity |
| **Overly strong selection** | Early high-payoff variant fixes before alternatives are explored | Rapid fixation, low diversity, path dependence, poor transfer | Reduce payoff gradient; cap elite reproductive skew; neutral-variant control | Softer ecological gradients, multiple niches, frequency dependence |
| **Weak selection** | Improvements do not outcompete noise or copying errors | Performance-neutral drift; no retention advantage | Controlled payoff differences with matched observability | Increase consequence signal or task recurrence; avoid external culture reward |
| **Sparse demonstrations** | Learners lack exposure despite having capacity | No adoption conditional on low exposure; acquisition among exposed learners may be high | Exposure-matched replay; log line of sight and attention | Resource aggregation, overlap, tolerance, longer juvenile learning window |
| **Social-learning parasites** | Copiers avoid innovation costs and overexploit stale information | High copier frequency, declining population adaptation | Vary innovation cost and environmental change; track producer/copier payoffs | Conditional copying, frequency-dependent costs, private information advantages |
| **Environmental instability** | Inherited information becomes obsolete before retransmission | Old variants spread despite declining payoff; high lag cost | Vary autocorrelation of resource and physics conditions | Evolve uncertainty-sensitive copying; preserve stable material laws while varying ecology |
| **Excessive migration** | Group variants homogenize before independent improvement | Between-group divergence collapses; one early variant dominates | Migration sweep with equal census size | Intermediate migration, seasonal links, costly travel |
| **Too little migration** | Useful innovations remain trapped; small groups lose skills | High divergence but low complexity and frequent local extinction | Island model with controlled bridge rates | Rare exchange, mobile life stage, artifact trade |
| **Premature convergence** | Genetic or cultural search locks into local optimum | Long stasis, low lineage branching, poor challenge transfer | Novelty-neutral challenge, restart from archived populations, selection-strength sweep | Niche diversity, partial mixing, protected exploration, changing constraints |
| **Reward hacking/specification gaming** | Agents exploit metric rather than ecological function | High score with behavior that fails intended functional test | Counterfactual evaluation, holdout physics, direct energy/time accounting | Remove proxy reward; ground consequences in conserved resources and physics |
| **Authored task solutions** | Environment encodes recipe, stage order, expert, or privileged action labels | “Discovery” follows designer's hidden ontology | Remove labels, randomize object identities, change morphology and scale | Generic state transitions and organism-level consequences only |
| **Lack of useful affordances** | No action can create durable, selectable differences | Exploration produces no persistent functional variation | Affordance audit and random-controller reachability analysis | Add generic carry/strike/place/join/deform/separate operations and material response |
| **Lack of persistent external state** | Culture must fit entirely in living memory | Lineages terminate at cohort turnover | Artifact reset versus persistence | Material persistence with decay, repair, and ownership-independent access |
| **Population too small** | Stochastic loss exceeds innovation and retransmission | High between-seed extinction; rare variants vanish | Size sweep holding density/contact constant | Increase effective model pool or artifact retention |
| **Population too large/dense** | Observation overload and homogenization reduce local experimentation | Rapid global fixation; high attention competition | Density and network sweeps independent of census size | Local attention, limited bandwidth, modular geography |
| **Runs too short** | Stasis is mistaken for impossibility or novelty for progress | Conclusions depend on arbitrary stopping point | Survival curves, plateau tests, longer confirmatory runs | Dependency-ordered cheap assays before expensive long runs |
| **Novelty confused with progress** | More forms or longer sequences are called cumulative | Novelty rises without retention or function | Require descendant survival and causal performance contribution | Separate novelty, diversity, adaptation, and dependency metrics |
| **Complexity confused with culture** | Genetic or hard-coded structures appear sophisticated | Behavior survives social isolation and learned-state reset | Genetic freeze, naïve rearing, controller swap, artifact reset | Report genetic, ontogenetic, and cultural contributions separately |
| **Observer leakage** | Learner receives hidden demonstrator actions, rewards, or weights | Near-perfect transmission unaffected by occlusion or morphology | Remove privileged channel; compare ordinary vision | Use raw sensory states and visible outcomes only |
| **External elite selection** | Researcher chooses “best teacher,” manufacturing directed progress | Accumulation disappears under endogenous model choice | Random, local, prestige-cue, and endogenous model controls | Let exposure and perceived success determine model influence |
| **Synchronized generational bottleneck** | All experts vanish simultaneously or one lineage is chosen | Artificial sawtooth performance and founder effects | Continuous-lifecycle control | Overlapping generations; artifact-mediated bridging |
| **Measurement-induced scripting** | Online classifier or novelty metric alters behavior | Behavior changes when observer categories change | Run identical kernel with analytics disabled | Keep all cultural analysis offline and read-only |

## 8.3 Catastrophic forgetting in evolvable neural controllers

Catastrophic forgetting is established in sequential neural learning: learning a new mapping can destroy earlier mappings when parameters overlap ([French, 1999](https://doi.org/10.1016/S1364-6613%2899%2901294-2); [Kirkpatrick et al., 2017](https://doi.org/10.1073/pnas.1611835114)). This literature is not direct evidence about biological culture, but it is directly relevant to Genesis controllers.

A social learner can appear to acquire a tradition in a short test while failing to retain it through ordinary life. Therefore:

- probe behavior repeatedly after unrelated experiences;
- measure memory half-life;
- distinguish access failure from storage loss;
- include old/new task interference matrices;
- compare bounded episodic replay, synaptic consolidation, modular plasticity, and no-protection controls;
- avoid an unlimited replay buffer that becomes a perfect cultural archive inside each organism.

## 8.4 Genetic assimilation as both outcome and confound

Repeatedly learned behaviors can alter genetic selection so that predispositions reduce learning costs—the Baldwin effect and genetic assimilation. In Genesis this can be scientifically interesting, but it can also erase the cultural dependence that the experiment is trying to establish.

Signs of assimilation include:

- naïve organisms expressing the behavior without observation;
- reduced learning time across genetic generations;
- persistence after social networks and artifacts are reset;
- genotype explaining more variance while cultural lineage explains less.

Required controls:

- genetic freeze during cultural assays;
- common-garden rearing without social information;
- cross-fostering among cultural groups;
- replay of ancestral genotypes into later cultural environments;
- decomposition of performance into genotype, learned state, artifact inheritance, and interaction terms.

Assimilation is not a failure of the world. It is a failure only if it is misreported as continuing cultural accumulation.

## 8.5 Social-learning parasites and the producer–scrounger problem

Innovation often has a cost. If copying is cheaper, copiers can spread until too few innovators remain, reducing population adaptation. A similar problem occurs when socially learned techniques are public goods: beneficiaries do not bear teaching or discovery costs. Models of cumulative culture therefore face a cooperation problem as well as an information problem: social learners can exploit information without producing it, and cumulative improvements can behave as public goods ([Rogers, 1988](https://www.jstor.org/stable/678986); [Rendell et al., 2010](https://doi.org/10.1126/science.1184719); [Kobayashi, Wakano & Ohtsuki, 2015](https://doi.org/10.1016/j.jtbi.2015.05.002)).

Genesis should not prevent free-riding. It should measure:

- individual innovation cost;
- observation cost;
- demonstrator cost;
- payoff to copied behavior;
- frequency dependence of producer and copier strategies;
- whether kinship, reciprocity, spatial assortment, or prestige stabilizes information production.

## 8.6 Authored solutions and hidden technology trees

A technology tree can be hidden in apparently low-level design. Examples include:

- object type A can combine only with B to create C;
- a sequence detector transforms three actions into a named tool;
- the first object unlocks perception of the second;
- a reward bonus is granted for “construction complexity”;
- a curriculum introduces increasingly advanced materials at fixed dates;
- a classifier decides that a behavior is a discovery and makes it inheritable;
- agents receive abstract action or skill IDs from demonstrators.

A safer implementation defines local physical transformations:

- contact forces;
- fracture and deformation;
- friction and adhesion;
- heat and phase effects if computationally feasible;
- containment and flow;
- occlusion and support;
- wear and durability;
- spatial rearrangement;
- organism-relevant energy and risk consequences.

Even generic physics can contain affordance bias. The correct standard is not “designer influence is zero,” which is impossible. It is that the designer does not specify which action sequence is a named solution and that alternative functional organizations remain physically possible.

## 8.7 Null results that matter

A rigorous program should expect and preserve nulls. Informative outcomes include:

- social exposure changes attention but not long-term behavior;
- traditions spread but collapse at turnover;
- one improvement is transmitted but a second cannot build on it;
- artifacts preserve behavior but no learner modifies them;
- group traditions form but are selectively neutral;
- high fidelity suppresses innovation;
- migration spreads variants but prevents divergence;
- signaling evolves genetically but meanings are not learned;
- construction emerges but remains genetically fixed;
- cumulative route optimization occurs, while tool domains remain absent;
- rich behavior persists only under an authored assay and disappears in the open ecology.

These results narrow the mechanism bottleneck and are more valuable than an unreplicated visually compelling run.


---

# 9. Design options for The Genesis Engine

## 9.1 Design principle

Genesis should implement **causal channels**, not cultural objects. Organisms should never receive a fact such as “this is a tool,” “that organism is demonstrating,” “this sequence is a recipe,” or “this group has reached an era.” They may receive only sensory consequences of bodies, objects, signals, and persistent world state. Learning, selection, and historical dependence determine whether those channels become cultural.

The design options below are ordered approximately from necessary foundations to speculative amplifiers.

## 9.2 Mechanism A: perception of other organisms' actions and visible outcomes

| Field | Specification |
|---|---|
| **Why it is needed** | A social causal pathway must exist. Without observable action or products, behavior cannot be acquired socially. |
| **Status** | **Necessary** for live social learning; artifact perception can substitute when demonstrators are absent. |
| **Smallest useful implementation** | Local line-of-sight sensing of conspecific position, orientation, coarse body-part or actuator motion, contacts, and the ordinary object-state changes caused by those contacts. |
| **Information perceived** | Relative pose and velocity; contact onset/offset; object motion, deformation, damage, transfer, and resource exposure; emitted signals through normal sensors. |
| **State retained** | None required for simultaneous following; bounded event traces are required for delayed reproduction. |
| **May be genetically inherited** | Sensor placement, range, acuity, salience biases, morphology-specific motor priors. |
| **Must be learned within lifetime** | Associations between observed events and useful outcomes; cross-body action mappings; context-sensitive value. |
| **Accidental scripting risk** | Abstract action IDs, “demonstration” flags, true object function, demonstrator reward, or labels such as `tool_contact`. |
| **Nondeterminism risk** | Occlusion queries, collision ordering, floating-point pose estimates, and parallel sensor aggregation. |
| **Simultaneous-update rule** | All observations at tick `t` derive from the same committed snapshot. Agents submit intents after sensing; resolved effects become observable at `t+1`. |
| **Behavioral prediction** | With a seeded arbitrary action variant, exposed learners should reproduce the demonstrated action or outcome more often than isolated and shuffled-replay controls. |
| **Required control** | Action masked/outcome visible; outcome masked/action visible; ghost object motion; no-model; time-shuffled replay. |
| **Possible null** | Organisms attend to demonstrators but cannot map observed motion into their own action space. |
| **Computational cost** | With spatial indexing, approximately `O(Nk)` social observations per tick for `N` organisms and mean visible neighborhood `k`; raw pose detail increases bandwidth and memory. |

**Recommendation.** Begin with coarse action kinematics and high-quality object-state transitions. This allows factorial tests of imitation versus emulation without building a privileged action ontology.

## 9.3 Mechanism B: selective attention and observation

| Field | Specification |
|---|---|
| **Why it is needed** | Social information is useful only if agents allocate finite perception and processing to relevant events. Unlimited omniscient observation removes ecological opportunity costs. |
| **Status** | **Helpful and likely necessary at scale.** Tiny assays can function without explicit attention if all local stimuli are processed. |
| **Smallest useful implementation** | A bounded attention budget selecting a small number of spatial sectors, organisms, or objects for high-resolution processing; low-resolution peripheral sensing remains possible. |
| **Information perceived** | Salience cues available to any organism: motion, proximity, signal intensity, novelty, prior reward association, kin/familiarity, and observed resource consequences. |
| **State retained** | Learned salience values and recent attention targets. |
| **May be genetically inherited** | Baseline attraction/avoidance to conspecifics, motion, juveniles, kin, threat, or novelty; attention capacity. |
| **Must be learned within lifetime** | Which individuals, locations, signals, and object events predict useful information. |
| **Accidental scripting risk** | Rewarding “watching,” highlighting successful demonstrators using true fitness, or labeling demonstrations. |
| **Nondeterminism risk** | Ties among equally salient targets and parallel top-k selection. |
| **Simultaneous-update rule** | Compute salience from snapshot `t`; resolve ties with a stable counter-based random key or deterministic hash independent of entity iteration order. |
| **Behavioral prediction** | Increasing attention capacity or socially learned salience should raise exposure-conditioned transmission up to a point, then show diminishing returns or distraction costs. |
| **Required control** | Same visual stream with attention weights frozen or demonstrator identity shuffled. |
| **Possible null** | More attention increases following but not retention, indicating a memory or correspondence bottleneck. |
| **Computational cost** | Top-k selection over local stimuli; `O(k log a)` or `O(k)` with bounded attention slots `a`; downstream sensory processing dominates. |

## 9.4 Mechanism C: bounded within-lifetime plasticity

| Field | Specification |
|---|---|
| **Why it is needed** | Culture requires acquired rather than exclusively genetic behavior. The learner must alter future action because of experience. |
| **Status** | **Necessary.** |
| **Smallest useful implementation** | Deterministic fixed-point updates to a small set of plastic synapses, action values, predictive associations, or recurrent fast weights. Plasticity may be gated by novelty, surprise, reward prediction error, or attention. |
| **Information perceived** | The organism's own sensor stream, actions, internal drives, and visible environmental consequences; social observations use the same learner rather than a separate knowledge injector. |
| **State retained** | Plastic parameters, eligibility traces, learned values, and bounded confidence. |
| **May be genetically inherited** | Initial network, learning rates, plasticity rules, modulatory architecture, memory capacity, and biases. |
| **Must be learned within lifetime** | Specific social associations, routes, action sequences, signal meanings, object affordances, and group traditions. |
| **Accidental scripting risk** | A specialized update that writes the demonstrator's action directly into the observer's policy; direct reward transfer; hard-coded skill slots. |
| **Nondeterminism risk** | Non-associative reductions, GPU atomic operations, and floating-point optimizer state. |
| **Simultaneous-update rule** | Accumulate learning events during tick `t`; apply plastic updates once, in canonical event order or through commutative fixed-point reductions, after world resolution. |
| **Behavioral prediction** | Socially exposed organisms should change behavior after the demonstrator is absent; the effect should scale with exposure and disappear under plasticity freeze. |
| **Required control** | Genetically identical learners with plasticity disabled, own-reward-only learning, and social event masking. |
| **Possible null** | Agents can learn asocially but social observations do not enter useful representations. |
| **Computational cost** | Proportional to number of plastic parameters and learning events; keep authoritative plastic state small enough to checkpoint exactly. |

## 9.5 Mechanism D: bounded episodic and sequence memory

| Field | Specification |
|---|---|
| **Why it is needed** | Many demonstrations and outcomes are temporally separated. Culture must survive beyond immediate mimicry. |
| **Status** | **Necessary for delayed, multi-step, and intergenerational transmission; helpful for simple traditions.** |
| **Smallest useful implementation** | A fixed-capacity ring buffer of compressed event embeddings plus decaying eligibility traces. Later versions may add learned recurrent compression. |
| **Information perceived** | Selected state–event–outcome tuples, not full omniscient world snapshots. |
| **State retained** | Event time, relative object and organism features, observed contacts/actions, visible outcome, confidence, and source identity if perceptible. |
| **May be genetically inherited** | Capacity, decay, compression architecture, attention gates, consolidation rate. |
| **Must be learned within lifetime** | Event content and its predictive associations. |
| **Accidental scripting risk** | Storing symbolic recipes, perfect trajectories, future outcomes, or true causal graphs. |
| **Nondeterminism risk** | Buffer eviction ties, variable compression order, recurrent numerical divergence. |
| **Simultaneous-update rule** | Events receive stable IDs and are appended after tick resolution in canonical order. Eviction uses age and deterministic priority. |
| **Behavioral prediction** | Transmission fidelity should fall with longer delay and sequence depth; increased memory should shift the failure boundary but may increase interference. |
| **Required control** | Memory capacity and decay sweeps; unrelated-interference tasks; replay disabled; observation-to-execution delay. |
| **Possible null** | Larger buffers do not help because the controller cannot retrieve or map observations to action. |
| **Computational cost** | `O(NLd)` storage for `N` organisms, trace length `L`, and embedding dimension `d`; retrieval can be `O(L)` or bounded approximate matching. |

## 9.6 Mechanism E: learner exploration and innovation

| Field | Specification |
|---|---|
| **Why it is needed** | Transmission without modification produces stasis. Cumulative culture requires new variants. |
| **Status** | **Necessary for accumulation.** |
| **Smallest useful implementation** | Existing evolvable controller stochasticity or deterministic pseudorandom action variation, curiosity driven by prediction error, and reinforcement from organism-level outcomes. |
| **Information perceived** | Own uncertainty, novelty, prediction error, energy and injury consequences, and socially observed alternatives. |
| **State retained** | Outcome history, uncertainty, action values, and possibly local novelty estimates. |
| **May be genetically inherited** | Exploration rate, uncertainty sensitivity, risk tolerance, plasticity schedule. |
| **Must be learned within lifetime** | Which deviations are useful in the current ecology. |
| **Accidental scripting risk** | An “innovate” button that samples from a designer-curated list; novelty reward tied to offline categories; increasing sequence length rewarded directly. |
| **Nondeterminism risk** | Random action sampling and parallel novelty archives. |
| **Simultaneous-update rule** | Draws come from counter-based streams keyed by seed, tick, entity, and draw purpose; novelty is calculated from committed prior history only. |
| **Behavioral prediction** | Intermediate exploration should maximize retained improvement; zero exploration yields copying stasis and excessive exploration yields lineage decay. |
| **Required control** | Exploration-rate sweep; same-compute asocial learners; copied-policy with no modification; shuffled social history. |
| **Possible null** | Innovations occur but do not survive, implying transmission or selection—not innovation—is limiting. |
| **Computational cost** | Usually modest in-kernel; experimental cost rises because more seeds and longer runs are needed to estimate rare innovation survival. |

## 9.7 Mechanism F: overlapping lifecycles and demonstration opportunity

| Field | Specification |
|---|---|
| **Why it is needed** | Live social transmission requires knowledgeable and naïve organisms to coexist. Turnover also introduces fresh exploration. |
| **Status** | **Necessary for live intergenerational transmission unless artifacts bridge non-overlap.** |
| **Smallest useful implementation** | Continuous birth, maturation, learning, reproduction, and death; no global generation reset. Age affects experience and possibly morphology, not an explicit teacher role. |
| **Information perceived** | Age only if physically perceptible through size, appearance, behavior, or signals; no true “expert score.” |
| **State retained** | Individual learned state until death; social associations; artifacts persist independently. |
| **May be genetically inherited** | Lifespan distribution, juvenile period, maturation, social attraction, reproductive strategy. |
| **Must be learned within lifetime** | Who is informative and which recurring activities are worth observing. |
| **Accidental scripting risk** | Automatically pairing each newborn with the best adult, resetting cohorts, or designating one cultural parent. |
| **Nondeterminism risk** | Simultaneous births/deaths, parent selection, and resource contests. |
| **Simultaneous-update rule** | Lifecycle intents resolve from snapshot state; births are instantiated after parental/resource resolution with deterministic IDs; dead organisms' actions do not resolve after death priority. |
| **Behavioral prediction** | Transmission peaks at intermediate overlap: too little causes loss; too much can reduce naïve exploration or slow turnover-driven improvement. |
| **Required control** | Continuous lifecycle versus synchronized cohorts; inventor removal; artifact persistence on/off. |
| **Possible null** | Abundant overlap produces no culture because attention, tolerance, or learning is absent. |
| **Computational cost** | No major asymptotic change, but long-lived heterogeneous state increases checkpoint size and analysis complexity. |

## 9.8 Mechanism G: persistent manipulable artifacts and structures

| Field | Specification |
|---|---|
| **Why it is needed** | Artifacts preserve products of behavior, create delayed demonstrations, constrain later action, and may open new affordances. They are the most plausible route to external memory and technological dependence. |
| **Status** | **Helpful for simple culture; probably necessary for Genesis's technological ambition; open-ended effects remain speculative.** |
| **Smallest useful implementation** | Objects have generic material state: position, orientation, mass, shape or voxel occupancy, integrity, friction, attachment/contact relations, and decay. Organisms can carry, strike, place, push, stack, separate, and perhaps join or deform them. |
| **Information perceived** | Ordinary geometry, material cues, motion, resistance, damage, residue, containment, support, accessibility, and prior wear. |
| **State retained** | World-owned object state, not organism-owned “technology” metadata; stable maker and modification provenance is logged offline only. |
| **May be genetically inherited** | Morphology and action capabilities that make manipulation possible; perceptual biases toward certain materials. |
| **Must be learned within lifetime** | Object uses, manufacture sequences, repair, reuse, and social meanings. |
| **Accidental scripting risk** | Fixed recipes, named parts, crafting slots, blueprint constraints, “tool quality,” or hidden behavior triggers. |
| **Nondeterminism risk** | Collision/contact ordering, constraint solvers, breakage thresholds, parallel modifications, and garbage collection. |
| **Simultaneous-update rule** | Agents submit object-manipulation intents; contacts are solved from a common snapshot; conflicting claims use commutative impulse aggregation or deterministic arbitration; breakage and attachment changes commit once per tick. |
| **Behavioral prediction** | Intact artifacts should enable naïve organisms to recover or perform behaviors after knowledgeable organisms are removed; resetting or scrambling artifacts should reduce performance. |
| **Required control** | Persistent versus reset world; artifact-only versus live demonstration; intact versus scrambled geometry; maker absent; identical resource distribution. |
| **Possible null** | Artifacts persist but carry no recoverable information, or organisms use them opportunistically without socially dependent learning. |
| **Computational cost** | Potentially dominant. Fine physics and unbounded object persistence can exceed organism compute. Deterministic decay, merging, sleeping, and level-of-detail rules require careful ablation. |

## 9.9 Mechanism H: spatial population structure and migration

| Field | Specification |
|---|---|
| **Why it is needed** | Local contact permits traditions to form and diverge; occasional migration permits exchange and recombination. |
| **Status** | **Helpful; necessary for group-divergence experiments.** |
| **Smallest useful implementation** | Geography, movement cost, local perception, resource clustering, and no global broadcast. Migration emerges from movement and reproduction rather than a culture-specific operator. |
| **Information perceived** | Local terrain, resources, organisms, signals, and artifacts; group membership need not be explicit. |
| **State retained** | Familiarity and learned social associations; offline contact graph. |
| **May be genetically inherited** | Dispersal tendency, territoriality, social attraction, homing, habitat preference. |
| **Must be learned within lifetime** | Local routes, group conventions, relationships, and model preferences. |
| **Accidental scripting risk** | Fixed tribes with hard borders, preassigned cultural IDs, automatic in-group copying, or scheduled cultural exchange. |
| **Nondeterminism risk** | Movement conflicts, boundary crossing, group detection analytics. |
| **Simultaneous-update rule** | Movement intents resolve deterministically; group labels are inferred offline from contact networks and never used by the kernel. |
| **Behavioral prediction** | Between-group divergence should be maximal at low-to-intermediate migration; cumulative retention may peak at intermediate connectivity. |
| **Required control** | Migration and contact sweeps holding population, density, ecology, and genotype constant; random rewiring controls. |
| **Possible null** | Groups remain behaviorally identical because payoffs force one optimum or social learning is weak. |
| **Computational cost** | Spatial indexing reduces social interactions to local `O(Nk)`; multi-island experiments multiply seed and storage requirements. |

## 9.10 Mechanism I: evolvable signals with learned meanings

| Field | Specification |
|---|---|
| **Why it is needed** | Signals can direct attention, coordinate joint action, preserve conventions, and potentially make difficult procedures easier to transmit. |
| **Status** | **Helpful and speculative for advanced accumulation; not required for initial culture.** |
| **Smallest useful implementation** | One or a few generic channels with evolvable production intensity/timing and local propagation; receivers learn associations through normal plasticity. |
| **Information perceived** | Signal waveform or token-like event plus local context; no decoded meaning. |
| **State retained** | Learned signal–context–outcome associations and source reliability. |
| **May be genetically inherited** | Signal anatomy/channel sensitivity, production costs, innate biases, learning rules. |
| **Must be learned within lifetime** | Group-specific meanings, context, pragmatics, and trust. |
| **Accidental scripting risk** | Fixed vocabulary, message schemas, direct coordinates, goal IDs, or guaranteed truthfulness. |
| **Nondeterminism risk** | Signal collision, propagation timing, and simultaneous sender aggregation. |
| **Simultaneous-update rule** | Signals emitted at `t` enter a buffered field and are perceived no earlier than `t+1`; combination uses deterministic attenuation and saturation. |
| **Behavioral prediction** | Signals should first become predictive cues or attention directors; learned group-specific mappings should transfer through exposure and fail under signal remapping. |
| **Required control** | Signal channel disabled; meaning remapped; sender identity shuffled; genetically frozen receivers; replay without contingent response. |
| **Possible null** | Signals evolve genetically for immediate coordination but no learned cultural semantics appear. |
| **Computational cost** | Low for sparse local events; higher for continuous spatial fields or many channels. |

## 9.11 Mechanism J: tolerance, opportunity provisioning, and emergent teaching

| Field | Specification |
|---|---|
| **Why it is needed** | Competition can prevent observation. Tolerance and active scaffolding may make difficult skills transmissible. |
| **Status** | **Helpful; teaching is speculative and task dependent.** |
| **Smallest useful implementation** | Ordinary choices to allow proximity, share or relinquish objects, repeat actions, slow movement, leave partial products, or emit attention-directing signals. No `teach` opcode. |
| **Information perceived** | Learner presence and behavior through normal senses; demonstrator costs through its own energy/time budget. |
| **State retained** | Social relationships, kin/familiarity, past reciprocation, learner responsiveness. |
| **May be genetically inherited** | Social tolerance, parental care, kin bias, aggression threshold, signal tendencies. |
| **Must be learned within lifetime** | Which individuals reciprocate, when demonstrations are effective, and culturally specific scaffolding behavior. |
| **Accidental scripting risk** | Automatic knowledge transfer when near juveniles; teaching reward; direct access to learner ignorance. |
| **Nondeterminism risk** | Resource ownership conflicts and simultaneous transfers. |
| **Simultaneous-update rule** | Offers, releases, and transfers are intents resolved once per tick; no same-tick re-transfer chain. |
| **Behavioral prediction** | Knowledgeable organisms should change behavior contingently in learners' presence, incur measurable cost, and accelerate learner acquisition. |
| **Required control** | Recorded non-contingent replay, learner occluded from demonstrator, cost removed, kinship and reciprocity controls. |
| **Possible null** | Tolerance evolves but active teaching does not; passive observation is sufficient. |
| **Computational cost** | Low in-kernel; proving functional teaching requires detailed interaction logs and matched counterfactual analyses. |

## 9.12 Mechanism K: offline cultural provenance and causal instrumentation

| Field | Specification |
|---|---|
| **Why it is needed** | Without provenance, genetic rediscovery, independent learning, and social transmission are observationally confounded. |
| **Status** | **Necessary for scientific claims, but not an organism mechanism.** |
| **Smallest useful implementation** | Stable event IDs linking observers, demonstrators, artifacts, object modifications, action signatures, births, genotypes, learned-state snapshots, and performance outcomes. |
| **Information perceived** | None; the observer system is read-only. |
| **State retained** | Append-only or checkpointed event/provenance records, versioned analytics, hash of authoritative state. |
| **May be genetically inherited** | Nothing. |
| **Must be learned within lifetime** | Nothing. |
| **Accidental scripting risk** | Feeding lineage labels, detected traditions, novelty scores, or classifications back into behavior. |
| **Nondeterminism risk** | Log ordering, asynchronous writes, unstable IDs, and analytics that depend on iteration order. |
| **Simultaneous-update rule** | Generate event records from the committed tick result with canonical ordering. Analytics run after the fact and cannot mutate state. |
| **Behavioral prediction** | None directly; it enables falsifiable estimates of social dependency and lineage. |
| **Required control** | Bit-identical simulation hashes with logging enabled and disabled. |
| **Possible null** | Apparent lineages disappear under causal controls, indicating independent rediscovery. |
| **Computational cost** | Potentially high storage: observation and artifact edges can grow faster than state. Use event hashing, sampling for low-value contacts, and lossless retention of predeclared critical events. |

## 9.13 Major design recommendations

| Finding | Evidence quality | Genesis implication | Proposed experiment | Required ablation | Determinism risk | Confidence level |
|---|---|---|---|---|---|---|
| Traditions do not require imitation or language | Strong comparative experiments and reviews | Begin with generic action/outcome perception and associative plasticity | Two-action diffusion assay | Local-only, stimulus-only, action-only, outcome-only, no-model | Sensor tie resolution and event order | **High** |
| Cumulative culture requires repeated retained improvement, not spread alone | Strong conceptual consensus and experiments | Make sequential functional dependency a mandatory claim criterion | Two-layer improvement chain | Same-compute asocial, shuffled lineage, component removal | Offline lineage reconstruction | **High** |
| No universal transmission-fidelity threshold exists | Strong modeling logic; threshold values are model specific | Sweep fidelity-producing mechanisms rather than hard-code a target | Memory/acuity/error sweep | Redundancy and repeated-model controls | Fixed-point similarity metrics | **High** |
| Social plus individual learning can outperform either alone | Multiple human, animal, and artificial studies | Social information should bias exploration, not clone policy | Innovation–imitation balance experiment | Social-only and individual-only | Random exploration streams | **High** |
| Turnover can generate improvement rather than only loss | Pigeon experiment and replicated model | Use overlapping lifecycles and test replacement rate | Route or continuous-foraging chain | Solo, fixed pair, no-memory | Birth/death and partner conflict order | **High for bounded tasks** |
| Persistent products can transmit information after maker absence | Strong social-learning theory; human product-chain evidence; stigmergy | Add world-owned artifacts before symbolic communication | Artifact-only transmission | World reset, scrambled artifact, maker present/absent | Contact solver and decay | **High** |
| Population size acts through effective connectivity and model access | Mixed empirical evidence, strong theory | Vary census, degree, modularity, and migration independently | Island/network sweep | Ecology/genotype held constant | Parallel neighborhood ordering | **Medium-high** |
| Teaching and language are task-dependent fidelity amplifiers | Strong human experiments, limited universality | Do not hard-code teaching; allow tolerance and signals to evolve later | Opaque multi-step task | Passive replay, no-signal, non-contingent demo | Transfer arbitration and signal timing | **High** |
| Copying hidden neural state bypasses the scientific problem | Strong conceptual inference | Prohibit weight/activity transfer in baseline; retain only as upper bound | Sensory social learner versus weight-copy oracle | Full hidden-state transfer control | Checkpoint and parameter-copy semantics | **High** |
| Rich tool use can emerge without culture | Strong multi-agent and robotics demonstrations | Instrument cultural dependency before interpreting visual strategies | Open manipulation ecology | Social isolation, learned-state reset, genotype swap | Physics and policy-update order | **High** |
| Partial connectivity can preserve diversity and enable exchange | Models, human networks, comparative evidence | Avoid default complete mixing | Migration/modularity sweep | Fully mixed and isolated islands | Migration/event IDs | **Medium-high** |
| Open-ended technological evolution remains unsolved | Broad artificial-life reviews and absence of robust demonstrations | Treat long-horizon technology as an exploratory endpoint, not acceptance criterion | Open ecology after all dependencies pass | Finite-task and novelty-only baselines | State growth and long-run drift | **High** |


---

# 10. Recommended minimum viable cultural-transmission system

## 10.1 Objective

The minimum viable system should answer one narrow question:

> Can a learned behavioral variant causally spread through ordinary local perception, persist beyond the inventor's death, and remain available for later modification in a deterministic embodied population?

It should not initially attempt language, institutions, metallurgy, agriculture, warfare, or open-ended technological evolution. Those outcomes depend on mechanisms that cannot be interpreted if basic transmission has not been isolated.

## 10.2 Minimum world affordances

Use a deliberately small assay ecology embedded in the same generic kernel principles intended for the long-term world.

### Organisms can

- move and orient;
- perceive local geometry, objects, resources, and conspecific motion;
- contact, push, pull, carry, drop, strike, and place one or a few generic object types;
- consume resources and experience ordinary energetic consequences;
- learn within a lifetime through bounded fixed-point plasticity;
- retain a short event history;
- reproduce and die under continuous lifecycle rules.

### Objects can

- persist after release and after the maker dies;
- occupy space and block movement;
- be displaced, stacked, or positioned;
- expose or deny access to a resource through geometry;
- accumulate wear or modification if the current physics supports it;
- decay according to material state rather than cultural status.

### The world does not contain

- recipes;
- “tool” classes;
- crafting interfaces;
- technology points;
- named action sequences;
- explicit demonstrator roles;
- researcher-selected cultural parents;
- online tradition labels;
- direct neural state transfer.

## 10.3 Minimum organism learning architecture

A scientifically conservative initial architecture is:

1. **Evolvable base controller.** Genetically inherited fixed-point recurrent network or existing Genesis neural controller.
2. **Small plastic subspace.** A bounded subset of synapses or action-value associations can change during life.
3. **Socially ordinary input.** Other organisms and their object contacts appear in the same perceptual coordinate system as all other moving bodies.
4. **Event memory.** A fixed-capacity buffer stores compressed local pre-state, observed motion/contact, and visible post-state.
5. **Own-value evaluation.** The learner updates using its own drive changes and predicted outcomes. Demonstrator reward and fitness are never exposed.
6. **Exploration gate.** The learner can try deviations from observed behavior based on uncertainty, prediction error, or inherited exploration parameters.
7. **No learned-state inheritance at birth.** Offspring inherit genes and enter a world containing organisms and artifacts, not parental synaptic changes.

This design allows imitation-like behavior, emulation, local enhancement, observational conditioning, and artifact learning to emerge as different uses of the same sensory and plastic substrate.

## 10.4 Recommended observation record

A compact observation event can be represented conceptually as:

```text
SocialObservationEvent {
    event_id
    tick
    observer_id
    perceived_actor_signature
    relative_actor_pose_before
    relative_actor_motion_summary
    perceived_object_signature
    object_state_before
    visible_contact_summary
    object_state_after
    visible_resource_or_hazard_change
    observer_attention_weight
    observation_confidence
}
```

Important restrictions:

- `perceived_actor_signature` contains only features the observer can sense, not a stable global identity unless individual recognition has evolved or been learned.
- `perceived_object_signature` contains physical features, not object class labels such as `hammer`.
- `visible_resource_or_hazard_change` is a sensed environmental change, not the demonstrator's internal reward.
- The authoritative learner need not store the full event; it may retain a deterministic compressed representation.
- The offline logger can retain true entity IDs for causal analysis, but those IDs are not supplied to the organism.

## 10.5 Tick-level semantics

A minimum deterministic tick should use the following phases:

1. **Snapshot:** freeze authoritative state `S_t` for sensing.
2. **Sense:** calculate all organism observations from `S_t` only.
3. **Attend and decide:** each organism updates transient inference from prior committed memory and submits an action, signal, observation, reproduction, or learning-gate intent.
4. **Resolve movement and contact:** combine or arbitrate intents deterministically from the common snapshot.
5. **Resolve object and resource transformations:** apply impulses, breakage, transfer, consumption, and attachment rules once.
6. **Resolve lifecycle:** process injury, energy, birth, and death according to a documented deterministic priority model.
7. **Construct learning events:** derive own-action and observed-action transitions from `S_t` and committed `S_{t+1}`.
8. **Apply plasticity:** update each organism's learned state once using a canonical ordering or commutative reduction.
9. **Commit:** finalize `S_{t+1}` and append canonical provenance records.
10. **Hash/checkpoint as scheduled:** include all learned and pending state.

A signal emitted during tick `t` should be observable no earlier than `t+1`. An object moved during `t` cannot be used by another agent's decision until `t+1`, although simultaneous physical contacts may be jointly resolved. This preserves simultaneous-update semantics and prevents iteration-order advantages.

## 10.6 Initial transmission targets

The first factorial experiment should expose four channels independently:

| Channel | Observer receives | Observer does not receive |
|---|---|---|
| **Location only** | Demonstrator position and movement destination | Fine action form and object outcome |
| **Object/stimulus only** | Which object is contacted | Fine body motion and object transformation |
| **Outcome/emulation** | Object-state transition and resulting access | Demonstrator action kinematics |
| **Action plus outcome** | Body motion, contact, and object transformation | Hidden neural state, reward, intention, action ID |

A fifth **weight-copy oracle** condition may be used only as an upper-bound control. It should be labeled direct policy transfer, not social learning.

## 10.7 Initial assay behavior

Use an arbitrary two-variant manipulation whose alternatives are physically equivalent. For example, a resource enclosure can be opened by moving an unconstrained object left or right, or by contacting one of two symmetric sides. The symmetry must be validated with isolated learners so neither variant has an inherent payoff or motor advantage.

The assay is not a crafting recipe because:

- the enclosure follows generic collision and support rules;
- multiple action trajectories can produce either outcome;
- no sequence detector recognizes a solution;
- the only consequence is physical resource access;
- variants are identified offline from trajectories.

After cultural diffusion is proven, replace the neutral alternatives with small payoff differences and then with a continuous improvement space.

## 10.8 Minimum persistent-artifact assay

Create a physical configuration whose function remains after the maker leaves—for example, a displaced blocker, a stable ramp-like arrangement, or a path cleared by moving obstacles. The configuration should be reachable through generic actions and should reduce future effort without a hidden artifact score.

The critical sequence is:

1. one organism discovers or is experimentally seeded with a configuration;
2. naïve observers encounter the maker and artifact;
3. the maker and all knowledgeable observers are removed;
4. a later naïve cohort encounters the intact artifact;
5. the later cohort either reuses, repairs, or reconstructs it;
6. intact, scrambled, reset, and naturally decayed conditions are compared.

The artifact functions as external memory only if later performance depends on its historically produced state and survives removal of knowledgeable organisms.

## 10.9 State that must be checkpointed

Verified restoration must include, at minimum:

- genetically inherited controller parameters;
- all plastic parameters and optimizer/learning traces;
- recurrent hidden state;
- event-memory buffer contents and insertion/eviction indices;
- learned salience, values, signal associations, familiarity, and confidence;
- organism age, energy, injury, reproductive state, and pending lifecycle intents;
- complete artifact/material state, including wear, attachments, ownership-independent placement, and decay timers;
- signal fields and propagation buffers;
- counter-based pseudorandom counters or keys for every stream;
- pending action and conflict-resolution intents if checkpoints can occur mid-tick;
- provenance event sequence number and log hash;
- analytics version identifiers, stored separately and excluded from behavior.

A restoration test should compare state hashes and subsequent event logs through culturally relevant interactions, not merely population counts or positions.

## 10.10 Features to defer

Do not include in the minimum system:

- direct imitation losses;
- expert bots in the natural ecology;
- prestige based on true fitness;
- explicit kin teaching bonuses;
- symbolic messages;
- joint intention representations;
- transferable skills or options;
- direct policy, hidden-state, or synapse copying;
- hierarchical planning inserted specifically for tools;
- complex chemistry or unrestricted soft-body physics;
- researcher-defined technology complexity rewards.

Each may later be introduced as a mechanistic treatment only after a lower-level baseline fails or reaches a measured limit.

## 10.11 Minimum acceptance criteria

The minimum system is successful only if all of the following are demonstrated in confirmatory runs:

1. Opposite seeded variants produce opposite population biases.
2. No-model controls do not show the same bias.
3. Behavior is expressed after the demonstrator is absent.
4. The variant survives the inventor's death.
5. The variant persists through at least three complete cohort turnovers under a predeclared threshold.
6. Genetic-freeze and common-garden controls show that persistence depends on social exposure or artifact history.
7. Transmission remains bit-reproducible for the same seed and diverges appropriately across independent seeds.
8. A causal intervention on the relevant observation channel removes or sharply reduces transmission.
9. Offline labels and logging have no effect on authoritative hashes.
10. At least one socially transmitted variant can be modified and the modification retained by a later learner before claiming a first ratchet.

The final item marks the boundary between a minimum **tradition system** and a minimum **cumulative-culture system**.


---

# 11. Dependency-ordered experimental program

## 11.1 General experimental rules

The program below uses hard quantitative thresholds to prevent retrospective storytelling. These thresholds are **proposed Genesis acceptance criteria**, not constants established by the literature. They should be preregistered, revised only before confirmatory runs, and interpreted with uncertainty intervals and effect sizes.

### Seed policy

- **Engineering pilots:** 5–10 seeds per cell to detect crashes, ceiling effects, and gross mechanism failures. Pilot results are not confirmatory.
- **Standard confirmatory assays:** at least **30 independent seeds per cell**.
- **Failure-heavy, multimodal, or rare-innovation assays:** preferably **50 or more seeds per cell**.
- **Long open-ecology runs:** use sequential analysis or a preregistered two-stage design, but retain at least 20–30 independent worlds if the claim concerns general emergence rather than one historical case.
- Seeds must control initialization, mutation, pseudorandom exploration, resource placement, and lifecycle events through documented independent streams. Re-running one seed with different thread scheduling is a determinism test, not a biological replicate.

### Statistical policy

- Report all seeds, including extinctions and degenerate runs.
- Use hierarchical models or seed-level bootstrap intervals when individuals within a world are not independent.
- Correct for repeated metric searches or predeclare a primary metric.
- Report survival curves and lineage distributions, not only means.
- Use permutation or shuffled-provenance nulls for lineage claims.
- Hold cumulative compute equal when comparing social and asocial learning.
- A null is not “no interesting screenshot”; it is failure to exceed a predeclared causal control.

## 11.2 Experiment 0 — deterministic instrumentation and assay symmetry

| Field | Design |
|---|---|
| **Hypothesis** | The observer and provenance system is behaviorally inert, restoration is exact, and the two assay variants are physically and genetically symmetric before social seeding. |
| **Mechanism under test** | Deterministic logging, checkpoint restoration, event IDs, and neutral two-variant affordance. |
| **Independent variables** | Logging on/off; checkpoint/restore versus uninterrupted run; left/right or A/B variant; thread count or deterministic execution partition. |
| **Dependent variables** | Bitwise state hashes; event-log hashes; action frequencies; success latency; energy cost; mortality; controller updates. |
| **Controls/ablations** | Analytics binaries absent; labels renamed; mirrored world geometry; object identities permuted; same seed under multiple execution layouts. |
| **Seeds** | 30 symmetry seeds plus at least 10 long restoration tests at multiple checkpoint ticks. |
| **Success threshold** | Identical seeds produce identical authoritative hashes and post-restore event streams. In isolated naïve organisms, A/B choice and performance differences have a 95% interval containing zero and a predeclared equivalence bound, such as absolute choice bias <0.05 and standardized performance difference <0.2. |
| **Failure interpretation** | Hash mismatch indicates nondeterminism or incomplete persistence. Variant bias means later diffusion cannot be attributed cleanly to social learning. |
| **Confounds** | Fixed entity-ID priority, asymmetric geometry, sensor orientation, collision order, spawn pattern, or analytics accidentally feeding state. |
| **Expected burden** | Low to moderate; short runs, high instrumentation. |
| **Falsification** | Any reproducible state divergence after restore, logging-induced behavior change, or material A/B asymmetry falsifies readiness for cultural assays. |

**Exit gate.** No later experiment should proceed until this gate passes.

## 11.3 Experiment 1 — spread of an arbitrary learned behavior

| Field | Design |
|---|---|
| **Hypothesis** | Ordinary perception plus within-lifetime plasticity causes an experimentally seeded arbitrary action variant to spread through a population. |
| **Mechanism under test** | Social observation, attention, and learned action/outcome association. |
| **Independent variables** | Seeded variant A, seeded variant B, no demonstrator; observation channel: location-only, stimulus-only, outcome-only, action-plus-outcome; exposure dose. |
| **Dependent variables** | Variant choice probability among eligible naïve learners; acquisition latency; exposure-conditioned adoption; action-form and outcome fidelity; social-learning effect size relative to no-model null. |
| **Controls/ablations** | No-model; demonstrator hidden; time-shuffled replay; actor identity shuffled; ghost object motion; plasticity disabled; same-compute asocial exploration. |
| **Seeds** | Pilot 10 per cell; confirmatory 30 per primary cell, 50 if acquisition is below 20%. |
| **Success threshold** | Opposite demonstrations produce opposite choice biases; at least 60% of eligible learners acquire the seeded variant in at least 80% of confirmatory seeds; treatment exceeds the 95th percentile of shuffled/no-model nulls; behavior is expressed after demonstrator removal. |
| **Failure interpretation** | No exposure effect implies a perception, attention, plasticity, or motor-correspondence bottleneck. An outcome-only effect indicates emulation rather than action imitation. A location-only effect indicates enhancement rather than procedural copying. |
| **Confounds** | Variant asymmetry, demonstrator disturbing resources, learners receiving direct payoff while co-acting, genetic relatedness, and repeated independent discovery. |
| **Expected burden** | Low; short assay world and small populations. |
| **Falsification** | If seeded A and B do not causally shift learner behavior relative to matched controls, the hypothesis is false for the implemented mechanism. |

**Required report.** State which channel transmits what. Do not report “imitation” unless action-form fidelity survives outcome and enhancement controls.

## 11.4 Experiment 2 — survival beyond inventor death and cohort turnover

| Field | Design |
|---|---|
| **Hypothesis** | A socially acquired variant can persist after the inventor dies and through complete replacement of the original cohort. |
| **Mechanism under test** | Learner-to-learner retransmission, memory retention, lifecycle overlap, and optionally persistent products. |
| **Independent variables** | Inventor lifespan after seeding; cohort overlap; turnover rate; artifact persistence on/off; number of initial learners. |
| **Dependent variables** | Variant prevalence after inventor death; number of complete cohort turnovers survived; cultural lineage length; cultural reproductive number `R_c`; extinction hazard; fraction of expression events causally linked to prior exposure. |
| **Controls/ablations** | Inventor removed before any observation; no learner-to-learner visibility; world reset at death; learned state reset while genotype retained; genetically frozen population. |
| **Seeds** | 30 per core condition; 50 when lineage extinction is common. |
| **Success threshold** | The seeded variant remains above 60% prevalence after at least **three complete cohort turnovers** in at least 80% of seeds and exceeds no-observation persistence by a predeclared large effect. The longest lineage must include at least four distinct carriers, not repeated expression by one organism. |
| **Failure interpretation** | Collapse immediately after inventor death means diffusion without cultural inheritance. Collapse at turnover indicates insufficient overlap, memory, expression, or artifact bridging. |
| **Confounds** | Genetic inheritance of predisposition, stable environmental asymmetry, long-lived original learners miscounted as turnover, and independent rediscovery. |
| **Expected burden** | Moderate; lifecycle duration dominates. |
| **Falsification** | If prevalence after complete turnover is indistinguishable from no-observation or shuffled-lineage controls, persistent tradition is not established. |

## 11.5 Experiment 3 — competition among cultural variants

| Field | Design |
|---|---|
| **Hypothesis** | Socially transmitted variants can coexist, drift, or be selected according to organism-perceptible performance consequences rather than researcher labels. |
| **Mechanism under test** | Cultural selection, model choice, payoff learning, conformity, and frequency dependence. |
| **Independent variables** | Relative payoff difference between A and B; observability of payoff; initial frequency; model prestige cues; environmental stability; copying cost. |
| **Dependent variables** | Variant frequency trajectory; fixation probability; time to fixation; lineage reproductive success; learner model-choice function; population performance; maintained diversity. |
| **Controls/ablations** | Neutral-payoff condition; payoff hidden from observers; no social learning; payoff reversed mid-run; frequency cues masked; true-fitness prestige replaced with noisy visible cues. |
| **Seeds** | 50 per key payoff condition because fixation is stochastic; fewer only after power analysis from pilots. |
| **Success threshold** | Under visible payoff differences, the better variant has a fixation or sustained-prevalence probability significantly above the neutral expectation and no-social control. Under equal payoffs, groups show drift or frequency effects without systematic global preference. After payoff reversal, conditional learners eventually shift while indiscriminate copiers lag. |
| **Failure interpretation** | No selection despite visible payoff suggests insufficient outcome learning. Instant global fixation suggests overly strong selection or an information leak. Equal-payoff convergence to one fixed side suggests hidden asymmetry. |
| **Confounds** | Differential motor cost, demonstrator abundance, kin structure, resource depletion, and genetic evolution during the assay. |
| **Expected burden** | Moderate; many seeds and long enough runs for fixation dynamics. |
| **Falsification** | If variant success is unrelated to perceptible consequences and instead follows hidden labels or initialization artifacts, cultural selection is not demonstrated. |

## 11.6 Experiment 4 — retention of one socially acquired improvement

| Field | Design |
|---|---|
| **Hypothesis** | A learner can modify a socially acquired behavior or artifact, improve organism-relevant performance, and transmit the improved variant to later learners. |
| **Mechanism under test** | Innovation on a cultural scaffold plus retransmission. |
| **Independent variables** | Social access to baseline variant; exploration rate; memory; artifact persistence; magnitude and observability of improvement. |
| **Dependent variables** | Improvement size; innovation survival probability; descendant adoption; post-inventor prevalence; causal dependency on baseline variant; lineage branch count. |
| **Controls/ablations** | Same-compute asocial learners with no baseline demonstration; baseline copying with exploration disabled; improved inventor isolated; artifact reset; genotype frozen. |
| **Seeds** | 50 per primary condition because successful innovation and transmission are joint rare events. |
| **Success threshold** | At least one improvement appears and survives the inventor in a prespecified fraction of seeds—recommended ≥40% for mechanism validation—and the socially scaffolded condition produces more retained improvements than same-compute asocial controls. Descendants must retain a measurable performance advantage after the innovator is gone. |
| **Failure interpretation** | Innovations without spread indicate transmission bottleneck; spread without retained performance indicates neutral tradition; performance improvement in asocial controls indicates independent rediscovery; no innovation indicates affordance or exploration bottleneck. |
| **Confounds** | Researcher identifying “better” variants after seeing results, hidden resource changes, longer action sequences receiving more opportunity, and genetic adaptation. |
| **Expected burden** | Moderate to high; requires automated lineage and performance analysis. |
| **Falsification** | If improved descendants do not outperform baseline descendants or cannot be causally linked to social access, the first ratchet step is not established. |

## 11.7 Experiment 5 — two or more causally dependent improvements

| Field | Design |
|---|---|
| **Hypothesis** | Multiple improvements can accumulate because each later improvement retains and depends on earlier culturally transmitted changes. |
| **Mechanism under test** | Ratcheting, dependency depth, error correction, and exploration on inherited scaffolds. |
| **Independent variables** | Transmission fidelity; learner exploration; memory capacity; number/diversity of models; artifact persistence; task fragility; social-network topology. |
| **Dependent variables** | Number of successive retained improvements; performance slope across turnover intervals; dependency depth; cultural lineage depth; component-removal effect; final performance relative to same-compute asocial controls. |
| **Controls/ablations** | No observation; no persistence; no exploration; perfect-copy/no-modification; random predecessor products; component removal from late variants; lineage-shuffled counterfactual; direct weight-copy upper bound. |
| **Seeds** | At least 50 per principal treatment; rare success may require 100+ lightweight seeds before a 30-seed confirmatory replication at chosen parameters. |
| **Success threshold** | Positive retained improvement on at least **four successive turnover intervals**, final performance above the **99th percentile** of same-compute asocial controls, and intervention evidence for at least **two nested dependencies**: removing an earlier component or resetting its artifact history destroys part of the late advantage. Success should occur in at least 30–50% of confirmatory seeds, with the exact criterion preregistered from pilots. |
| **Failure interpretation** | One improvement only suggests a second-order fidelity or affordance barrier. Performance increase without nested dependency suggests parallel independent innovation. Perfect-copy success but sensory-copy failure localizes the transmission bottleneck. |
| **Confounds** | A finite authored sequence hidden in the assay, scalar reward shaping, environmental drift, genetic change, and survivor bias from excluding failed worlds. |
| **Expected burden** | High; long runs, many seeds, causal artifact interventions, and lineage reconstruction. |
| **Falsification** | The hypothesis is falsified for the tested mechanism if late performance is reproducible by naïve asocial agents with equal compute or survives removal of supposed historical prerequisites. |

**Interpretation boundary.** Passing this experiment demonstrates bounded cumulative culture in the assay domain. It does not establish open-ended technology.

## 11.8 Experiment 6 — cultural divergence between groups

| Field | Design |
|---|---|
| **Hypothesis** | Partial connectivity permits within-group convergence, between-group divergence, and occasional spread or recombination of cultural variants. |
| **Mechanism under test** | Social-network topology, migration, conformity, and local ecological adaptation. |
| **Independent variables** | Migration rate; bridge frequency; group size; network modularity; environmental similarity; payoff neutrality versus local adaptation. |
| **Dependent variables** | Within-group behavioral similarity; between-group Jensen–Shannon divergence; F_ST-like cultural differentiation; number and duration of group-specific variants; migration-associated transfer; recombination events; skill loss. |
| **Controls/ablations** | Fully mixed population; completely isolated groups; random network rewiring; common ecology; genetic freeze; group labels hidden because groups are inferred offline. |
| **Seeds** | 30–50 worlds per topology/migration band; multiple groups within each world are not independent seed replacements. |
| **Success threshold** | Between-group divergence exceeds within-group divergence and shuffled-network nulls for at least three complete turnovers. Oppositely seeded groups retain their variants under low migration, exchange under intermediate migration, and homogenize under high migration. |
| **Failure interpretation** | No divergence may reflect one overwhelmingly superior variant, too much mixing, weak copying, or insufficient duration. Permanent isolation without exchange tests tradition, not cultural recombination. |
| **Confounds** | Genetic drift, unequal resources, geography changing action costs, founder effects, and analyst-defined groups. |
| **Expected burden** | High population and storage cost; network analysis substantial. |
| **Falsification** | If behavioral clustering disappears after controlling genotype, ecology, and contact or matches shuffled exposure, group traditions are not established. |

## 11.9 Experiment 7 — artifact-mediated learning after maker absence

| Field | Design |
|---|---|
| **Hypothesis** | Persistent products of behavior transmit useful information to organisms that never observed the maker. |
| **Mechanism under test** | Product emulation, wear/geometry cues, delayed social transmission, and externalized information. |
| **Independent variables** | Intact artifact; geometrically scrambled artifact; resource-matched reset; live demonstration; end-state only; decay level; maker identity/history hidden. |
| **Dependent variables** | Acquisition rate among artifact-only learners; performance; reconstruction fidelity; artifact reuse; time to first successful use; behavior after artifact removal; lineage links through artifacts. |
| **Controls/ablations** | Naturally occurring equivalent geometry not made by organisms; artifact with provenance erased but geometry intact; live maker with object hidden; resource location held constant; plasticity disabled. |
| **Seeds** | 30 per condition if artifact effects are common; 50 if acquisition is sparse. |
| **Success threshold** | Artifact-only naïve cohorts outperform reset and scrambled controls, and the advantage disappears when the informative feature is removed. At least one later cohort should reconstruct or reproduce the behavior after all knowledgeable organisms are absent. |
| **Failure interpretation** | Intact-artifact advantage without later reconstruction may be opportunistic reuse, not transmitted manufacture. No advantage means artifacts lack recoverable information or learners lack emulation capacity. |
| **Confounds** | Artifact directly changes resource access without learning, residual odors/signals, unbalanced geometry, and genotype differences. |
| **Expected burden** | Moderate physics cost plus controlled world cloning. |
| **Falsification** | If intact and history-matched natural configurations have identical effects and no learned behavior persists after removal, artifact-mediated cultural learning is not supported. |

## 11.10 Experiment 8 — structures as external memory

| Field | Design |
|---|---|
| **Hypothesis** | A persistent structure stores a functional constraint or partial solution that allows later organisms to continue a behavioral lineage beyond the knowledge of any living individual. |
| **Mechanism under test** | Stigmergy plus learned interpretation, repair, incremental construction, and niche construction. |
| **Independent variables** | Persistence duration; partial versus complete structure; structural damage; maker/knowledgeable population removal; construction material abundance; observation channel. |
| **Dependent variables** | Non-maker reuse; repair probability; addition of functional modifications; structure lifetime; generations served; performance drop after reset; dependency depth; culturally caused niche modification. |
| **Controls/ablations** | Genetically fixed builder with plasticity disabled; structure reset each cohort; randomized block arrangement; externally placed matched structure; no social observation but artifact persistence; no persistence but live teachers. |
| **Seeds** | 50 for modification claims; 30 may suffice for simple reuse. |
| **Success threshold** | The structure benefits at least three cohorts after all original builders are gone; naïve organisms repair or extend it more often than matched random structures; removing an inherited structural component reduces later function; learned-state reset does not erase the benefit while artifact reset does. |
| **Failure interpretation** | Reuse without repair indicates a durable niche but not cumulative construction. Genetically fixed rebuilding indicates extended phenotype, not culture. |
| **Confounds** | Structure directly funnels organisms without learning, block placement bias, analysis labeling visual complexity as function, and persistence rules freezing accidental arrangements forever. |
| **Expected burden** | High physics, world-state, and provenance storage cost. |
| **Falsification** | If late organisms perform equally after structure reset or behavior survives social and learned-state removal through genotype alone, external cultural memory is not demonstrated. |

## 11.11 Experiment 9 — evolvable signals and emergent teaching

| Field | Design |
|---|---|
| **Hypothesis** | When difficult behavior is observation-limited, costly signals, tolerance, or contingent scaffolding can evolve or be learned because they increase transmission to socially relevant recipients. |
| **Mechanism under test** | Sender–receiver coadaptation, learned semantics, attention direction, tolerance, opportunity provisioning, and functional teaching. |
| **Independent variables** | Signal availability and cost; relatedness; repeated interaction; learner need; task opacity; demonstrator visibility; recipient response; ecological benefit distribution. |
| **Dependent variables** | Mutual information between signal and context; receiver behavior change; learned versus genetic signal mapping; demonstrator cost; contingent behavior change in learner presence; learner acquisition speed; population fitness effects; deception frequency. |
| **Controls/ablations** | Signal remapping; sender identity shuffle; playback without contingent response; learner hidden from demonstrator; genetically frozen sender/receiver; no kin assortment; no learner benefit; no signal cost. |
| **Seeds** | 50–100 evolutionary seeds because communication equilibria are multimodal and path dependent. |
| **Success threshold** | Signals or scaffolding must causally accelerate learning, mappings must be acquired within life or vary culturally between groups, and demonstrators must meet functional teaching criteria when teaching is claimed. Learned meanings should transfer to naïve organisms through exposure and fail under remapping. |
| **Failure interpretation** | Genetically evolved immediate coordination is communication but not cultural transmission. Signal use without learner retention is not teaching. No signal equilibrium may reflect weak shared interest or high deception incentives. |
| **Confounds** | True-fitness information leakage, fixed message semantics, automatic kin recognition, group reward, and experimenter-defined teacher roles. |
| **Expected burden** | Very high evolutionary seed count and long social-history analysis. |
| **Falsification** | If behavior is unchanged by contingent learner presence or signal mappings remain entirely genetic, the cultural-teaching hypothesis is false for the tested setting. |

## 11.12 Experiment 10 — weakly specified open ecology

| Field | Design |
|---|---|
| **Hypothesis** | Once basic transmission, retention, artifacts, and bounded ratcheting are established, a heterogeneous persistent ecology can sustain continued emergence of new cultural dependencies beyond the original assay domain. |
| **Mechanism under test** | Interaction among generic affordances, cultural inheritance, niche construction, diversification, recombination, and ecological feedback. |
| **Independent variables** | Artifact persistence; material diversity; spatial heterogeneity; population/network structure; social-learning channel; innovation rate; environmental stationarity; signal availability. |
| **Dependent variables** | Rate of new retained functional variants; dependency-depth growth; number of independent cultural lineages; diversification/recombination/exaptation events; new resource transformations; artifact reuse; plateau time; social and artifact causal dependence; expansion into held-out affordance probes. |
| **Controls/ablations** | No social learning; no lifetime plasticity; no persistent artifacts; genetic freeze; world reset; same-compute isolated populations; novelty-only optimizer; fixed-recipe benchmark; direct weight-copy upper bound. |
| **Seeds** | At least 20–30 long independent worlds for any general emergence claim, preceded by cheaper 50–100-seed shortened screens. One exceptional run may be a case study only. |
| **Success threshold** | No single scalar threshold is sufficient. A predeclared composite should require: continued creation and survival of functionally novel lineages after the initial assay optimum; increasing dependency depth over multiple windows; causal loss under social/artifact ablation; repeated success across seeds; and no evidence that a fixed finite sequence or metric exploit explains the pattern. |
| **Failure interpretation** | Plateau after bounded ratchets suggests missing affordance classes, recombination, population structure, or selection dynamics. Visual complexity without causal cultural dependence is a negative cultural result. |
| **Confounds** | Analyst hindsight, moving goalposts, metric gaming, genetic complexity, physics bugs, ever-growing garbage counted as artifacts, and environmental curriculum hidden in resource generation. |
| **Expected burden** | Extreme. Long horizons, large populations, deterministic physics, complete checkpoints, many seeds, and offline causal analysis dominate. |
| **Falsification** | The open-ended hypothesis is falsified for the tested configuration if all lineages plateau, new “innovations” are independently rediscoverable by asocial controls, dependency depth stops growing, or cultural ablations do not reduce late capabilities. |

## 11.13 Dependency graph

```text
Determinism + symmetric assay
        |
        v
Causal diffusion of arbitrary variants
        |
        v
Persistence after inventor death and turnover
        |
        +--------------------+
        |                    |
        v                    v
Variant competition     Artifact-mediated learning
        |                    |
        v                    v
One retained improvement  External memory/reuse
        |                    |
        +----------+---------+
                   v
Two or more dependent improvements
                   |
          +--------+---------+
          |                  |
          v                  v
Group divergence      Signals/tolerance/teaching
          |                  |
          +--------+---------+
                   v
        Weakly specified open ecology
```

## 11.14 Stopping rules

Stop and diagnose rather than adding complexity when:

- experiment 1 fails: do not add artifacts or language; fix perception/plasticity;
- experiment 2 fails: do not call a spread behavior a tradition;
- experiment 4 fails: do not infer ratcheting from complexity curves;
- experiment 5 fails: do not proceed to an open-world “civilization” run expecting technology;
- artifact tests fail: do not increase material variety until recoverable information is demonstrated;
- genetic controls fail: separate assimilation from cultural inheritance before continuing;
- determinism fails: no statistical conclusion is trustworthy until restoration and event order are repaired.


---

# 12. Metrics and ablations

## 12.1 Measurement principles

A cultural claim should be supported by at least three independent measurement layers:

1. **Behavioral layer:** what organisms do and how performance changes.
2. **Transmission layer:** who or what causally influenced acquisition.
3. **Inheritance-source layer:** whether the effect is genetic, individually learned, socially learned, or artifact mediated.

No single complexity score can substitute for these layers.

## 12.2 Transmission fidelity

Fidelity should be decomposed rather than summarized immediately.

### Action-form fidelity

For an observed demonstrator sequence `D` and learner sequence `L`, use metrics appropriate to representation:

- normalized edit distance for discrete contact/action events;
- dynamic time warping for trajectories;
- distributional similarity of state-conditioned action choices;
- contact-order agreement;
- orientation- and morphology-normalized pose similarity.

A possible normalized score is:

\[
F_{action}=1-\frac{\operatorname{EditDistance}(D,L)}{\max(|D|,|L|)}
\]

The score must be compared with no-model and time-shuffled null distributions. A high raw score can arise because the environment permits only one action.

### Outcome fidelity

Measure similarity of post-action object and world states:

\[
F_{outcome}=1-d(S^{D}_{post},S^{L}_{post})
\]

where `d` is a normalized physical-state distance over predeclared features such as object position, orientation, integrity, containment, accessibility, or resource exposure.

### Functional fidelity

Measure preservation of relevant function across transmission:

\[
F_{function}=\frac{P_L-P_{null}}{P_D-P_{null}}
\]

where `P_D` is demonstrator performance, `P_L` learner performance, and `P_null` a matched naïve baseline. Values above 1 are possible if the learner improves the variant.

### Causal fidelity

A learner's behavior should vary with the demonstrator variant under a two-action intervention. Define:

\[
F_{causal}=P(L=A\mid D=A)-P(L=A\mid D=B)
\]

This controls for generic attraction to the task. Report it with seed-level intervals.

## 12.3 Persistence beyond individual lifetimes

Recommended measures:

- **post-inventor prevalence:** variant prevalence after inventor death;
- **cohort survival count:** number of complete cohort turnovers with prevalence above threshold;
- **cultural half-life:** time until prevalence falls to half its post-diffusion level;
- **extinction hazard:** seed-level hazard that a lineage disappears;
- **carrier-independent duration:** duration excluding the original inventor and initial trained demonstrators.

A behavior that remains because one very long-lived carrier survives is not intergenerational persistence.

## 12.4 Cultural lineage reconstruction

Represent cultural history as a directed acyclic multigraph when possible:

- organism nodes;
- artifact-version nodes;
- observation edges;
- direct interaction or object-transfer edges;
- artifact encounter edges;
- innovation/modification edges;
- genetic parentage edges stored separately.

An adoption edge should receive a causal confidence based on:

- temporal precedence;
- actual perceptual exposure;
- variant match;
- absence or rate of independent discovery;
- intervention evidence from matched worlds;
- genotype and ecology controls.

Core metrics:

- longest cultural path;
- median path length to current carriers;
- branch factor;
- lineage survival time;
- number of independent origins;
- recombination count where one descendant depends on two prior lineages;
- artifact-mediated edge fraction;
- causal-confidence-weighted lineage depth.

Validate lineage metrics against shuffled exposure graphs and simulations with known injected causal structure.

## 12.5 Cultural reproductive number

For variant or artifact lineage `i`:

\[
R_{c,i}=\frac{\text{number of new independent carriers or descendant artifacts causally attributable to }i}{\text{number of active carriers or artifacts of }i}
\]

Estimate separately for:

- live observation;
- direct object transfer;
- artifact-only exposure;
- signals;
- teaching-like interactions.

`R_c > 1` indicates expected spread under current conditions. It does not indicate adaptive improvement.

## 12.6 Innovation survival

An innovation is a behavior or artifact modification outside a predeclared similarity radius of the parent's form. Novelty definition should be based on raw behavior or physical state, not a named technology ontology.

Report:

- invention rate per organism-time;
- proportion expressed more than once;
- proportion adopted by another organism;
- proportion surviving inventor death;
- survival function by functional effect;
- cultural reproductive number;
- descendants produced;
- time to extinction or fixation;
- probability of later modification.

The key ratio is not total novelty but **retained useful novelty**:

\[
I_{survival}=\frac{\#\text{innovations with positive function and descendants after inventor death}}{\#\text{all candidate innovations}}
\]

## 12.7 Improvement accumulation

Use multiple measures:

### Sequential improvement count

Count non-overlapping intervals in which:

1. a new variant appears;
2. it is socially transmitted;
3. descendants outperform their predecessor under matched conditions;
4. the advantage survives the innovator.

### Cultural performance slope

Fit performance against cultural generation or turnover interval, not organism age alone. Report nonlinear plateaus and seed heterogeneity.

### Historical dependency depth

For a late variant, remove or restore earlier components and estimate:

\[
D_{history}=P_{late}-P_{late\ without\ prerequisite}
\]

A genuine ratchet should have positive dependency depth for multiple nested prerequisites.

### Asocial-excess performance

\[
A_{social}=P_{social}-Q_{0.99}(P_{asocial,same\ compute})
\]

Using the 99th percentile is intentionally demanding for rare-search domains. Also report mean and distributional effects; do not hide overlap.

## 12.8 Between-group divergence and within-group similarity

For discrete variant distributions, use Jensen–Shannon divergence between groups and mean divergence within resampled subgroups. For continuous behavior embeddings, use variance partitioning or energy distance.

A cultural differentiation index analogous to `F_ST` can be defined as:

\[
C_{ST}=\frac{V_{between}}{V_{between}+V_{within}}
\]

This index is descriptive. Cultural causation requires common-garden, genetic, ecological, and contact-network controls.

Report:

- within-group similarity;
- between-group divergence;
- persistence duration;
- migrant assimilation rate;
- migrant export rate;
- divergence by neutral versus adaptive trait;
- topology-conditioned diffusion.

## 12.9 Artifact reuse and external memory

Recommended metrics:

- fraction of artifact interactions by non-makers;
- number of distinct organisms and cohorts using an artifact;
- uses after maker death;
- repair events by non-makers;
- functional modifications by later cohorts;
- artifact lineage depth;
- time saved or risk reduced relative to reset world;
- reconstruction after original artifact removal;
- proportion of cultural edges mediated by artifacts;
- performance loss after artifact reset.

Define an external-memory effect:

\[
M_{external}=P_{intact\ artifact}-P_{history\ matched\ reset}
\]

To distinguish direct physical assistance from learning, remove the artifact after exposure and test whether behavior persists.

## 12.10 Dependency on social observation

Estimate a causal treatment effect using matched seeds or cloned checkpoints:

\[
D_{social}=P_{full}-P_{observation\ blocked}
\]

Useful intervention variants:

- block all conspecific perception;
- preserve location but mask action;
- preserve action but mask outcome;
- preserve outcome but use ghost motion;
- time-shuffle demonstrations;
- replace demonstrator with a behaviorally unrelated organism;
- allow resource disturbance but no visual access.

A behavior is socially dependent when these interventions reduce acquisition or lineage survival while holding opportunities and physical state constant.

## 12.11 Dependency on persistent artifacts

Estimate:

\[
D_{artifact}=P_{persistent}-P_{reset}
\]

Use checkpoint cloning so populations and random streams are identical until the intervention. Additional controls include scrambled geometry, naturally generated matched structures, and decay manipulation.

A culture may depend jointly on live observation and artifacts. Use a 2×2 factorial design:

| | Artifacts absent/reset | Artifacts persistent |
|---|---:|---:|
| **Social observation blocked** | Pure asocial baseline | Artifact-only transmission |
| **Social observation available** | Live-only transmission | Full cultural ecology |

## 12.12 Genetic versus cultural inheritance

No single control is sufficient. Use a package:

### Genetic freeze

Prevent mutation/recombination while retaining births and cultural turnover. Continued divergence or accumulation under freeze supports non-genetic inheritance.

### Common-garden rearing

Raise genotypes from different cultural groups without their native demonstrators or artifacts in a common environment. Persistent differences indicate genetic or prenatal effects.

### Cross-fostering

Place naïve offspring or cloned genotypes into another group's social and artifact environment. Adoption of host variants supports cultural transmission.

### Ancestral-genotype replay

Instantiate an archived early genotype in a later cultural environment. If it acquires late behavior through observation/artifacts, genetic change is not sufficient to explain the behavior.

### Learned-state reset

Retain genotype and body but reset lifetime learning. Loss of behavior identifies ontogenetic dependence; reacquisition from group or artifacts identifies culture.

### Variance decomposition

Model performance as:

\[
P \sim G + E + C + A + G\times C + C\times A + \text{seed}
\]

where `G` is genotype, `E` current ecology, `C` social exposure/cultural group, and `A` inherited artifact state. This is a statistical decomposition, not a metaphysical partition; interactions may dominate.

## 12.13 Cultural complexity not explained solely by genetic change

A late capability has strong cultural evidence when all of the following hold:

1. it disappears or degrades when cultural access is removed without changing genotype;
2. it transfers to a naïve or ancestral genotype through observation or artifacts;
3. its lineage includes multiple learned carriers or artifact versions;
4. common-garden controls reduce group differences;
5. cultural or artifact variables explain residual performance after genotype and ecology;
6. late functionality depends on historical components unavailable to isolated same-compute learners.

## 12.14 Novelty, complexity, progress, and open-endedness

Measure these separately:

- **Novelty:** distance from prior behavior or artifact states.
- **Diversity:** contemporaneous spread of forms.
- **Complexity:** description length, dependency depth, number of functional components, or interaction order.
- **Performance:** organism-relevant consequence.
- **Cumulative progress:** repeated socially transmitted performance improvement.
- **Open-endedness:** continued creation of adaptive novelty and new dependencies without rapid saturation.

A practical open-endedness dashboard should include:

- adaptive innovation rate over rolling windows;
- lineage birth and extinction rates;
- dependency-depth trend;
- diversification and recombination rate;
- number of distinct resource transformations;
- proportion of late capabilities lost under cultural ablation;
- plateau diagnostics;
- metric robustness under alternative embeddings and classifiers.

No one curve proves open-endedness.

## 12.15 Core ablation matrix

| Ablation | Isolates | Interpretation if performance collapses |
|---|---|---|
| No social perception | All live social learning | Live observation is causally necessary |
| Location-only perception | Local enhancement | Location cues explain some or all diffusion |
| Object-only perception | Stimulus enhancement | Object salience is sufficient |
| Action masked, outcome visible | Emulation/outcome learning | Outcome channel is sufficient |
| Outcome masked, action visible | Action-form learning | Motor pattern carries essential information |
| Ghost object motion | Actor-specific versus object-motion learning | Actor presence is not necessary if ghost succeeds |
| End-state only | Product reconstruction | Final product contains enough information |
| Demonstrator time shuffle | Temporal contingency | Sequence timing matters |
| Demonstrator identity shuffle | Model-specific trust/prestige | Individual identity contributes |
| Plasticity disabled | Within-lifetime learning | Effect is learned rather than fixed |
| Memory disabled/shortened | Retention and sequence binding | Memory is limiting |
| Exploration disabled | Innovation | Copying alone cannot improve |
| Social-only/no own reward | Individual reinforcement | Own evaluation is required to refine copied behavior |
| Artifact reset | Environmental inheritance | Persistent state carries culture |
| Artifact scrambled | Specific artifact information | Geometry/history, not mere object presence, matters |
| Maker removed | Delayed product learning | Live demonstration is unnecessary if effect survives |
| Genetic freeze | Genetic adaptation | Culture can operate without genetic change |
| Common garden | Genetic/ecological confounds | Native social environment explains group difference |
| Cross-foster | Host cultural influence | Variants transfer culturally |
| Continuous versus synchronized turnover | Lifecycle artifact | Result depends on researcher-defined generations |
| Fully mixed versus modular network | Topology | Partial structure preserves variants or enables ratchet |
| Direct weight copy | Transmission upper bound | Quantifies bottleneck but is not baseline culture |
| Analytics disabled | Observer effect | Any difference indicates invalid feedback or timing interference |

---

# 13. Determinism and reproducibility implications

## 13.1 Determinism is compatible with evolution and culture

A deterministic simulation can still contain mutation, exploration, stochastic mortality, and historical contingency if all pseudorandom choices are deterministic functions of an explicit seed and event key. Multiple seeds sample possible histories; determinism guarantees that each history can be exactly reproduced.

## 13.2 Counter-based random streams

Use order-independent random draws keyed by fields such as:

```text
random(seed, subsystem, tick, entity_id, event_id, draw_index)
```

Separate streams should exist for:

- genetic mutation and recombination;
- action exploration;
- attention ties;
- reproduction and mate choice;
- mortality and environmental events;
- object breakage if probabilistic;
- experiment interventions.

Do not consume one global stream in entity-iteration order. Adding a logger or changing thread partition must not alter later random draws.

## 13.3 Snapshot–intent–resolve–commit semantics

All organisms decide from the same snapshot. Actions are intents, not immediate writes. This prevents an organism processed earlier in an array from becoming an accidental demonstrator within the same tick.

Recommended rules:

- observation at `t` uses committed state `S_t`;
- intents are immutable after submission;
- movement/contact resolution is independent of iteration order;
- learning uses transitions from `S_t` to committed `S_{t+1}`;
- plastic updates affect decisions no earlier than `t+1`;
- signals emitted at `t` are perceived at `t+1` or later;
- object modifications become observationally available after commit;
- no agent reads another agent's pending action.

## 13.4 Deterministic conflict resolution

Prefer commutative physical aggregation where physically appropriate. Where exclusive access is required, use a documented stable rule that avoids permanent entity-ID bias.

Options include:

- sum and clamp simultaneous impulses in fixed-point arithmetic;
- deterministic constraint solving with sorted contact manifolds;
- stable priority hash `H(seed, tick, location, contender_id)` for contested pickup or occupancy;
- fair rotating priority derived from tick and location;
- simultaneous failure when mutually exclusive intents are exactly tied.

The choice is part of the authored physics and can shape social behavior. It must be versioned and ablated when conflict or ownership is studied.

## 13.5 Object transformation and artifacts

Persistent artifacts create difficult deterministic cases:

- two organisms striking the same object;
- simultaneous attachment and separation;
- one organism carrying while another pulls;
- multiple placements in one support region;
- breakage at threshold;
- decay and repair in the same tick.

Define a strict phase order based on physics, not entity order. For example:

1. aggregate forces;
2. solve movement and contacts;
3. calculate stress and breakage;
4. resolve attachments/transfers;
5. apply wear and decay;
6. calculate resource access;
7. emit committed event records.

A different phase order can change which traditions are viable, so engine versions must be treated as different experimental environments.

## 13.6 Learning-update determinism

Plasticity can introduce hidden divergence if:

- event lists arrive in nondeterministic order;
- floating-point reductions differ across hardware;
- GPU kernels use nondeterministic atomics;
- recurrent state is not checkpointed;
- optimizer moments or eligibility traces are omitted;
- clipping and saturation order differ.

Use fixed-point arithmetic in the authoritative learner, canonical event ordering, and explicit saturation behavior. If GPU acceleration is used, it should produce validated bit-identical outputs or be restricted to offline analysis/non-authoritative inference.

## 13.7 Stable event and lineage IDs

A cultural provenance system needs IDs stable across restoration and thread layouts. Derive event IDs from committed sequence and causal identity, not memory addresses or write timing.

Example:

```text
EventID = hash(engine_version, world_seed, tick, phase, location_key,
               primary_entity, secondary_entity, local_event_ordinal)
```

`local_event_ordinal` must itself be generated from a canonical sorted event list.

## 13.8 Checkpoint atomicity

Checkpoint only at a documented phase boundary, preferably after commit and learning update. If mid-tick checkpoints are required, persist all:

- snapshot state;
- pending intents;
- conflict-resolution queues;
- unresolved contacts;
- learning-event buffers;
- signal propagation buffers;
- lifecycle queues;
- random draw counters/keys;
- provenance sequence state.

Atomic checkpoint creation should use write-to-new-file, fsync, integrity hash, and atomic rename. Restoration validation should run a continuation and compare event hashes, not only deserialize successfully.

## 13.9 Versioned persistence

A cultural run is scientifically reproducible only with:

- kernel version and commit;
- fixed-point format and saturation policy;
- physics/material schema version;
- controller and plasticity schema;
- PRNG algorithm and stream-key specification;
- conflict-resolution semantics;
- experiment configuration;
- initial state and seed;
- checkpoint hash chain;
- offline analytics version;
- classification thresholds and embedding model, where used.

Migration code between persistence versions should never silently reinterpret learned or artifact state. Either provide a deterministic verified migration or mark runs as belonging to different environments.

## 13.10 Observer protocol implications

The binary observer protocol should expose enough raw state to reconstruct behavior while preserving a strict read-only boundary. Recommended additions include:

- object contact and transformation events;
- social line-of-sight and attention events;
- learned-state summaries or hashes, with full state available only in research checkpoints;
- artifact version and physical-state deltas;
- action intents and resolved actions as separate records;
- provenance edges;
- genetic parentage and genotype hashes;
- checkpoint and state hashes.

Offline classifiers can evolve independently. A new classifier may relabel an old run without changing the run itself.

## 13.11 Reproducibility tests specific to culture

Beyond ordinary state restoration, run:

1. **Diffusion replay:** restore immediately before a key observation and verify identical adoption history.
2. **Artifact replay:** restore before a contested modification and verify identical final geometry and later use.
3. **Lineage replay:** restore from multiple ancestral checkpoints and verify the same cultural DAG.
4. **Thread-layout replay:** run with different worker partitions and verify identical hashes.
5. **Observer-off replay:** disable observer output and verify the same authoritative state.
6. **Long-horizon drift test:** compare periodic hashes over millions of ticks.
7. **Cross-machine conformance:** run authoritative fixtures on supported CPU architectures and operating systems.

---

# 14. Compute and scaling implications

## 14.1 Primary cost drivers

Cultural evolution is expensive less because of any single neural update than because it requires:

- long lifetimes and many turnover events;
- enough organisms for network structure and rare innovators;
- many independent seeds;
- persistent artifact physics;
- rich observation streams;
- complete learned-state checkpoints;
- causal provenance and counterfactual cloned runs;
- parameter sweeps across fidelity, memory, topology, and migration.

## 14.2 Asymptotic considerations

Let:

- `N` = number of organisms;
- `k` = mean number of locally relevant organisms/objects after spatial indexing;
- `P` = plastic parameter count per organism;
- `L` = retained event-memory length;
- `d` = event embedding size;
- `A` = active dynamic artifacts;
- `c` = mean contacts per active artifact;
- `E` = logged high-value events per tick.

Approximate authoritative costs are:

| Component | Time | State/storage |
|---|---:|---:|
| Local sensing/social observation | `O(Nk)` | transient `O(Nk)` or streamed |
| Controller inference | `O(N × controller_cost)` | recurrent state per organism |
| Plasticity | `O(NP)` in dense form; lower if sparse | `O(NP)` |
| Event memory | retrieval `O(NL)` worst case | `O(NLd)` |
| Artifact contacts | roughly `O(Ac)` after broad phase | material and constraint state `O(A)` |
| Social graph/provenance | kernel emission `O(E)` | cumulative `O(total E)` without compression |
| Checkpoint | proportional to all authoritative state | full snapshot plus integrity metadata |

Without spatial indexing, social and object interactions can approach `O(N²)` and become prohibitive.

## 14.3 Persistent artifact growth

Artifacts can accumulate indefinitely. A naive persistent world will eventually spend more compute maintaining abandoned debris than living ecology.

Safe controls include:

- physically grounded decay;
- material recycling by organisms;
- deterministic sleeping for static objects;
- merging only when physical equivalence is exact and provenance remains recoverable;
- bounded spatial regions or inactive-region serialization;
- level of detail for distant inert objects that preserves collision and affordance equivalence;
- garbage collection only for states proven unreachable and with a versioned deterministic rule.

Because decay changes cultural memory, it is an experimental variable, not merely an optimization. Any optimization that deletes or coarsens objects must be tested for behavioral equivalence.

## 14.4 Provenance storage

Logging every visible interaction can dwarf simulation state. Use tiers:

1. **Mandatory exact events:** births, deaths, reproduction, actions affecting artifacts/resources, object transfers, learned-variant expression, experiment interventions, and checkpoint hashes.
2. **Conditionally exact events:** high-attention social observations and signal events.
3. **Aggregated events:** low-resolution proximity without attention or behavioral consequence.
4. **Offline derived records:** cultural labels, embeddings, lineages, and traditions.

Maintain cryptographic or strong deterministic hash chains so sampled/aggregated storage can still be tied to authoritative event order.

## 14.5 Deterministic parallelism

Parallelism is possible if work is partitioned into order-independent phases:

- spatial sensing partitions read one snapshot;
- controllers compute intents independently;
- contacts are bucketed and sorted canonically;
- commutative reductions use fixed-point arithmetic;
- exclusive conflicts use stable hashed priority;
- learning events are sorted before updates;
- logs are merged by event ID rather than completion time.

Avoid nondeterministic GPU training kernels in the authoritative simulation. GPU or approximate inference can be used in offline analytics so long as classifications never affect behavior and the analytics version is recorded.

## 14.6 Experimental scaling plan

| Stage | Typical scope | Purpose | Relative burden |
|---|---|---|---|
| **Unit fixtures** | 1–4 organisms, 1–10 objects, hundreds of ticks | Determinism, perception, contact, plasticity | Very low |
| **Diffusion assay** | 10–50 organisms, simple objects, short lifetimes | Establish causal social learning | Low |
| **Turnover assay** | 20–100 organisms, several lifecycles | Establish traditions | Moderate |
| **Ratchet assay** | 20–100 organisms, modifiable artifacts, many seeds | Establish one and multiple improvements | High |
| **Island/network assay** | 4–20 groups, 100–1,000 organisms | Divergence, migration, recombination | High to very high |
| **Open ecology** | Hundreds to thousands of organisms, persistent world, very long horizon | Exploratory open-endedness | Extreme |

Do not spend open-ecology compute until the cheaper dependencies pass.

## 14.7 Seed count versus world size

A common mistake is to spend all compute on one enormous world. For mechanism discovery, 30 smaller independent worlds usually provide stronger evidence than one visually rich run. Large worlds become necessary when the hypothesized mechanism itself depends on rare contact, specialization, or multilevel network structure.

A practical allocation is:

- many small seeds for parameter screening;
- preregistered medium-scale confirmatory seeds;
- a smaller number of large worlds only after effect sizes and failure modes are known;
- checkpoint branching for causal interventions from identical histories.

## 14.8 Counterfactual cloning

Determinism makes matched counterfactuals unusually powerful. At a key checkpoint, clone the world and change one mechanism:

- observation blocked;
- artifact reset;
- inventor removed;
- genotype frozen;
- signal remapped;
- network bridge closed.

Because all earlier history is identical, divergence after intervention is a direct causal estimate. Storage can be controlled through copy-on-write snapshots and content-addressed artifact chunks.

## 14.9 Expected bottlenecks by research stage

| Stage | Likely scientific bottleneck | Likely computational bottleneck |
|---|---|---|
| Transmission | Attention and action correspondence | Social sensing/logging |
| Persistence | Overlap, memory, repeated expression | Long lifecycle simulation |
| First ratchet | Joint rarity of useful innovation and retransmission | Many seeds and lineage analysis |
| Artifact memory | Recoverable physical information | Deterministic contact physics and persistent state |
| Group divergence | Topology and migration balance | Population and network storage |
| Signals/teaching | Multimodal evolutionary equilibria | Very high seed count |
| Open-endedness | New affordance classes and diversity preservation | Long horizon, artifact growth, metric computation |


# 15. Claims the evidence does not support

The following claims should be treated as unsupported, overstated, or definitionally confused. Each has appeared in stronger or weaker form in discussions of artificial life, animal culture, cultural evolution, or multi-agent learning.

| Unsupported claim | Why the evidence is insufficient | Defensible formulation |
|---|---|---|
| **Any socially influenced behavior is a tradition.** | Local or stimulus enhancement can temporarily concentrate behavior without durable transmission, group stability, or persistence through turnover. | Social influence is evidence for a possible transmission pathway. A tradition additionally requires durable group-typical persistence that cannot be explained by ecology or genotype alone. |
| **Any tradition is cumulative culture.** | Traditions may remain unchanged, drift, or repeatedly reappear. Cumulative culture requires retention of modifications, and stronger claims require sequential functional dependence. | A tradition is a prerequisite substrate for accumulation, not proof of a ratchet. |
| **Imitation is synonymous with social learning.** | Social learning includes many processes that do not reproduce action form, including local enhancement, emulation, outcome copying, and socially mediated reinforcement. | Name the operational transmission pathway and measure what information crosses between organisms. |
| **Imitation is universally necessary for cumulative culture.** | Human laboratory chains can improve through product information, emulation, and trial-and-error; pigeon route improvements accumulate through social coupling rather than action-form imitation. These results concern bounded tasks. | Action-form imitation can raise fidelity for some behaviors but is not a universal logical requirement for every bounded ratchet. |
| **Imitation is sufficient for cumulative culture.** | High-fidelity copying without useful innovation, retention, selection, or retransmission preserves stasis. | Imitation can provide a retention mechanism; accumulation also needs beneficial variation and repeated lineage continuity. |
| **Teaching is universally necessary.** | Teaching often raises efficiency and may be important for opaque or costly skills, but cumulative improvement has occurred experimentally without intentional teaching. | Teaching is a potentially powerful adaptation that should be allowed to evolve only if its costs and benefits can be selected. |
| **Language or symbols are universally necessary for the first ratchet.** | Bounded cumulative improvements have been demonstrated without language, and nonhuman evidence shows socially acquired above-baseline skills. | Symbolic communication may greatly expand fidelity, abstraction, coordination, and scope; it should not be treated as a prerequisite for initial accumulation. |
| **Learners must understand demonstrators' intentions.** | Product-copying and causally opaque human transmission experiments show that useful improvements can be preserved without full causal understanding. | Intention and causal understanding may improve selective copying, flexible recombination, and generalization, but functional retention can precede them. |
| **There is a universal numerical fidelity threshold for cumulative culture.** | The relevant threshold depends on task depth, mutation distribution, selection, redundancy, population structure, and how many errors are neutral or self-correcting. | Estimate an experiment-specific error threshold from the probability that a functional lineage survives long enough to receive further improvements. |
| **Larger populations always support more culture.** | Effective population size, connectivity, model diversity, migration, ecological stability, and transmission topology matter. Large well-mixed populations can converge, erase local variants, or support freeloading. | Population size is one component of cultural opportunity; test it jointly with connectivity and turnover. |
| **More connectivity is always better.** | Connectivity can rescue rare variants and combine innovations, but excessive migration can homogenize groups before local refinements mature. | Expect an intermediate, ecology-dependent connectivity regime rather than a monotonic effect. |
| **Copying neural weights is the artificial equivalent of culture.** | Weight copying transmits latent competencies, biases, and possibly task solutions without requiring observation, embodiment, correspondence, or reconstruction. It resembles engineered state inheritance more than ordinary social learning. | Weight transfer is a useful upper-bound or control treatment. It should not be the default empirical model of cultural transmission. |
| **Shared-policy multi-agent reinforcement learning demonstrates culture.** | Agents sharing one policy do not transmit learned variants among distinct individuals. Improvement is normally optimizer-mediated and persists because parameters are centrally retained. | Shared-policy systems can demonstrate collective strategy and autocurricula, not individual-level cultural inheritance without additional evidence. |
| **Emergent tool use demonstrates technological culture.** | Tool use can be rediscovered independently, genetically encoded, produced by a fixed policy, or optimized directly from reward. | Demonstrate historical dependence on social observation or artifacts and persistence through naïve replacement before calling tool use cultural. |
| **Construction demonstrates a settlement tradition.** | Stigmergic structures can result from fixed local rules, environmental gradients, or authored blueprints. | Construction becomes evidence of cultural tradition only when acquired variants spread socially and persist beyond makers under matched ecological conditions. |
| **Persistent objects automatically function as external memory.** | Persistence alone does not establish readable information. An artifact must alter later behavior in a history-dependent way, ideally after the maker and direct demonstrators are absent. | Call an object external memory only after artifact-reset and maker-removal tests show causal informational dependence. |
| **Increasing behavioral complexity is cumulative progress.** | Complexity may increase through drift, inefficiency, arms-race elaboration, or metric artifacts. A complex behavior can be less functional than a simple one. | Report complexity, novelty, and performance separately, and reconstruct whether later variants depend on retained predecessors. |
| **Novelty equals open-ended innovation.** | Endless minor variation inside a fixed behavioral class can score as novel without creating new affordance combinations, dependencies, or ecological roles. | Open-endedness requires sustained production of consequential novelty with expanding or non-saturating adaptive scope, not merely distance in a descriptor. |
| **A sequence of qualitative phases is a technology ladder.** | Phase labels are often analyst summaries of one optimization run. They do not establish cultural transmission, lineage continuity, or indefinite extensibility. | Treat phase narratives as hypotheses to test across seeds and interventions. |
| **An authored task can prove unscripted technological evolution.** | A finite target, reward, demonstrator, blueprint, or solution grammar can make improvement inevitable within a researcher-defined space. | Authored assays can establish component mechanisms. Claims about open-ended technology require weakly specified ecologies and explicit accounting of authored solution information. |
| **Genetic assimilation is evidence that culture succeeded.** | A behavior becoming genetically canalized can eliminate dependence on social transmission, reducing cultural flexibility and lineage visibility. | Genetic assimilation is a possible gene–culture outcome. Measure whether the behavior remains socially dependent. |
| **One spectacular run is evidence of robustness.** | Rare-event systems are highly vulnerable to seed selection, observer bias, and post hoc metrics. | Report seed distributions, null conditions, stopping rules, failed runs, and intervention-based causality. |
| **Determinism eliminates the need for multiple seeds.** | Determinism makes a seed reproducible; it does not make it representative. Different deterministic seeds still sample different initial conditions and event histories. | Use many preregistered seeds plus exact replay and matched checkpoint branches. |
| **Current animal evidence establishes human-like open-ended technology.** | Recent chimpanzee and bumblebee studies show acquisition of difficult or non-innovated behaviors through social information. They do not show an indefinitely extensible technological ecology. | Nonhuman evidence weakens categorical claims that all accumulation is uniquely human while leaving major differences in scope, recombination, and open-endedness. |
| **Organized inter-group conflict should emerge once culture exists.** | Territoriality and conflict depend on resource defensibility, coalition benefits, recognition, memory, risk, demography, and competition. Culture alone does not imply warfare. | Treat group conflict as one possible downstream outcome of ecology and social organization, not a milestone or objective. |
| **Open-ended technological evolution is solved.** | No reviewed artificial system jointly demonstrates embodied invention, learned social inheritance, persistent artifacts, cumulative functional dependence, expanding affordance use, long-run novelty, and robust replication without substantial authored scaffolding. | The component mechanisms are individually tractable; their integration into robust open-ended technological evolution remains an open research problem. |

## 15.1 Interpretive rule for Genesis results

A result should be described at the lowest evidential level it actually passes:

1. **Social influence** — exposure changes behavior.
2. **Social transmission** — an identifiable behavioral variant spreads through a contact pathway.
3. **Tradition** — the variant persists as a group-typical pattern through turnover.
4. **Cumulative change** — a descendant variant retains and modifies predecessor information.
5. **Cumulative functional improvement** — retained modifications improve a preregistered functional measure.
6. **Dependency-bearing ratchet** — later performance requires multiple predecessor-derived components and is inaccessible to matched naïve controls.
7. **Open-ended cultural evolution** — consequential novelty continues without evident saturation across expanding behavioral or artifact domains.

Passing one level does not imply the next. Results should not be promoted by visual resemblance, analyst naming, or organism count.

# 16. Open research questions

These questions remain unresolved enough that Genesis should treat them as experimental targets rather than assumptions.

## 16.1 Minimal cognitive architecture

1. **What is the minimal correspondence mechanism for copying an embodied action?** It is unknown how much body-model alignment is required when demonstrator and learner morphologies differ.
2. **Can outcome emulation support dependency-bearing technological ratchets without action-form imitation?** Bounded human tasks suggest it can support improvement, but the limits under opaque physics are unclear.
3. **How should attention be allocated when observation has an energetic or opportunity cost?** Fixed “observe” modes may create artificial teaching channels; fully endogenous attention may make demonstrations too rare.
4. **How much episodic memory is needed relative to action duration and lifecycle length?** There is no general scaling law connecting memory capacity to cultural depth.
5. **Can local reinforcement alone bind a delayed observed outcome to another organism's earlier action?** Credit assignment may be the limiting cognitive mechanism in physically extended tasks.
6. **What forms of abstraction are needed before behavior transfers across objects, locations, or morphologies?** Copying exact trajectories may produce brittle traditions rather than technology.
7. **When does selective overimitation help rather than waste effort?** Copying apparently irrelevant details can preserve hidden causal structure, but it also propagates inefficiency.

## 16.2 Transmission fidelity and innovation

8. **What error distributions produce useful variation rather than destructive decay?** Mean fidelity is insufficient; errors may be neutral, biased, correlated, or occasionally constructive.
9. **Does a culturally effective error threshold depend more on lineage depth or branching?** Redundant parallel learners may preserve information even with poor pairwise fidelity.
10. **How should innovation be identified without embedding a task-specific value judgment?** A change can be locally beneficial, globally harmful, or useful only after later recombination.
11. **Can innovation rates evolve toward a stable balance with social learning?** Theory predicts freeloading and public-good problems, but embodied ecologies may create private or kin-structured returns.
12. **What prevents a successful convention from suppressing exploration permanently?** Conformity stabilizes traditions while risking cultural lock-in.
13. **Can populations maintain multiple partially compatible techniques and recombine them?** Most models either converge to one variant or store skills in an external archive.

## 16.3 Artifacts, structures, and external memory

14. **Which physical properties make an artifact readable rather than merely persistent?** Shape, wear, orientation, residue, location, and material composition may carry different kinds of information.
15. **How much artifact decay maximizes innovation?** No decay can saturate storage and freeze obsolete structures; rapid decay destroys external memory.
16. **Can affordance discovery remain open when object operations are finite?** A finite verb set can still generate combinatorial depth, but it may eventually expose a hard ceiling.
17. **When does reuse become copying?** A learner may exploit an artifact without reconstructing or understanding its production method.
18. **Can structures encode procedural information through staged affordances?** For example, one component may constrain the order in which later components can be added without any explicit symbolic representation.
19. **How can maintenance traditions be distinguished from repeated local repair caused by the same physics?** Provenance and counterfactual object replacement are likely necessary.
20. **Can artifact lineages be reconstructed when objects merge, split, erode, and exchange parts?** A single-parent genealogy is inadequate for composite technologies.

## 16.4 Population structure and demography

21. **What overlap between learner and demonstrator lifetimes is sufficient for persistent traditions?** The answer likely depends on demonstration visibility and artifact persistence.
22. **Is there an optimal migration regime for local divergence plus recombination?** Theory and comparative evidence suggest non-monotonic effects, but the optimum is likely task-dependent.
23. **How important are weak ties versus stable close ties?** Stable ties raise repeated exposure; bridges introduce rare variants and prevent isolation.
24. **Can cumulative culture survive severe population bottlenecks if artifacts remain?** External state may partially substitute for living demonstrators.
25. **How does age structure affect invention, teaching, and conservatism?** Juvenile exploration and older demonstrator competence could create division of cognitive labor without scripted roles.
26. **When do prestige, payoff, conformity, kin, and frequency biases evolve rather than merely being imposed?** Most cultural-evolution models start with the bias as a rule.
27. **How does group fission preserve or destroy traditions?** Founder composition may be more important than group size.

## 16.5 Gene–culture coevolution

28. **Will selection favor general learning capacity, narrow instincts, or a mixed architecture?** Stable ecologies favor assimilation; changing cultural niches may favor plasticity.
29. **Can cultural lineages alter selection without collapsing into genetic encoding?** The key may be environmental variability, frequency dependence, and continual artifact change.
30. **How should inherited morphology and lifetime skill co-adapt?** A tool tradition can create selection for manipulators, perception, lifespan, or tolerance.
31. **Can genetically evolved attention biases bootstrap culture while preserving open-ended content?** A broad bias toward successful conspecific actions may be less scripting-prone than innate action templates.
32. **How can genetic and cultural hitchhiking be separated?** A cultural variant may spread because its carriers reproduce more, because learners copy it, or both.
33. **Can culture maintain maladaptive practices under frequency-dependent social incentives?** The existence and duration of such practices would test whether the system has cultural dynamics distinct from simple reward optimization.

## 16.6 Communication, teaching, and cooperation

34. **Under what ecology do signals acquire behavior-specific meanings without a named vocabulary?** Grounding requires recurrent coordination benefits and observable referents.
35. **Can signals scaffold attention before they encode propositions?** Alarm, recruitment, readiness, and location cues may precede compositional communication.
36. **When is teaching evolutionarily stable?** Benefits may return through kinship, reciprocity, group productivity, mate choice, or reputation; each creates different predictions.
37. **Can deceptive signaling coexist with cumulative traditions?** Deception may select for reliability assessment and social memory but can also destroy transmission.
38. **Does cumulative technology require role differentiation?** Some tasks may become feasible only through complementary expertise, yet specialization creates dependency and vulnerability.
39. **Can institutions emerge from repeated enforcement without explicit rules?** Territorial boundaries, access conventions, and punishment could be traditions if socially acquired and historically contingent.

## 16.7 Open-endedness and ecology

40. **What is an operational test for expanding cultural scope?** Existing novelty metrics can rise indefinitely inside a fixed descriptor space.
41. **Can a fixed physics engine support effectively open-ended technology?** Real physics is also governed by compact laws, but simulated material categories and resolution may impose much lower combinatorial ceilings.
42. **How much ecological change is productive?** Stable environments permit accumulation; changing environments prevent stasis but can erase fragile lineages.
43. **Do arms races generate transferable technology or only specialized counters?** Competitive complexity may not generalize to other contexts.
44. **Can organisms create new niches that support new cultural specializations?** Niche construction may be a stronger engine of cultural expansion than exogenous task changes.
45. **Will energy accounting suppress long-horizon construction before culture can bootstrap?** Immediate metabolic selection may eliminate costly innovations with delayed benefits.
46. **Can neutral or aesthetic traditions later become functional stepping stones?** A system focused only on immediate reward would miss this path.
47. **What causes open-ended systems to saturate: representation, physics, population, search, measurement, or selection?** These failure causes require different remedies.

## 16.8 Methodology and reproducibility

48. **How many seeds are enough when innovation is rare and heavy-tailed?** Conventional sample-size rules assuming near-normal outcomes may be inappropriate.
49. **How should null distributions be constructed for post hoc discovered traditions?** Analyst-selected behaviors invite multiple-comparison and hindsight bias.
50. **Can lineage metrics remain tractable when behavior is continuous and recombinational?** Discrete variant labels may impose artificial cultural atoms.
51. **How should analyst classifications be validated without feeding them back into the simulation?** Multiple offline detectors and blinded human coding may be needed.
52. **What checkpoint interventions best establish cultural causality?** Observation blocking, cross-fostering, artifact resets, and genotype replay test different pathways.
53. **How can deterministic counterfactual branches be compared when a one-tick intervention causes chaotic divergence?** Early divergence is causal, but later endpoint differences may be difficult to attribute mechanistically.
54. **How should negative results be reported when a mechanism might simply require more compute?** Predefined budget ceilings and power analyses are necessary to distinguish “not observed” from “disconfirmed.”
55. **Can independent implementations reproduce a cultural result under the same abstract specification?** Engine-specific artifacts may otherwise masquerade as general mechanisms.

# 17. Actionable recommendations

## 17.1 Priority-zero scientific contract

Before adding cultural mechanisms, freeze a written contract containing the following rules:

- Organisms receive only embodied, local, causally available information.
- No behavior is named as a technology, tradition, era, role, or civilization state inside the authoritative simulation.
- No offline classifier, lineage label, novelty score, or era detector becomes an organism input or reward.
- No organism receives a fixed recipe or a task-specific imitation target.
- Direct neural-state transfer is reserved for an explicitly labeled upper-bound control.
- Every claim of culture requires a genetic control, an ecological control, and a transmission intervention.
- Every claim of cumulative improvement requires a lineage reconstruction and a naïve rediscovery baseline.
- Every headline result requires a preregistered multi-seed confirmation set and publication of failures.

This contract is more important than any individual feature because accidental scripting usually enters through instrumentation, curriculum logic, or rewards rather than through the object API itself.

## 17.2 Implement now: minimum scientific infrastructure

1. **Deterministic event provenance.** Assign stable IDs to organisms, actions, observations, signals, objects, contacts, material modifications, births, deaths, and learning updates. Record parent events for copied or reconstructed variants where inferable.
2. **Counterfactual checkpoint branching.** Support exact branch creation from a checkpoint with one intervention mask changed. Required interventions include observation disabled, inventor removed, artifact reset, learner memory reset, genotype frozen, migration blocked, and signal remapped.
3. **Common-garden and cross-fostering harness.** Reinstantiate genotypes into standardized environments with or without demonstrators and move newborns between cultural groups without changing genomes.
4. **Read-only social sensing.** Expose conspecific pose, orientation, manipuland contact, coarse action primitives, object state transitions, outcome signals, and distance/occlusion through the same sensory pipeline used for ordinary world perception.
5. **Bounded lifetime plasticity.** Add a small deterministic plastic component with explicit memory budget, learning-rate bounds, stable update order, and complete checkpoint serialization. Keep the genetically specified controller and learned state separable.
6. **Generic object state.** Implement carry, place, strike, push/pull, join/separate, and limited deformation or wear through material properties rather than named craft operations. Start with the smallest set that can support more than one causal route to an outcome.
7. **Persistent artifacts with controlled decay.** Give objects identity and history across organism deaths. Make decay, weathering, displacement, and maintenance versioned physics parameters.
8. **Cultural assay framework.** Automate the metrics and ablations in Section 12. No visual dashboard should be allowed to substitute for those tests.

## 17.3 Implement next: minimum viable cultural-transmission system

The first integrated system should contain:

- overlapping lifetimes with continuous births and deaths;
- local observation chosen by the learner's ordinary attention dynamics;
- short episodic traces of observed action–object–outcome sequences;
- reinforcement-modulated plasticity that can bias later action selection;
- individual exploration and sensorimotor learning;
- persistent, movable, modifiable objects;
- spatially structured groups with configurable migration;
- no explicit teaching action, symbolic channel, reputation system, or inherited learned weights.

The initial test behavior should be arbitrary enough to distinguish transmission from ecological rediscovery, yet physically grounded enough to avoid a symbolic “culture token.” A suitable assay is a two-route manipulation problem in which both routes are individually discoverable, payoffs are matched, and group founders are seeded with opposite variants. The next assay should add one physically recoverable improvement and turnover. Only after those pass should the engine test multiple dependent improvements.

## 17.4 Default transmission-channel policy

Use the following order:

1. **Observed outcomes** — implement first; low scripting risk and biologically plausible.
2. **Visible actions and action tendencies** — implement through ordinary perception, not a demonstrator API.
3. **Demonstrator trajectories** — allow learners to perceive trajectories, but do not replay them into the learner controller.
4. **Signals associated with behavior** — add only after nonsymbolic transmission works; meanings must evolve from consequences.
5. **Internal neural activity** — use only as a diagnostic or upper-bound ablation, never as the default social channel.
6. **Learned synaptic changes or complete weights** — reserve for a clearly labeled engineered-inheritance comparison.

A useful experiment is to match the bandwidth of these channels. Otherwise a privileged neural channel will win simply because it transmits more bits, not because it is a better scientific model.

## 17.5 Mechanisms to defer until dependencies pass

Defer the following:

- symbolic or compositional communication;
- explicit teaching, reputation, prestige, punishment, or norm representations;
- complex composite materials;
- group identities assigned by the engine;
- mating preferences for skill or prestige;
- territorial markers with innate meanings;
- inherited lifetime weights;
- automated open-endedness claims based on a single novelty archive;
- continent-scale worlds;
- organized conflict as a target metric.

These mechanisms may eventually be valuable. Adding them before basic transmission is causally established would make failures uninterpretable and positive results easier to script accidentally.

## 17.6 Do not build

Do not add any of the following to the authoritative simulation:

- a technology tree, research point, unlock flag, era variable, or civilization score;
- a named recipe known to organisms;
- a reward for “being cultural,” “teaching,” “inventing,” “building a settlement,” or reaching an analyst-defined stage;
- a global archive that organisms query for best-known solutions;
- an offline detector whose output changes selection, reward, attention, mutation, or world generation;
- a centralized policy shared by all members of a group if the scientific question concerns individual cultural transmission;
- an LLM organism controller;
- a hidden curriculum that injects increasingly difficult authored tasks and is later described as spontaneous technological progress.

## 17.7 Acceptance gates

Proceed only when the prior gate passes under preregistered thresholds.

| Gate | Required evidence | Minimum control set | Decision if failed |
|---|---|---|---|
| **G0 — Replay** | Exact checkpoint and cross-thread replay hashes | Single-thread vs parallel; save/restore at arbitrary ticks | Fix determinism before cultural work |
| **G1 — Individual learning** | Within-lifetime performance improvement on held-out initial conditions | Plasticity-off; reward-shuffled; memory-reset | Repair learning and credit assignment |
| **G2 — Social causality** | Exposure causes variant acquisition above matched asocial controls | No-demonstrator; invisible-demonstrator; yoked nonsocial object motion | Do not call behavior socially learned |
| **G3 — Tradition** | Variant persists beyond inventor death and at least two naïve cohort turnovers | Founder removed; ecology swapped; genotype common garden | Increase overlap, retransmission, or memory; do not add complexity |
| **G4 — Variant competition** | Two arbitrary variants show measurable frequency dynamics and possible stable coexistence/path dependence | Equal-payoff founders; label swap; spatial shuffle | Diagnose ecological bias or copying asymmetry |
| **G5 — One retained improvement** | Improvement survives turnover and remains socially dependent | Improvement not demonstrated; artifact removed; inventor removed | Adjust fidelity/visibility; do not claim a ratchet |
| **G6 — Multi-step dependency** | At least two causally dependent improvements exceed naïve rediscovery and one-step controls | Component deletion; lineage truncation; order permutation | The system has bounded improvement but not a dependency-bearing ratchet |
| **G7 — Group divergence** | High within-group similarity and between-group divergence under matched ecology | Migration sweep; founder-label swap; common-garden assay | Tune topology; reject ecological/genetic explanations |
| **G8 — Artifact memory** | Naïve organisms recover behavior from artifacts after maker absence | Artifact reset/replacement; visually matched inert object; provenance destruction | Do not call artifacts external memory |
| **G9 — Weakly specified ecology** | Multiple independent lineages produce non-authored useful variants across seeds | Affordance deletion; reward decomposition; hidden-task audit | Report bounded assay success only |
| **G10 — Open-endedness candidate** | Sustained consequential novelty with no detected saturation across independent metrics and longer horizons | Descriptor replacement; shuffled lineage; equal-compute baseline systems | Treat as exploratory evidence, not proof of solved open-ended evolution |

## 17.8 Recommended parameter sweeps

Use factorial or response-surface designs around the mechanisms most likely to interact:

- observation radius × occlusion × attention cost;
- memory capacity × action-sequence duration;
- individual exploration rate × social-learning strength;
- transmission error × number of available demonstrators;
- lifespan overlap × turnover rate;
- group size × migration rate × network modularity;
- artifact persistence × environmental disturbance;
- innovation cost × payoff delay;
- genetic mutation rate × controller plasticity;
- selection strength × ecological volatility.

Do not tune all parameters to maximize accumulation. The purpose is to map phase boundaries, failure regions, and tradeoffs. Hold out confirmatory parameter regions selected before the final seed set.

## 17.9 Statistical and seed policy

- Use at least **30 independent seeds per confirmatory condition** for common outcomes; increase substantially for rare innovations or heavy zero inflation.
- Report all seeds, including extinctions and runs with no invention.
- Use survival analysis for time-to-invention and time-to-loss, hurdle or zero-inflated models for sparse lineage metrics, and hierarchical models when groups are nested within worlds.
- Predefine the unit of replication. Organisms inside one world are not independent seeds.
- Report effect distributions and uncertainty, not only mean best performance.
- Correct for multiple discovered behaviors or use held-out confirmation worlds.
- Use deterministic checkpoint branches for paired causal contrasts in addition to independent-seed estimates.

The number 30 is a starting convention, not a universal power guarantee. Simulated rare-event tails should drive formal power calculations once pilot distributions exist.

## 17.10 Documentation required for every claimed cultural result

For each result, publish or archive:

1. engine version, physics version, observer protocol version, and persistence schema version;
2. seed list and initialization manifest;
3. full intervention definitions;
4. authored information inventory: rewards, object types, actions, task constraints, demonstrator state, curriculum logic, and any privileged channels;
5. genotype and learned-state snapshots;
6. event-log hashes and checkpoint hashes;
7. cultural-lineage reconstruction code and detector version;
8. preregistered metrics and thresholds;
9. complete seed-level outcome table;
10. negative and null results;
11. videos or visualizations as illustrations only, linked to authoritative event IDs;
12. a claim-level statement using the evidence ladder in Section 15.1.

## 17.11 Near-term experiment sequence

The recommended sequence is:

1. complete deterministic learning and observation fixtures;
2. demonstrate arbitrary variant diffusion;
3. demonstrate persistence through turnover;
4. quantify variant competition and copying error;
5. add persistent generic artifacts and establish artifact-mediated retrieval;
6. establish one retained functional improvement;
7. establish two dependent improvements;
8. test group divergence and migration;
9. permit evolvable signals and measure whether they improve transmission without privileged semantics;
10. permit teaching-like opportunity creation to evolve under explicit costs;
11. run weakly specified material ecologies only after all earlier causal gates pass;
12. study territoriality or conflict only as an ecological outcome, using resource and coalition ablations.

This order minimizes the number of simultaneous explanations. It also allows a scientifically useful result even if the project never produces open-ended technology: the engine can still establish which mechanisms are sufficient for transmission, tradition, artifact memory, and finite ratcheting.

## 17.12 Bottom-line design decision

The best-supported first implementation is **not** direct copying of neural weights, a language module, or a crafting system. It is a deterministic embodied ecology in which:

- organisms can see what others do to persistent objects;
- they can retain bounded action–object–outcome memories;
- lifetime plasticity changes their own action tendencies;
- individual exploration can modify copied behavior;
- births and deaths force information to cross organism boundaries;
- objects retain physical consequences after makers die;
- spatial population structure allows traditions to diverge;
- exact provenance and causal ablations distinguish culture from genes, ecology, and rediscovery.

That system is small enough to analyze, broad enough to fail honestly, and aligned with the principle **author physics and affordances, not progress**.

# 18. Annotated bibliography

## 18.1 Reading guide and evidence-status conventions

This bibliography covers the sources cited in the report and a small number of canonical works used to interpret them. Publication status is stated explicitly because a journal experiment, a mathematical model, a conference paper, a preprint, a software repository, and a scholarly book do not carry the same evidential weight. An annotation explains what each source establishes, what it does **not** establish, and why it matters for The Genesis Engine.

The classifications below are descriptive rather than rankings. A formal model can give a strong sufficiency result under its assumptions while offering weak evidence that the same outcome will occur in a richer ecology. Conversely, a field observation may establish that a tradition exists while leaving its transmission mechanism unresolved. “Canonical” means historically influential, not immune to later criticism.

## 18.2 Core frameworks for cultural evolution and cumulative culture

- **[Peer-reviewed review] Mesoudi, A., & Thornton, A. (2018). “What is cumulative cultural evolution?” *Proceedings of the Royal Society B: Biological Sciences*, 285, 20180712. [https://doi.org/10.1098/rspb.2018.0712](https://doi.org/10.1098/rspb.2018.0712).**  Proposes a multidimensional definition centered on modification, improvement, social transmission, and repetition rather than treating every intergenerational change as cumulative culture. It is the principal definitional basis for this report’s requirement that improvements be retained and causally dependent on prior cultural states.

- **[Peer-reviewed review] Lewis, H. M., & Laland, K. N. (2012). “Transmission fidelity is the key to the build-up of cumulative culture.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 367, 2171–2180. [https://doi.org/10.1098/rstb.2012.0119](https://doi.org/10.1098/rstb.2012.0119).**  Argues that sufficiently faithful transmission is central to preserving improvements, while also discussing mechanisms that can compensate for imperfect individual copying. The paper supports treating fidelity as a measured end-to-end property, not as a fixed universal threshold or a synonym for imitation.

- **[Peer-reviewed review] Tennie, C., Call, J., & Tomasello, M. (2009). “Ratcheting up the ratchet: on the evolution of cumulative culture.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 364, 2405–2415. [https://doi.org/10.1098/rstb.2009.0052](https://doi.org/10.1098/rstb.2009.0052).**  Develops the “cultural ratchet” problem and emphasizes mechanisms that prevent loss of previously acquired improvements. Its stronger claims about human uniqueness should be read alongside later animal work, but its loss-prevention framing remains directly useful for experimental design.

- **[Peer-reviewed theoretical article] Tomasello, M., Kruger, A. C., & Ratner, H. H. (1993). “Cultural learning.” *Behavioral and Brain Sciences*, 16, 495–552. [https://doi.org/10.1017/S0140525X0003123X](https://doi.org/10.1017/S0140525X0003123X).**  Distinguishes forms of social learning and links human cultural learning to perspective taking and intentional understanding. The paper is historically central, but later experiments show that some cumulative improvement can occur without full intention reading, so its cognitive requirements should not be built into Genesis as assumptions.

- **[Peer-reviewed review] Miton, H., & Charbonneau, M. (2018). “Cumulative culture in the laboratory: methodological and theoretical challenges.” *Proceedings of the Royal Society B: Biological Sciences*, 285, 20180677. [https://doi.org/10.1098/rspb.2018.0677](https://doi.org/10.1098/rspb.2018.0677).**  Reviews experimental paradigms and warns that performance gains alone do not necessarily demonstrate genuine cultural dependence or cumulative complexity. It motivates the report’s insistence on lineage reconstruction, replacement controls, and separation of rediscovery from inheritance.

- **[Peer-reviewed review] Caldwell, C. A., Atkinson, M., Blakey, K. H., Dunstone, J., Kean, D., Mackintosh, G., Renner, E., & Wilks, C. E. H. (2020). “Experimental assessment of capacities for cumulative culture: review and evaluation of methods.” *WIREs Cognitive Science*, 11, e1516. [https://doi.org/10.1002/wcs.1516](https://doi.org/10.1002/wcs.1516).**  Evaluates laboratory methods for identifying cumulative culture and the inferential limits of transmission chains, replacement designs, and open diffusion. It is especially relevant to defining what a Genesis experiment must compare against before calling a result cumulative.

- **[Peer-reviewed review] Derex, M. (2022). “Human cumulative culture and the exploitation of natural phenomena.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 377, 20200311. [https://doi.org/10.1098/rstb.2020.0311](https://doi.org/10.1098/rstb.2020.0311).**  Argues that cumulative technologies can advance through selective retention of effects that users do not fully understand. This directly supports exposing organisms to reliable physics and observable outcomes rather than requiring explicit causal models or naming technologies.

- **[Peer-reviewed perspective/review] Morgan, T. J. H., & Feldman, M. W. (2025). “Human culture is uniquely open-ended rather than uniquely cumulative.” *Nature Human Behaviour*, 9, 28–42. [https://doi.org/10.1038/s41562-024-02035-y](https://doi.org/10.1038/s41562-024-02035-y).**  Reframes the comparative question by arguing that bounded cumulative change is not uniquely human, whereas the breadth and apparent open-endedness of human culture remain unusual. It supports the report’s refusal to treat a finite ratchet as evidence that open-ended technological evolution has been solved.

- **[Peer-reviewed review] Muthukrishna, M., & Henrich, J. (2016). “Innovation in the collective brain.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 371, 20150192. [https://doi.org/10.1098/rstb.2015.0192](https://doi.org/10.1098/rstb.2015.0192).**  Synthesizes how population size, connectivity, specialization, transmission, and institutions affect collective innovation. It motivates treating demographic variables as interacting parameters rather than assuming that more agents monotonically yield more culture.

- **[Peer-reviewed review] Gruber, T., Chimento, M., Aplin, L. M., & Biro, D. (2022). “Efficiency fosters cumulative culture across species.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 377, 20200308. [https://doi.org/10.1098/rstb.2020.0308](https://doi.org/10.1098/rstb.2020.0308).**  Proposes efficiency gains as a cross-species route to cumulative change and emphasizes that cumulative culture need not begin with human-like cognition. For Genesis, it supports beginning with measurable improvements in cost, time, reliability, or yield rather than semantic notions of technological sophistication.

- **[Book] Boyd, R., & Richerson, P. J. (1985). *Culture and the Evolutionary Process*. Chicago: University of Chicago Press. ISBN 9780226069333.**  A foundational mathematical treatment of gene–culture coevolution, transmission biases, and population-level cultural dynamics. Its abstractions are valuable for null models and hypotheses, although Genesis should not reduce culture to disembodied trait copying when artifact and action mechanisms are under study.

- **[Book] Cavalli-Sforza, L. L., & Feldman, M. W. (1981). *Cultural Transmission and Evolution: A Quantitative Approach*. Princeton, NJ: Princeton University Press. ISBN 9780691082837.**  Establishes classic vertical, horizontal, and oblique transmission models and their population consequences. It provides terminology and baseline theory, but its trait-level representation leaves embodiment, affordances, and reconstruction unspecified.

- **[Book] Mesoudi, A. (2011). *Cultural Evolution: How Darwinian Theory Can Explain Human Culture and Synthesize the Social Sciences*. Chicago: University of Chicago Press. ISBN 9780226520445.**  Provides a broad synthesis of cultural-evolution methods, including experiments, phylogenies, and population models. It is useful for framing cultural variation and inheritance without assuming that cultural evolution is identical to genetic evolution.

- **[Book] Henrich, J. (2015). *The Secret of Our Success: How Culture Is Driving Human Evolution, Domesticating Our Species, and Making Us Smarter*. Princeton, NJ: Princeton University Press. ISBN 9780691166858.**  Develops the collective-brain account of human adaptation and the importance of socially acquired know-how. It is a broad explanatory synthesis rather than a specification for artificial organisms, and its human institutional examples should not be transcribed into authored stages.

- **[Book] Sterelny, K. (2012). *The Evolved Apprentice: How Evolution Made Humans Unique*. Cambridge, MA: MIT Press. ISBN 9780262016797.**  Emphasizes informationally structured developmental environments, cooperation, and apprenticeship. It is relevant to how toleration, repeated exposure, and modified environments can improve learning opportunities without requiring a privileged “teach” command.

- **[Book] Heyes, C. (2018). *Cognitive Gadgets: The Cultural Evolution of Thinking*. Cambridge, MA: Harvard University Press. ISBN 9780674980150.**  Argues that important human cognitive mechanisms may themselves be culturally constructed. For Genesis, it is a warning against hard-coding advanced social cognition merely because it is associated with culture in humans.

- **[Edited book] Fragaszy, D. M., & Perry, S. (Eds.). (2003). *The Biology of Traditions: Models and Evidence*. Cambridge: Cambridge University Press. ISBN 9780521815970.**  Collects theoretical and empirical work on animal traditions and the evidential standards needed to infer them. It supports a broad definition of tradition while preserving a stricter threshold for cumulative culture.

## 18.3 Social learning, imitation, teaching, and transmission mechanisms

- **[Peer-reviewed review] Heyes, C. M. (1994). “Social learning in animals: categories and mechanisms.” *Biological Reviews*, 69, 207–231. [https://doi.org/10.1111/j.1469-185X.1994.tb01506.x](https://doi.org/10.1111/j.1469-185X.1994.tb01506.x).**  Separates social-learning outcomes from candidate mechanisms and reviews associative explanations. It is a key source for avoiding the common error of labeling any socially correlated behavior as imitation.

- **[Peer-reviewed review] Hopper, L. M. (2010). “‘Ghost’ experiments and the dissection of social learning in humans and animals.” *Biological Reviews*, 85, 685–701. [https://doi.org/10.1111/j.1469-185X.2010.00120.x](https://doi.org/10.1111/j.1469-185X.2010.00120.x).**  Reviews ghost-control designs that display object movement or outcomes without a visible actor. This is directly translatable into Genesis ablations separating action copying from outcome emulation and artifact-mediated learning.

- **[Peer-reviewed review] Caro, T. M., & Hauser, M. D. (1992). “Is there teaching in nonhuman animals?” *The Quarterly Review of Biology*, 67, 151–174. [https://doi.org/10.1086/417553](https://doi.org/10.1086/417553).**  Supplies the influential functional criteria for animal teaching: behavior modification in the presence of a naïve observer, cost or no immediate benefit to the demonstrator, and accelerated or improved learning by the pupil. Genesis should use these behavioral criteria rather than giving agents a researcher-labeled teaching state.

- **[Peer-reviewed review] Hoppitt, W., Brown, G., Kendal, R., Rendell, L., Thornton, A., Webster, M. M., & Laland, K. N. (2008). “Lessons from animal teaching.” *Trends in Ecology & Evolution*, 23, 486–493. [https://doi.org/10.1016/j.tree.2008.05.008](https://doi.org/10.1016/j.tree.2008.05.008).**  Reviews teaching across taxa and stresses functional evidence and ecological context. It supports looking for costly opportunity creation or altered demonstrations as evolved behavior, not assuming intentional pedagogy.

- **[Book] Hoppitt, W., & Laland, K. N. (2013). *Social Learning: An Introduction to Mechanisms, Methods, and Models*. Princeton, NJ: Princeton University Press. [Publisher page](https://press.princeton.edu/books/hardcover/9780691150710/social-learning).**  A comprehensive methodological treatment of social-learning mechanisms, diffusion analysis, and theoretical models. It is the main reference for operational definitions and for distinguishing acquisition mechanism from population-level spread.

- **[Peer-reviewed model] Lind, J., Ghirlanda, S., & Enquist, M. (2019). “Social learning through associative processes: a computational theory.” *Royal Society Open Science*, 6, 181777. [https://doi.org/10.1098/rsos.181777](https://doi.org/10.1098/rsos.181777).**  Shows how multiple phenomena attributed to specialized social-learning modules can arise through domain-general associative learning exposed to social cues. It supports testing ordinary perception and reinforcement before introducing privileged copying machinery.

- **[Peer-reviewed model] Fogarty, L., Strimling, P., & Laland, K. N. (2011). “The evolution of teaching.” *Evolution*, 65, 2760–2770. [https://doi.org/10.1111/j.1558-5646.2011.01370.x](https://doi.org/10.1111/j.1558-5646.2011.01370.x).**  Models conditions under which teaching can evolve despite costs. Its relevance is conditional: it motivates costed opportunities and relatedness or repeated-interaction sweeps, but it does not show that teaching will evolve in an embodied artificial ecology.

- **[Peer-reviewed methods/model] Hoppitt, W., Boogert, N. J., & Laland, K. N. (2010). “Detecting social transmission in networks.” *Journal of Theoretical Biology*, 263, 544–555. [https://doi.org/10.1016/j.jtbi.2010.01.004](https://doi.org/10.1016/j.jtbi.2010.01.004).**  Develops network-based diffusion analysis for testing whether acquisition follows social ties. It informs Genesis metrics that compare exposure-weighted acquisition against asocial baselines instead of inferring transmission from spatial clustering alone.

- **[Peer-reviewed experiment/review] Zentall, T. R. (2022). “Mechanisms of copying, social learning, and imitation in animals.” *Learning and Motivation*, 80, 101844. [https://doi.org/10.1016/j.lmot.2022.101844](https://doi.org/10.1016/j.lmot.2022.101844).**  Reviews experimental distinctions among copying processes and the evidence for imitation. It reinforces the need to expose and ablate action, object, location, and outcome information separately.

- **[Peer-reviewed theoretical model] Rogers, A. R. (1988). “Does biology constrain culture?” *American Anthropologist*, 90, 819–831. [Stable record](https://www.jstor.org/stable/678986).**  Introduces the result commonly called Rogers’ paradox: social learning may spread without raising average fitness when it only copies information produced by costly individual learning. The model is a critical warning that social learners can exploit but not replenish an information pool.

- **[Peer-reviewed tournament/model] Rendell, L., Boyd, R., Cownden, D., Enquist, M., Eriksson, K., Feldman, M. W., Fogarty, L., Ghirlanda, S., Lillicrap, T., & Laland, K. N. (2010). “Why copy others? Insights from the social learning strategies tournament.” *Science*, 328, 208–213. [https://doi.org/10.1126/science.1184719](https://doi.org/10.1126/science.1184719).**  Compares strategies in changing environments and shows the importance of when and whom to copy. The result supports evolvable or learnable attention policies, but the tournament’s abstract payoff structure is not evidence of embodied traditions.

- **[Peer-reviewed theoretical model] Kobayashi, Y., Wakano, J. Y., & Ohtsuki, H. (2015). “A paradox of cumulative culture.” *Journal of Theoretical Biology*, 379, 79–88. [https://doi.org/10.1016/j.jtbi.2015.05.002](https://doi.org/10.1016/j.jtbi.2015.05.002).**  Analyzes conditions under which cumulative cultural systems can become vulnerable to insufficient individual innovation. It supports maintaining exploration and measuring the contribution of inventors rather than optimizing only for copying fidelity.

- **[Peer-reviewed experiment] Caldwell, C. A., & Millen, A. E. (2009). “Social learning mechanisms and cumulative cultural evolution: is imitation necessary?” *Psychological Science*, 20, 1478–1483. [https://doi.org/10.1111/j.1467-9280.2009.02469.x](https://doi.org/10.1111/j.1467-9280.2009.02469.x).**  Uses transmission chains to show cumulative improvement under information conditions that do not require faithful copying of exact motor actions. This supports outcome observation and emulation as plausible initial mechanisms, while not proving that all technological ratchets can proceed without imitation.

- **[Peer-reviewed experiment] Zwirner, E., & Thornton, A. (2015). “Cognitive requirements of cumulative culture: teaching is useful but not essential.” *Scientific Reports*, 5, 16781. [https://doi.org/10.1038/srep16781](https://doi.org/10.1038/srep16781).**  Finds that teaching can improve performance in a cumulative task but is not always necessary for cumulative gains. Genesis can therefore defer explicit teaching mechanisms and first test whether ordinary observation, tolerance, and repeated exposure suffice.

- **[Peer-reviewed experiment] Derex, M., Bonnefon, J.-F., Boyd, R., & Mesoudi, A. (2019). “Causal understanding is not necessary for the improvement of culturally evolving technology.” *Nature Human Behaviour*, 3, 446–452. [https://doi.org/10.1038/s41562-019-0567-9](https://doi.org/10.1038/s41562-019-0567-9).**  Demonstrates improvement of a physical system despite participants lacking complete causal understanding of the relevant phenomenon. It supports retaining observable functional outcomes and allowing selection among variants without installing explicit technological concepts.

- **[Peer-reviewed experiment] Morgan, T. J. H., Uomini, N. T., Rendell, L. E., Chouinard-Thuly, L., Street, S. E., Lewis, H. M., Cross, C. P., Evans, C., Kearney, R., de la Torre, I., Whiten, A., & Laland, K. N. (2015). “Experimental evidence for the co-evolution of hominin tool-making teaching and language.” *Nature Communications*, 6, 6029. [https://doi.org/10.1038/ncomms7029](https://doi.org/10.1038/ncomms7029).**  Compares transmission conditions for stone-tool manufacture and finds strong benefits from teaching and language in this demanding task. It establishes that communication can materially improve fidelity for some skills, not that symbolic language is a universal prerequisite for cumulative culture.

## 18.4 Human cumulative-culture experiments, demography, and archaeology

- **[Peer-reviewed experiment] Dean, L. G., Kendal, R. L., Schapiro, S. J., Thierry, B., & Laland, K. N. (2012). “Identification of the social and cognitive processes underlying human cumulative culture.” *Science*, 335, 1114–1118. [https://doi.org/10.1126/science.1213969](https://doi.org/10.1126/science.1213969).**  Compares children, chimpanzees, and capuchins on a staged puzzle and associates human success with teaching, imitation, prosociality, and communication. The task contains researcher-defined levels, so it informs transmission capacities but is not a model for authoring Genesis progression.

- **[Peer-reviewed experiment] Kirby, S., Cornish, H., & Smith, K. (2008). “Cumulative cultural evolution in the laboratory: an experimental approach to the origins of structure in human language.” *Proceedings of the National Academy of Sciences*, 105, 10681–10686. [https://doi.org/10.1073/pnas.0707835105](https://doi.org/10.1073/pnas.0707835105).**  Iterated learning produces increasingly learnable linguistic structure as languages pass through learner bottlenecks. It is strong evidence that repeated reconstruction can create cumulative organization, but the output is a communication system under a structured laboratory mapping task rather than embodied technology.

- **[Peer-reviewed experiment] Derex, M., Beugin, M.-P., Godelle, B., & Raymond, M. (2013). “Experimental evidence for the influence of group size on cultural complexity.” *Nature*, 503, 389–391. [https://doi.org/10.1038/nature12774](https://doi.org/10.1038/nature12774).**  Finds that larger groups preserve or improve performance on laboratory cultural tasks more effectively under the tested conditions. It supports group-size sweeps, while later critiques show that demography is not a single sufficient explanation for cultural complexity.

- **[Peer-reviewed experiment] Muthukrishna, M., Shulman, B. W., Vasilescu, V., & Henrich, J. (2014). “Sociality influences cultural complexity.” *Proceedings of the Royal Society B: Biological Sciences*, 281, 20132511. [https://doi.org/10.1098/rspb.2013.2511](https://doi.org/10.1098/rspb.2013.2511).**  Shows that access to more models can improve the retention and accumulation of complex performance. For Genesis, the relevant variable is not census size alone but effective access to diverse, competent demonstrators.

- **[Peer-reviewed model] Henrich, J. (2004). “Demography and cultural evolution: how adaptive cultural processes can produce maladaptive losses—the Tasmanian case.” *American Antiquity*, 69, 197–214. [https://doi.org/10.2307/4128416](https://doi.org/10.2307/4128416).**  Models how population size can affect the maintenance of complex skills under imperfect copying. The historical interpretation is contested, so the paper is best used as a mechanistic hypothesis about stochastic skill loss rather than a settled account of Tasmania.

- **[Peer-reviewed model/archaeological analysis] Powell, A., Shennan, S., & Thomas, M. G. (2009). “Late Pleistocene demography and the appearance of modern human behavior.” *Science*, 324, 1298–1301. [https://doi.org/10.1126/science.1170165](https://doi.org/10.1126/science.1170165).**  Links demographic density and connectivity to the maintenance of cultural complexity in a formal model and archaeological interpretation. It supports testing effective population and migration, but not treating population size as a universal control knob for progress.

- **[Peer-reviewed commentary/model critique] Andersson, C., & Read, D. (2014). “Group size and cultural complexity.” *Nature*, 511, E1. [https://doi.org/10.1038/nature13411](https://doi.org/10.1038/nature13411).**  Challenges simple interpretations of group-size experiments and highlights assumptions connecting social access to cultural outcomes. It is a useful caution that nominal group size can be less important than interaction structure and task design.

- **[Peer-reviewed archaeological analysis] Vaesen, K., Collard, M., Cosgrove, R., & Roebroeks, W. (2016). “Population size does not explain past changes in cultural complexity.” *Proceedings of the National Academy of Sciences*, 113, E2241–E2247. [https://doi.org/10.1073/pnas.1520288113](https://doi.org/10.1073/pnas.1520288113).**  Tests demographic explanations against archaeological cases and finds that population size alone performs poorly. Genesis experiments should therefore manipulate ecology, connectivity, fidelity, and specialization alongside population size.

- **[Peer-reviewed review] Collard, M., Vaesen, K., Cosgrove, R., & Roebroeks, W. (2016). “The empirical case against the ‘demographic turn’ in Palaeolithic archaeology.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 371, 20150242. [https://doi.org/10.1098/rstb.2015.0242](https://doi.org/10.1098/rstb.2015.0242).**  Reviews weaknesses in claims that demographic change broadly explains archaeological complexity. It reinforces the report’s recommendation to model demography as an interacting cause and to preclude post-hoc stories based solely on larger populations.

- **[Peer-reviewed empirical/network study] Migliano, A. B., Battiston, F., Viguier, S., et al. (2020). “Hunter-gatherer multilevel sociality accelerates cumulative cultural evolution.” *Science Advances*, 6, eaax5913. [https://doi.org/10.1126/sciadv.aax5913](https://doi.org/10.1126/sciadv.aax5913).**  Combines observed social-network structure with experiments and modeling to argue that multilevel networks can support recombination and cultural accumulation. The implication for Genesis is to test modular networks with bridging ties rather than only isolated groups or well-mixed populations.

- **[Peer-reviewed archaeological/modeling study] Eerkens, J. W., & Lipo, C. P. (2005). “Cultural transmission, copying errors, and the generation of variation in material culture and the archaeological record.” *Journal of Anthropological Archaeology*, 24, 316–334. [https://doi.org/10.1016/j.jaa.2005.08.001](https://doi.org/10.1016/j.jaa.2005.08.001).**  Connects copying error distributions to measurable patterns in material artifacts. It motivates recording artifact dimensions and mutation-like deviations so that Genesis can distinguish drift, biased transformation, and functional selection.

- **[Peer-reviewed archaeological analysis] Paige, J., & Perreault, C. (2024). “3.3 million years of stone tool complexity suggests that cumulative culture began during the Middle Pleistocene.” *Proceedings of the National Academy of Sciences*, 121, e2319175121. [https://doi.org/10.1073/pnas.2319175121](https://doi.org/10.1073/pnas.2319175121).**  Quantifies long-run change in stone-tool production complexity and argues for a comparatively late transition to cumulative culture. The analysis shows the value of explicit complexity measures but also illustrates how archaeological inference depends on proxy choice and preservation.

- **[Book] Shennan, S. (2002). *Genes, Memes and Human History: Darwinian Archaeology and Cultural Evolution*. London: Thames & Hudson. ISBN 9780500051184.**  Connects cultural-evolution theory to archaeological variation and inheritance. It is useful for thinking about artifacts as records of descent with modification, while Genesis has the advantage of complete event provenance unavailable to archaeologists.

- **[Book] Richerson, P. J., & Boyd, R. (2005). *Not by Genes Alone: How Culture Transformed Human Evolution*. Chicago: University of Chicago Press. ISBN 9780226712123.**  Synthesizes dual-inheritance theory for a broad audience and explains how cultural processes can alter selection. It supports maintaining separate genetic and cultural causal channels rather than treating learned behavior as an informal extension of genotype.

## 18.5 Animal culture, traditions, tool use, and bounded ratchets

- **[Peer-reviewed field experiment] Aplin, L. M., Farine, D. R., Morand-Ferron, J., Cockburn, A., Thornton, A., & Sheldon, B. C. (2015). “Experimentally induced innovations lead to persistent culture via conformity in wild birds.” *Nature*, 518, 538–541. [https://doi.org/10.1038/nature13998](https://doi.org/10.1038/nature13998).**  Seeds alternative foraging solutions in wild great-tit populations and documents diffusion, conformity, and persistence across turnover. This is among the strongest demonstrations that arbitrary group traditions can survive individual replacement, but it does not demonstrate cumulative functional improvement.

- **[Peer-reviewed field experiment] van de Waal, E., Borgeaud, C., & Whiten, A. (2013). “Potent social learning and conformity shape a wild primate’s foraging decisions.” *Science*, 340, 483–485. [https://doi.org/10.1126/science.1232769](https://doi.org/10.1126/science.1232769).**  Establishes experimentally induced group preferences and conformity in wild vervet monkeys. It supports group-specific tradition mechanisms and migration tests, not a claim of technological ratcheting.

- **[Peer-reviewed comparative field study] Whiten, A., Goodall, J., McGrew, W. C., Nishida, T., Reynolds, V., Sugiyama, Y., Tutin, C. E. G., Wrangham, R. W., & Boesch, C. (1999). “Cultures in chimpanzees.” *Nature*, 399, 682–685. [https://doi.org/10.1038/21415](https://doi.org/10.1038/21415).**  Catalogues geographically patterned chimpanzee behaviors not readily explained by ecology alone. It is canonical evidence for animal cultural variation, though observational exclusion methods do not by themselves identify the exact learning mechanism.

- **[Peer-reviewed experiment] Whiten, A., Horner, V., & de Waal, F. B. M. (2005). “Conformity to cultural norms of tool use in chimpanzees.” *Nature*, 437, 737–740. [https://doi.org/10.1038/nature04047](https://doi.org/10.1038/nature04047).**  Uses seeded alternative techniques to demonstrate group-level tool-use traditions and conformity. The result motivates arbitrary-solution diffusion tests and a distinction between norm persistence and performance improvement.

- **[Peer-reviewed genetic and behavioral analysis] Krützen, M., Mann, J., Heithaus, M. R., Connor, R. C., Bejder, L., & Sherwin, W. B. (2005). “Cultural transmission of tool use in bottlenose dolphins.” *Proceedings of the National Academy of Sciences*, 102, 8939–8943. [https://doi.org/10.1073/pnas.0500232102](https://doi.org/10.1073/pnas.0500232102).**  Uses genetic, ecological, and behavioral evidence to support primarily vertical social transmission of sponge tool use. It illustrates how culture can be entangled with kinship and ecology, motivating parent-randomization and genetic controls in Genesis.

- **[Peer-reviewed experiment] Fehér, O., Wang, H., Saar, S., Mitra, P. P., & Tchernichovski, O. (2009). “De novo establishment of wild-type song culture in the zebra finch.” *Nature*, 459, 564–568. [https://doi.org/10.1038/nature07994](https://doi.org/10.1038/nature07994).**  Shows that iterated transmission from initially abnormal song can converge toward species-typical structure over generations. This is a clear case of reconstructive cultural regularization, but the endpoint is constrained by species-specific biases and should not be equated with open-ended novelty.

- **[Peer-reviewed experiment/model] Sasaki, T., & Biro, D. (2017). “Cumulative culture can emerge from collective intelligence in animal groups.” *Nature Communications*, 8, 15049. [https://doi.org/10.1038/ncomms15049](https://doi.org/10.1038/ncomms15049).**  Demonstrates cumulative route improvement in pigeon chains through repeated replacement of group members. It provides a concrete model for turnover-mediated improvement without symbolic communication, while remaining a narrow optimization domain rather than general technology.

- **[Peer-reviewed experiment] Bridges, A. D., Royka, A., Wilson, T., et al. (2024). “Bumblebees socially learn behaviour too complex to innovate alone.” *Nature*, 627, 572–578. [https://doi.org/10.1038/s41586-024-07126-4](https://doi.org/10.1038/s41586-024-07126-4).**  Reports acquisition of a difficult two-step task through social demonstration when naïve individuals generally fail to innovate it independently. This strengthens evidence that social transmission can cross an innovation barrier in nonhuman animals, but the task apparatus and solution space remain heavily authored.

- **[Peer-reviewed experiment] van Leeuwen, E. J. C., DeTroy, S. E., Haun, D. B. M., & Call, J. (2024). “Chimpanzees use social information to acquire a skill they fail to innovate.” *Nature Human Behaviour*, 8, 891–902. [https://doi.org/10.1038/s41562-024-01836-5](https://doi.org/10.1038/s41562-024-01836-5).**  Shows chimpanzees acquiring a sequential behavior after observing a trained model despite failing to solve it asocially. It narrows claims about uniquely human social learning but does not by itself show retention of successive improvements across generations.

- **[Peer-reviewed comparative/network study] Gunasekaram, C., Battiston, F., Sadekar, O., et al. (2024). “Population connectivity shapes the distribution and complexity of chimpanzee cumulative culture.” *Science*, 386, 920–925. [https://doi.org/10.1126/science.adk3381](https://doi.org/10.1126/science.adk3381).**  Relates inferred long-term population connectivity to the distribution of complex chimpanzee behaviors. The study supports a role for migration and cultural contact, but its classifications and historical inferences are observational rather than direct demonstrations of a ratchet.

- **[Peer-reviewed field observation] Pruetz, J. D., & Bertolani, P. (2007). “Savanna chimpanzees, *Pan troglodytes verus*, hunt with tools.” *Current Biology*, 17, 412–417. [https://doi.org/10.1016/j.cub.2006.12.042](https://doi.org/10.1016/j.cub.2006.12.042).**  Documents manufacture and use of modified sticks in hunting. It demonstrates flexible tool behavior and object modification, but neither the observation nor the behavior’s sophistication alone establishes cumulative cultural evolution.

- **[Peer-reviewed comparative study] Hunt, G. R., & Gray, R. D. (2003). “Diversification and cumulative evolution in New Caledonian crow tool manufacture.” *Proceedings of the Royal Society B: Biological Sciences*, 270, 867–874. [https://doi.org/10.1098/rspb.2002.2302](https://doi.org/10.1098/rspb.2002.2302).**  Analyzes geographic variation in crow tool forms and argues for cumulative diversification. The interpretation remains less causally resolved than experimental transmission chains, making it evidence for a plausible historical process rather than a direct demonstration of each transmission step.

- **[Peer-reviewed field study] Kawai, M. (1965). “Newly-acquired pre-cultural behavior of the natural troop of Japanese monkeys on Koshima Islet.” *Primates*, 6, 1–30. [https://doi.org/10.1007/BF01794457](https://doi.org/10.1007/BF01794457).**  The classic report of sweet-potato washing and related diffusion in Japanese macaques. It remains historically important for traditions, but later methodological standards caution against inferring detailed transmission mechanisms from diffusion narratives alone.

- **[Peer-reviewed review] Whiten, A. (2021). “The burgeoning reach of animal culture.” *Science*, 372, eabe6514. [https://doi.org/10.1126/science.abe6514](https://doi.org/10.1126/science.abe6514).**  Reviews evidence for culture across animal taxa and the expansion of experimental methods. It provides comparative context while preserving the distinction between widespread traditions and rarer, contested cumulative processes.

## 18.6 Artificial life, evolutionary robotics, agent-based models, and multi-agent learning

- **[Peer-reviewed model] Dalmaijer, E. S. (2024). “Cumulative route improvements spontaneously emerge in artificial navigators even in the absence of sophisticated communication or thought.” *PLOS Biology*, 22, e3002644. [https://doi.org/10.1371/journal.pbio.3002644](https://doi.org/10.1371/journal.pbio.3002644).**  Simulates navigation chains in which replacing one member at a time can produce cumulative route improvements from simple movement rules. It is a strong minimal sufficiency result for a narrow route-optimization ratchet, not evidence of general-purpose technological evolution or embodied construction.

- **[Conference paper] Acerbi, A., & Nolfi, S. (2007). “Social learning and cultural evolution in embodied and situated agents.” In *Proceedings of the 2007 IEEE Symposium on Artificial Life*, 333–340. [https://doi.org/10.1109/ALIFE.2007.367814](https://doi.org/10.1109/ALIFE.2007.367814).**  Studies socially mediated acquisition in embodied agents under evolutionary conditions. The work is relevant because agents learn in a sensorimotor loop, but its task, social channel, and fitness conditions are researcher-designed and the result is not an open-ended technological ratchet.

- **[Peer-reviewed conference paper] Cook, J., Lu, C., Hughes, E., Leibo, J. Z., & Foerster, J. N. (2024). “Artificial Generational Intelligence: cultural accumulation in reinforcement learning.” In *Advances in Neural Information Processing Systems 37*. [https://doi.org/10.52202/079017-1907](https://doi.org/10.52202/079017-1907).**  Introduces a generational training framework in which agents inherit information through cultural channels and can improve across generations on designed tasks. It is useful as evidence that generational replacement can change learning dynamics, but task rewards, training interfaces, and inheritance machinery remain authored.

- **[Peer-reviewed model] Bhoopchand, A., Brownfield, B., Collister, A., et al. (2023). “Learning few-shot imitation as cultural transmission.” *Nature Communications*, 14, 7536. [https://doi.org/10.1038/s41467-023-42875-2](https://doi.org/10.1038/s41467-023-42875-2).**  Trains agents to infer and reproduce behaviors from a small number of demonstrations, showing that a learned imitation capacity can support transmission. The demonstrations, objectives, and training distribution are explicit; this establishes a capable social-learning algorithm, not spontaneous culture in an open ecology.

- **[Peer-reviewed robotics/model] Winfield, A. F. T., & Erbas, M. D. (2011). “On embodied memetic evolution and the emergence of behavioural traditions in robots.” *Memetic Computing*, 3, 261–270. [https://doi.org/10.1007/s12293-011-0063-x](https://doi.org/10.1007/s12293-011-0063-x).**  Explores robot-to-robot transfer of behavioral controllers and the formation of lineages or traditions. It is a useful engineered comparison condition for direct policy transfer, but directly copying controller information bypasses the observational reconstruction problem central to biological cultural learning.

- **[Peer-reviewed robotics/model] Erbas, M. D., Winfield, A. F. T., & Bull, L. (2014). “Embodied imitation-enhanced reinforcement learning in multi-agent systems.” *Adaptive Behavior*, 22, 31–50. [https://doi.org/10.1177/1059712313500503](https://doi.org/10.1177/1059712313500503).**  Combines embodied imitation with reinforcement learning to accelerate acquisition. It informs a possible demonstrator-trajectory condition, but it does not establish that internal controller transfer is necessary or scientifically preferable for Genesis.

- **[Peer-reviewed model] Chisausky, J., Daras, I. M., Weissing, F. J., & Kozielska, M. (2025). “A neural network model for the evolution of reconstructive social learning.” *Scientific Reports*, 15, 14977. [https://doi.org/10.1038/s41598-025-97492-4](https://doi.org/10.1038/s41598-025-97492-4).**  Models inherited neural networks that are modified by lifetime learning under social guidance or instruction and reports systematic replicated parameter sweeps. It is especially relevant to learner-side reconstruction: demonstrations interact with inherited architecture and prior learning rather than transmitting a finished neural state. The model does not demonstrate cumulative culture.

- **[Conference model] Gabora, L. (2008; repository version 2013). “EVOC: a computer model of the evolution of culture.” In *Proceedings of the 30th Annual Meeting of the Cognitive Science Society*, 1466–1471. [Canonical manuscript](https://arxiv.org/abs/1310.0522).**  EVOC models agents that invent and imitate abstract actions, often producing rising mean fitness and diversity dynamics. It is useful for controlled studies of invention–imitation balance, but action components, fitness functions, and the space of valid improvements are authored, so increased fitness is not evidence of open-ended technology.

- **[Conference model] Gabora, L., Chia, W. W., & Firouzi, H. (2013). “A computational model of two cognitive transitions underlying cultural evolution.” In *Proceedings of the Annual Meeting of the Cognitive Science Society*. [Canonical manuscript](https://arxiv.org/abs/1310.4086).**  Extends EVOC with mechanisms intended to represent contextual focus and chained action. The model demonstrates consequences of those designed capacities in an abstract action space, not the spontaneous emergence of the capacities or of technological affordances.

- **[Conference model] Gabora, L., & Saberi, M. (2011; repository version 2013). “An agent-based model of the cognitive mechanisms underlying the origins of creative cultural evolution.” In *Proceedings of the 8th ACM Conference on Creativity and Cognition*, 299–306. [Canonical manuscript](https://arxiv.org/abs/1310.3781).**  Investigates how invention and imitation parameters affect cultural outputs. It is informative for exploration–copying tradeoffs but relies on researcher-defined action schemas and evaluation functions.

- **[Peer-reviewed model] Axelrod, R. (1997). “The dissemination of culture: a model with local convergence and global polarization.” *Journal of Conflict Resolution*, 41, 203–226. [https://doi.org/10.1177/0022002797041002001](https://doi.org/10.1177/0022002797041002001).**  Shows that local homophilic interaction can yield internally similar but mutually different cultural regions. It is canonical for between-group divergence and boundary dynamics, although its feature vectors are static symbolic traits rather than embodied skills or artifacts.

- **[Peer-reviewed model] Steels, L., & McIntyre, A. (1998). “Spatially distributed naming games.” *Advances in Complex Systems*, 1, 301–323. [https://doi.org/10.1142/S021952599800020X](https://doi.org/10.1142/S021952599800020X).**  Demonstrates decentralized convergence on shared labels through local interactions. It supports evolvable signal–referent association as a possible later experiment, but the game supplies communicative episodes and a structured referential problem.

- **[Book/documented system] Steels, L. (1999). *The Talking Heads Experiment, Volume 1: Words and Meanings*. Antwerp: Laboratorium. [MIT Press record](https://mitpress.mit.edu/9780262692325/the-talking-heads-experiment/).**  Documents embodied language games in which agents coordinate grounded lexicons through shared scenes. It provides a major example of emergent communication under designed interaction protocols; the protocol itself should be counted as authored scaffolding.

- **[Peer-reviewed evolutionary-robotics experiment] Floreano, D., Mitri, S., Magnenat, S., & Keller, L. (2007). “Evolutionary conditions for the emergence of communication in robots.” *Current Biology*, 17, 514–519. [https://doi.org/10.1016/j.cub.2007.01.058](https://doi.org/10.1016/j.cub.2007.01.058).**  Evolves signaling behavior under different relatedness and selection conditions in robot groups. The result shows that signals can acquire functional meaning through selection, but the task, sensory channels, and reward contingencies constrain what messages can become useful.

- **[Peer-reviewed conference paper/preprint record] Baker, B., Kanitscheider, I., Markov, T., Wu, Y., Powell, G., McGrew, B., & Mordatch, I. (2020). “Emergent tool use from multi-agent autocurricula.” In *International Conference on Learning Representations*. [Canonical manuscript](https://arxiv.org/abs/1909.07528).**  Multi-agent hide-and-seek produces striking object manipulation and sequential strategies through competition. The environment contains authored game rewards, teams, phases, and highly useful object affordances; agents do not transmit traditions across deaths or generations, so the work is evidence for emergent tool use under autocurricula, not cumulative culture.

- **[Preprint/model] Wang, R., Lehman, J., Clune, J., & Stanley, K. O. (2019). “Paired Open-Ended Trailblazer (POET): endlessly generating increasingly complex and diverse learning environments and their solutions.” [Canonical manuscript](https://arxiv.org/abs/1901.01753).**  Co-generates environments and agents while transferring solutions among niches, producing an open-ended-search mechanism. Complexity and transfer are notable, but environments are generated within an authored parameterization and there is no embodied social inheritance among organisms.

- **[Peer-reviewed digital-evolution experiment] Lenski, R. E., Ofria, C., Pennock, R. T., & Adami, C. (2003). “The evolutionary origin of complex features.” *Nature*, 423, 139–144. [https://doi.org/10.1038/nature01568](https://doi.org/10.1038/nature01568).**  Shows the evolution of a complex computational function in Avida through genetic evolution and historical contingency. It is rigorous evidence for cumulative genetic adaptation in digital organisms, not cultural inheritance, because improvements are encoded in replicating genomes and rewarded by authored computational tasks.

- **[Peer-reviewed robotics experiment] Werfel, J., Petersen, K., & Nagpal, R. (2014). “Designing collective behavior in a termite-inspired robot construction team.” *Science*, 343, 754–758. [https://doi.org/10.1126/science.1245842](https://doi.org/10.1126/science.1245842).**  Demonstrates decentralized construction through local sensing and stigmergic coordination. It establishes that persistent structures can coordinate agents without central control, but construction rules and target structures are engineered and no cultural learning occurs.

- **[Book/documented agent-based system] Epstein, J. M., & Axtell, R. (1996). *Growing Artificial Societies: Social Science from the Bottom Up*. Washington, DC/Cambridge, MA: Brookings Institution Press and MIT Press. [Publisher record](https://www.brookings.edu/books/growing-artificial-societies/).**  Introduces Sugarscape and demonstrates how migration, trade, inequality, disease, and cultural-tag patterns can emerge from local rules. It is foundational for agent-based social dynamics, but its “culture” variables are authored feature vectors rather than learned embodied practices.

- **[Book chapter/documented artificial-life system] Ray, T. S. (1991). “An approach to the synthesis of life.” In C. G. Langton, C. Taylor, J. D. Farmer, & S. Rasmussen (Eds.), *Artificial Life II*, 371–408. Redwood City, CA: Addison-Wesley. [Canonical Tierra archive](https://tomray.me/pubs/tierra/).**  Tierra demonstrates evolving self-replicating programs, ecological interactions, and parasites in a digital substrate. Its significance is open-ended evolutionary ecology; it does not provide social observation, lifetime learning, persistent artifacts, or cultural inheritance.

- **[Conference chapter/documented artificial-life system] Yaeger, L. S. (1994). “Computational genetics, physiology, metabolism, neural systems, learning, vision, and behavior—or PolyWorld: life in a new context.” In C. G. Langton (Ed.), *Artificial Life III*, 263–298. Reading, MA: Addison-Wesley. [Canonical project page](https://shinyverse.org/larryy/polyworld.html).**  PolyWorld integrates evolving neural agents, metabolism, learning, vision, mating, and ecological behavior. It is unusually relevant as a whole-organism precedent, but published demonstrations do not establish durable cumulative culture or open-ended technology.

- **[Peer-reviewed model] Williams, S., & Yaeger, L. (2017). “Evolution of neural dynamics in an ecological model.” *Geosciences*, 7, 49. [https://doi.org/10.3390/geosciences7030049](https://doi.org/10.3390/geosciences7030049).**  Analyzes evolved neural dynamics in PolyWorld-like ecological simulations. It helps frame neural-controller evolution and behavioral diversity, but it does not supply evidence for intergenerational social transmission.

- **[Peer-reviewed review] Taylor, T., Bedau, M., Channon, A., et al. (2016). “Open-ended evolution: perspectives from the OEE workshop in York.” *Artificial Life*, 22, 408–423. [https://doi.org/10.1162/ARTL_a_00210](https://doi.org/10.1162/ARTL_a_00210).**  Surveys competing definitions and research challenges for open-ended evolution. It supports defining operational criteria and acknowledging that indefinite novelty, complexity, and adaptation remain unresolved rather than assuming they will follow from scale.

- **[Peer-reviewed editorial/review] Packard, N., Bedau, M. A., Channon, A., Ikegami, T., Rasmussen, S., Stanley, K. O., & Taylor, T. (2019). “An overview of open-ended evolution: editorial introduction to the Open-Ended Evolution II special issue.” *Artificial Life*, 25, 93–103. [https://doi.org/10.1162/artl_a_00291](https://doi.org/10.1162/artl_a_00291).**  Organizes the open-ended-evolution problem and identifies limitations of existing systems. It is a direct basis for the report’s claim that open-ended technological evolution is not a solved engineering capability.

- **[Peer-reviewed theoretical/modeling paper] Guttenberg, N., Virgo, N., & Penn, A. S. (2019). “On the potential for open-endedness in neural networks.” *Artificial Life*, 25, 145–167. [https://doi.org/10.1162/artl_a_00286](https://doi.org/10.1162/artl_a_00286).**  Examines how representational and training assumptions can constrain novelty in neural systems. It is relevant to evolvable neural controllers, but potential for open-endedness is not evidence that culture or technology will emerge in an ecological simulation.

- **[Peer-reviewed review] Theraulaz, G., & Bonabeau, E. (1999). “A brief history of stigmergy.” *Artificial Life*, 5, 97–116. [https://doi.org/10.1162/106454699568700](https://doi.org/10.1162/106454699568700).**  Reviews coordination through environmental traces and modifications. It provides the conceptual basis for persistent artifacts as public state, while warning that stigmergic coordination does not necessarily involve observation, imitation, or culture.

- **[Peer-reviewed agent-based model] Acerbi, A., & Parisi, D. (2006). “Cultural transmission between and within generations.” *Journal of Artificial Societies and Social Simulation*, 9(1), 9. [Stable publication page](https://www.jasss.org/9/1/9.html).**  Investigates interactions between within-generation and intergenerational learning in artificial agents. It is directly relevant to separating lifetime acquisition from inherited predispositions, but the cultural task and transmission architecture are modeled abstractions.

- **[Peer-reviewed model] Vogt, P., & Haasdijk, E. W. (2010). “Modelling social learning of language and skills.” *Artificial Life*, 16, 289–309. [https://doi.org/10.1162/artl_a_00007](https://doi.org/10.1162/artl_a_00007).**  Compares mechanisms for socially learning linguistic and practical behaviors in artificial agents. It is useful for mechanism-level contrasts, but its learning episodes and target domains are explicitly constructed.

- **[Peer-reviewed evolutionary-robotics model] Nolfi, S. (2005). “Emergence of communication in embodied agents: co-adapting communicative and non-communicative behaviours.” *Connection Science*, 17, 231–248. [https://doi.org/10.1080/09540090500177554](https://doi.org/10.1080/09540090500177554).**  Demonstrates coevolution of signaling and task behavior in embodied agents. It supports allowing signals to acquire meaning only through their consequences, while its designed selection task limits claims about semantic or cultural open-endedness.

## 18.7 External memory, niche construction, learning stability, and failure analysis

- **[Book] Odling-Smee, F. J., Laland, K. N., & Feldman, M. W. (2003). *Niche Construction: The Neglected Process in Evolution*. Princeton, NJ: Princeton University Press. ISBN 9780691044378.**  Develops the theory that organisms modify selective environments and inherit ecological legacies. It is the principal theoretical basis for treating structures, paths, caches, and altered materials as persistent causal state rather than decorative world history.

- **[Peer-reviewed philosophy article] Clark, A., & Chalmers, D. J. (1998). “The extended mind.” *Analysis*, 58, 7–19. [https://doi.org/10.1093/analys/58.1.7](https://doi.org/10.1093/analys/58.1.7).**  Argues that reliably integrated external resources can function as components of cognition. The report uses this as a conceptual analogy for tools and structures as external memory, not as proof that every persistent artifact is literally cognitive.

- **[Peer-reviewed review] French, R. M. (1999). “Catastrophic forgetting in connectionist networks.” *Trends in Cognitive Sciences*, 3, 128–135. [https://doi.org/10.1016/S1364-6613%2899%2901294-2](https://doi.org/10.1016/S1364-6613%2899%2901294-2).**  Reviews why sequential neural learning can overwrite earlier capabilities. It motivates explicit retention tests and memory-capacity sweeps, because a culture cannot ratchet if individual learners repeatedly erase the component skills on which later variants depend.

- **[Peer-reviewed machine-learning experiment] Kirkpatrick, J., Pascanu, R., Rabinowitz, N., et al. (2017). “Overcoming catastrophic forgetting in neural networks.” *Proceedings of the National Academy of Sciences*, 114, 3521–3526. [https://doi.org/10.1073/pnas.1611835114](https://doi.org/10.1073/pnas.1611835114).**  Introduces elastic weight consolidation as one approach to preserving previously learned tasks. Genesis need not adopt this algorithm, but the work provides a clear warning and a comparison class for stability–plasticity tradeoffs.

- **[Peer-reviewed model] Hinton, G. E., & Nowlan, S. J. (1987). “How learning can guide evolution.” *Complex Systems*, 1, 495–502. [Stable publication page](https://www.complex-systems.com/abstracts/v01_i03_a06/).**  Demonstrates the Baldwin effect in a stylized genetic-search model: learning can reshape the evolutionary search landscape and later permit genetic assimilation. It motivates measuring whether formerly cultural behavior becomes genetically available and whether that removes dependence on social transmission.

- **[Preprint/perspective] Amodei, D., Olah, C., Steinhardt, J., Christiano, P., Schulman, J., & Mané, D. (2016). “Concrete problems in AI safety.” [Canonical manuscript](https://arxiv.org/abs/1606.06565).**  Catalogues specification gaming, unsafe exploration, distribution shift, and related failure classes. Its relevance is methodological: authored reward proxies can create impressive but invalid “progress,” so Genesis should prefer ecological consequences and audit every selection signal.

- **[Peer-reviewed review] Laland, K. N., Atton, N., & Webster, M. M. (2011). “From fish to fashion: experimental and theoretical insights into the evolution of culture.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 366, 958–968. [https://doi.org/10.1098/rstb.2010.0328](https://doi.org/10.1098/rstb.2010.0328).**  Reviews how social learning strategies interact with ecology and population structure. It supports conditional copying and cautions that indiscriminate social learning can be maladaptive.

- **[Peer-reviewed review] Rendell, L., Fogarty, L., Hoppitt, W. J. E., Morgan, T. J. H., Webster, M. M., & Laland, K. N. (2011). “Cognitive culture: theoretical and empirical insights into social learning strategies.” *Trends in Cognitive Sciences*, 15, 68–76. [https://doi.org/10.1016/j.tics.2010.12.002](https://doi.org/10.1016/j.tics.2010.12.002).**  Synthesizes evidence that learners use state-, model-, and frequency-based social-learning strategies. It motivates making demonstrator selection and attention observable, evolvable, and measurable rather than granting uniform access to a population-wide cultural database.

- **[Peer-reviewed theoretical article] Laland, K. N. (2004). “Social learning strategies.” *Learning & Behavior*, 32, 4–14. [https://doi.org/10.3758/BF03196002](https://doi.org/10.3758/BF03196002).**  Frames the adaptive problem as when, what, and whom to copy. This supports experiments in which observation has an opportunity cost and social information varies in reliability.

- **[Peer-reviewed review] Smolla, M., Jansson, F., Lehmann, L., et al. (2021). “Underappreciated features of cultural evolution.” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 376, 20200259. [https://doi.org/10.1098/rstb.2020.0259](https://doi.org/10.1098/rstb.2020.0259).**  Highlights reconstruction, transformation, networks, and non-copying processes that are often lost in simple cultural-transmission models. It supports modeling learning as state-dependent transformation rather than assuming exact trait duplication.

- **[Peer-reviewed review] Buskell, A., Enquist, M., & Jansson, F. (2019). “A systems approach to cultural evolution.” *Palgrave Communications*, 5, 131. [https://doi.org/10.1057/s41599-019-0343-5](https://doi.org/10.1057/s41599-019-0343-5).**  Argues for analyzing culture as interacting developmental, ecological, and population processes. This is closely aligned with Genesis’s aim to study causal affordances rather than isolate a disembodied culture module.

- **[Peer-reviewed theoretical article] Claidière, N., Scott-Phillips, T. C., & Sperber, D. (2014). “How Darwinian is cultural evolution?” *Philosophical Transactions of the Royal Society B: Biological Sciences*, 369, 20130368. [https://doi.org/10.1098/rstb.2013.0368](https://doi.org/10.1098/rstb.2013.0368).**  Examines where cultural change resembles selection among copied variants and where reconstructive attraction is a better description. It supports recording transformation processes rather than forcing all cultural lineages into a genetic analogy.

- **[Peer-reviewed review] Kendal, R. L., Boogert, N. J., Rendell, L., Laland, K. N., Webster, M., & Jones, P. L. (2018). “Social learning strategies: bridge-building between fields.” *Trends in Cognitive Sciences*, 22, 651–665. [https://doi.org/10.1016/j.tics.2018.04.003](https://doi.org/10.1016/j.tics.2018.04.003).**  Integrates theoretical and empirical work on selective social learning. It is a useful guide for parameterizing attention costs, uncertainty, model quality, and conformity without granting agents an omniscient rule such as “copy the best.”

- **[Peer-reviewed review] Heyes, C. (2016). “Who knows? Metacognitive social learning strategies.” *Trends in Cognitive Sciences*, 20, 204–213. [https://doi.org/10.1016/j.tics.2015.12.007](https://doi.org/10.1016/j.tics.2015.12.007).**  Reviews how uncertainty monitoring can regulate reliance on social information. For Genesis this is a later, optional mechanism; early experiments can approximate it with accessible confidence or prediction-error state rather than a symbolic theory of mind.

- **[Peer-reviewed model] Enquist, M., Eriksson, K., & Ghirlanda, S. (2007). “Critical social learning: a solution to Rogers’ paradox of nonadaptive culture.” *American Anthropologist*, 109, 727–734. [https://doi.org/10.1525/aa.2007.109.4.727](https://doi.org/10.1525/aa.2007.109.4.727).**  Models a strategy that first copies and then individually evaluates or corrects acquired behavior. It motivates combining observation with learner exploration rather than treating social and individual learning as mutually exclusive modes.

## 18.8 Software, code, and data records

- **[Software repository] Dalmaijer, E. S. *Artificial Navigators*. [https://github.com/esdalmaijer/artificial_navigators](https://github.com/esdalmaijer/artificial_navigators).**  Canonical source repository for the artificial-navigation model associated with the 2024 *PLOS Biology* paper. It enables inspection of implementation choices and replication; the repository is supporting material, not an independent empirical result.

- **[Data repository] Dalmaijer, E. S. (2022). *Artificial navigators data archive*. Zenodo. [https://doi.org/10.5281/zenodo.6944185](https://doi.org/10.5281/zenodo.6944185).**  Archived output data supporting analysis of cumulative route improvement. Its presence improves reproducibility and illustrates the seed-level data practice recommended for Genesis.

- **[Software archive] Dalmaijer, E. S. (2024). *Artificial navigators code archive*. Zenodo. [https://doi.org/10.5281/zenodo.10997495](https://doi.org/10.5281/zenodo.10997495).**  Versioned archival release of the model code. A citable code snapshot is preferable to relying only on a mutable branch and provides a model for Genesis releases tied to reported experiments.

- **[Software repository] FLAIR. *Cultural Accumulation*. [https://github.com/FLAIROx/cultural-accumulation](https://github.com/FLAIROx/cultural-accumulation).**  Repository associated with artificial generational intelligence and cultural-accumulation experiments. It is useful for auditing the exact training and transfer interfaces that are compressed by the phrase “cultural transmission.”

- **[Software/document archive] Ray, T. S. *Tierra publications and software archive*. [https://tomray.me/pubs/tierra/](https://tomray.me/pubs/tierra/).**  Canonical materials for Tierra. They provide historical implementation context for digital ecology and open-ended-evolution research, but do not add a cultural channel absent from the system itself.

- **[Software/document archive] Yaeger, L. S. *PolyWorld project archive*. [https://shinyverse.org/larryy/polyworld.html](https://shinyverse.org/larryy/polyworld.html).**  Canonical documentation and source materials for PolyWorld. It is useful for examining integrated neural, ecological, reproductive, and learning mechanics that are often omitted from simplified summaries.

- **[Preprint and implementation record] Baker, B., et al. *Emergent Tool Use From Multi-Agent Autocurricula*. [https://arxiv.org/abs/1909.07528](https://arxiv.org/abs/1909.07528).**  The manuscript and linked implementation materials document environment details that are essential for deciding which tool-use affordances and curricula were authored. Visual demonstrations should be interpreted through these mechanics rather than used as stand-alone evidence.

- **[Preprint and implementation record] Wang, R., Lehman, J., Clune, J., & Stanley, K. O. *POET*. [https://arxiv.org/abs/1901.01753](https://arxiv.org/abs/1901.01753).**  The canonical manuscript describes the environment-generation and transfer algorithm. It is relevant as a comparator for open-ended search, not a drop-in cultural system for embodied organisms.

## 18.9 Bibliographic synthesis

Across this literature, the most reproducible findings are narrower than the project’s ultimate ambition:

1. **Arbitrary traditions can spread and survive turnover** when social exposure is sufficiently structured, as shown especially by seeded field experiments.
2. **Finite cumulative improvement can occur without language, teaching, or full causal understanding** in several human, animal, and computational tasks.
3. **High-fidelity exact action imitation is not universally necessary**, because outcome emulation, reconstruction, collective averaging, environmental traces, and repeated selection can preserve useful change.
4. **Demography matters through access, diversity, turnover, and network topology**, but population size alone does not guarantee complexity.
5. **Persistent environmental state can coordinate behavior and preserve information**, but stigmergy and niche construction are not automatically culture.
6. **Existing artificial systems demonstrate pieces of the target**—communication, conventions, direct policy transfer, construction, tool use, genetic complexity, or narrow cultural ratchets—but no cited system establishes unscripted, open-ended technological evolution across generations.

The defensible engineering response is therefore an experimental platform that can make transmission, retention, transformation, artifact dependence, genetic assimilation, and cumulative functional dependence separately observable. The report’s proposed minimum system and dependency-ordered experiments are designed to produce interpretable null results as well as positive ones.
