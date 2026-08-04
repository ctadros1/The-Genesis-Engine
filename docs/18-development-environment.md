# Development Environment

## Captured Local State

At planning capture, the macOS arm64 workstation had Git, Node.js, npm, pnpm,
Bun, Docker, and jq available but no Rust toolchain. For Phase 0, Rust 1.97.1
was installed only under ignored `.phase0-tools/`; system shell profiles and
global toolchains were not changed. The recorded browser build used Node 25.6.0
and npm 11.8.0. These are benchmark provenance, not production prerequisites.

## Recommended Toolchain

| Area | Proposed Tooling | Purpose |
|---|---|---|
| Rust | stable rustup toolchain, rustfmt, clippy | kernel/server/build checks |
| Rust tests | cargo test, nextest if adopted, proptest, cargo-fuzz | correctness/property/fuzz |
| Observer | Node LTS current at implementation, pnpm, TypeScript, Vite | browser build/test |
| Browser tests | Playwright | desktop/mobile E2E and visual interaction |
| Containers | Docker Compose locally; target choice after Phase 0 | repeatable dev/deploy shape |
| Docs | Markdown link/check scripts | planning integrity |

## Setup Sequence

1. Install current stable Rust through the official rustup flow and confirm rustc, cargo, rustfmt, and clippy.
2. Use the repository-pinned Node/package-manager version when one is introduced; do not infer it from a global tool.
3. Create local configuration from a tracked example file; never use production secrets.
4. Start a single deterministic headless world before starting the observer.
5. Run formatting, unit tests, deterministic fixtures, and protocol/browser tests appropriate to the active phase.

## Reproducibility

Pin package lockfiles and Rust toolchain channel once code begins. Development profiles may be fast, but benchmark/release profiles must be explicit. Record OS/CPU/toolchain metadata alongside benchmark results. Avoid relying on a local Docker image tag, browser cache, or untracked environment file as source of truth.

## Phase 0 Commands

~~~sh
scripts/bootstrap-phase0-toolchain.sh
cd spikes/renderer-spike && npm ci && cd ../..
scripts/run-all-phase0-benchmarks.sh
~~~

The all-benchmark script writes raw JSON, CSV, logs, and screenshots under an
ignored timestamped `benchmarks/raw/` directory. Run `cargo fmt --check`,
`cargo test`, `cargo clippy -- -D warnings`, `npm run build`, and
`npm run test:smoke` before recording a comparison.

## Phase 1 Commands

~~~sh
scripts/bootstrap-phase0-toolchain.sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/verify-phase1-determinism.sh
scripts/run-phase1-benchmarks.sh
cargo test --release -p sim-core --test longrun -- --ignored
~~~

The headless runner is `target/release/lifesim` with `run`, `fixture`,
`inspect`, `benchmark`, and `analyze` subcommands; `lifesim run
--metrics-out -` emits the Prometheus text exposition for the implemented
metrics. The `--phase2` flag enables the Phase 2 policy on any subcommand.

## Phase 4 Commands

~~~sh
cargo test -p sim-persist
target/release/lifesim run --ticks 2000 --phase2 --save-path world.alif --csv-out run.csv
target/release/lifesim verify-save world.alif
target/release/lifesim run --ticks 500 --load-save world.alif
target/release/lifesim compare run-a.json run-b.json
target/release/lifesim-server --data-dir ./data --checkpoint-interval-secs 60
scripts/run-phase4-benchmarks.sh
~~~

`data/` and `saves/` are ignored paths for local worlds. The server's
save endpoints are `GET/POST /api/worlds/1/saves` and
`POST /api/worlds/1/saves/{id}/verify` (admin for mutations).

## Phase 3 Commands

~~~sh
cargo test -p sim-protocol -p sim-server
target/release/lifesim-server --organisms 500 --speed 8
cd apps/observer && npm install && npm run dev
scripts/run-observer-e2e.sh
scripts/run-phase3-benchmarks.sh
~~~

The server prints generated observer/admin tokens at startup unless
`LIFESIM_OBSERVER_TOKEN`/`LIFESIM_ADMIN_TOKEN` are set; paste the observer
token into the app's connection panel (admin token optional, enables
controls). REST defaults to 127.0.0.1:8940 and WebSocket to
127.0.0.1:8941.

## Phase 2 Commands

~~~sh
scripts/verify-phase2-determinism.sh
scripts/run-phase2-benchmarks.sh
cargo test --release -p sim-core --test phase2_longrun -- --ignored
LIFESIM_MALFORMED_ITERS=200000 cargo test -p sim-core --test genome_malformed_harness
~~~
