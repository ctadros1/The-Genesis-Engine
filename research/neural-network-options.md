# Neural Network Options

| Option | Benefit | Cost/Risk | Recommendation |
|---|---|---|---|
| Custom fixed f32 matrices | exact schema, easy mutation/save/inspect, low overhead | implement kernels/guards | baseline |
| CPU SIMD | higher throughput with stable batch layout | platform/precision complexity; assumes a stable batch layout that variable topology removes | backlog; needs profiling evidence |
| Candle/Burn | Rust ML ecosystem | framework complexity, dynamic tensors | evaluate only if models grow |
| ONNX Runtime | external model interoperability | unnecessary model/runtime boundary | not initial |
| LibTorch | mature kernels | large operational footprint | not initial |
| CUDA kernels | potential high throughput | GPU passthrough/copy/determinism cost | benchmark-gated |

The decisive metric is end-to-end tick cost, including sensor gather, batch packing, transfers, output scatter, and recovery.

## Topology Evolution: Deferral Reversed (2026-08-04)

The original note read: "Controller topology evolution is deferred because
variable shapes reduce batching, explainability, and migration simplicity."

All three costs are real and none of them was wrong. The deferral is
reversed by ADR-0013 because the frozen topology blocks the project's new
goal: every new capability becomes a channel and a schema bump performed by
a human, and open-ended complexity cannot come from a structure only we can
change.

How each cost is handled rather than dismissed:

| Original cost | Response |
|---|---|
| Reduced batching | Real. Grouping by topology ID stops working when topologies are per-organism. Measured in Phase 8 rather than assumed; the replacement is a later performance slice; loss of the zero-per-tick-allocation property is recorded, not absorbed |
| Reduced explainability | Real. Mitigated by innovation IDs, which give a traceable structural history, and by the event log recording every structural mutation with its operator class |
| Migration complexity | Sidestepped entirely: there is **no** schema 1 to schema 2 migration. Schema 1 worlds stay schema 1, and schema 1 decode, evaluation, and fixtures stay in the build permanently |

## Encoding Options For Variable Topology

| Option | Benefit | Cost/Risk | Recommendation |
|---|---|---|---|
| Fixed topology (schema 1) | Cheap, batchable, inspectable | Every capability is a human schema bump | Retained for schema 1 worlds; insufficient for the goal |
| NEAT-style graph editing | Well understood; historical markings solve alignment | An authored graph-editing scheme; does nothing for genetic realism | Operators available, but not as the primary mechanism |
| Chromosomal genome with duplication-driven growth | Structural evolution and genetic realism from one mechanism; alignment falls out of ordinary inheritance | Slower and less directed growth; not established as superior at this scale | **Proposed baseline (ADR-0013)**; compared against explicit insertion in Phase 8 criterion C8.5 |
| Indirect or developmental encoding | Highest ceiling in principle | Hardest to make deterministic and to analyze; unevidenced at this scale | Deferred; a regulatory locus type is reserved and unallocated |

No claim is made that duplication-based encodings outperform direct ones.
The ALife literature does not establish it, and Phase 8 measures the
comparison instead of asserting an answer.
