# Emergence And Epistemic Position

## Why This Document Exists

The project's long-term ambition changed on 2026-08-04. Earlier documents
disclaimed complex behavior outright. That disclaimer conflated three
different things, and the conflation made the project's actual position
impossible to state honestly. This document separates them and is the
reference every other document defers to.

The three things are:

1. **What the simulation makes possible.** A property of the authored
   physics. Verifiable by reading the code.
2. **What we hope to observe.** A research aspiration. Neither a promise nor
   a prediction.
3. **What we predict will actually happen.** A falsifiable expectation, with
   the honest prior attached.

Confusing (1) with (3) produces hype. Confusing (3) with (1) produces the old
text, which described an ambition ceiling as if it were a design limit.

## The Governing Philosophy: Author Physics, Not Progress

We author the possibility space. We do not author the trajectory through it.

The simulation defines what is physically possible: that stone is hard and
can be struck, that an object can be held, carried, placed, and combined,
that a placed object persists in the world, that a signal can be emitted and
perceived, that one organism's actions are visible to another, that a
synapse can change during a lifetime and that the rule governing that change
is inherited.

The simulation never defines a technology tree, a research prerequisite
graph, an era, a building recipe, a civilization stage, a tool category, or
a reward for inventing anything. There is no list of things an organism can
discover. There is a physics, and a search process, and whatever the search
process finds inside the physics.

### The line, stated operationally

| Authored (physics) | Never authored (progress) |
|---|---|
| Materials have hardness, mass, durability, decay | Which material combinations are "useful" |
| Objects can be combined; composite properties derive from constituents | A recipe list, a crafting table, a tech tree |
| A signal can be emitted on N channels and perceived within a radius | What any signal means |
| A synapse can be marked plastic and change under an inherited rule | What the organism should learn |
| A modulatory neuron can gate plasticity | What counts as reward |
| Damage can be dealt and received | Who is an enemy; what a war is |
| Genomes can gain and lose nodes and edges by duplication and deletion | A target architecture or complexity budget to reach |
| Events are recorded | What era the world is in |

### Starting conditions are authored; trajectories are not

A world may begin from bounded-random founders, from biome-matched founder
archetypes, or from a chemistry field with no organisms at all
(`specifications/world-origin-modes.md`). All three are admissible because
**authoring where the search begins is not authoring the path it takes.**
A seeded world is then subject to exactly the same rules as a random one;
nothing about its starting point constrains, guides, or rewards what happens
next.

Two guards keep that line real rather than nominal:

- **Archetypes are distributions, not organisms**, and their names are
  presentation only. No rule, input channel, mating gate, or analysis
  grouping may read an archetype ID, and a test asserts inertness by
  permuting the IDs and requiring identical trajectories. No archetype is
  named after a real species.
- **A seeded run is a weaker basis for a reachability claim** than a random
  one, because the starting point was chosen. Reports state which mode
  produced them, and "behavior X evolved" means something different under
  each.

### The one amendment: scaffolding, and what it costs

ADR-0018 permits deliberately shaping **environmental and selective
structure** toward the major evolutionary transitions in the `scratch` mode.
This fails the "can you name the outcome" test above, by design, and it is
recorded as an explicit bounded exception rather than absorbed by
reinterpretation.

