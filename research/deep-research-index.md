# Deep Research Index

Six commissioned engineering-scientific reviews, delivered 2026-08-04. They
are the primary evidence base for the Phase 5 to 18 design and are the first
thing to consult before designing any complex part of the engine.

**Read these before designing, not after.** ADR-0022 exists because the plan
was written first and then had to be corrected against them: two orderings
were wrong, two perception channels were scripted, and every seed count was
under-powered. That correction cost a full revision pass and would have been
avoided by consulting the reviews first.

## Where they live

Each review is carried in the repository inside its matching skill, so a
clone has both the distilled guidance and the full source:

| Topic | Skill | Full review |
|---|---|---|
| Neuroevolution, lifetime learning, plasticity, replay | `genesis-neuroevolution` | `.agents/skills/genesis-neuroevolution/references/genesis_neuroevolution_and_lifetime_learning.md` |
| Genome representation, meiosis, lineage, mutation validity | `genesis-artificial-genetics` | `.agents/skills/genesis-artificial-genetics/references/genesis_artificial_genetics_and_lineages_full.md` |
| Social learning, traditions, cumulative culture, technology | `genesis-cumulative-culture` | `.agents/skills/genesis-cumulative-culture/references/genesis_cumulative_culture_and_technology.md` |
| Objects, affordances, tool use, mutable terrain, persistence | `genesis-mutable-world-tool-use` | `.agents/skills/genesis-mutable-world-tool-use/references/genesis_mutable_world_artifacts_and_tool_use.md` |
| Grouping, recognition, cooperation, territory, conflict | `genesis-social-organization-territory-conflict` | `.agents/skills/genesis-social-organization-territory-conflict/references/genesis_social_organization_territory_and_conflict.md` |
| Open-ended evolution, experimental method, replication, scale | `genesis-experimental-methodology` | `.agents/skills/genesis-experimental-methodology/references/genesis_open_ended_evolution_experiments_and_scale.md` |

Invoke the skill for guidance; open the review for the argument, the
evidence grade, and the citations behind it.

## Which review governs which decision

| Designing... | Consult |
|---|---|
| Controller topology, evaluation order, plasticity rules, learned-state persistence | neuroevolution |
| Genome encoding, inheritance modes, crossover, structural mutation, identity fields | artificial-genetics |
| Anything that claims transmission, tradition, or accumulation | cumulative-culture |
| Materials, objects, carrying, structures, terrain mutation, save format | mutable-world-tool-use |
| Perception of conspecifics, recognition, grouping, aggression, territory | social-organization |
| Acceptance criteria, seed counts, controls, claim wording, metric choice | experimental-methodology |

## Standing constraints these reviews impose

Adopted in ADR-0022 and binding on all future design work:

- **The world is the experimental unit.** Organisms and ticks are nested
  observations. Aggregate to a world-level statistic before analysis.
- **30 independent worlds minimum** per confirmatory condition, 50 for rare
  or fixation-driven outcomes, with a pilot and a power analysis setting the
  final number.
- **One primary endpoint per phase; acceptance is conjunctive.** Secondary
  metrics never rescue a failed primary.
- **Nulls report a smallest effect of interest, an interval, and an
  equivalence result**, so "no effect" is distinguishable from
  "underpowered".
- **Perception delivers cues, not labels.** No action identifiers, no
  genotype distance, no pedigree, no observer labels.
- **Novelty is not progress.** Reserve innovation claims for novelty with
  demonstrated consequence and persistence.
- **Structural identity is four fields**, derived by hash, never a single
  global counter.

## Boundary

These reviews are evidence, not instructions. Where one contradicts a
recorded project decision, the contradiction is resolved in an ADR with the
reasoning written down. ADR-0022 records fourteen adoptions and three
declines; a future contradiction gets the same treatment rather than being
silently followed or silently ignored.

They are also not a substitute for the project's own evidence: a claim in a
review is a reason to design something a particular way, never a substitute
for measuring it here.
