# Artificial Life Simulation

## Status

**Phase 5 experiment instrument.** The stack spans the deterministic
Phase 1/2 kernel (`crates/sim-core`), the binary observer protocol and
private server (`crates/sim-protocol`, `crates/sim-server`), the
TypeScript/PixiJS observer (`apps/observer`), versioned persistence
(`crates/sim-persist`), and the multi-seed experiment harness
(`crates/sim-experiment`).

Phase 7 closed the loop that instrument was built for. Its primary
endpoint needed a world-level spatial statistic that did not exist; that is
now `crates/sim-analysis`, computing a two-scale Morisita index offline
from a versioned position-sample artifact, with no kernel change of any
kind. Measured over 300 worlds, contest reduces short-range co-occurrence
by about a quarter -- and does so *against* the direction its own
population decline would push the measure, which is what makes the result
worth having. The companion result is that the aggregation half of the same
endpoint is confounded, and it is reported as confounded.

Phase 5 turned the prototype into an instrument: an append-only ALEV event
log with fail-closed decode, an independent-world scheduler proven not to
reach a result at any worker count, campaigns whose conditions are named
config deltas with their own hashes, manifests that carry their own
campaign source, comparison reports that refuse to aggregate anything they
cannot justify, and asynchronous checkpointing that cuts the tick-thread
stall by 14 to 18 times. Both original fixtures still reproduce from clean
processes under every new execution path.

There is still no deployment configuration; the server binds 127.0.0.1
only. No Proxmox or other homelab service is accessed or changed by any of
this work. Infrastructure and physical-device gates remain open; see
`planning/backlog.md`.

**The project goal changed on 2026-08-04** (see Purpose below). Phases 0
through 4 are unchanged and their records, fixtures, and benchmarks are
preserved exactly. Phases 5, 6, and 7 are complete. Phases 8 through 18 are
planned and none has started. All ADRs remain Proposed.

Run it locally:

~~~sh
scripts/bootstrap-phase0-toolchain.sh
cargo build --release -p sim-server
target/release/lifesim-server        # prints generated tokens
cd apps/observer && npm install && npm run dev
~~~

Run a multi-seed experiment:

~~~sh
cargo build --release -p sim-cli
target/release/lifesim fields                        # settable config fields
target/release/lifesim batch --campaign my.campaign --output runs/ --workers 4
target/release/lifesim report --manifest runs/manifest.txt
~~~

## Purpose

Build a persistent, browser-observable artificial-life ecosystem inspired by the readability and sandbox appeal of WorldBox, not a scripted game. A continuous 2D bounded continent contains evolving organisms with inherited neural controllers, simulating biology and genetics as realistically as the determinism contract and the compute budget allow.

The long-term ambition is that organisms evolve from simple foragers toward tool use, persistent structures, transmitted knowledge, technological accumulation, territoriality, and organized inter-group conflict, without any of that being scripted as stages.

The governing philosophy is **emergence inside an authored possibility space**: we author physics, not progress. The simulation defines what is physically possible, and never defines a technology tree, a research prerequisite graph, an era, a building recipe, or a civilization stage. What organisms do inside that space is discovered by evolution and learning. "Eras" are a narrative an observer detects post hoc from the event log, never a state the simulation enters and never something an organism is told about.

Three things are kept separate, because conflating them is how projects like this mislead people:

- **What the simulation makes possible** is a property of the code.
- **What we hope to observe** is a research aspiration.
- **What we predict will actually happen** is a falsifiable expectation with an honest prior attached, and that prior is not favorable. Open-ended evolution is an unsolved grand challenge in artificial life, and no system has produced a technological era progression from genuine evolution.

Tool use, persistent structures, behavioral traditions that outlive individuals, and organized inter-group conflict are plausible-to-remarkable outcomes. A recognizable stone-age-to-enlightenment arc, language, and civilization are **not planned around and not promised**. A null result is a likely outcome of several phases and an acceptable outcome of all of them, because every phase states its ablation before it is run.

The full position, including why the project's determinism and benchmark discipline let it make claims most artificial-life projects cannot, is in [docs/25-emergence-and-epistemic-position.md](docs/25-emergence-and-epistemic-position.md).

