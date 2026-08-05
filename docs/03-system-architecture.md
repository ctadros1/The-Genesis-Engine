# System Architecture

## Recommended Baseline

Use a single authoritative Rust process for each world. Its pure simulation kernel owns all organism and environment state. A server layer schedules ticks, saves snapshots, exposes control/metadata APIs, streams viewport updates, and publishes metrics. A separate TypeScript observer renders state and requests validated actions. An experiment runner launches independent world processes/configurations; it does not distribute one tick across hosts.

~~~mermaid
flowchart TB
  subgraph VM["Proposed isolated Ubuntu VM"]
    UI["Observer static assets\nReact + PixiJS"]
    S["sim-server\nREST, WS, auth, metrics"]
    C["sim-core\nfixed deterministic tick"]
    P["Persistence adapter\nsnapshots, event log, SQLite"]
    X["Experiment runner\nindependent worlds"]
    UI <--> S
    S --> C
    C --> P
    X --> C
  end
  Client["Browser: desktop, mobile, wall display"] <--> UI
  Prom["Existing Prometheus"] --> S
  Graf["Existing Grafana"] --> Prom
~~~

## Component Responsibilities

| Component | Owns | Must Not Own |
|---|---|---|
| sim-core | Entity state, tick phases, RNG streams, formulas, invariants | HTTP, UI state, filesystem, wall clock, **any dependency on sim-analysis** |
| sim-server | World lifecycle, scheduling, authenticated controls, stream fan-out | Hidden simulation rules |
| persistence | Framed snapshot encoding, migration, integrity verification | Unversioned raw memory dumps |
| observer | Rendering, input UX, client interpolation, local cache | Authoritative organism behavior |
| sim-experiment (Phase 5) | Isolated run orchestration, multi-world scheduling, campaign manifests, comparison metadata | Cross-world shared mutable state, any path back into a rule |
| sim-analysis (Phase 16) | Offline similarity, era, and tradition detection over the event log and read-only views | A mutable world handle, an RNG stream, any path back into a rule |
| metrics/events | Operational and scientific observability | Blocking the tick |

### The Experiment Runner (Phase 5, implemented)

`crates/sim-experiment` depends on `sim-core` and `sim-persist` and is
depended on by neither. It introduces no randomness of its own, draws from
no RNG stream, and nothing it computes can reach world state, so it sits on
the same side of the ADR-0016 boundary as analysis.

Two structural choices carry determinism rule 10, and both matter more than
they look:

- **A worker owns its world completely.** It builds the config, builds the
  world, ticks it, and writes its own files. Nothing is borrowed across
  worlds, so there is no shared buffer that could carry state from one world
  into another.
- **Results are stored by unit index, never appended on completion.** Output
  ordering is the campaign's canonical (condition, seed) order no matter
  which worker finished first. Appending on completion would make the
  manifest depend on thread scheduling, which is the exact class of bug A5.2
  exists to catch.

Which worker claims which world is genuinely nondeterministic and is
supposed to be. If that choice could reach a result, A5.2 fails, and it is
designed to fail loudly rather than be argued away.

A panicking or failing world is isolated: its unit is recorded as failed
with its reason and the campaign continues. A failed world is reported,
never silently dropped, and the comparison report refuses to aggregate a
campaign that contains one.

### The Analysis Boundary Is A Dependency Direction

`sim-analysis` depends on `sim-core`. **`sim-core` never depends on
`sim-analysis`**, asserted in CI. This makes "analysis observes, it never
instructs" (ADR-0016) a compile error rather than a review finding, which
matters because the pressure to add a small hook will be strongest exactly
when a phase is returning null results.

`lifesim-similarity-v1` moves into `sim-analysis` unchanged, keeping its
version string and report format so existing reports stay comparable.

### Multi-World Execution (Phase 5)

The experiment runner gains an independent-world scheduler: N worlds on one
host, sharing no mutable state, with headless execution decoupled from
observer pacing. The correctness requirement is an equality test rather than
an argument: per-world final checksums must be identical at scheduler
concurrency 1, 2, and C, and identical to running each world alone.

This does not open intra-world parallelism, which remains gated on
ADR-0010's ordering and reduction evidence, nor distributed multi-host
scheduling, which stays out of scope.

## Technical Stack Evaluation

### Simulation Language

| Option | Strengths | Risks | Verdict |
|---|---|---|---|
| Rust | Memory safety, explicit concurrency, high performance, strong serialization/testing ecosystem, good Codex ergonomics | Learning curve; borrow discipline; floating-point care still required | Recommended |
| C++ | Maximum ecosystem/performance control | Memory/concurrency safety burden and higher long-term debugging cost | Not initial choice |
| Go | Simple service deployment and concurrency | Less natural for cache-sensitive numeric kernel; garbage collection variability | Useful tooling option, not kernel default |
| Python plus native acceleration | Fast prototyping and scientific ecosystem | Two-language boundary and performance ceiling at target scale | Phase 0 comparison spike only |

### Entity Storage

Choose data-oriented struct-of-arrays (SoA) storage with system-specific dense views, stable entity IDs, and sparse membership maps. An ECS vocabulary can describe systems, but avoid a general dynamic ECS framework until it proves its overhead and serialization model. This hybrid gives cache locality, straightforward snapshotting, and clear deterministic iteration.

### Coordinates

Use continuous x/y coordinates for organisms and a fixed raster field for terrain, climate, food, and spatial buckets. This supports smooth movement and WorldBox-like presentation while keeping environment and neighbor queries bounded. It is less fragile than all-continuous environmental simulation and more expressive than a pure grid organism model.

### Neural Control

Use a custom compact f32 matrix evaluator first. Its fixed topology, genome-owned weights, and bounded memory vector make it easier to batch, serialize, mutate, test, and inspect than ONNX Runtime, Candle, Burn, LibTorch, or CUDA kernels. Evaluate SIMD and GPU only when profiling shows neural inference is a dominant measured cost.

### Observer Rendering

Use a TypeScript application shell with PixiJS v8. PixiJS supports WebGPU with WebGL fallback and supports culling/batched 2D rendering; it fits pixel sprites, maps, pan/zoom, and mobile better than a bespoke WebGL layer. Canvas is the fallback prototype/control surface, Three.js is unnecessary for top-down 2D, and Phaser adds game-framework concepts the observer does not need.

## Data Ownership And Backpressure

The simulation advances independently of clients. A client subscribes to a world ID, viewport, layer selection, and update budget. The server emits a keyframe on subscribe/resync and compact deltas thereafter. If a client is slow, its pending world update is replaced by a newer aggregated update, never queued indefinitely and never allowed to block a tick.

## Architecture Acceptance Criteria

- sim-core compiles/tests with no UI or transport dependency.
- A browser reconnects and resynchronizes from a keyframe without altering the world.
- Save/load and an identical seeded run satisfy the declared determinism policy.
- A slow observer cannot lower the tick rate beyond the configured observation budget.
- Architecture changes have an ADR, tests, and benchmark impact statement.
