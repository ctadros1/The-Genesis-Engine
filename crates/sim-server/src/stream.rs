//! WebSocket streaming: session handshake, subscription management, frame
//! building, and per-client backpressure.

use crate::state::{
    ClientSlot, MAX_PENDING_FRAMES, MAX_RATE_HZ, MIN_RATE_HZ, Role, Shared, TILE_REFRESH_INTERVAL,
    now_unix_ms,
};
use sim_core::{RenderEntity, World};
use sim_protocol::{
    ENTITY_FLAG_MATURE, EntityRecord, Frame, FrameMeta, LAYER_METRICS, LAYER_ORGANISMS,
    LAYER_TERRAIN, PROTOCOL_MAJOR, PROTOCOL_MINOR, TileBlock, Viewport, decode, encode,
};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tungstenite::{Message, WebSocket, accept};

const KNOWN_LAYERS: u32 = LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS;

pub fn websocket_listener(shared: Arc<Shared>, listener: TcpListener) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            let _ = handle_connection(shared, stream);
        });
    }
}

fn send_frame(
    socket: &mut WebSocket<TcpStream>,
    slot: Option<&ClientSlot>,
    frame: &Frame,
    meta: FrameMeta,
) -> Result<(), ()> {
    let bytes = encode(frame, meta);
    if let Some(slot) = slot {
        slot.bytes_sent
            .fetch_add(bytes.len() as u64, Ordering::Relaxed);
    }
    socket.send(Message::Binary(bytes)).map_err(|_| ())
}

fn handle_connection(shared: Arc<Shared>, stream: TcpStream) -> Result<(), ()> {
    stream
        .set_read_timeout(Some(Duration::from_millis(10)))
        .map_err(|_| ())?;
    let mut socket = accept(stream).map_err(|_| ())?;
    let epoch = shared.world_epoch;
    let meta = |sequence: u64| FrameMeta {
        world_epoch: epoch,
        sequence,
        checksummed: false,
    };

    // 1. Hello with a valid observer or admin token, before anything else.
    let hello = read_binary_blocking(&mut socket, Duration::from_secs(5))?;
    let authorized = match decode(&hello) {
        Ok((_, Frame::Hello { major, token, .. })) => {
            let token = String::from_utf8(token).unwrap_or_default();
            if major != PROTOCOL_MAJOR {
                None
            } else {
                shared.role_for(&token)
            }
        }
        _ => None,
    };
    if !matches!(authorized, Some(Role::Observer | Role::Admin)) {
        let _ = send_frame(
            &mut socket,
            None,
            &Frame::Error {
                code: 401,
                message: "unauthorized".to_owned(),
            },
            meta(0),
        );
        return Err(());
    }

    // 2. Welcome with world metadata and effective limits.
    {
        let world = shared.world.lock().expect("world");
        let config = world.config();
        send_frame(
            &mut socket,
            None,
            &Frame::Welcome {
                major: PROTOCOL_MAJOR,
                minor: PROTOCOL_MINOR,
                capabilities: KNOWN_LAYERS,
                world_id: 1,
                config_hash: world.config_hash(),
                cells_x: config.cells_x,
                cells_y: config.cells_y,
                cell_size_m: config.cell_size_m,
                dt_ms: config.dt_ms,
                phase2: world.phase2_enabled(),
                max_rate_hz: MAX_RATE_HZ,
            },
            meta(0),
        )?;
    }

    // 3. Register a slot; frames flow after the first Subscribe.
    let slot = ClientSlot::new(shared.next_client_id.fetch_add(1, Ordering::Relaxed));
    shared
        .clients
        .lock()
        .expect("clients")
        .push(Arc::clone(&slot));

    let result = session_loop(&shared, &mut socket, &slot, epoch);

    slot.closed.store(true, Ordering::Relaxed);
    shared
        .clients
        .lock()
        .expect("clients")
        .retain(|candidate| candidate.id != slot.id);
    result
}

