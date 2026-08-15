// LifeSim Observer: connection, subscription, input, inspector, charts,
// and controls. The browser never owns world truth: every mutation goes
// through the authenticated REST control API, and all state arrives from
// the server stream.

import { Application } from "pixi.js";
import {
  LAYER_METRICS,
  LAYER_ORGANISMS,
  LAYER_TERRAIN,
  decodeFrame,
  encodeAck,
  encodeHello,
  encodeSubscribe,
  type MetricsSample,
  type ServerFrame,
} from "./protocol";
import { FP_PER_METER, WorldRenderer } from "./render";
import "./style.css";

interface ObserverState {
  connected: boolean;
  tick: bigint;
  population: number;
  paused: boolean;
  reconnects: number;
  keyframes: number;
  deltas: number;
  selectedId: string | null;
  frameIntervals: number[];
}

declare global {
  interface Window {
    __OBS__: ObserverState & {
      cameraX: number;
      cameraY: number;
      zoom: number;
      screenOf(id: string): { x: number; y: number } | null;
      anyOrganismId(): string | null;
      forceCloseSocket(): void;
    };
  }
}

const query = new URLSearchParams(window.location.search);
// Development keeps the two loopback endpoints explicit. A deployed build is
// served by the same private origin as its reverse proxy, so it never embeds a
// host address in the artifact and works from LAN/WireGuard clients.
const defaultRestBase = import.meta.env.DEV
  ? "http://127.0.0.1:8940"
  : window.location.origin;
const defaultWsUrl = import.meta.env.DEV
  ? "ws://127.0.0.1:8941"
  : `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host}/ws`;
const restBase = query.get("rest") ?? defaultRestBase;
const wsUrl = query.get("ws") ?? defaultWsUrl;

const statusElement = mustGet("status");
const connectPanel = mustGet("connect-panel");
const stageWrap = mustGet("stage-wrap");
const viewportElement = mustGet("viewport");
const inspector = mustGet("inspector");
const inspectBody = mustGet("inspect-body");
const inspectId = mustGet("inspect-id");
const chartCanvas = mustGet("chart") as HTMLCanvasElement;
const chartText = mustGet("chart-text");

const state: ObserverState = {
  connected: false,
  tick: 0n,
  population: 0,
  paused: false,
  reconnects: 0,
  keyframes: 0,
  deltas: 0,
  selectedId: null,
  frameIntervals: [],
};

let socket: WebSocket | null = null;
let renderer: WorldRenderer | null = null;
let app: Application | null = null;
let observerToken = query.get("token") ?? "";
let adminToken = query.get("admin") ?? "";
let selectedSensorRangeM: number | null = null;
let reconnectDelayMs = 500;
let populationHistory: number[] = [];
let intentionalClose = false;

function mustGet(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) throw new Error(`missing #${id}`);
  return element;
}

function setStatus(text: string): void {
  statusElement.textContent = text;
}

async function restFetch(path: string, options: RequestInit = {}, admin = false): Promise<Response> {
  const headers = new Headers(options.headers);
  headers.set("Authorization", `Bearer ${admin ? adminToken : observerToken}`);
  return fetch(`${restBase}${path}`, { ...options, headers });
}

// --- Connection --------------------------------------------------------------

function connect(): void {
  intentionalClose = false;
  const ws = new WebSocket(wsUrl);
  ws.binaryType = "arraybuffer";
  socket = ws;
  setStatus("Connecting...");

  ws.onopen = () => {
    ws.send(encodeHello(observerToken));
  };
  ws.onmessage = (event: MessageEvent<ArrayBuffer>) => {
    const frame = decodeFrame(event.data);
    if (frame) handleFrame(frame, ws);
  };
  ws.onclose = () => {
    state.connected = false;
    if (intentionalClose) return;
    setStatus(`Reconnecting (attempt ${state.reconnects + 1})...`);
    state.reconnects += 1;
    const delay = reconnectDelayMs;
    reconnectDelayMs = Math.min(reconnectDelayMs * 2, 5_000);
    setTimeout(() => connect(), delay);
  };
  ws.onerror = () => ws.close();
}

