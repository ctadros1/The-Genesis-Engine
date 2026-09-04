// Copied unchanged from apps/observer/src/protocol.ts on 2026-09-04.
// Source of truth stays apps/observer; do not diverge without re-copying.

// Binary observer protocol codec (client mirror of crates/sim-protocol).
// Network byte order via DataView; one WebSocket message = one frame.

export const PROTOCOL_MAJOR = 1;
export const PROTOCOL_MINOR = 0;
export const MAGIC = 0x414c5350; // "ALSP"

export const LAYER_TERRAIN = 1;
export const LAYER_ORGANISMS = 2;
export const LAYER_METRICS = 4;

export const FRAME_HELLO = 1;
export const FRAME_WELCOME = 2;
export const FRAME_SUBSCRIBE = 3;
export const FRAME_SUBSCRIBED = 4;
export const FRAME_KEYFRAME = 6;
export const FRAME_DELTA = 7;
export const FRAME_METRICS = 8;
export const FRAME_ACK = 9;
export const FRAME_ERROR = 10;

const HEADER_LEN = 32;

export interface EntityRecord {
  id: bigint;
  xFp: number;
  yFp: number;
  headingBam: number;
  flags: number;
  hue: number;
  pattern: number;
  bodyScale: number;
  energyFrac: number;
}

export interface TileBlock {
  x0: number;
  y0: number;
  width: number;
  height: number;
  cells: Uint8Array; // interleaved (flags, food)
}

export interface Welcome {
  kind: "welcome";
  configHash: bigint;
  cellsX: number;
  cellsY: number;
  cellSizeM: number;
  dtMs: number;
  phase2: boolean;
  maxRateHz: number;
}

export interface Keyframe {
  kind: "keyframe";
  sequence: bigint;
  tick: bigint;
  tiles: TileBlock | null;
  entities: EntityRecord[];
}

export interface Delta {
  kind: "delta";
  sequence: bigint;
  tick: bigint;
  removed: bigint[];
  upserts: EntityRecord[];
  tiles: TileBlock | null;
}

export interface MetricsSample {
  kind: "metrics";
  tick: bigint;
  population: number;
  birthsTotal: bigint;
  deathsStarvation: bigint;
  deathsOldAge: bigint;
  pairedBirths: bigint;
  totalBiomassMilli: bigint;
  maxAncestryDepth: number;
}

export interface Subscribed {
  kind: "subscribed";
  x0Fp: number;
  y0Fp: number;
  x1Fp: number;
  y1Fp: number;
  layers: number;
  maxRateHz: number;
}

export interface ErrorFrame {
  kind: "error";
  code: number;
  message: string;
}

export type ServerFrame =
  | Welcome
  | Subscribed
  | Keyframe
  | Delta
  | MetricsSample
  | ErrorFrame;

function envelope(frameType: number, payload: Uint8Array): ArrayBuffer {
  const buffer = new ArrayBuffer(HEADER_LEN + payload.length);
  const view = new DataView(buffer);
  view.setUint32(0, MAGIC);
  view.setUint16(4, PROTOCOL_MAJOR);
  view.setUint16(6, PROTOCOL_MINOR);
  view.setUint8(8, frameType);
  view.setUint8(9, 0);
  view.setUint16(10, HEADER_LEN);
  view.setBigUint64(12, 0n); // epoch (client-to-server frames carry zero)
  view.setBigUint64(20, 0n); // sequence
  view.setUint32(28, payload.length);
  new Uint8Array(buffer, HEADER_LEN).set(payload);
  return buffer;
}

export function encodeHello(token: string): ArrayBuffer {
  const tokenBytes = new TextEncoder().encode(token);
  const payload = new Uint8Array(10 + tokenBytes.length);
  const view = new DataView(payload.buffer);
  view.setUint16(0, PROTOCOL_MAJOR);
  view.setUint16(2, PROTOCOL_MINOR);
  view.setUint32(4, 0);
  view.setUint16(8, tokenBytes.length);
  payload.set(tokenBytes, 10);
  return envelope(FRAME_HELLO, payload);
}

export function encodeSubscribe(
  x0Fp: number,
  y0Fp: number,
  x1Fp: number,
  y1Fp: number,
  layers: number,
  maxRateHz: number,
): ArrayBuffer {
  const payload = new Uint8Array(22);
  const view = new DataView(payload.buffer);
  view.setInt32(0, x0Fp);
  view.setInt32(4, y0Fp);
  view.setInt32(8, x1Fp);
  view.setInt32(12, y1Fp);
  view.setUint8(16, 0); // LOD reserved
  view.setUint32(17, layers);
  view.setUint8(21, maxRateHz);
  return envelope(FRAME_SUBSCRIBE, payload);
}

export function encodeAck(appliedSequence: bigint): ArrayBuffer {
  const payload = new Uint8Array(8);
  new DataView(payload.buffer).setBigUint64(0, appliedSequence);
  return envelope(FRAME_ACK, payload);
}

