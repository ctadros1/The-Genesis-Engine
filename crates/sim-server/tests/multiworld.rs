//! Multi-world acceptance tests (S1-S4 of
//! `planning/console-and-multi-world-server.md`). Real process, real
//! sockets. S5 - every test that predates ADR-0039 passing unmodified - is
//! `tests/integration.rs` and `tests/phase5_acceleration.rs` themselves.

mod common;

use common::{
    ADMIN_TOKEN, OBSERVER_TOKEN, ServerGuard, full_viewport, http, number_field, post_json,
    read_frame, request, send, spawn_server, text_field, ws_connect,
};
use sim_protocol::{Frame, LAYER_ORGANISMS, LAYER_TERRAIN};
use std::time::Duration;

/// Small enough to build and step quickly, large enough to be a legal
/// world. Every created world in these tests uses this shape so the tests
/// measure hosting, not world generation.
const SMALL_WORLD: &str = "\"cells_x\":48,\"cells_y\":48,\"initial_organisms\":20";

fn create_world(guard: &ServerGuard, body: &str) -> (u16, String) {
    post_json(guard, "/api/worlds", ADMIN_TOKEN, body)
}

fn summary(guard: &ServerGuard, id: u64) -> String {
    let (status, body) = http(guard, "GET", &format!("/api/worlds/{id}"), OBSERVER_TOKEN);
    assert_eq!(status, 200, "world {id} summary: {body}");
    body
}

fn tick_of(guard: &ServerGuard, id: u64) -> u64 {
    number_field(&summary(guard, id), "tick")
}

fn control(guard: &ServerGuard, id: u64, action: &str) -> (u16, String) {
    // The per-world rate limit is 100 ms; these tests exercise the routes,
    // not the limiter, so they stay clear of it.
    std::thread::sleep(Duration::from_millis(150));
    post_json(
        guard,
        &format!("/api/worlds/{id}/control?action={action}"),
        ADMIN_TOKEN,
        "",
    )
}