function subscribeAll(ws: WebSocket): void {
  const worldFp = 1_048_576; // server clamps to actual extent
  ws.send(
    encodeSubscribe(0, 0, worldFp - 1, worldFp - 1, LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS, 20),
  );
}

function handleFrame(frame: ServerFrame, ws: WebSocket): void {
  switch (frame.kind) {
    case "welcome": {
      if (!renderer) {
        void initialiseStage({
          cellsX: frame.cellsX,
          cellsY: frame.cellsY,
          cellSizeM: frame.cellSizeM,
        }).then(() => subscribeAll(ws));
      } else {
        subscribeAll(ws);
      }
      break;
    }
    case "subscribed":
      state.connected = true;
      reconnectDelayMs = 500;
      setStatus("Connected");
      break;
    case "keyframe": {
      if (!renderer) break;
      state.keyframes += 1;
      state.tick = frame.tick;
      if (frame.tiles) renderer.applyTiles(frame.tiles);
      renderer.replaceEntities(frame.entities);
      ws.send(encodeAck(frame.sequence));
      updateHud();
      break;
    }
    case "delta": {
      if (!renderer) break;
      state.deltas += 1;
      state.tick = frame.tick;
      for (const id of frame.removed) renderer.remove(id);
      for (const record of frame.upserts) renderer.upsert(record);
      if (frame.tiles) renderer.applyTiles(frame.tiles);
      ws.send(encodeAck(frame.sequence));
      updateHud();
      break;
    }
    case "metrics":
      onMetrics(frame);
      break;
    case "error":
      setStatus(`Server error ${frame.code}: ${frame.message}`);
      if (frame.code === 401) intentionalClose = true;
      break;
    default:
      break;
  }
}

// --- Stage / input -----------------------------------------------------------

async function initialiseStage(world: {
  cellsX: number;
  cellsY: number;
  cellSizeM: number;
}): Promise<void> {
  connectPanel.hidden = true;
  stageWrap.hidden = false;
  app = new Application();
  await app.init({
    width: viewportElement.clientWidth,
    height: viewportElement.clientHeight,
    // WebGPU preferred with WebGL fallback per ADR-0005; PixiJS negotiates.
    preference: "webgl",
    background: 0x10182a,
    resolution: Math.min(window.devicePixelRatio, 2),
    autoDensity: true,
  });
  viewportElement.replaceChildren(app.canvas);
  renderer = new WorldRenderer(app, world);
  installInput(app, renderer);
  installFrameSampler(app);
  window.addEventListener("resize", () => {
    app?.renderer.resize(viewportElement.clientWidth, viewportElement.clientHeight);
  });
}

function installInput(application: Application, worldRenderer: WorldRenderer): void {
  const canvas = application.canvas;
  const pointers = new Map<number, { x: number; y: number }>();
  let pinchDistance = 0;
  let dragged = false;

  canvas.addEventListener("pointerdown", (event) => {
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    dragged = false;
    canvas.setPointerCapture(event.pointerId);
  });
  canvas.addEventListener("pointermove", (event) => {
    const previous = pointers.get(event.pointerId);
    if (!previous) return;
    if (pointers.size === 1) {
      const dx = event.clientX - previous.x;
      const dy = event.clientY - previous.y;
      if (Math.abs(dx) + Math.abs(dy) > 2) dragged = true;
      worldRenderer.camera.x += dx;
      worldRenderer.camera.y += dy;
    } else if (pointers.size === 2) {
      const entries = [...pointers.values()];
      const other = entries.find((entry) => entry !== previous);
      if (other) {
        const distance = Math.hypot(event.clientX - other.x, event.clientY - other.y);
        if (pinchDistance > 0) applyZoom(worldRenderer, distance / pinchDistance, event.clientX, event.clientY);
        pinchDistance = distance;
      }
      dragged = true;
    }
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
  });
  canvas.addEventListener("pointerup", (event) => {
    pointers.delete(event.pointerId);
    pinchDistance = 0;
    if (!dragged) void handleSelect(event, worldRenderer);
  });
  canvas.addEventListener("pointercancel", (event) => {
    pointers.delete(event.pointerId);
    pinchDistance = 0;
  });
  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      applyZoom(worldRenderer, event.deltaY < 0 ? 1.15 : 1 / 1.15, event.clientX, event.clientY);
    },
    { passive: false },
  );
}

