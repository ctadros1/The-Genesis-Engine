# Codex Operating Guide

## Start A Work Session

1. Read README.md, AGENTS.md, and this file.
2. Read the active phase plan and its dependencies.
3. Review docs/22-decision-log.md, docs/21-open-questions.md, and relevant specifications.
4. Inspect the working tree and run existing focused tests.
5. Write a short implementation plan before edits unless the task is a trivial, isolated correction.

## Identify The Active Phase

The current project status is declared in README.md and planning/backlog.md. Do not begin the next phase until the previous phase acceptance criteria, documentation updates, and benchmark gates are satisfied or explicitly waived in a decision record.

## Choosing The Next Task

Choose the smallest backlog item that:

- advances an accepted phase deliverable,
- has explicit acceptance criteria,
- does not require unresolved high-risk decisions,
- can be validated with current tools, and
- does not widen deployment scope accidentally.

Prefer a thin vertical slice over a framework-first buildout. Do not implement later ecosystem, GPU, database, or public-access features early.

## Skills And Research: Consult Before Designing

This repository carries two evidence assets that are **not optional
background**. Before designing any complex part of the engine, consult both.

### The six deep-research reviews

Indexed in `research/deep-research-index.md`, carried in-repo under
`.agents/skills/genesis-*/references/`, covering neuroevolution and lifetime
learning, artificial genetics and lineages, cumulative culture, mutable
worlds and tool use, social organization and conflict, and open-ended-
evolution methodology.

Consult the matching review **before** writing a specification, an
acceptance criterion, or an ADR for:

- controller topology, evaluation order, plasticity, learned-state
  persistence;
- genome encoding, inheritance, crossover, structural mutation, identity;
- any claim of transmission, tradition, or accumulation;
- objects, materials, structures, terrain mutation, save format;
- perception of conspecifics, recognition, grouping, aggression, territory;
- acceptance criteria, seed counts, controls, claim wording.

ADR-0022 is the record of what happens when this is skipped: the plan was
written first, and correcting it against the reviews reversed two phase
orderings, removed two scripted perception channels, and tripled every seed
count.

### The project skills

`.claude/skills/` provides both the six `genesis-*` skills above and general
method skills. Use them rather than improvising:

| Skill | Use for |
|---|---|
| `experimental-design` | Choosing controls, ablations, blocking, and avoiding pseudoreplication |
| `statistical-power` | Seed counts, minimum detectable effect, power analysis |
| `statistical-analysis` | Effect sizes, intervals, equivalence tests, reporting |
| `hypothesis-generation` | Turning an observation into a falsifiable claim with rival explanations |
| `scientific-critical-thinking` | Grading evidence, finding confounders in a proposed criterion |
| `agent-based-modeling` | Simulation structure and emergence questions |
| `property-based-testing`, `cargo-fuzz` | Codec, genome, and protocol boundary testing |
| `rust-skills` | Kernel implementation conventions |

### The boundary

Reviews and skills are **evidence and method, not authority**. Where one
contradicts a recorded project decision, resolve it in an ADR with the
reasoning written down, as ADR-0022 does. Never silently follow a review
against a decision, and never silently ignore one. A finding in a review is
a reason to design something a particular way; it is never a substitute for
measuring it in this system.

## Validation

Run format/lint/type checks, targeted tests, deterministic tests where relevant, and phase-required tests. For observer work, validate desktop and mobile viewports and avoid assuming one rendering backend. For persistence or protocol changes, add compatibility/negative tests.

## Documentation Discipline

Whenever implementation changes behavior or architecture:

1. Update the affected docs/ and specifications/ documents.
2. Add or revise a proposed ADR.
3. Update the decision log and benchmark record.
4. Explain replay, migration, API, and deployment impact in the completion report.

## Safe Stop Conditions

Stop and ask for clarification when a choice affects persistent data compatibility, experimental interpretation, access control, production infrastructure, or requires secrets. Otherwise document a bounded assumption and continue.

## Preventing Context Loss

At the end of a meaningful session, leave the active phase, completed acceptance criteria, test evidence, benchmark evidence, changed invariants, and next smallest task in planning/backlog.md or the decision log. Do not rely on chat history as project memory.
