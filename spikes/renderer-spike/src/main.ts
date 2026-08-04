import { Application, Container, Culler, Rectangle, Sprite, Texture, VERSION } from "pixi.js";
import "./style.css";

type RequestedBackend = "webgl" | "webgpu";

interface Percentiles {
  p50: number;
  p95: number;
  p99: number;
  min: number;
  max: number;
}

export interface RendererBenchmarkResult {
  benchmark_schema_version: 1;
  pixi_version: string;
  requested_backend: RequestedBackend;
  actual_backend: string;
  renderer_constructor: string;
  organism_count: number;
  viewport_css_pixels: { width: number; height: number };
  device_pixel_ratio: number;
  world_multiplier: number;
  warmup_frames: number;
  sampled_frames: number;
  frame_interval_ms: Percentiles;
  update_and_cull_cpu_ms: Percentiles;
  render_submit_cpu_ms: Percentiles;
  draw_calls_per_frame: Percentiles;
  visible_organisms: Percentiles;
  webgpu_exposed: boolean;
  user_agent: string;
  limitations: string[];
}

declare global {
  interface Window {
    __PHASE0_RESULT__?: RendererBenchmarkResult;
    __PHASE0_ERROR__?: string;
  }
}

const statusElement = requiredElement("status");
const resultElement = requiredElement("result");
const viewportElement = requiredElement("viewport");

void run().catch((error: unknown) => {
  const message = error instanceof Error ? error.stack ?? error.message : String(error);
  window.__PHASE0_ERROR__ = message;
  statusElement.textContent = "Benchmark failed";
  resultElement.textContent = message;
});

async function run(): Promise<void> {
  const parameters = new URLSearchParams(window.location.search);
  const requestedBackend = parseBackend(parameters.get("backend"));
  const organismCount = boundedInteger(parameters.get("count"), 2_000, 1, 100_000);
  const warmupFrames = boundedInteger(parameters.get("warmup"), 30, 0, 600);
  const sampledFrames = boundedInteger(parameters.get("frames"), 120, 10, 2_000);
  const drawCounter = installDrawCounter();
  const size = measureViewport();

  const app = new Application();
  await app.init({
    width: size.width,
    height: size.height,
    preference: requestedBackend,
    autoStart: false,
    sharedTicker: false,
    antialias: false,
    resolution: Math.min(window.devicePixelRatio, 2),
    autoDensity: true,
    background: 0x10182a,
    powerPreference: "high-performance",
  });
  app.stop();
  viewportElement.replaceChildren(app.canvas);

  const worldMultiplier = 2;
  const worldWidth = size.width * worldMultiplier;
  const worldHeight = size.height * worldMultiplier;
  const world = new Container();
  // Keep the parent measurable across camera movement. Culling the parent after
  // its children have been culled can shrink its computed bounds and prevent a
  // later recursive cull pass from reactivating newly visible children.
  world.cullable = false;
  world.cullableChildren = true;
  app.stage.addChild(world);

  const sprites: Sprite[] = [];
  for (let index = 0; index < organismCount; index += 1) {
    const sprite = new Sprite(Texture.WHITE);
    sprite.label = `synthetic-organism-${index + 1}`;
    sprite.width = 5;
    sprite.height = 5;
    sprite.anchor.set(0.5);
    sprite.position.set(
      deterministicCoordinate(index + 1, 0, worldWidth),
      deterministicCoordinate(index + 1, 1, worldHeight),
    );
    sprite.tint = deterministicTint(index + 1);
    sprite.cullable = true;
    sprite.cullArea = new Rectangle(-2.5, -2.5, 5, 5);
    world.addChild(sprite);
    sprites.push(sprite);
  }

  const rendererConstructor = app.renderer.constructor.name;
  const actualBackend = rendererConstructor.toLowerCase().includes("webgpu") ? "webgpu" : "webgl";
  statusElement.textContent = `${actualBackend} · ${organismCount.toLocaleString()} synthetic organisms · sampling`;

  const frameIntervals: number[] = [];
  const updateSamples: number[] = [];
  const renderSamples: number[] = [];
  const drawCallSamples: number[] = [];
  const visibleSamples: number[] = [];
  let previousFrameTimestamp: number | undefined;
  const totalFrames = warmupFrames + sampledFrames;

  for (let frame = 0; frame < totalFrames; frame += 1) {
    const timestamp = await nextAnimationFrame();
    const recording = frame >= warmupFrames;
    if (recording && previousFrameTimestamp !== undefined) {
      frameIntervals.push(timestamp - previousFrameTimestamp);
    }
    previousFrameTimestamp = timestamp;

    const updateStarted = performance.now();
    const cameraX = ((frame * 3) % Math.max(1, worldWidth - size.width));
    const cameraY = ((frame * 2) % Math.max(1, worldHeight - size.height));
    world.position.set(-cameraX, -cameraY);
    Culler.shared.cull(app.stage, app.renderer.screen, false);
    const visible = countVisible(sprites);
    const updateDuration = performance.now() - updateStarted;

    drawCounter.reset();
    const renderStarted = performance.now();
    app.renderer.render(app.stage);
    const renderDuration = performance.now() - renderStarted;

    if (recording) {
      updateSamples.push(updateDuration);
      renderSamples.push(renderDuration);
      drawCallSamples.push(drawCounter.read());
      visibleSamples.push(visible);
    }
  }

  const result: RendererBenchmarkResult = {
    benchmark_schema_version: 1,
    pixi_version: VERSION,
    requested_backend: requestedBackend,
    actual_backend: actualBackend,
    renderer_constructor: rendererConstructor,
    organism_count: organismCount,
    viewport_css_pixels: size,
    device_pixel_ratio: window.devicePixelRatio,
    world_multiplier: worldMultiplier,
    warmup_frames: warmupFrames,
    sampled_frames: sampledFrames,
    frame_interval_ms: summarize(frameIntervals),
    update_and_cull_cpu_ms: summarize(updateSamples),
    render_submit_cpu_ms: summarize(renderSamples),
    draw_calls_per_frame: summarize(drawCallSamples),
    visible_organisms: summarize(visibleSamples),
    webgpu_exposed: "gpu" in navigator,
    user_agent: navigator.userAgent,
    limitations: [
      "Synthetic square markers are benchmark fixtures, not production art.",
      "Render timing measures JavaScript CPU submission, not GPU completion.",
      "Draw calls are counted by instrumenting browser draw entry points and may omit implementation-private batching paths.",
      "Mobile results use a mobile viewport in the same local browser engine, not physical mobile hardware.",
    ],
  };
  window.__PHASE0_RESULT__ = result;
  statusElement.textContent = `${actualBackend} · ${organismCount.toLocaleString()} synthetic organisms · complete`;
  resultElement.textContent = JSON.stringify(result, null, 2);
}

function requiredElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (element === null) {
    throw new Error(`missing required element #${id}`);
  }
  return element;
}

function parseBackend(value: string | null): RequestedBackend {
  if (value === null || value === "webgl") {
    return "webgl";
  }
  if (value === "webgpu") {
    return "webgpu";
  }
  throw new Error(`unsupported backend: ${value}`);
}

function boundedInteger(value: string | null, fallback: number, minimum: number, maximum: number): number {
  const parsed = value === null ? fallback : Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`integer parameter must be between ${minimum} and ${maximum}`);
  }
  return parsed;
}

function measureViewport(): { width: number; height: number } {
  const bounds = viewportElement.getBoundingClientRect();
  return {
    width: Math.max(1, Math.floor(bounds.width)),
    height: Math.max(1, Math.floor(bounds.height)),
  };
}

function deterministicCoordinate(entityId: number, axis: number, extent: number): number {
  const mask = 0xffff_ffff_ffff_ffffn;
  let value = (BigInt(entityId) << 1n) ^ BigInt(axis);
  value = (value + 0x9e37_79b9_7f4a_7c15n) & mask;
  value = ((value ^ (value >> 30n)) * 0xbf58_476d_1ce4_e5b9n) & mask;
  value = ((value ^ (value >> 27n)) * 0x94d0_49bb_1331_11ebn) & mask;
  value ^= value >> 31n;
  const unit = Number(value >> 11n) / 9_007_199_254_740_992;
  return Math.floor(unit * Math.max(1, extent));
}

function deterministicTint(entityId: number): number {
  const palette = [0x8bd17c, 0x59c3c3, 0xf2c14e, 0xf78154, 0xb79ced] as const;
  return palette[entityId % palette.length] ?? palette[0];
}

function nextAnimationFrame(): Promise<number> {
  return new Promise((resolve) => window.requestAnimationFrame(resolve));
}

function countVisible(sprites: readonly Sprite[]): number {
  let visible = 0;
  for (const sprite of sprites) {
    if (sprite.renderable && !sprite.culled) {
      visible += 1;
    }
  }
  return visible;
}

function summarize(samples: readonly number[]): Percentiles {
  if (samples.length === 0) {
    return { p50: 0, p95: 0, p99: 0, min: 0, max: 0 };
  }
  const sorted = [...samples].sort((left, right) => left - right);
  return {
    p50: round(percentile(sorted, 0.5)),
    p95: round(percentile(sorted, 0.95)),
    p99: round(percentile(sorted, 0.99)),
    min: round(sorted[0] ?? 0),
    max: round(sorted.at(-1) ?? 0),
  };
}

function percentile(sorted: readonly number[], fraction: number): number {
  const index = Math.ceil((sorted.length - 1) * fraction);
  return sorted[index] ?? 0;
}

function round(value: number): number {
  return Math.round(value * 1_000) / 1_000;
}

interface DrawCounter {
  reset(): void;
  read(): number;
}

function installDrawCounter(): DrawCounter {
  let drawCalls = 0;
  const constructorNames = [
    "WebGLRenderingContext",
    "WebGL2RenderingContext",
    "GPURenderPassEncoder",
  ];
  const methodNames = [
    "drawArrays",
    "drawElements",
    "drawArraysInstanced",
    "drawElementsInstanced",
    "draw",
    "drawIndexed",
  ];
  const globals = globalThis as unknown as Record<string, unknown>;

  for (const constructorName of constructorNames) {
    const candidate = globals[constructorName];
    if (typeof candidate !== "function") {
      continue;
    }
    const prototype = (candidate as unknown as { prototype: Record<string, unknown> }).prototype;
    for (const methodName of methodNames) {
      const original = prototype[methodName];
      if (typeof original !== "function") {
        continue;
      }
      prototype[methodName] = function countedDraw(this: unknown, ...args: unknown[]): unknown {
        drawCalls += 1;
        return Reflect.apply(original, this, args);
      };
    }
  }

  return {
    reset(): void {
      drawCalls = 0;
    },
    read(): number {
      return drawCalls;
    },
  };
}