fn session_loop(
    shared: &Arc<Shared>,
    socket: &mut WebSocket<TcpStream>,
    slot: &Arc<ClientSlot>,
    epoch: u64,
) -> Result<(), ()> {
    loop {
        if slot.closed.load(Ordering::Relaxed) {
            return Ok(());
        }
        // Drain any queued frames first (writer side).
        loop {
            let frame = {
                let mut queue = slot.queue.lock().expect("queue");
                queue.frames.pop_front()
            };
            match frame {
                Some(bytes) => {
                    slot.bytes_sent
                        .fetch_add(bytes.len() as u64, Ordering::Relaxed);
                    socket.send(Message::Binary(bytes)).map_err(|_| ())?;
                }
                None => break,
            }
        }

        // Reader side with a short timeout so the loop keeps draining.
        match socket.read() {
            Ok(Message::Binary(bytes)) => {
                let Ok((_, frame)) = decode(&bytes) else {
                    let _ = send_frame(
                        socket,
                        Some(slot),
                        &Frame::Error {
                            code: 400,
                            message: "malformed frame".to_owned(),
                        },
                        FrameMeta {
                            world_epoch: epoch,
                            sequence: 0,
                            checksummed: false,
                        },
                    );
                    continue;
                };
                match frame {
                    Frame::Subscribe {
                        viewport,
                        layers,
                        max_rate_hz,
                    } => {
                        let (effective_viewport, effective_layers, effective_rate) =
                            clamp_subscription(shared, viewport, layers, max_rate_hz);
                        {
                            let mut queue = slot.queue.lock().expect("queue");
                            queue.viewport = Some((
                                effective_viewport.x0_fp,
                                effective_viewport.y0_fp,
                                effective_viewport.x1_fp,
                                effective_viewport.y1_fp,
                            ));
                            queue.layers = effective_layers;
                            queue.rate_hz = effective_rate;
                            queue.needs_keyframe = true;
                            queue.last_entities.clear();
                            queue.frames.clear();
                        }
                        let sequence = next_sequence(slot);
                        send_frame(
                            socket,
                            Some(slot),
                            &Frame::Subscribed {
                                viewport: effective_viewport,
                                layers: effective_layers,
                                max_rate_hz: effective_rate,
                            },
                            FrameMeta {
                                world_epoch: epoch,
                                sequence,
                                checksummed: false,
                            },
                        )?;
                    }
                    Frame::Unsubscribe => {
                        let mut queue = slot.queue.lock().expect("queue");
                        queue.viewport = None;
                        queue.frames.clear();
                        queue.last_entities.clear();
                    }
                    Frame::Ack { applied_sequence } => {
                        let mut queue = slot.queue.lock().expect("queue");
                        queue.acked_sequence = applied_sequence;
                    }
                    // Clients cannot control tick state through the socket.
                    _ => {
                        let _ = send_frame(
                            socket,
                            Some(slot),
                            &Frame::Error {
                                code: 403,
                                message: "unsupported client frame".to_owned(),
                            },
                            FrameMeta {
                                world_epoch: epoch,
                                sequence: 0,
                                checksummed: false,
                            },
                        );
                    }
                }
            }
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return Err(()),
        }
    }
}

fn read_binary_blocking(
    socket: &mut WebSocket<TcpStream>,
    deadline: Duration,
) -> Result<Vec<u8>, ()> {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > deadline {
            return Err(());
        }
        match socket.read() {
            Ok(Message::Binary(bytes)) => return Ok(bytes.to_vec()),
            Ok(Message::Close(_)) => return Err(()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return Err(()),
        }
    }
}

fn next_sequence(slot: &ClientSlot) -> u64 {
    let mut queue = slot.queue.lock().expect("queue");
    let sequence = queue.next_sequence;
    queue.next_sequence += 1;
    sequence
}

