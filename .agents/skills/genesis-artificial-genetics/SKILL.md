---
name: genesis-artificial-genetics
description: Expert guidance for designing, reviewing, and implementing artificial genome systems, inheritance, mutation, recombination, lineage tracking, and evolutionary experiments in The Genesis Engine. Use when working on genetics architecture, reproduction, genome schemas, mutation operators, ancestry systems, evolutionary experiments, or related simulation infrastructure.
---

# Genesis Artificial Genetics Skill

## Purpose

This skill converts artificial-life genetics research into engineering guidance for The Genesis Engine. It helps AI assistants design evolvable but deterministic genome systems without assuming that complexity, technology, culture, or intelligence will emerge.

The skill is based on the supporting research document in `references/`.

## Activate when

Use this skill when tasks involve:

- Artificial genome design.
- Neural controller evolution.
- Variable topology genomes.
- Mutation and recombination systems.
- Sexual or paired reproduction.
- Gene, module, chromosome, or lineage schemas.
- Duplication, deletion, or innovation tracking.
- Plasticity and neuromodulation inheritance.
- Species, relatedness, or ancestry analysis.
- Evolution experiments involving genetic mechanisms.
- Reviewing proposals for evolutionary architecture.

Do not activate this skill for ordinary software features unrelated to Genesis genetics.

## Genesis Engine principles

All recommendations must preserve:

1. Author physics and affordances, not progress.
2. Emergence must result from mechanisms, not scripted outcomes.
3. Offline analysis observes; it never controls organisms.
4. No LLM or external intelligence acts as an organism controller.
5. Determinism is a first-class requirement.
6. Every stochastic process requires named deterministic RNG streams.
7. Serialization, replay, migration, and versioning are part of system design.
8. Evolutionary claims must be measurable and experimentally testable.

## Core architectural guidance

Prefer a deterministic tagged modular graph genome approach:

- Stable historical identities for loci.
- Separate identifiers for:
  - historical locus identity,
  - innovation origin,
  - duplication family,
  - current structural similarity.
- Modules as explicit organizational units.
- Direct graph representations for inspectable controllers.
- Optional future developmental/generative modules with strict bounds.

Do not collapse:

- ancestry and similarity,
- observer classifications and causal mechanisms,
- genome identity and organism identity,
- learned lifetime state and inherited genotype.

## Genome design rules

When reviewing genome proposals, check:

### Identity

Require separate concepts:

- OrganismId.
- GenomeContentId.
- GenomeTransmissionId.
- ReproductionEventId.
- LocusId.
- InnovationId.
- GeneFamilyId.
- StructuralSignature.

A structural match is not proof of common ancestry.

### Mutation

Prefer:

- valid-by-construction mutation,
- deterministic candidate selection,
- transactional mutation,
- explicit rejection reasons,
- complete mutation event records.

Avoid:

- heuristic repair,
- arbitrary "closest" fixes,
- hidden cleanup,
- unordered traversal decisions.

### Recombination

Treat these as separate:

- paired reproduction,
- biparental inheritance,
- assortment,
- crossover,
- mutation.

Do not assume crossover is always beneficial.

Preferred progression:

1. Clonal baseline.
2. Module-level inheritance.
3. Homology-aware crossover.
4. Experimental rearrangement operators.

### Duplication

A duplication should:

- create new locus identities,
- preserve family ancestry,
- record source lineage,
- copy structure deterministically,
- enforce size limits.

Duplication is an opportunity for divergence, not guaranteed innovation.

### Developmental encodings

Treat developmental programs, grammars, CPPNs, and regulatory systems as optional advanced modules.

Require:

- deterministic execution,
- hard expansion limits,
- provenance mapping,
- semantic versioning,
- fail-closed behavior.

Never introduce unrestricted executable genomes.

## Determinism checklist

For every genetics feature ask:

- Are RNG streams named?
- Are random draws independent?
- Is candidate ordering canonical?
- Are thread schedules irrelevant?
- Are IDs deterministic?
- Are hashes versioned?
- Are floating point assumptions removed or controlled?
- Are save and replay semantics preserved?
- Does migration have explicit rules?

## Experiment guidance

Never conclude that a mechanism works from one impressive run.

Require:

- independent seeds,
- controls and ablations,
- measurable metrics,
- reproducible checkpoints,
- documented policy versions.

Useful comparisons:

- mutation-only vs recombination,
- clonal vs paired inheritance,
- direct vs indirect encoding,
- duplication enabled vs disabled,
- plasticity enabled vs disabled,
- observer-only analysis vs causal mechanisms.

## Common anti-patterns

Reject designs that:

- Add technology trees or civilization stages.
- Feed species labels back into behavior.
- Use genome length as a complexity metric.
- Assume bigger neural networks are better.
- Treat tags as true kinship.
- Use observer clusters as organism inputs.
- Use process-global counters for evolution.
- Depend on hash-map ordering.
- Silently migrate old genomes.
- Copy learned state into offspring without an explicit experiment.
- Add biological mechanisms only because they exist in nature.

## Review framework

When reviewing a genetics proposal:

1. Define the causal mechanism.
2. Identify what organisms can perceive.
3. Identify what is heritable.
4. Identify what is lifetime state.
5. Define deterministic execution.
6. Define serialization.
7. Define migration behavior.
8. Define measurable outcomes.
9. Define ablations.
10. Identify unsupported assumptions.

## Genesis Engine relevance

Supports:

- Future genome successor work.
- Evolvable neural topology.
- Lifetime learning architecture.
- Social transmission prerequisites.
- Artifact and object interaction prerequisites.
- Long-running evolutionary experiments.
- Lineage and ancestry infrastructure.

Interacts with:

- deterministic simulation kernel,
- persistence system,
- checkpoint system,
- replay system,
- neural controllers,
- reproduction mechanics,
- observer analytics.

Does not decide:

- world physics,
- ecology,
- organism goals,
- cultural outcomes,
- technology emergence,
- final evolutionary mechanisms.

## Limitations

This skill does not prove that:

- open-ended evolution will occur,
- cumulative culture will emerge,
- intelligence will evolve,
- technology will appear,
- any genome architecture is universally superior.

Research evidence comes from specific biological, computational, and artificial-life systems. Recommendations are engineering judgments adapted for Genesis constraints.

Validation must occur through Genesis experiments.