/// S1. Three worlds created over REST tick independently: pausing one
/// leaves the others advancing, stopping one ends only its thread, and
/// deleting is refused until a world is stopped.
#[test]
fn s1_three_worlds_tick_pause_stop_and_delete_independently() {
    let guard = spawn_server(&[]);

    let (status, denied) = post_json(
        &guard,
        "/api/worlds",
        OBSERVER_TOKEN,
        &format!("{{\"name\":\"Nope\",\"preset\":\"phase1\",\"settings\":{{{SMALL_WORLD}}}}}"),
    );
    assert_eq!(status, 403, "observers must not create worlds: {denied}");

    let (status, alpha) = create_world(
        &guard,
        &format!(
            "{{\"name\":\"Alpha\",\"preset\":\"phase1\",\"seed\":\"0x11\",\"speed\":16,\
             \"settings\":{{{SMALL_WORLD}}}}}"
        ),
    );
    assert_eq!(status, 201, "create alpha: {alpha}");
    let (status, beta) = create_world(
        &guard,
        &format!(
            "{{\"name\":\"Beta\",\"preset\":\"phase2\",\"seed\":\"0x22\",\"speed\":16,\
             \"settings\":{{{SMALL_WORLD}}}}}"
        ),
    );
    assert_eq!(status, 201, "create beta: {beta}");
    let (status, gamma) = create_world(
        &guard,
        &format!(
            "{{\"name\":\"Gamma\",\"preset\":\"phase2\",\"seed\":\"0x22\",\"speed\":16,\
             \"settings\":{{{SMALL_WORLD},\"crowding_threshold\":9}}}}"
        ),
    );
    assert_eq!(status, 201, "create gamma: {gamma}");

    let alpha_id = number_field(&alpha, "world_id");
    let beta_id = number_field(&beta, "world_id");
    let gamma_id = number_field(&gamma, "world_id");
    assert_eq!((alpha_id, beta_id, gamma_id), (2, 3, 4));
    assert_eq!(text_field(&alpha, "name"), "Alpha");
    assert_eq!(text_field(&gamma, "parent_world_id"), "0");

    // Gamma differs from Beta only by a setting, so distinct hashes here
    // are the settings landing, not the seed or the preset.
    let hashes = [
        text_field(&alpha, "config_hash"),
        text_field(&beta, "config_hash"),
        text_field(&gamma, "config_hash"),
    ];
    assert_ne!(hashes[0], hashes[1]);
    assert_ne!(hashes[1], hashes[2], "a setting must change the hash");
    assert_ne!(hashes[0], hashes[2]);

    let (status, listing) = http(&guard, "GET", "/api/worlds", OBSERVER_TOKEN);
    assert_eq!(status, 200);
    assert_eq!(
        listing.matches("\"world_id\":").count(),
        4,
        "world 1 and the three created worlds: {listing}"
    );

    // All four are advancing before anything is paused.
    let before: Vec<u64> = (1..=4).map(|id| tick_of(&guard, id)).collect();
    std::thread::sleep(Duration::from_millis(700));
    let after: Vec<u64> = (1..=4).map(|id| tick_of(&guard, id)).collect();
    for id in 0..4 {
        assert!(
            after[id] > before[id],
            "world {} did not advance ({} -> {})",
            id + 1,
            before[id],
            after[id]
        );
    }

    // Pause beta only.
    let (status, paused) = control(&guard, beta_id, "pause");
    assert_eq!(status, 200, "pause beta: {paused}");
    assert!(paused.contains("\"paused\":true"));
    std::thread::sleep(Duration::from_millis(150));
    let beta_held = tick_of(&guard, beta_id);
    let others_before = [tick_of(&guard, alpha_id), tick_of(&guard, gamma_id)];
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(
        tick_of(&guard, beta_id),
        beta_held,
        "a paused world advanced"
    );
    assert_eq!(text_field(&summary(&guard, beta_id), "status"), "paused");
    assert!(
        tick_of(&guard, alpha_id) > others_before[0]
            && tick_of(&guard, gamma_id) > others_before[1],
        "pausing one world stopped another"
    );

    // Stop gamma only.
    let (status, stopped) = control(&guard, gamma_id, "stop");
    assert_eq!(status, 200, "stop gamma: {stopped}");
    std::thread::sleep(Duration::from_millis(200));
    let gamma_final = tick_of(&guard, gamma_id);
    let alpha_before = tick_of(&guard, alpha_id);
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(
        tick_of(&guard, gamma_id),
        gamma_final,
        "a stopped world kept ticking"
    );
    assert_eq!(text_field(&summary(&guard, gamma_id), "status"), "stopped");
    assert!(
        tick_of(&guard, alpha_id) > alpha_before,
        "stopping one world stopped another"
    );
    // A stopped world stays readable and saveable.
    assert_eq!(
        number_field(&summary(&guard, gamma_id), "world_id"),
        gamma_id
    );

    // Delete refuses a running world and accepts a stopped one.
    let (status, refused) = request(
        &guard,
        "DELETE",
        &format!("/api/worlds/{alpha_id}"),
        Some(ADMIN_TOKEN),
        None,
        None,
    );
    assert_eq!(status, 409, "deleting a running world: {refused}");
    let (status, deleted) = request(
        &guard,
        "DELETE",
        &format!("/api/worlds/{gamma_id}"),
        Some(ADMIN_TOKEN),
        None,
        None,
    );
    assert_eq!(status, 200, "deleting a stopped world: {deleted}");
    let (status, _) = http(
        &guard,
        "GET",
        &format!("/api/worlds/{gamma_id}"),
        OBSERVER_TOKEN,
    );
    assert_eq!(status, 404, "a deleted world is gone");
    let (_, listing) = http(&guard, "GET", "/api/worlds", OBSERVER_TOKEN);
    assert_eq!(listing.matches("\"world_id\":").count(), 3);

    // Every mutation is audited with the world it addressed.
    let (status, audit) = http(&guard, "GET", "/api/audit", ADMIN_TOKEN);
    assert_eq!(status, 200);
    assert!(
        audit.contains(&format!("\"world_id\":{gamma_id},\"role\":\"admin\"")),
        "audit does not carry the world id: {audit}"
    );
}

