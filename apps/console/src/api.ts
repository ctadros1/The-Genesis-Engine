// Typed REST client for the multi-world server, per the routes table in
// planning/console-and-multi-world-server.md. Every call returns a typed
// result instead of throwing on an HTTP error, so screens can render a
// message inline rather than unwind: { ok: true, value } | { ok: false,
// status, message }. Network failure (no response at all) reports status 0.
//
// Auth: bearer token from the active profile, admin token when present and
// the call needs it, else the observer token. Mutations carry an
// Idempotency-Key header (crypto.randomUUID()) so a retried click cannot
// double-apply.
//
// Field-name note: the plan's summary row writes "deaths_*"; this client
// follows the two death counters the streaming protocol already reports
// (protocol.ts's MetricsSample: deathsStarvation, deathsOldAge) as
// deaths_starvation / deaths_old_age.

import type { Profile } from "./profiles";

export type ApiResult<T> = { ok: true; value: T } | { ok: false; status: number; message: string };

export type FieldType = "u32" | "u64" | "i32" | "i64" | "bool" | "choice";

export interface SchemaField {
  name: string;
  type: FieldType;
  choices?: string[];
  defaults: Record<string, string>; // keyed by preset name
}

export interface SchemaPreset {
  name: string;
  description: string;
}

export interface SchemaLimits {
  max_worlds: number;
  max_cells_x: number;
  max_cells_y: number;
}

export interface Schema {
  presets: SchemaPreset[];
  fields: SchemaField[];
  limits: SchemaLimits;
}

export interface SchemaPreview {
  config_hash: string;
  seed: string;
  valid: boolean;
  errors: string[];
}

export type WorldStatus = "running" | "paused" | "stopped";

export interface WorldSummary {
  world_id: number;
  name: string;
  status: WorldStatus;
  created_unix_ms: number;
  parent_world_id: number | null;
  world_epoch: number;
  preset: string;
  seed: string;
  config_hash: string;
  cells_x: number;
  cells_y: number;
  cell_size_m: number;
  dt_ms: number;
  tick: number;
  population: number;
  births_total: number;
  deaths_starvation: number;
  deaths_old_age: number;
  extinct: boolean;
  phase2: boolean;
  paused: boolean;
  speed_multiplier: number;
  tick_mean_us: number;
  ticks_per_second: number;
  total_biomass_milli: number;
  total_energy_milli: number;
}

export interface CreateWorldRequest {
  name: string;
  preset: string;
  seed?: string;
  settings: Record<string, string>;
  paused?: boolean;
  speed?: number;
}

export type ControlAction = "pause" | "resume" | "speed" | "stop";

export interface SaveRecord {
  save_id: number;
  world_id: number;
  name: string;
  kind: string;
  tick: number;
  bytes: number;
  compressed: boolean;
  format_version: number;
  config_hash: string;
  state_checksum: string;
  created_unix_ms: number;
  verified: boolean;
}

export interface AuditRecord {
  id: number;
  unix_ms: number;
  role: "observer" | "admin";
  action: string;
  accepted: boolean;
  detail: string;
  tick: number;
  world_id: number;
  idempotency_key: string;
}

export interface OrganismDetail {
  id: string;
  [key: string]: unknown;
}

export interface AnalysisReport {
  [key: string]: unknown;
}

export interface BenchmarkSeries {
  world_id: number;
  samples: { tick: number; mean_us: number }[];
}

export interface HealthStatus {
  status: string;
}

function randomIdempotencyKey(): string {
  return crypto.randomUUID();
}

export class ApiClient {
  constructor(private profile: Profile) {}

  private token(admin: boolean): string {
    if (admin) {
      if (!this.profile.adminToken) {
        throw new Error("admin action requested without an admin token on the active profile");
      }
      return this.profile.adminToken;
    }
    return this.profile.observerToken;
  }

  private authToken(): string {
    return this.profile.adminToken && this.profile.adminToken.length > 0
      ? this.profile.adminToken
      : this.profile.observerToken;
  }

  private async request<T>(
    path: string,
    options: { method?: string; body?: unknown; admin?: boolean; mutating?: boolean } = {},
  ): Promise<ApiResult<T>> {
    const { method = "GET", body, admin = false, mutating = false } = options;
    const headers = new Headers();
    const token = admin ? this.token(true) : this.authToken();
    headers.set("Authorization", `Bearer ${token}`);
    if (body !== undefined) headers.set("Content-Type", "application/json");
    if (mutating) headers.set("Idempotency-Key", randomIdempotencyKey());

    let response: Response;
    try {
      response = await fetch(`${this.profile.restBase}${path}`, {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
      });
    } catch (error) {
      return { ok: false, status: 0, message: error instanceof Error ? error.message : "network error" };
    }

    const text = await response.text();
    if (!response.ok) {
      return { ok: false, status: response.status, message: extractMessage(text, response.statusText) };
    }
    if (text.length === 0) {
      return { ok: true, value: undefined as T };
    }
    try {
      return { ok: true, value: JSON.parse(text) as T };
    } catch {
      return { ok: false, status: response.status, message: "malformed response body" };
    }
  }

