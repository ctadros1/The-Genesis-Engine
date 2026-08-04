# Simulation Engine Options

## Evaluation Criteria

Performance, memory safety, deterministic control, concurrency, serialization, testing, debugging, long-term maintainability, agent effectiveness, and deployment simplicity.

| Option | Strengths | Costs | Planning Recommendation |
|---|---|---|---|
| Rust custom SoA | safety, native speed, explicit layout, single deployment runtime | learning curve and bespoke data structures | Phase 0 baseline |
| Rust ECS framework | systems vocabulary and tooling | dynamic/archetype overhead/serialization complexity | compare only if custom SoA proves painful |
| C++ custom engine | mature low-level optimization | safety/debug cost | not first path |
| Go service plus kernel | service ergonomics | GC/cache-control uncertainty | tooling candidate only |
| Python + native module | fast modeling iteration | split runtime/performance risk | benchmarking comparison only |

## Recommended Experiment

Build a tiny fixed-tick loop with arrays, spatial lookup, and snapshot serialization. Measure correctness ergonomics plus p95/RSS. Do not benchmark an empty language loop and call it simulation capacity.
