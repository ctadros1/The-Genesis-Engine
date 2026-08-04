# User Requirements And Decision Status

## Explicit Product Decisions

| Area | Decision | Status |
|---|---|---|
| World | Continuous 2D, WorldBox-like large island/continent, bounded coast | Accepted product direction |
| Environment | High-realism direction; day/night, seasons, rare configurable disasters, renewable resources | Accepted product direction |
| Organism visual | Pixel art plus switchable scientific-marker overlay | Accepted product direction |
| Body plan | One adaptable initial body plan with visible trait evolution | Accepted product direction |
| Ecology | Herbivory and predation emerge from traits | Accepted product direction |
| Evolution | Sexual reproduction and lineage tracking in the evolution release | Accepted product direction |
| Controls | Sandbox interventions, experiment templates, protected specimens | Accepted product direction |
| Goals | Balanced entertainment and scientific/educational use | Accepted product direction |
| Evidence | Fixed seeds, replayable configurations, exportable data | Accepted product direction |
| Viewing | Mobile and wall dashboard support | Accepted product direction |
| Scope | Small working prototype first | Accepted product direction |
| Operations | Continuous private operation; servernode3 proposed; 16-24 GiB RAM; no initial GPU passthrough | Accepted product direction |
| Long-term ambition | Organisms should be able to evolve toward tool use, persistent structures, transmitted knowledge, technological accumulation, territoriality, and organized inter-group conflict, none of it scripted as stages (D-020) | Accepted product direction |
| Design philosophy | Author physics, never progress. No technology tree, research graph, era state, recipe, or civilization mechanic (D-021, ADR-0012) | Accepted product direction |
| Biological fidelity | Simulate biology and genetics as realistically as determinism and compute budget allow, with mechanisms checked against textbook results they were not tuned to produce (D-027, ADR-0017) | Accepted product direction |
| Evidence standard | Every behavioral claim is a multi-seed measurement with a stated control or ablation; nulls are reportable results | Accepted product direction |

## User Rule-Change Policy

The user explicitly does not want early simulation rules to become fixed doctrine. Treat formulas, traits, interaction models, and environmental rules as versioned guidelines that can evolve after evidence. A change is acceptable when it:

1. has a named configuration or simulation-version effect,
2. records its rationale and expected behavioral impact,
3. has appropriate tests and benchmark comparison,
4. declares replay/save compatibility, and
5. does not silently reinterpret old experiment results.

## Technical Recommendations

| Area | Recommendation | Confidence | Validation Gate |
|---|---|---|---|
| Kernel | Rust, data-oriented storage, deterministic fixed tick | High | Phase 0 microbenchmarks and prototype |
| Coordinates | Continuous organisms over raster environmental fields | High | World generation and spatial-query benchmark |
| Neural control | Custom compact feed-forward network plus bounded memory vector | High | Phase 2 fitness/stability benchmark |
| Observer | TypeScript, React shell, PixiJS v8 renderer, WebGPU then WebGL fallback | Medium-high | Phase 0 rendering spike on target browsers |
| Streaming | REST control/metadata plus binary WebSocket viewport deltas | High | Protocol load test |
| Saves | Versioned custom binary snapshots compressed with zstd, SQLite metadata catalog | Medium-high | Restore/migration test |
| Deployment | Isolated Ubuntu LTS VM on servernode3, no GPU initially | Medium | Live read-only capacity audit and workload benchmark |

## Deferred Decisions

- Exact world dimensions, map-cell resolution, and biome count.
- Baseline population density and species-label thresholds.
- Exact visual sprite set and pixel-art palette.
- Disaster catalog and intervention permission granularity.
- Authentication implementation and reverse-proxy choice.
- Specific Rust/Node package versions.
- Whether a GPU beats CPU batching on the actual VM.

## Assumptions Requiring Validation

- CPU features and vCPU scheduling exposed by servernode3's proposed VM.
- Available local SSD capacity, backup target, and snapshot policy.
- Existing Prometheus scrape topology and Grafana dashboard ownership.
- Browser WebGPU availability on primary desktop, mobile, and kiosk clients.
- Network bandwidth/latency for wall dashboard and remote WireGuard use.

## Requirement Traceability

Phase plans must cite this document for user-visible decisions. If a requirement changes, update this table and add a decision-log entry rather than editing a phase plan in isolation.