/// S2. Settings are creation-only: a bad one is refused by name, the
/// preview's hash is the created world's hash, only the settings sent
/// differ from the preset, and no control changes a running world's config.
#[test]
fn s2_settings_are_creation_only_and_exactly_what_was_sent() {
    let guard = spawn_server(&[]);

    let (status, body) = create_world(
        &guard,
        "{\"name\":\"Bad\",\"preset\":\"phase2\",\"settings\":{\"not_a_field\":1}}",
    );
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("not_a_field"),
        "the message must name the field: {body}"
    );
    assert!(body.contains("\"field\":\"not_a_field\""), "{body}");

    let (status, body) = create_world(
        &guard,
        "{\"name\":\"Wide\",\"preset\":\"phase2\",\"settings\":{\"cells_x\":4096}}",
    );
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("cells_x"), "{body}");

    let (status, body) = create_world(
        &guard,
        "{\"name\":\"Odd\",\"preset\":\"phase2\",\"settings\":{\"cells_x\":\"wide\"}}",
    );
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("cells_x"), "{body}");

    let (status, body) = create_world(&guard, "{\"name\":\"NoPreset\"}");
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("preset"), "{body}");

    // The preview and the creation agree, for the same inputs.
    let settings = format!("{SMALL_WORLD},\"crowding_threshold\":9");
    let preview_body =
        format!("{{\"preset\":\"phase2\",\"seed\":\"0x2a\",\"settings\":{{{settings}}}}}");
    let (status, preview) = post_json(
        &guard,
        "/api/schema/preview",
        OBSERVER_TOKEN,
        &preview_body,
    );
    assert_eq!(status, 200, "{preview}");
    assert!(preview.contains("\"valid\":true"), "{preview}");
    let previewed_hash = text_field(&preview, "config_hash");

    let (status, created) = create_world(
        &guard,
        &format!(
            "{{\"name\":\"Measured\",\"preset\":\"phase2\",\"seed\":\"0x2a\",\
             \"settings\":{{{settings}}}}}"
        ),
    );
    assert_eq!(status, 201, "{created}");
    let world_id = number_field(&created, "world_id");
    let created_hash = text_field(&created, "config_hash");
    assert_eq!(
        previewed_hash, created_hash,
        "the preview promised a different world than was created"
    );
    assert_eq!(text_field(&created, "seed"), "0x000000000000002a");

    // The bare preset hashes differently, and each setting sent is one of
    // the reasons why.
    let bare = post_json(
        &guard,
        "/api/schema/preview",
        OBSERVER_TOKEN,
        "{\"preset\":\"phase2\",\"seed\":\"0x2a\",\"settings\":{}}",
    )
    .1;
    let bare_hash = text_field(&bare, "config_hash");
    assert_ne!(bare_hash, created_hash);
    for dropped in ["\"cells_x\":48", "\"cells_y\":48", "\"crowding_threshold\":9"] {
        let reduced: Vec<&str> = settings
            .split(',')
            .filter(|setting| *setting != dropped)
            .collect();
        let body = format!(
            "{{\"preset\":\"phase2\",\"seed\":\"0x2a\",\"settings\":{{{}}}}}",
            reduced.join(",")
        );
        let hash = text_field(
            &post_json(&guard, "/api/schema/preview", OBSERVER_TOKEN, &body).1,
            "config_hash",
        );
        assert_ne!(hash, created_hash, "dropping {dropped} changed nothing");
    }

    // Nothing else moved: sending every one of those fields at the value
    // the schema reports as the preset's default reproduces the bare hash,
    // so the fields that were not sent are still at their defaults.
    let (status, schema) = http(&guard, "GET", "/api/schema", OBSERVER_TOKEN);
    assert_eq!(status, 200);
    let defaults: Vec<String> = ["cells_x", "cells_y", "initial_organisms", "crowding_threshold"]
        .iter()
        .map(|field| format!("\"{field}\":\"{}\"", schema_default(&schema, field, "phase2")))
        .collect();
    let restored = post_json(
        &guard,
        "/api/schema/preview",
        OBSERVER_TOKEN,
        &format!(
            "{{\"preset\":\"phase2\",\"seed\":\"0x2a\",\"settings\":{{{}}}}}",
            defaults.join(",")
        ),
    )
    .1;
    assert_eq!(
        text_field(&restored, "config_hash"),
        bare_hash,
        "the schema's defaults are not the preset's"
    );
    // The summary echoes the sent settings and the untouched defaults.
    assert_eq!(number_field(&created, "cells_x"), 48);
    assert_eq!(number_field(&created, "cells_y"), 48);
    assert_eq!(
        number_field(&created, "cell_size_m"),
        schema_default(&schema, "cell_size_m", "phase2")
            .parse()
            .expect("default")
    );
    assert_eq!(
        number_field(&created, "dt_ms"),
        schema_default(&schema, "dt_ms", "phase2")
            .parse()
            .expect("default")
    );

    // No control changes the config it operates on.
    for action in ["pause", "resume", "speed&multiplier=4", "stop"] {
        let (status, body) = control(&guard, world_id, action);
        assert_eq!(status, 200, "control {action}: {body}");
        assert_eq!(
            text_field(&summary(&guard, world_id), "config_hash"),
            created_hash,
            "control {action} moved the config hash"
        );
    }
    // A preview creates nothing.
    let (_, listing) = http(&guard, "GET", "/api/worlds", OBSERVER_TOKEN);
    assert_eq!(listing.matches("\"world_id\":").count(), 2);
}

