# Rendering Options

| Option | Fit | Tradeoff | Decision Gate |
|---|---|---|---|
| Canvas 2D | easiest spike/control layers | limited dense sprite performance | fallback prototype |
| PixiJS v8 | 2D sprites, batching, culling, WebGPU/WebGL path | library/API/backend matrix | recommended Phase 0 spike |
| Raw WebGL/WebGPU | maximum control | high development/debug cost | only if PixiJS fails measurable needs |
| Three.js | mature scene tooling | excess 3D abstraction | not default for top-down 2D |
| Phaser | game workflow | unneeded game framework/state concepts | not default |
| Native desktop | tighter hardware control | distribution/platform burden, weak mobile | not initial |

The PixiJS official current documentation advertises WebGPU/WebGL rendering and supports fallback selection/culling concepts. Verify target-browser behavior directly during Phase 0; do not assume WebGPU availability from the library capability alone.

References: https://pixijs.com/ and https://pixijs.com/8.x/guides/getting-started/intro