class Cursor {
  view: DataView;
  offset: number;
  constructor(view: DataView, offset: number) {
    this.view = view;
    this.offset = offset;
  }
  u8(): number {
    const value = this.view.getUint8(this.offset);
    this.offset += 1;
    return value;
  }
  u16(): number {
    const value = this.view.getUint16(this.offset);
    this.offset += 2;
    return value;
  }
  u32(): number {
    const value = this.view.getUint32(this.offset);
    this.offset += 4;
    return value;
  }
  i32(): number {
    const value = this.view.getInt32(this.offset);
    this.offset += 4;
    return value;
  }
  u64(): bigint {
    const value = this.view.getBigUint64(this.offset);
    this.offset += 8;
    return value;
  }
  i64(): bigint {
    const value = this.view.getBigInt64(this.offset);
    this.offset += 8;
    return value;
  }
}

function readEntity(cursor: Cursor): EntityRecord {
  return {
    id: cursor.u64(),
    xFp: cursor.i32(),
    yFp: cursor.i32(),
    headingBam: cursor.u16(),
    flags: cursor.u8(),
    hue: cursor.u8(),
    pattern: cursor.u8(),
    bodyScale: cursor.u8(),
    energyFrac: cursor.u8(),
  };
}

function readTiles(cursor: Cursor, raw: Uint8Array): TileBlock | null {
  const mode = cursor.u8();
  if (mode === 0) return null;
  const x0 = cursor.u16();
  const y0 = cursor.u16();
  const width = cursor.u16();
  const height = cursor.u16();
  const byteLength = width * height * 2;
  const cells = raw.slice(cursor.offset, cursor.offset + byteLength);
  cursor.offset += byteLength;
  return { x0, y0, width, height, cells };
}

export function decodeFrame(buffer: ArrayBuffer): ServerFrame | null {
  const view = new DataView(buffer);
  if (buffer.byteLength < HEADER_LEN) return null;
  if (view.getUint32(0) !== MAGIC) return null;
  if (view.getUint16(4) !== PROTOCOL_MAJOR) return null;
  const frameType = view.getUint8(8);
  const headerLen = view.getUint16(10);
  const sequence = view.getBigUint64(20);
  const raw = new Uint8Array(buffer);
  const cursor = new Cursor(view, headerLen);
  switch (frameType) {
    case FRAME_WELCOME: {
      cursor.offset += 8; // major/minor/capabilities
      cursor.u64(); // world id
      const configHash = cursor.u64();
      return {
        kind: "welcome",
        configHash,
        cellsX: cursor.u32(),
        cellsY: cursor.u32(),
        cellSizeM: cursor.u32(),
        dtMs: cursor.u32(),
        phase2: cursor.u8() !== 0,
        maxRateHz: cursor.u8(),
      };
    }
    case FRAME_SUBSCRIBED: {
      const x0Fp = cursor.i32();
      const y0Fp = cursor.i32();
      const x1Fp = cursor.i32();
      const y1Fp = cursor.i32();
      cursor.u8(); // lod
      return {
        kind: "subscribed",
        x0Fp,
        y0Fp,
        x1Fp,
        y1Fp,
        layers: cursor.u32(),
        maxRateHz: cursor.u8(),
      };
    }
    case FRAME_KEYFRAME: {
      const tick = cursor.u64();
      const tiles = readTiles(cursor, raw);
      const count = cursor.u32();
      const entities: EntityRecord[] = [];
      for (let index = 0; index < count; index += 1) entities.push(readEntity(cursor));
      return { kind: "keyframe", sequence, tick, tiles, entities };
    }
    case FRAME_DELTA: {
      const tick = cursor.u64();
      cursor.u64(); // base sequence
      const removedCount = cursor.u32();
      const removed: bigint[] = [];
      for (let index = 0; index < removedCount; index += 1) removed.push(cursor.u64());
      const upsertCount = cursor.u32();
      const upserts: EntityRecord[] = [];
      for (let index = 0; index < upsertCount; index += 1) upserts.push(readEntity(cursor));
      const tiles = readTiles(cursor, raw);
      return { kind: "delta", sequence, tick, removed, upserts, tiles };
    }
    case FRAME_METRICS: {
      const tick = cursor.u64();
      const population = cursor.u32();
      const birthsTotal = cursor.u64();
      const deathsStarvation = cursor.u64();
      const deathsOldAge = cursor.u64();
      const pairedBirths = cursor.u64();
      const totalBiomassMilli = cursor.i64();
      cursor.i64(); // total energy (not charted yet)
      const maxAncestryDepth = cursor.u32();
      return {
        kind: "metrics",
        tick,
        population,
        birthsTotal,
        deathsStarvation,
        deathsOldAge,
        pairedBirths,
        totalBiomassMilli,
        maxAncestryDepth,
      };
    }
    case FRAME_ERROR: {
      const code = cursor.u16();
      const length = cursor.u16();
      const message = new TextDecoder().decode(
        raw.slice(cursor.offset, cursor.offset + length),
      );
      return { kind: "error", code, message };
    }
    default:
      return null;
  }
}
