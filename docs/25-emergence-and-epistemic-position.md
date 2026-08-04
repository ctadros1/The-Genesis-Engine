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

- Organized inter-group conflict arising from scarcity plus kin-biased
  grouping.
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
| Territoriality and organized inter-group conflict | Likely. Scarcity plus kin-biased grouping produces this in many models, and it does not depend on the culture stack. |
| Tool use in the weak sense (carry and apply an object) | Plausible. Requires only that carrying pay off somewhere in the physics. |
| Persistent structures that alter the landscape | Plausible. |
| Behavioral traditions outliving individuals | Plausible to remarkable. Depends entirely on whether transmission fidelity clears the accumulation threshold. This is the single most likely place for a null result. |
| Cumulative technological accumulation | Remarkable if observed. We do not plan around it. |
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

> "Transmission occurs in 8 of 12 seeds under condition A and 0 of 12 under
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