Still forbidden, unchanged: rewarding the target trait, granting it, or any
recipe, stage, or grade. A scaffold must be describable **without naming its
target** ("patchy resources with high between-patch variance and a dispersal
cost", not "conditions that favor multicellularity").

The honest cost: **every claim from the scaffolded phases is weaker than an
unscaffolded one would have been.** Each requires an unscaffolded control on
the same seeds, and the reportable quantity is the difference between
conditions rather than the scaffolded result alone. The realistic risk is
not that the rule gets broken once; it is that the word "scaffolded" quietly
disappears from a summary three documents downstream.

*Status of the from-scratch prediction (2026-09-02, D-128, D-131).* The
chain is now measured link by link. Chemistry to a persistent microbial
field: reached, and at the ceiling in the neutral field, not only the
scaffolded ones (Phase 15). Field to individual organisms: reached in
every world with the transition enabled and in none without it (Phase
16) - a representation change at a physical threshold, with no organism
gaining anything by crossing it (the neutrality tests). Individual to
multicellular: **not reached** - the pre-registered null - and the reason
is measured rather than guessed: a materialized unicell is a slow,
blind gut that cannot eat the substrate it condensed from (coupling v1),
so it starves before maturity and reproduces on the order of once per
thousand arrivals; structural evolution had almost nothing to act on.
That is the honest prior this document has always stated, now with a
number on it, and the next lever is physics (a mouth for the field,
Phase 19), not a threshold.

### The corollary: analysis observes, it never instructs

This project already holds this line for similarity clustering
(`lifesim-similarity-v1`): species labels are computed offline, recorded in
reports, and never fed back into mating rules, action eligibility, or
rendering-driven behavior. Era detection, tradition detection, and every
future analysis obey the same rule, enforced structurally rather than by
convention. See ADR-0016 and
`specifications/era-and-tradition-detection.md`.

An "era" is a narrative an observer detects post hoc from the event log. It
is never a state the simulation enters, and never something an organism is
told about. If an era boundary is visible in a chart, that fact must have had
zero effect on the tick that produced it, and a test must assert exactly
that: state checksums are bit-identical with analysis enabled and disabled.

### Genetic compatibility is physics, not a label

One subtle case deserves its own statement, because it looks like a
violation and is not. Reproduction may be gated on genetic compatibility
computed directly from two genomes (Phase 2 already does this with
`compatibility_threshold_q16`). That is a physical fact about whether two
records can recombine, derived from the records themselves. It is not an
analysis-assigned cluster label. Cluster labels never gate anything. The
test that distinguishes them: if you deleted the analysis module entirely,
compatibility gating would still work identically, and clustering would not
exist.

## 1. What The Simulation Is Being Built To Make Possible

Stated as physics, not as outcomes. After the phases in
`docs/19-implementation-roadmap.md` land, the possibility space includes:

- Controllers whose topology can grow and shrink across generations, so new
  capability does not require a schema bump by a human.
- Synaptic change within a single lifetime, under a rule that is itself a
  gene under selection.
- Perception of what another organism just did, and a costly local signal
  channel with no authored meaning.
- Materials with physical properties; holding, carrying, placing, striking,
  and combining; composite objects whose properties derive from constituents.
- Persistent world modification that outlives the organism that made it.
- Damage, resource contention, and spatial grouping.

Whether any of that is *used* is not a property of the physics. It is an
empirical question, and the whole point of the instrument.

## 2. What We Hope To Observe

In rough order of decreasing plausibility:

- Organized inter-group conflict. Note the correction in ADR-0022 A1: the
  shortcut "scarcity plus kin bias plus damage implies war" is withdrawn.
  Organized conflict needs recognition, memory, coalition recruitment, and
  capturable value, so it is a Phase 13 question rather than a Phase 7 one.
- Tool use: an object acquired, retained, and applied to a task with a
  measurable fitness effect.
- Persistent structures: placed objects that outlast their makers and change
  the fitness landscape of the cells they occupy.
- Behavioral traditions: a behavioral variant that persists across more than
  three median lifespans in a local group and is not explained by the local
  genotype distribution.
- Cumulative dependency: composite artifacts of combination depth two or
  more increasing in frequency over time, that is, later constructions
  depending on earlier ones.

## 3. What We Actually Predict, With The Honest Prior

**Open-ended evolution is an unsolved grand challenge in artificial life.**
No system has produced a technological era progression from genuine
evolution. Systems that display something resembling technological
accumulation get it from an authored progression structure; systems with
genuinely open search have not produced it. We are not aware of a
counterexample, and we should not describe this project as one before it is.

Given that, our predictions:

| Outcome | Prediction |
|---|---|
| Territoriality and organized inter-group conflict | Plausible, and **downgraded from "likely"**. The commissioned review rejects the scarcity-plus-kin-bias shortcut and lists roughly eleven further dependencies, most of which are Phase 11 to 13 machinery. It is no longer the cheap early win the plan assumed (ADR-0022 A1). |
| Tool use in the weak sense (carry and apply an object) | Plausible. Requires only that carrying pay off somewhere in the physics. |
| Persistent structures that alter the landscape | Plausible. |
| Behavioral traditions outliving individuals | Plausible to remarkable. Depends entirely on whether transmission fidelity clears the accumulation threshold. This is the single most likely place for a null result. |
| Cumulative technological accumulation | Remarkable if observed. We do not plan around it. |
| Morphological change under selection | Plausible. Phase 10 gives structure a real morphospace, and whether selection can act across a discontinuous developmental encoding is itself measured (C10.4). |
| Abiogenesis producing a persistent population | Remarkable if observed unscaffolded; expected to require scaffolding, and expected to return null under a neutral chemistry. Recorded in advance. |
| A unicell-to-differentiated-multicell transition | **Remarkable.** This is one of the least tractable problems in artificial life. The plan is built so a null here is a measurement rather than a surprise. |
| A microbes-to-fish-to-reptiles progression | **Not promised, and not a sequence the simulation contains.** There is no ladder, no grade, and no stage. The arrow in that phrase is a hypothesis being tested, never a mechanism being executed. Reaching anything fish-like is not planned around. |
| A recognizable stone-age-to-enlightenment arc | **Not planned around and not promised.** |
| Language | **Not planned around and not promised.** A signal channel is not a language, and we will not call it one. |
| Civilization | **Not planned around and not promised.** No civilization mechanic will ever be authored; see the table above. |

A null result is a likely outcome of several phases and is an acceptable
outcome of all of them. Phases are designed so that a null result is
*informative* rather than merely disappointing: each has a stated ablation,
so "transmission did not occur" is a measurement, not a shrug.

## Why This Project Can Say Things Most ALife Projects Cannot

The value of this project does not depend on reaching the ceiling. It rests
on the discipline already in place before the ambition changed.

The kernel is deterministic under a declared policy (ADR-0010, ADR-0011).
Runs are keyed by seed and a canonical config hash. Fixtures are preserved
across four phases and verified from clean processes
(`0x1e3158a26afd3b39` at Phase 1, `0xff9dfcff5dffbf42` at Phase 2). Every
behavioral change creates a new named policy version and a new replay
lineage instead of silently redefining the old one. Benchmarks carry
provenance and are never relabeled. Saves restore bit-identically and
continue identically.

That machinery is exactly what turns an anecdote into a measurement. It lets
this project make claims of the form:

> "Transmission occurs in 20 of 30 seeds under condition A and 0 of 30 under
> condition B, where B differs from A only in that the signal channel is
> delivered to a randomly chosen receiver rather than to the neighbours in
> range. Config hashes, seeds, and per-seed checksums are recorded."

instead of claims of the form:

> "We saw something interesting once."

Most artificial-life results are the second kind, not because their authors
are careless but because the infrastructure needed for the first kind is
expensive and has to exist before the interesting run happens. Here it
already exists. Every phase from here on is required to state its ablation
before it is implemented, and "it looked interesting" is never an acceptance
criterion.

That is the claim the project should make about itself, and it is true today.

## What Stays Permanently Out Of Scope

These survive the change of goal unchanged and are not reopened by it:

- **No large language model, or any language model, as an organism decision
  engine.** Optional narration may consume recorded events after the fact and
  may never influence a tick.
- **No authored technology tree, recipe list, research graph, era state, or
  civilization mechanic.** See the philosophy table.
- **No reinforcement learning against a hand-authored reward.** Lifetime
  learning is permitted; the signal that gates it must be an evolved output
  of the organism's own network, never a fitness function supplied by us.
  See ADR-0014.
- **No analysis output feeding back into simulation state.** See ADR-0016.
- **No claim of scale, cross-platform determinism, or GPU value without a
  recorded benchmark.**

## Related Documents

- Scope: `02-scope-and-non-goals.md`
- Biological realism policy: `26-biological-realism-policy.md`
- Roadmap and dependency order: `19-implementation-roadmap.md`
- ADR-0012 (philosophy), ADR-0016 (analysis boundary)