## How A World Starts

Three origin modes ([specifications/world-origin-modes.md](specifications/world-origin-modes.md)). Each is a **starting condition**, never a trajectory: authoring where the search begins is not authoring the path it takes.

| Mode | Founders |
|---|---|
| `random` | Bounded-random organisms of one body plan, optionally in several separated demes. The current behavior |
| `seeded` | A head start: founder archetypes placed in the biomes that suit them. On a large enough map, several adapted populations start at once in different biomes |
| `scratch` | No organisms at all. A chemistry field from which protocells may arise, then unicells, then possibly differentiated multicellular bodies |

Two things this deliberately does **not** do. There is no scripted progression through microbe, fish, and reptile grades; that arrow is a hypothesis being tested, not a mechanism being executed, and reaching anything fish-like is not planned around. And there is no ice age the world *enters*: cold regions exist at generation and climate genuinely drifts over long timescales, so an observer may label a cold stretch afterwards, but no rule ever reads an era.

Archetypes are trait distributions, not designed creatures, and none is named after a real species. No rule or analysis may read an archetype ID, and a test enforces it.

For the `scratch` mode only, ADR-0018 permits deliberately shaping environments toward the major transitions. That genuinely weakens the resulting claims, so every scaffolded result carries an unscaffolded control on the same seeds and reports both.

## Product Decisions

- World: continuous 2D big-continent island with bounded coastlines.
- Environment: high realism as a direction, including day/night, seasons, renewable resources, and rare configurable disasters.
- Organisms: one adaptable body plan with visible trait variation; predation and herbivory emerge from traits.
- Evolution release: sexual reproduction and lineage tracking are in scope.
- Experience: pixel-art world view with a scientific-marker overlay, sandbox controls, mobile support, and wall-dashboard support.
- Science: fixed seeds, replayable configurations, and exportable data are required.
- Operations: private continuous deployment is acceptable; servernode3 is the proposed initial host with 16-24 GiB RAM and no initial GPU passthrough.

These are adjustable project policies, not permanent laws. Any behavioral rule or formula may change through a versioned configuration, a documented decision, migration/replay impact analysis, tests, and benchmark evidence.

## Core Goals

1. Keep the simulation kernel deterministic, inspectable, and independent from UI and transport.
2. Start correct at 500-2,000 organisms; prove every higher scale through benchmarks.
3. Keep the browser responsive through viewport-based binary deltas rather than full-world updates.
4. Preserve worlds safely with versioned snapshots, checkpoints, crash recovery, and validated restore.
5. Run privately on the homelab without changing existing production services during planning.
6. Make every behavioral claim a multi-seed measurement with a stated control or ablation. "It looked interesting" is never an acceptance criterion.
7. Model biology and genetics realistically enough that mechanisms can be checked against textbook results they were not tuned to produce.

## Major Non-Goals

These are permanent, and the change of ambition does not reopen any of them.

- A large language model, or any language model, as an organism decision engine. Optional narration may consume recorded events after the fact and may never influence a tick.
- An authored technology tree, recipe list, research graph, era state, or civilization mechanic. We author physics; we never author progress.
- Reinforcement learning against a hand-authored reward. Lifetime learning is in scope; the signal that gates it must be an evolved output of the organism's own network.
- Analysis output feeding back into simulation state. Analysis observes; it never instructs.
- Claims of emergent cognition, language, or society. The ambition is stated; the claims are earned by measurement or not made.
- A distributed single-world simulation before a single-node baseline is proven.
- Public internet exposure in the first deployment.
- GPU passthrough or a database cluster without measured benefit.

## Proposed Architecture

~~~mermaid
flowchart LR
  UI["TypeScript observer\nPixiJS pixel view + scientific overlay"] <-- "REST + binary WebSocket" --> API["Rust simulation server"]
  API --> K["Deterministic simulation kernel"]
  K --> S["Versioned snapshots + SQLite catalog"]
  API --> M["Prometheus metrics"]
  M --> G["Existing Grafana"]
  E["Experiment runner"] --> K
~~~

