# Open Questions

## Must Resolve In Phase 0

- What vCPU/RAM/storage allocation can servernode3 safely dedicate after a live audit?
- Which CPU features and scheduling behavior are exposed to the proposed VM?
- What is the existing backup target and retention policy suitable for application-consistent saves?
- Which desktop, mobile, and kiosk browsers must support the observer, and do they expose WebGPU?
- Does the initial prototype need an authenticated observer role immediately or only an admin boundary behind WireGuard?

## Resolve Before The Referenced Phase

| Question | Deadline | Default If Unresolved |
|---|---|---|
| Exact cell size/world dimensions | Phase 1 | Resolved as configurable: `SimConfig` defaults to 256x256 cells at 4 m, benchmarked locally; nothing is hard-coded |
| Genome schema and mutation ranges | Phase 2 | Resolved as versioned policy: `lifesim-genome-v1` (14 bounded scalar traits, fixed topology 1) with `uniform-bounded-v1` variation; all ranges in `docs/06-organism-model.md` and ADR-0011, changeable only through a new schema/policy version |
| Pixel-art visual system | Phase 3 | readable placeholder sprites with overlay |
| Snapshot codec library | Phase 4 | framed custom logical record plus zstd |
| Analytics columnar format | Phase 4 | CSV export |
| SIMD strategy | backlog, needs profiling evidence | scalar/SoA baseline |
| Disaster rules | not scheduled | disabled |
| Communication rules | Phase 13 | `specifications/social-signal-channel.md` |
| Disease rules | Phase 14, optional slice | disabled |

## Open Questions From The 2026-08-04 Goal Change

None of these blocks the next phase. Each is recorded so that a later
implementer does not have to rediscover that it was considered.

| Question | Deadline | Default If Unresolved |
|---|---|---|
| How many seeds and how many ticks make a null result meaningful, given the measured throughput? | Phase 5, from its own measurements | 30 seeds; run length set so the median seed reaches at least an order of magnitude more ancestry generations than Phase 2's 127 |
| Should controller activations move to fixed point along with the learned accumulator? | Phase 9 | Keep f32 activations (proven and benchmarked under ADR-0011) with a fixed-point learned accumulator. Full fixed-point evaluation stays the recorded fallback if cross-platform replay ever becomes a requirement |
| What is the right `propagation_passes_per_tick`? | **Resolved 2026-08-04, ADR-0022 A9.** Superseded by hybrid evaluation: zero-delay edges propagate within a tick in canonical topological order, delayed edges read prior-state buffers. The knob is gone | - |
| Duplication-only structural growth, or duplication plus explicit insertion? | Phase 9 | Measure both (C9.5). If duplication alone cannot produce structural change within the run budget, insertion becomes the default with the measurement recorded |
| Should a regulatory locus type (schema 2 type tag 5) exist? | not scheduled | Reserved and unallocated. Adding it needs its own ADR; `docs/26-biological-realism-policy.md` records why molecular-level regulation is excluded |
| Is providing computed genetic distance as a kinship input channel too generous? | **Resolved 2026-08-04, ADR-0022 A3.** No genotype-distance channel exists. Recognition must be solved from perceptible cues or reported unsolved | - |
| Should plasticity rule form 5 (observational) exist, or must imitation be discovered from generic plasticity plus perception? | Phase 13, by measurement | Run both as conditions P and S rather than deciding by assertion. See `specifications/social-signal-channel.md` |
| When should elevation become mutable? | not scheduled | Immutable. It feeds coastline derivation, drainage, and the lapse term, and the generator validates land fraction and connectivity against it |
| What density threshold selects the dense terrain-modification representation? | Phase 12 | Set from measurement, recorded as versioned config, never a magic constant |
| Do the Phase 7 to 13 campaigns need re-running under `lifesim-physiology-v2`? | Phase 14 | Yes for any result that is to become a standing finding. Stated in ADR-0017 |
| Can the deployment VM supply the compute a full campaign needs? | before any campaign | Unknown and unmeasured. This is a live risk, not a resolved default |

## Open Questions From The Research Reconciliation

| Question | Deadline | Default If Unresolved |
|---|---|---|
| Can 30 to 50 worlds per condition actually be run on the available hardware? | Phase 5 | Unknown. If not, the number of claims is reduced rather than the seeds per claim, and every affected criterion says so |
| What is the smallest effect of interest for each primary endpoint? | Before each campaign | Must be fixed before data collection; there is no defensible default and picking one afterwards invalidates the null |
| Is recognition reachable from phenotype cues alone in this physics? | Phase 13 | Unknown, and the honest answer may be no. C13.7 reports either way |
| Does stigmergic transmission occur, given that Phase 13 was reordered to depend on it? | Phase 12 | Unknown. If not, Phase 13 loses its baseline and reverts to the weaker comparison |
| Does the developmental encoding survive its discontinuity gate? | Phase 10 | Unknown. Failure takes the parameterized fallback and forces a Phase 16 re-plan |
| Should the 2.5D height and support subset land in Phase 12 or later? | Phase 12 | Later. Until it lands the plan does not claim stacked construction |

## Open Questions From Intra-World Parallelism (ADR-0026)

| Question | Deadline | Default If Unresolved |
|---|---|---|
| Does thread-count invariance (Tier 1) hold, or does the fallback to per-thread-count determinism get taken? | Phase 18 | Unknown. C18.2 decides it. Tier 1 is proposed because the all-fixed-point state makes cross-partition reductions order-independent by construction |
| What is the real serial fraction, and does it survive Phases 7, 12, and 13? | Phase 18, re-measured after each | Unknown. The ~3.1 percent estimate mixes two benchmark records. `apply` is the serial part and those three phases each add conflict resolution to it |
| Below what population is parallelism a net loss? | Phase 18 | Unknown. Barrier overhead is roughly fixed per tick while parallel work scales with population, so a crossover exists. Default stays disabled until it is measured |
| Is 200,000 organisms in one world reachable? | Phase 18 | Plausible at current phase composition, marginal at full Phase 13 complexity. This is the question the phase exists to answer |

## Decision Protocol

An unanswered question is not permission to invent production behavior. If it affects persistent compatibility, security, budget, or meaning of an experiment, create a proposed ADR and seek the minimum needed evidence/approval.

## Phase 0 Local Evidence And Remaining Gaps

| Question | Local Evidence | Remaining Gap |
|---|---|---|
| Rust deterministic baseline | Two clean Rust processes matched at 500 organisms/500 ticks; 500 and 2,000 benchmarks completed | Deployment-shaped VM and cross-platform replay policy |
| Snapshot framing | Version/cap/checksum/negative decoding and uncompressed encode/decode measured | Compression library, migration fixtures, atomic filesystem write |
| Browser renderer | Chrome 150 on the Mac completed PixiJS 8.19 WebGL and WebGPU at desktop and mobile-sized viewports | Named supported browsers, physical mobile/kiosk hardware, non-headless WebGPU matrix |
| Proposed VM | No host or VM was accessed | Every VM acceptance checklist item remains open |
| Authentication boundary | Not exercised by these local spikes | Observer/admin decision remains open |
