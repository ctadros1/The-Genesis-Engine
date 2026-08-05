# Related Projects And Inspiration

## Purpose

These references are design/research inputs, not dependencies or claims of feature parity. Check licensing, current status, and source documentation before adopting code or models.

| Project/Source | Relevant Lesson | Not A Requirement |
|---|---|---|
| WorldBox | Readable sandbox presentation and world-level inspection are valuable | Scripted game systems or its implementation |
| Avida | Digital evolution benefits from explicit experimental controls and analysis | Matching its organism/instruction model |
| MABE2 | Modular evolutionary experiments and configuration are useful reference concepts | Importing its architectural complexity |
| NetLogo models | Small transparent models help validate causal rules | Single-threaded educational-scale execution |
| PixiJS | 2D rendering/culling backend for a browser observer | Owning simulation logic in the browser |

## Reference Links

- WorldBox: https://www.superworldbox.com/
- Avida: https://avida.devosoft.org/
- MABE2: https://github.com/Mutualism-Project/mabe2
- NetLogo: https://ccl.northwestern.edu/netlogo/
- PixiJS: https://pixijs.com/

## Open-Ended Evolution References (2026-08-04)

Added when the project goal changed. These are research context for
`docs/25-emergence-and-epistemic-position.md`, not dependencies and not
claims of parity. **Look up current canonical sources before citing any of
them**; URLs are deliberately omitted here rather than guessed.

| Area | Why it matters here | Not a requirement |
|---|---|---|
| Open-endedness as an ALife grand challenge | The framing behind the honest prior: no system has produced a technological era progression from genuine evolution. This is the claim the project must not quietly contradict | Adopting any particular open-endedness metric as an acceptance criterion |
| Tierra and Avida (digital evolution) | Long-running self-replicator systems with explicit experimental controls; Avida is already listed above for its analysis discipline | Matching their instruction-set organism model |
| Polyworld and similar embodied neural-agent worlds | Prior art for evolving neural controllers in a spatial ecology with vision and combat | Their topology or metric choices |
| NEAT and structural neuroevolution | The historical-marking idea for aligning structurally different genomes, obtained here as a consequence of chromosomal inheritance rather than as a separate algorithm (ADR-0013) | Its explicit add-node/add-connection operators as the primary growth mechanism |
| Cultural evolution and transmission-fidelity modelling | The accumulation threshold that Phase 13's fidelity criterion is measured against: improvements accumulate only when expected persistence per transmission exceeds one | Any specific published fidelity value |
| Gene duplication and divergence in molecular evolution | The biological basis for structural growth by duplication rather than graph editing | Molecular-level modelling, which is explicitly excluded |

## Evaluation Rule

Borrow an idea only after identifying the exact problem it solves, its licensing/maintenance status, the implementation cost, and a test/benchmark that can prove it helps this project's stated goals.