The proposed baseline is a Rust simulation server with a TypeScript/PixiJS observer, custom compact neural networks, compressed binary saves, SQLite world metadata, and Prometheus metrics. All technology choices remain proposed until Phase 0 benchmarks.

## Target Hardware

The proposed primary host is servernode3 because its 64 GiB ECC memory and large CPU capacity are better suited to an isolated VM and concurrent experiment jobs than the other described nodes. Start with an Ubuntu LTS VM, 16 vCPUs, 16-24 GiB RAM, and verified fast local storage. Do not assume storage capacity, CPU feature exposure, GPU availability, or Prometheus targets without a live Phase 0 audit.

## Expected Scale

- Prototype: 500-2,000 organisms at a deterministic 10 Hz baseline.
- Early optimized: 5,000-20,000 organisms after profiling and spatial indexing.
- Long-term experiment: 10,000-50,000 active organisms only if benchmark evidence supports it.
- Live observer: target 10-30 state frames/second per client, decoupled from tick rate.

The open-ended-evolution goal adds a second scale axis that matters more
than organism count: **generations reached, multiplied by seeds, multiplied
by ablation conditions.** The Phase 2 long run reached 127 ancestry
generations in 200,000 ticks; a cultural ratchet plausibly needs far more,
and every ablation multiplies the requirement. Phase 5 decoupled runs from
observer pacing and added an independent-world scheduler. Measured on the
development host: 8,805 ticks/s per world at the 500 tier, 1,653 at the
2,000 tier, and 3.67x aggregate throughput across 16 worlds at 4 workers.
**No supported campaign size is claimed from that**; it is one host, and the
deployment-VM measurement remains an open gate.

## Observer Experience

The initial browser observer emphasizes clear top-down pixel art, pan/zoom, terrain and biome visibility, selection and lineage inspection, live charts, replay controls, intervention audit trails, and a scientific marker/heatmap overlay. The UI must remain an observer/control client, not the owner of simulation truth.

## Repository Map

- [Project vision](docs/00-project-vision.md)
- [Emergence and epistemic position](docs/25-emergence-and-epistemic-position.md)
- [Biological realism policy](docs/26-biological-realism-policy.md)
- [Requirements and decision status](docs/01-user-requirements.md)
- [Architecture](docs/03-system-architecture.md)
- [Simulation model](docs/04-simulation-model.md)
- [Determinism extensions](specifications/determinism-extensions.md)
- [Observer design](docs/10-observer-interface.md)
- [Deployment plan](docs/17-proxmox-deployment.md)
- [Implementation roadmap](docs/19-implementation-roadmap.md)
- [Agent rules](AGENTS.md)
- [Codex operating guide](CODEX.md)
- [Complete file manifest](FILE_MANIFEST.md)
- [Simulation kernel](crates/sim-core/src/lib.rs)
- [Experiment harness](crates/sim-experiment/src/lib.rs)
- [Offline analysis](crates/sim-analysis/src/lib.rs)
- [Campaign definitions](experiments/)
- [Event log format](crates/sim-persist/src/eventlog.rs)
- [Headless CLI](crates/sim-cli/src/main.rs)
- [Observer protocol](crates/sim-protocol/src/lib.rs)
- [Observer server](crates/sim-server/src/main.rs)
- [Observer app](apps/observer/src/main.ts)
- [Phase 0 simulation spike](spikes/sim-spike/README.md)
- [Phase 0 renderer spike](spikes/renderer-spike/README.md)
- [Benchmark reproduction guide](benchmarks/README.md)

## Starting A Future Codex Session

Read README.md, AGENTS.md, CODEX.md, the active phase file in planning/, the relevant specifications/, and docs/22-decision-log.md. Run existing tests before changing code. Establish the current benchmark baseline before claiming performance. Record behavioral, schema, protocol, or operational changes in the relevant documents and a proposed/accepted ADR.

## Important Warnings

- Do not deploy, change Proxmox, alter DNS/firewalls, configure GPU passthrough, or edit existing monitoring during the planning phase.
- Never silently change deterministic rules, save schemas, or binary WebSocket schemas.
- Simulation rules are mutable experiments, but replay compatibility and past experiment interpretation are not optional.
- Performance targets are hypotheses until benchmarked on the deployment VM.
