# Agent Operating Contract

## Authority And Reading Order

This file is authoritative for coding agents. Before any implementation work:

1. Read README.md, this file, and CODEX.md.
2. Read the active planning phase and all specifications named by that phase.
3. Read docs/01-user-requirements.md, docs/02-scope-and-non-goals.md, docs/21-open-questions.md, and docs/22-decision-log.md.
4. Inspect existing code, tests, schemas, migrations, and benchmark history.
5. Run the relevant existing tests before editing.

If a conflict exists, explicit current user instructions override this file. Record the resulting change in the decision log.

## Project Principles

- **Author physics, never progress.** The simulation defines what is
  physically possible. It never defines a technology tree, research
  prerequisite graph, era, building recipe, tool category, civilization
  stage, or reward for inventing anything. Before adding a mechanism, ask
  whether you can name the specific outcome it makes more likely; if you
  can, it is authored progress and it does not belong here. See ADR-0012 and
  `docs/25-emergence-and-epistemic-position.md`.
- **Analysis observes, it never instructs.** Offline analysis is recorded in
  reports and can never reach a rule, an input channel, a config trigger, or
  an intervention. Enforced by crate dependency direction: `sim-core` must
  not depend on `sim-analysis`. See ADR-0016.
- **Every behavioral claim is a multi-seed measurement with a stated control
  or ablation.** "It looked interesting" is never an acceptance criterion. A
  measured null is a result; a weakened threshold after seeing the data is a
  different experiment.
- The kernel is deterministic simulation infrastructure, not UI code.
- The browser observes and requests actions; the server validates and owns state.
- Behavioral rules are versioned, measured hypotheses. Do not petrify early formulas into product doctrine.
- Model biology and genetics as realistically as determinism and the compute
  budget allow, and check mechanisms against textbook results they were not
  tuned to produce. See ADR-0017.
- Make state transitions explicit, bounded, serializable, and testable.
- Prefer simple correct data flow over clever abstractions.
- Keep live infrastructure private and make reversible changes only after approval.

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

## Architecture Boundaries

- sim-core may not depend on HTTP, WebSocket, browser, database, wall clock, or GPU APIs.
- sim-server may orchestrate ticks, persistence, and transport but may not embed business logic that belongs in sim-core.
- observer code may interpolate and render, but may not authoritatively simulate organisms.
- storage adapters may encode/decode versioned data, but may not alter meaning during load.
- infrastructure configuration may launch services, but must not contain gameplay/simulation logic.
- optional LLM narration must consume recorded events and may never decide organism actions.

## Coding Standards

- Prefer stable Rust on the server and strict TypeScript in the observer.
- Use explicit types for public data, units, IDs, version numbers, and error states.
- Make ordering deterministic: sort by stable entity ID before order-sensitive work.
- Treat float behavior as a controlled risk. Clamp externally-derived values and test platform tolerance.
- Use small modules with documented invariants. Do not hide material state changes behind convenience helpers.
- Use ASCII unless an existing file or protocol requires another encoding.

## Determinism Rules

- A world seed, configuration version, simulation build version, and ordered intervention log define a run.
- No wall-clock reads, random global state, unordered map iteration, or scheduling-dependent reductions in strict mode.
- Randomness must derive from named deterministic substreams keyed by world seed, tick, system, and entity or cell identity.
- Parallel systems are opt-in only after equality/tolerance tests establish their determinism policy.
- Changing a rule or config hash creates a new replay lineage; never claim byte-identical replay across an incompatible version.

`specifications/determinism-extensions.md` is normative for everything added
from Phase 5 onward and takes precedence over any contradicting text. The
rules there that are easiest to violate by accident:

- Pairwise draws use the canonical pair key, so an outcome never depends on
  which participant the tick visited first.
- Roles in an asymmetric interaction are assigned by ID comparison, never by
  traversal order.
- Every candidate set is materialized, sorted by `(distance_squared,
  object_id)`, and truncated before any selection. A spatial bucket built in
  scan order may never be read in scan order.