function applyZoom(worldRenderer: WorldRenderer, factor: number, pivotX: number, pivotY: number): void {
  const camera = worldRenderer.camera;
  const oldScale = camera.scale.x;
  const newScale = Math.min(Math.max(oldScale * factor, worldRenderer.baseScale * 0.5), worldRenderer.baseScale * 60);
  const worldX = (pivotX - camera.x) / oldScale;
  const worldY = (pivotY - camera.y) / oldScale;
  camera.scale.set(newScale);
  camera.x = pivotX - worldX * newScale;
  camera.y = pivotY - worldY * newScale;
}

async function handleSelect(event: PointerEvent, worldRenderer: WorldRenderer): Promise<void> {
  const camera = worldRenderer.camera;
  const worldX = (event.clientX - camera.x) / camera.scale.x;
  const worldY = (event.clientY - camera.y) / camera.scale.y;
  const pickRadius = Math.max(2, 8 / camera.scale.x);
  const id = worldRenderer.pick(worldX, worldY, pickRadius);
  worldRenderer.select(id);
  state.selectedId = id === null ? null : id.toString();
  if (id === null) {
    inspector.hidden = true;
    selectedSensorRangeM = null;
    worldRenderer.redrawOverlay(null);
    return;
  }
  const response = await restFetch(`/api/worlds/1/organisms/${id}`);
  if (!response.ok) {
    inspector.hidden = true;
    return;
  }
  const detail = (await response.json()) as Record<string, unknown>;
  inspectId.textContent = String(detail.id);
  const rows: [string, string][] = [
    ["Energy", `${((detail.energy_milli as number) / 1000).toFixed(2)} EU`],
    ["Age", `${detail.age_ticks} ticks`],
    ["Position", `${((detail.x_fp as number) / FP_PER_METER).toFixed(1)}, ${((detail.y_fp as number) / FP_PER_METER).toFixed(1)} m`],
  ];
  const phenotype = detail.phenotype as Record<string, number> | undefined;
  if (phenotype) {
    rows.push(
      ["Parents", (detail.parents as number[]).join(", ") || "founders"],
      ["Generation", String(detail.ancestry_depth)],
      ["Offspring", String(detail.child_count)],
      ["Max speed", `${(phenotype.max_speed_milli ?? 0) / 1000} m/s`],
      ["Sensor range", `${(phenotype.sensor_range_milli ?? 0) / 1000} m`],
      ["Genome", String(detail.genome_hash)],
    );
    selectedSensorRangeM = (phenotype.sensor_range_milli ?? 0) / 1000;
  }
  inspectBody.replaceChildren(
    ...rows.flatMap(([term, value]) => {
      const dt = document.createElement("dt");
      dt.textContent = term;
      const dd = document.createElement("dd");
      dd.textContent = value;
      return [dt, dd];
    }),
  );
  inspector.hidden = false;
  worldRenderer.redrawOverlay(selectedSensorRangeM);
}

function installFrameSampler(application: Application): void {
  let previous = performance.now();
  application.ticker.add(() => {
    const now = performance.now();
    state.frameIntervals.push(now - previous);
    if (state.frameIntervals.length > 240) state.frameIntervals.shift();
    previous = now;
  });
}

// --- HUD / charts / controls -------------------------------------------------

function updateHud(): void {
  const paused = state.paused ? " | PAUSED" : "";
  setStatus(
    `Connected | tick ${state.tick} | ${state.population} organisms${paused}` +
      (state.selectedId ? ` | selected ${state.selectedId}` : ""),
  );
}