/// The default `field` reports for `preset` in `GET /api/schema`.
fn schema_default(schema: &str, field: &str, preset: &str) -> String {
    let start = schema
        .find(&format!("\"name\":\"{field}\","))
        .unwrap_or_else(|| panic!("no schema entry for {field}"));
    let rest = &schema[start..];
    let end = rest.find("}}").unwrap_or_else(|| panic!("unterminated {field}"));
    text_field(&rest[..end], preset)
}

/// S3. The WebSocket path selects the world: `/worlds/{id}` streams that
/// world, `/` is world 1, an unknown id is refused, and a world stopped
/// under an open session ends it.
#[test]
fn s3_the_stream_selects_a_world_by_path() {
    let guard = spawn_server(&[]);
    let (status, created) = create_world(
        &guard,
        &format!(
            "{{\"name\":\"Streamed\",\"preset\":\"phase2\",\"seed\":\"0x33\",\"speed\":16,\
             \"settings\":{{{SMALL_WORLD}}}}}"
        ),
    );
    assert_eq!(status, 201, "{created}");
    let world_id = number_field(&created, "world_id");

    // The created world: its own id, its own dimensions, its own entities.
    let mut socket = ws_connect(&guard, &format!("/worlds/{world_id}"), OBSERVER_TOKEN);
    let (_, welcome) = read_frame(&mut socket, Duration::from_secs(5)).expect("welcome");
    let Frame::Welcome {
        world_id: welcomed,
        cells_x,
        ..
    } = welcome
    else {
        panic!("expected welcome, got {welcome:?}");
    };
    assert_eq!(welcomed, world_id, "the path did not select the world");
    assert_eq!(cells_x, 48);
    let small_entities = first_keyframe_entities(&mut socket);
    assert!(
        small_entities < 60,
        "world {world_id} holds 20 organisms, not {small_entities}"
    );

    // `/` is world 1, which is what every client written before ADR-0039
    // asks for.
    let mut root = ws_connect(&guard, "/", OBSERVER_TOKEN);
    let (_, welcome) = read_frame(&mut root, Duration::from_secs(5)).expect("welcome");
    let Frame::Welcome {
        world_id: welcomed,
        cells_x,
        ..
    } = welcome
    else {
        panic!("expected welcome, got {welcome:?}");
    };
    assert_eq!(welcomed, 1);
    assert_eq!(cells_x, 256);
    assert!(
        first_keyframe_entities(&mut root) > 60,
        "world 1 holds 150 organisms"
    );
    // The explicit spelling of world 1 selects the same world.
    let mut explicit = ws_connect(&guard, "/worlds/1", OBSERVER_TOKEN);
    assert!(matches!(
        read_frame(&mut explicit, Duration::from_secs(5)),
        Some((_, Frame::Welcome { world_id: 1, .. }))
    ));

    // An unknown world is refused after the token is checked.
    let mut missing = ws_connect(&guard, "/worlds/99", OBSERVER_TOKEN);
    match read_frame(&mut missing, Duration::from_secs(5)) {
        Some((_, Frame::Error { code, .. })) => assert_eq!(code, 404),
        other => panic!("expected a 404 error frame, got {other:?}"),
    }

    // A world stopped under an open session ends that session.
    let (status, stopped) = control(&guard, world_id, "stop");
    assert_eq!(status, 200, "{stopped}");
    let mut gone = false;
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        match read_frame(&mut socket, Duration::from_secs(2)) {
            Some((_, Frame::Error { code, .. })) => {
                assert_eq!(code, 410);
                gone = true;
                break;
            }
            Some(_) => {}
            None => break,
        }
    }
    assert!(gone, "a stopped world must close its sessions with 410");
}

