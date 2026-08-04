# WebSocket Protocol

## Phase 3 Implementation Notes (protocol `ALSP` 1.0)

Implemented in `crates/sim-protocol` (pure codec, no I/O) and served by
`crates/sim-server`. Concrete layout: magic `ALSP`, major 1, minor 0, frame
types Hello/Welcome/Subscribe/Subscribed/Unsubscribe/Keyframe/Delta/
MetricsSample/Ack/Error, flags bit 0 = optional payload CRC32 in the
header, big-endian throughout, one frame per WebSocket binary message.
Bounds enforced before allocation: 4 MiB payload, 65,536 entities or
removals per frame, 262,144 tile cells, 128-byte token, 256-byte error
message. Entity records are 23 bytes (ID, position, heading, flags, four
quantized render traits); genome/controller matrices never appear in
state frames. Terrain travels as quantized (land, food) cell blocks in
keyframes and periodic delta refreshes (every 20th state frame). The
Hello frame carries the bearer token; the server answers unauthorized
sockets with Error 401 and closes. Golden-byte, negative, endianness,
hostile-count, and a 20,000-case seeded corruption sweep cover the codec;
sequence-gap/lag resync and slow-client collapse are covered by server
integration tests.

## Planned Successor: `ALSP` 1.1 (Phase 11)

Objects and organism-modified terrain must reach the observer, and neither
fits the current frame set. The protocol version increments; frames are
never silently extended.

- **Object records** in keyframes and deltas, bounded by viewport exactly as
  entity records are: object ID, position, material, integrity bucket,
  composition depth, and a held-by flag. Composition lists never travel in a
  state frame; deep detail stays on the HTTP lookup path.
- **Terrain modification deltas**: the existing quantized `(land, food)`
  cell blocks gain the modified layers, carried in keyframes and periodic
  delta refreshes on the same cadence.
- Per-frame bounds gain object-record and modified-cell counts, enforced
  before allocation like every existing count.

Unchanged and non-negotiable: genome and controller matrices never appear in
state frames, which matters more after Phase 8 because variable-topology
genomes are larger and more variable in size than schema 1's fixed 2,862
bytes. Health, learned weights, and signal-field values likewise stay out of
state frames; the observer gets bounded summaries and uses HTTP for detail.

Version negotiation, bounded fail-closed decode, and the corruption-sweep
discipline apply to the new records exactly as to the existing ones.

## Envelope

All binary frames begin with: magic u32, protocol major u16, protocol minor u16, frame type u8, flags u8, header length u16, world epoch u64, sequence u64, payload length u32, payload checksum optional by flag. Network byte order is required. Reject invalid magic/version/header/payload length before allocation.

## Subscription

Client requests world ID, viewport bounds, zoom/LOD, selected layers, maximum state-frame rate, and protocol capabilities. Server clamps all fields to configured limits and returns effective values in Welcome/Subscribed response.

## Keyframe

Contains visible terrain tile references/data and all visible entity records at one sequence. It is self-contained for the subscription. Clients discard prior deltas for that epoch and apply the keyframe atomically.

## Delta

Contains adds, updates, removals, tile revisions, and optional aggregated cells for one base sequence. Entity records carry stable IDs, position/heading, render traits, selected state flags, and compact metrics only. Deep details use HTTP lookup.

## Backpressure

Server tracks acknowledgement age and queue budget. On lag, it drops superseded deltas, sends an aggregated keyframe, or disconnects abusive clients. No client action can force a full-world stream or stall ticks.

## Tests

Golden encode/decode, endian, truncated/oversized/corrupt frame, version negotiation, sequence gap, duplicate, resync, and slow-client tests are mandatory.