fn clamp_subscription(
    shared: &Shared,
    viewport: Viewport,
    layers: u32,
    max_rate_hz: u8,
) -> (Viewport, u32, u8) {
    let world = shared.world.lock().expect("world");
    let config = world.config();
    let extent_x = config.world_extent_x_fp();
    let extent_y = config.world_extent_y_fp();
    let clamped = Viewport {
        x0_fp: viewport.x0_fp.clamp(0, extent_x - 1),
        y0_fp: viewport.y0_fp.clamp(0, extent_y - 1),
        x1_fp: viewport.x1_fp.clamp(0, extent_x - 1),
        y1_fp: viewport.y1_fp.clamp(0, extent_y - 1),
        lod: viewport.lod.min(3),
    };
    (
        clamped,
        layers & KNOWN_LAYERS,
        max_rate_hz.clamp(MIN_RATE_HZ, MAX_RATE_HZ),
    )
}

// --- Frame building (called from the tick thread) ---------------------------

fn tile_block(world: &World, bounds: (i32, i32, i32, i32)) -> TileBlock {
    let config = world.config();
    let terrain = world.terrain();
    let biomass = world.biomass_cells();
    let cell_fp = config.cell_size_fp();
    let cell_x0 = (bounds.0 / cell_fp).clamp(0, terrain.cells_x as i32 - 1) as u16;
    let cell_y0 = (bounds.1 / cell_fp).clamp(0, terrain.cells_y as i32 - 1) as u16;
    let cell_x1 = (bounds.2 / cell_fp).clamp(0, terrain.cells_x as i32 - 1) as u16;
    let cell_y1 = (bounds.3 / cell_fp).clamp(0, terrain.cells_y as i32 - 1) as u16;
    let width = cell_x1 - cell_x0 + 1;
    let height = cell_y1 - cell_y0 + 1;
    let mut cells = Vec::with_capacity(usize::from(width) * usize::from(height));
    for cell_y in cell_y0..=cell_y1 {
        for cell_x in cell_x0..=cell_x1 {
            let index = usize::from(cell_y) * terrain.cells_x as usize + usize::from(cell_x);
            let land = terrain.land[index];
            let capacity = terrain.capacity_milli[index];
            let food = if land && capacity > 0 {
                ((biomass[index].clamp(0, capacity) * 255) / capacity) as u8
            } else {
                0
            };
            cells.push((u8::from(land), food));
        }
    }
    TileBlock {
        x0: cell_x0,
        y0: cell_y0,
        width,
        height,
        cells,
    }
}

fn to_protocol_record(entity: &RenderEntity) -> EntityRecord {
    EntityRecord {
        id: entity.id,
        x_fp: entity.x_fp,
        y_fp: entity.y_fp,
        heading_bam: entity.heading_bam,
        flags: if entity.mature { ENTITY_FLAG_MATURE } else { 0 },
        pigment_hue: entity.pigment_hue_q8,
        pigment_pattern: entity.pigment_pattern_q8,
        body_scale: entity.body_scale_q8,
        energy_frac: entity.energy_frac_q8,
    }
}

