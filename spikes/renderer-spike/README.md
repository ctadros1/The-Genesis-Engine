# Phase 0 PixiJS Renderer Spike

This is a local benchmark surface, not an observer application scaffold. It
creates deterministic synthetic markers, pans a four-viewport world, invokes
PixiJS culling, and records browser frame interval, update/cull CPU time,
render-submit CPU time, visible marker count, and instrumented draw calls.

Query parameters:

- `backend=webgl|webgpu`
- `count=1..100000`
- `warmup=0..600`
- `frames=10..2000`

The benchmark reports the requested and actual renderer. Browser/device results
are not transferable: a mobile viewport in desktop Chrome is a smoke test, not
evidence for a physical phone GPU.

Run `npm run build`, `npm run test:smoke`, and then the repository-level
`scripts/run-renderer-benchmarks.sh`.