- All social perception and learning is read-prior, commit-after.
- Per-node float summation is pinned to ascending edge `homology_id` order.
  Float addition is not associative, so a storage-order sum is a replay bug
  that stays invisible until a compaction changes layout.
- Anything that accumulates over a lifetime is fixed point, not float.
- New RNG stream values and checksum sections are appended, never
  renumbered or reordered, so fixtures `0x1e3158a26afd3b39` and
  `0xff9dfcff5dffbf42` stay reproducible forever.

## Testing Requirements

Every change must add or update tests at the appropriate layer:

- Unit tests for formulas, state transitions, bounds, and invalid input.
- Deterministic seed tests for any changed tick behavior.
- Property/fuzz tests for genome, save, and protocol boundary code.
- Integration tests for API and persistence changes.
- Browser tests for observer interactions and rendering regressions.
- Benchmark updates when performance-sensitive paths change.

Run the smallest relevant suite first, then the phase-required suite. Report commands and results honestly. A test that was not run is not a passing test.

## Performance Rules

- No claims about organism count, tick rate, bandwidth, memory use, GPU gain, or speedup without a recorded benchmark.
- Establish a baseline before optimizing. Profile before redesigning data layout.
- Never block the simulation on a slow observer, a save operation, or a metrics scrape.
- Make allocation, serialization, and observer fan-out visible in metrics.
- Do not add distributed single-world execution without an accepted ADR and comparative benchmark.

## Documentation And Change Management

Update documentation in the same change when behavior, formulas, architecture, protocol, persistence, security, deployment, or performance expectations change.

- Add a proposed ADR before a consequential choice; accept it only with evidence and approval.
- Update docs/22-decision-log.md with date, rationale, impacted versions, and revisit condition.
- Append benchmark results using the schema in specifications/metrics-schema.md.
- Preserve older experiment configs and migration notes. Do not rewrite history to make changed rules appear unchanged.

## Prohibited Shortcuts

- Do not run an LLM for per-organism decisions.
- Do not author a technology tree, recipe list, research graph, era state,
  crafting table, tool taxonomy, or civilization mechanic.
- Do not deliver a reward, fitness signal, or authored objective to any
  network. Lifetime learning is gated by an evolved output of the organism's
  own network.
- Do not let any analysis output reach a simulation rule, an input channel,
  a config trigger, or an intervention.
- Do not call a signal channel a language, a detected segment an era, or a
  behavioral cluster a culture without its genotype-matched control.
- Do not weaken a stated acceptance threshold after seeing the data.
- Do not send entire world state every tick to every browser.
- Do not use a global mutable RNG.
- Do not serialize raw language/runtime memory as a save format.
- Never silently change schemas, formulas, protocol fields, or replay semantics.
- Do not hard-code production host addresses, secrets, or credentials.
- Do not bypass schema versions, input validation, or load-time bounds checks.
- Do not introduce a database service, GPU backend, or distributed coordination because it sounds scalable.
- Do not perform broad refactors without profiler or correctness evidence.

## Security And Deployment Safety

- Default to LAN/WireGuard-only access and least-privilege service accounts.
- Keep control actions separate from observer access and audit every intervention.
- Store secrets in the existing approved secret manager; never commit or echo them.
- Require explicit approval before accessing or changing Proxmox, VMs, containers, DNS, firewall, WireGuard, reverse proxy, Grafana, Prometheus, or backups.
- Validate restores in an isolated location before replacing a live world.

## Definition Of Done

A task is done only when its scope is implemented, relevant tests and validations pass, documentation and decision records are current, benchmark claims have evidence, failure/rollback behavior is known, and unrelated files were not changed.

## Ambiguity And Blocking

Use the narrowest safe interpretation. If an ambiguity changes safety, data compatibility, experimental meaning, or infrastructure impact, stop and ask. Otherwise choose the documented default, label it as an assumption, and add it to docs/21-open-questions.md or the decision log.