  health(): Promise<ApiResult<HealthStatus>> {
    return this.request<HealthStatus>("/api/health");
  }

  schema(): Promise<ApiResult<Schema>> {
    return this.request<Schema>("/api/schema");
  }

  schemaPreview(body: {
    preset: string;
    seed?: string;
    settings: Record<string, string>;
  }): Promise<ApiResult<SchemaPreview>> {
    return this.request<SchemaPreview>("/api/schema/preview", { method: "POST", body });
  }

  listWorlds(): Promise<ApiResult<WorldSummary[]>> {
    return this.request<WorldSummary[]>("/api/worlds");
  }

  createWorld(body: CreateWorldRequest): Promise<ApiResult<WorldSummary>> {
    return this.request<WorldSummary>("/api/worlds", { method: "POST", body, admin: true, mutating: true });
  }

  getWorld(worldId: number): Promise<ApiResult<WorldSummary>> {
    return this.request<WorldSummary>(`/api/worlds/${worldId}`);
  }

  control(worldId: number, action: ControlAction, multiplier?: number): Promise<ApiResult<WorldSummary>> {
    const params = new URLSearchParams({ action });
    if (multiplier !== undefined) params.set("multiplier", String(multiplier));
    return this.request<WorldSummary>(`/api/worlds/${worldId}/control?${params.toString()}`, {
      method: "POST",
      admin: true,
      mutating: true,
    });
  }

  deleteWorld(worldId: number): Promise<ApiResult<void>> {
    return this.request<void>(`/api/worlds/${worldId}`, { method: "DELETE", admin: true, mutating: true });
  }

  listSaves(worldId: number): Promise<ApiResult<SaveRecord[]>> {
    return this.request<SaveRecord[]>(`/api/worlds/${worldId}/saves`);
  }

  createSave(worldId: number, name: string): Promise<ApiResult<SaveRecord>> {
    return this.request<SaveRecord>(`/api/worlds/${worldId}/saves`, {
      method: "POST",
      body: { name },
      admin: true,
      mutating: true,
    });
  }

  verifySave(worldId: number, saveId: number): Promise<ApiResult<SaveRecord>> {
    return this.request<SaveRecord>(`/api/worlds/${worldId}/saves/${saveId}/verify`, {
      method: "POST",
      admin: true,
      mutating: true,
    });
  }

  branch(worldId: number, saveId: number, name: string): Promise<ApiResult<WorldSummary>> {
    const params = new URLSearchParams({ save_id: String(saveId), name });
    return this.request<WorldSummary>(`/api/worlds/${worldId}/branch?${params.toString()}`, {
      method: "POST",
      admin: true,
      mutating: true,
    });
  }

  organism(worldId: number, organismId: string): Promise<ApiResult<OrganismDetail>> {
    return this.request<OrganismDetail>(`/api/worlds/${worldId}/organisms/${organismId}`);
  }

  analysis(worldId: number): Promise<ApiResult<AnalysisReport>> {
    return this.request<AnalysisReport>(`/api/worlds/${worldId}/analysis`);
  }

  audit(): Promise<ApiResult<AuditRecord[]>> {
    return this.request<AuditRecord[]>("/api/audit", { admin: true });
  }

  benchmarks(worldId: number): Promise<ApiResult<BenchmarkSeries>> {
    const params = new URLSearchParams({ world: String(worldId) });
    return this.request<BenchmarkSeries>(`/api/benchmarks/ticks?${params.toString()}`);
  }

  /// WebSocket URL for a world's stream: "/worlds/{id}" on the profile's WS
  /// base ("/" selects world 1, per the plan's path selection).
  wsUrl(worldId: number): string {
    const base = this.profile.wsBase.replace(/\/$/, "");
    return `${base}/worlds/${worldId}`;
  }
}

function extractMessage(bodyText: string, fallback: string): string {
  if (bodyText.length === 0) return fallback;
  try {
    const parsed = JSON.parse(bodyText) as unknown;
    if (parsed && typeof parsed === "object") {
      const record = parsed as Record<string, unknown>;
      if (typeof record.error === "string") return record.error;
      if (typeof record.message === "string") return record.message;
    }
  } catch {
    // Not JSON: fall through to the raw body.
  }
  return bodyText;
}