/// Build and enqueue state frames for every subscribed client whose rate
/// window has elapsed. Runs on the tick thread with the world lock held
/// briefly per client; encoding happens outside the world lock.
pub fn broadcast(shared: &Shared, scratch: &mut Vec<RenderEntity>) {
    let now_ms = now_unix_ms();
    let clients: Vec<Arc<ClientSlot>> = shared.clients.lock().expect("clients").clone();
    for slot in clients {
        if slot.closed.load(Ordering::Relaxed) {
            continue;
        }
        let (bounds, layers, due, needs_keyframe, state_frames_sent, sequence, acked, backlog) = {
            let queue = slot.queue.lock().expect("queue");
            let Some(bounds) = queue.viewport else {
                continue;
            };
            let interval_ms = 1_000_u64 / u64::from(queue.rate_hz.max(1));
            let due = now_ms.saturating_sub(queue.last_state_frame_ms) >= interval_ms;
            (
                bounds,
                queue.layers,
                due,
                queue.needs_keyframe,
                queue.state_frames_sent,
                queue.next_sequence,
                queue.acked_sequence,
                queue.frames.len(),
            )
        };
        if !due {
            continue;
        }
        // Severe unacknowledged lag: collapse to keyframe on next window.
        let lagging = backlog >= MAX_PENDING_FRAMES / 2 || sequence.saturating_sub(acked) > 64;

        let (frame, entity_map) = {
            let world = shared.world.lock().expect("world");
            let tick = world.tick_number();
            if layers & LAYER_ORGANISMS != 0 {
                world.render_entities_in(bounds.0, bounds.1, bounds.2, bounds.3, scratch);
            } else {
                scratch.clear();
            }
            let records: Vec<EntityRecord> = scratch.iter().map(to_protocol_record).collect();
            if needs_keyframe || lagging {
                let tiles = if layers & LAYER_TERRAIN != 0 {
                    Some(tile_block(&world, bounds))
                } else {
                    None
                };
                let map = records
                    .iter()
                    .map(|record| (record.id, *record))
                    .collect::<std::collections::HashMap<_, _>>();
                (
                    Frame::Keyframe {
                        tick,
                        tiles,
                        entities: records,
                    },
                    map,
                )
            } else {
                // Delta against this client's previous view.
                let queue = slot.queue.lock().expect("queue");
                let previous = &queue.last_entities;
                let mut upserts = Vec::new();
                let mut map = std::collections::HashMap::with_capacity(records.len());
                for record in &records {
                    map.insert(record.id, *record);
                    match previous.get(&record.id) {
                        Some(previous_record) if previous_record == record => {}
                        _ => upserts.push(*record),
                    }
                }
                let removed: Vec<u64> = previous
                    .keys()
                    .filter(|id| !map.contains_key(id))
                    .copied()
                    .collect();
                let tiles = if layers & LAYER_TERRAIN != 0
                    && state_frames_sent % TILE_REFRESH_INTERVAL == 0
                {
                    Some(tile_block(&world, bounds))
                } else {
                    None
                };
                (
                    Frame::Delta {
                        tick,
                        base_sequence: sequence.saturating_sub(1),
                        removed,
                        upserts,
                        tiles,
                    },
                    map,
                )
            }
        };

        // Encode and enqueue outside the world lock.
        let sequence = next_sequence(&slot);
        let bytes = encode(
            &frame,
            FrameMeta {
                world_epoch: shared.world_epoch,
                sequence,
                checksummed: false,
            },
        );
        {
            let mut queue = slot.queue.lock().expect("queue");
            queue.last_entities = entity_map;
            queue.needs_keyframe = false;
            queue.last_state_frame_ms = now_ms;
            queue.state_frames_sent += 1;
        }
        slot.push_frame(bytes);
    }
}

/// Enqueue a metrics sample for clients subscribed to the metrics layer.
pub fn broadcast_metrics(shared: &Shared) {
    let metrics = {
        let world = shared.world.lock().expect("world");
        world.metrics()
    };
    let frame = Frame::MetricsSample {
        tick: metrics.tick,
        population: metrics.population as u32,
        births_total: metrics.births_total,
        deaths_starvation_total: metrics.deaths_starvation_total,
        deaths_old_age_total: metrics.deaths_old_age_total,
        paired_births_total: metrics.paired_births_total,
        total_biomass_milli: metrics.total_biomass_milli,
        total_energy_milli: metrics.total_energy_milli,
        max_ancestry_depth: metrics.max_ancestry_depth,
    };
    let clients: Vec<Arc<ClientSlot>> = shared.clients.lock().expect("clients").clone();
    for slot in clients {
        if slot.closed.load(Ordering::Relaxed) {
            continue;
        }
        let subscribed = {
            let queue = slot.queue.lock().expect("queue");
            queue.viewport.is_some() && queue.layers & LAYER_METRICS != 0
        };
        if !subscribed {
            continue;
        }
        let sequence = next_sequence(&slot);
        let bytes = encode(
            &frame,
            FrameMeta {
                world_epoch: shared.world_epoch,
                sequence,
                checksummed: false,
            },
        );
        slot.push_frame(bytes);
    }
}
