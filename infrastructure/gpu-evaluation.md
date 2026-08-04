# GPU Evaluation

## Default Decision

Do not pass through a GPU initially. The compact network/inference workload may be cheaper on CPU once input gathering, batching, PCIe transfer, guest drivers, scheduling, power, and operational recovery are counted.

## Evaluation Method

Compare an identical deterministic world/config on CPU scalar/SoA, CPU SIMD if available, and a candidate GPU path. Measure end-to-end tick p50/p95, throughput, RSS/VRAM, host/guest CPU, energy if available, failure/restart complexity, and observer impact. Include 2,000, 10,000, and any validated higher population tiers.

## Adopt Only If

GPU materially improves an actual supported tier while preserving determinism policy, does not harm host workloads, has a reproducible guest-driver/container setup, and has a tested rollback to CPU. A microbenchmark that excludes gather/copy/synchronization is insufficient.