function onMetrics(sample: MetricsSample): void {
  state.population = sample.population;
  populationHistory.push(sample.population);
  if (populationHistory.length > 130) populationHistory.shift();
  const context = chartCanvas.getContext("2d");
  if (context) {
    context.clearRect(0, 0, chartCanvas.width, chartCanvas.height);
    context.strokeStyle = "#59c3c3";
    context.lineWidth = 2;
    const max = Math.max(...populationHistory, 1);
    context.beginPath();
    populationHistory.forEach((value, index) => {
      const x = (index / Math.max(populationHistory.length - 1, 1)) * chartCanvas.width;
      const y = chartCanvas.height - (value / max) * (chartCanvas.height - 6) - 3;
      if (index === 0) context.moveTo(x, y);
      else context.lineTo(x, y);
    });
    context.stroke();
  }
  // Text alternative for the chart (accessibility).
  chartText.textContent =
    `Population ${sample.population}, births ${sample.birthsTotal}, ` +
    `deaths ${sample.deathsStarvation + sample.deathsOldAge}, generation depth ${sample.maxAncestryDepth}`;
  updateHud();
}

function installControls(): void {
  const pause = mustGet("btn-pause") as HTMLButtonElement;
  const resume = mustGet("btn-resume") as HTMLButtonElement;
  const speed = mustGet("speed") as HTMLSelectElement;
  const overlay = mustGet("btn-overlay") as HTMLButtonElement;

  const adminEnabled = () => adminToken.length > 0;
  const refresh = () => {
    pause.disabled = !adminEnabled();
    resume.disabled = !adminEnabled();
    speed.disabled = !adminEnabled();
  };
  refresh();

  const control = async (query: string) => {
    const response = await restFetch(
      `/api/worlds/1/control?${query}`,
      { method: "POST", headers: { "Idempotency-Key": crypto.randomUUID() } },
      true,
    );
    if (response.ok) {
      const body = (await response.json()) as { paused: boolean };
      state.paused = body.paused;
      updateHud();
    } else {
      setStatus(`Control rejected (${response.status})`);
    }
  };
  pause.addEventListener("click", () => void control("action=pause"));
  resume.addEventListener("click", () => void control("action=resume"));
  speed.addEventListener("change", () => void control(`action=speed&multiplier=${speed.value}`));
  overlay.addEventListener("click", () => {
    if (!renderer) return;
    renderer.overlayEnabled = !renderer.overlayEnabled;
    overlay.setAttribute("aria-pressed", String(renderer.overlayEnabled));
    renderer.redrawOverlay(selectedSensorRangeM);
  });

  mustGet("inspect-close").addEventListener("click", () => {
    inspector.hidden = true;
    renderer?.select(null);
    state.selectedId = null;
  });

  mustGet("connect").addEventListener("click", () => {
    observerToken = (mustGet("observer-token") as HTMLInputElement).value.trim();
    adminToken = (mustGet("admin-token") as HTMLInputElement).value.trim();
    refresh();
    connect();
  });
}

// --- Test hook ---------------------------------------------------------------

// Getters must stay live (Object.assign would snapshot their values).
Object.defineProperties(state, {
  cameraX: { get: () => renderer?.camera.x ?? 0 },
  cameraY: { get: () => renderer?.camera.y ?? 0 },
  zoom: { get: () => renderer?.camera.scale.x ?? 1 },
});
window.__OBS__ = Object.assign(state as typeof window.__OBS__, {
  screenOf(id: string) {
    if (!renderer) return null;
    const record = renderer.records.get(BigInt(id));
    if (!record) return null;
    const camera = renderer.camera;
    return {
      x: camera.x + (record.xFp / FP_PER_METER) * camera.scale.x,
      y: camera.y + (record.yFp / FP_PER_METER) * camera.scale.y,
    };
  },
  anyOrganismId() {
    if (!renderer) return null;
    const first = renderer.records.keys().next();
    return first.done ? null : first.value.toString();
  },
  forceCloseSocket() {
    socket?.close();
  },
});

installControls();
if (observerToken) {
  // Token supplied via query parameter (test/kiosk mode on a private
  // network only; the interactive path uses the input fields).
  connect();
}