fn first_keyframe_entities(socket: &mut tungstenite::WebSocket<std::net::TcpStream>) -> usize {
    send(
        socket,
        &Frame::Subscribe {
            viewport: full_viewport(),
            layers: LAYER_TERRAIN | LAYER_ORGANISMS,
            max_rate_hz: 10,
        },
    );
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        match read_frame(socket, Duration::from_secs(5)) {
            Some((_, Frame::Keyframe { entities, .. })) => return entities.len(),
            Some(_) => {}
            None => break,
        }
    }
    panic!("no keyframe arrived");
}

/// S4. Saves belong to the world that made them, and a branch is a new
/// world carrying that save's state, parent and epoch.
#[test]
fn s4_saves_and_branches_belong_to_their_world() {
    let data_dir =
        std::env::temp_dir().join(format!("lifesim-multiworld-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let guard = spawn_server(&["--data-dir", &data_dir_text]);

    let (status, created) = create_world(
        &guard,
        &format!(
            "{{\"name\":\"Saved\",\"preset\":\"phase2\",\"seed\":\"0x44\",\"speed\":16,\
             \"settings\":{{{SMALL_WORLD}}}}}"
        ),
    );
    assert_eq!(status, 201, "{created}");
    let world_id = number_field(&created, "world_id");
    std::thread::sleep(Duration::from_millis(400));

    let (status, save) = post_json(
        &guard,
        &format!("/api/worlds/{world_id}/saves?name=snap"),
        ADMIN_TOKEN,
        "",
    );
    assert_eq!(status, 200, "save: {save}");
    let save_id = number_field(&save, "save_id");
    let save_tick = number_field(&save, "tick");
    let save_checksum = text_field(&save, "state_checksum");
    assert_eq!(number_field(&save, "world_id"), world_id);
    assert!(save_tick > 0, "the world should have ticked before saving");

    // The row lists under its own world and nowhere else.
    let (status, listing) = http(
        &guard,
        "GET",
        &format!("/api/worlds/{world_id}/saves"),
        OBSERVER_TOKEN,
    );
    assert_eq!(status, 200);
    assert!(listing.contains(&format!("\"save_id\":{save_id}")), "{listing}");
    let (status, primary_listing) = http(&guard, "GET", "/api/worlds/1/saves", OBSERVER_TOKEN);
    assert_eq!(status, 200);
    assert_eq!(
        primary_listing, "[]",
        "world 1 listed another world's save: {primary_listing}"
    );

    // Isolated verification still rebuilds the save in a throwaway world.
    let (status, verified) = post_json(
        &guard,
        &format!("/api/worlds/{world_id}/saves/{save_id}/verify"),
        ADMIN_TOKEN,
        "",
    );
    assert_eq!(status, 200, "verify: {verified}");
    assert!(verified.contains("\"result\":\"ok\""), "{verified}");
    // ...and only through the path of the world that owns it.
    let (status, wrong_world) = post_json(
        &guard,
        &format!("/api/worlds/1/saves/{save_id}/verify"),
        ADMIN_TOKEN,
        "",
    );
    assert_eq!(status, 404, "verify through the wrong world: {wrong_world}");

    // Branching that save makes a new world at the saved tick.
    let (status, branch) = post_json(
        &guard,
        &format!("/api/worlds/{world_id}/branch?save_id={save_id}&name=Branched"),
        ADMIN_TOKEN,
        "",
    );
    assert_eq!(status, 201, "branch: {branch}");
    let branch_id = number_field(&branch, "world_id");
    assert_ne!(branch_id, world_id);
    assert_eq!(number_field(&branch, "parent_world_id"), world_id);
    assert_eq!(number_field(&branch, "world_epoch"), 2);
    assert_eq!(text_field(&branch, "name"), "Branched");
    assert_eq!(text_field(&branch, "status"), "paused");
    assert_eq!(number_field(&branch, "tick"), save_tick);
    assert_eq!(text_field(&branch, "config_hash"), text_field(&created, "config_hash"));

    // The branch carries the save's state, not a fresh world's: saving it
    // reproduces the checksum at the same tick.
    let (status, branch_save) = post_json(
        &guard,
        &format!("/api/worlds/{branch_id}/saves?name=rebranch"),
        ADMIN_TOKEN,
        "",
    );
    assert_eq!(status, 200, "branch save: {branch_save}");
    assert_eq!(number_field(&branch_save, "tick"), save_tick);
    assert_eq!(
        text_field(&branch_save, "state_checksum"),
        save_checksum,
        "the branch is not the state that was branched"
    );
    assert_eq!(number_field(&branch_save, "world_id"), branch_id);

    // The parent's listing did not acquire the branch's save.
    let (_, listing) = http(
        &guard,
        "GET",
        &format!("/api/worlds/{world_id}/saves"),
        OBSERVER_TOKEN,
    );
    assert_eq!(
        listing.matches("\"save_id\":").count(),
        1,
        "the parent's saves changed: {listing}"
    );

    let (status, missing) = post_json(
        &guard,
        &format!("/api/worlds/{world_id}/branch?save_id=9999"),
        ADMIN_TOKEN,
        "",
    );
    assert_eq!(status, 404, "branching an unknown save: {missing}");

    drop(guard);
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// The bound is `--max-worlds`, counting world 1: a process that hosts
/// sandbox worlds beside the flag-built one has to have a ceiling, or the
/// tick threads are unbounded.
#[test]
fn the_world_count_bound_refuses_a_creation_rather_than_growing() {
    let guard = spawn_server(&["--max-worlds", "2"]);
    // Seeded like every other world these tests create: an unseeded body
    // takes the server's clock-derived seed, and some of those seeds put a
    // 48x48 world's land fraction outside the legal band, which fails
    // creation with a 400 and would make this test measure world
    // generation rather than the bound.
    let body = format!(
        "{{\"name\":\"Only\",\"preset\":\"phase1\",\"seed\":\"0x11\",\
         \"settings\":{{{SMALL_WORLD}}}}}"
    );
    let (status, first) = create_world(&guard, &body);
    assert_eq!(status, 201, "{first}");
    let (status, refused) = create_world(&guard, &body);
    assert_eq!(status, 409, "{refused}");
    assert!(refused.contains("max-worlds"), "{refused}");
    let (_, listing) = http(&guard, "GET", "/api/worlds", OBSERVER_TOKEN);
    assert_eq!(listing.matches("\"world_id\":").count(), 2);

    // The schema reports the same bound the route enforces.
    let (_, schema) = http(&guard, "GET", "/api/schema", OBSERVER_TOKEN);
    assert!(schema.contains("\"max_worlds\":2"), "schema limits");

    // Stopping and deleting one makes room again.
    let id = number_field(&first, "world_id");
    let (status, _) = control(&guard, id, "stop");
    assert_eq!(status, 200);
    let (status, _) = request(
        &guard,
        "DELETE",
        &format!("/api/worlds/{id}"),
        Some(ADMIN_TOKEN),
        None,
        None,
    );
    assert_eq!(status, 200);
    let (status, again) = create_world(&guard, &body);
    assert_eq!(status, 201, "{again}");
}
