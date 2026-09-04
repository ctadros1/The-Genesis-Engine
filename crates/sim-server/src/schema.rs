//! The creation vocabulary: presets, the campaign field registry, and the
//! request bodies that turn them into a config.
//!
//! ADR-0039 decision 2: a world is a preset plus named settings from
//! `sim_experiment::fields`, validated by `SimConfig::validate`, with the
//! seed set at creation and never a setting. The console's builder is
//! generated from `/api/schema`, so the settings it can name are exactly
//! the settings a campaign can name; that is the point of reusing the
//! registry rather than inventing a second one here.

use crate::json::{Json, escape, field, parse_object};
use sim_core::SimConfig;
use sim_experiment::fields::{FIELD_NAMES, field_choices, field_kind, read_field, set_field};

/// The protocol's tile block addresses at most 512x512 cells, so a world
/// larger than that could not be streamed to an observer.
pub const MAX_CELLS_PER_AXIS: u32 = 512;

pub const PRESETS: [(&str, &str); 2] = [
    (
        "phase1",
        "Phase 1 baseline: metabolism, movement and asexual reproduction",
    ),
    (
        "phase2",
        "Phase 2: inherited controllers and paired-parent reproduction",
    ),
];

/// A rejected request, with the field to blame when there is one so the
/// console can put the message beside the input that caused it.
#[derive(Debug)]
pub struct BadRequest {
    pub message: String,
    pub field: Option<String>,
}

impl BadRequest {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: None,
        }
    }

    fn about(field: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: Some(field.to_owned()),
        }
    }

    pub fn to_json(&self) -> String {
        match &self.field {
            Some(field) => format!(
                "{{\"error\":\"{}\",\"field\":\"{}\"}}",
                escape(&self.message),
                escape(field)
            ),
            None => format!("{{\"error\":\"{}\"}}", escape(&self.message)),
        }
    }
}

fn preset_config(preset: &str, seed: u64) -> Option<SimConfig> {
    match preset {
        "phase1" => Some(SimConfig::phase1_default(seed)),
        "phase2" => Some(SimConfig::phase2_default(seed)),
        _ => None,
    }
}

/// A validated creation request. `settings` keeps the order the client
/// sent, because `set_field` order is observable: two settings can touch
/// the same coordinated pair (`climate.enabled` moves the generator
/// version with it).
pub struct CreateRequest {
    pub name: String,
    pub preset: String,
    pub seed: u64,
    pub settings: Vec<(String, String)>,
    pub paused: bool,
    pub speed_q16: u32,
}

/// Sanitize a world name to a bounded, printable label. Names are echoed
/// in summaries and audit records; nothing else about them matters.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '_' || *c == '-')
        .take(64)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "unnamed".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Decimal, `0x`-prefixed hex, or a JSON number: all three reach here as
/// text, because a `u64` seed does not survive `f64`.
pub fn parse_seed(text: &str) -> Option<u64> {
    let text = text.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

fn read_settings(object: &[(String, Json)]) -> Result<Vec<(String, String)>, BadRequest> {
    let Some(value) = field(object, "settings") else {
        return Ok(Vec::new());
    };
    if matches!(value, Json::Null) {
        return Ok(Vec::new());
    }
    let Some(members) = value.as_object() else {
        return Err(BadRequest::about(
            "settings",
            "settings must be an object of field names to values",
        ));
    };
    let mut settings = Vec::with_capacity(members.len());
    for (name, value) in members {
        let Some(text) = value.scalar_text() else {
            return Err(BadRequest::about(
                name,
                format!("setting '{name}' must be a string, number or boolean"),
            ));
        };
        settings.push((name.clone(), text));
    }
    Ok(settings)
}

fn read_preset(object: &[(String, Json)]) -> Result<String, BadRequest> {
    let preset = field(object, "preset")
        .and_then(Json::scalar_text)
        .ok_or_else(|| BadRequest::about("preset", "preset is required"))?;
    if preset_config(&preset, 0).is_none() {
        let names: Vec<&str> = PRESETS.iter().map(|(name, _)| *name).collect();
        return Err(BadRequest::about(
            "preset",
            format!("unknown preset '{preset}'; known presets are {}", names.join(", ")),
        ));
    }
    Ok(preset)
}

fn read_seed(object: &[(String, Json)], fallback: u64) -> Result<u64, BadRequest> {
    match field(object, "seed") {
        None | Some(Json::Null) => Ok(fallback),
        Some(value) => {
            let text = value
                .scalar_text()
                .ok_or_else(|| BadRequest::about("seed", "seed must be a number or a string"))?;
            parse_seed(&text)
                .ok_or_else(|| BadRequest::about("seed", format!("invalid seed '{text}'")))
        }
    }
}

/// `POST /api/worlds`. `fallback_seed` is used when the body names none.
pub fn parse_create(body: &str, fallback_seed: u64) -> Result<CreateRequest, BadRequest> {
    let object = parse_object(body).map_err(BadRequest::new)?;
    let preset = read_preset(&object)?;
    let seed = read_seed(&object, fallback_seed)?;
    let settings = read_settings(&object)?;
    let name = sanitize_name(
        &field(&object, "name")
            .and_then(Json::scalar_text)
            .unwrap_or_default(),
    );
    let paused = match field(&object, "paused") {
        None | Some(Json::Null) => false,
        Some(Json::Bool(value)) => *value,
        Some(_) => return Err(BadRequest::about("paused", "paused must be true or false")),
    };
    let speed_q16 = match field(&object, "speed") {
        None | Some(Json::Null) => 1 << 16,
        Some(value) => {
            let text = value
                .scalar_text()
                .ok_or_else(|| BadRequest::about("speed", "speed must be a number"))?;
            let speed: f64 = text
                .parse()
                .map_err(|_| BadRequest::about("speed", format!("invalid speed '{text}'")))?;
            if !(0.0..=64.0).contains(&speed) {
                return Err(BadRequest::about("speed", "speed must be in [0, 64]"));
            }
            (speed * 65_536.0) as u32
        }
    };
    Ok(CreateRequest {
        name,
        preset,
        seed,
        settings,
        paused,
        speed_q16,
    })
}

/// Apply settings to a preset and validate. Settings are applied *before*
/// the world is built, because a world is built from a config and hashes it
/// at construction; a setting applied afterwards would leave the reported
/// config hash describing a world that does not exist.
pub fn build_config(
    preset: &str,
    seed: u64,
    settings: &[(String, String)],
) -> Result<SimConfig, BadRequest> {
    let mut config = preset_config(preset, seed)
        .ok_or_else(|| BadRequest::about("preset", format!("unknown preset '{preset}'")))?;
    for (name, value) in settings {
        set_field(&mut config, name, value)
            .map_err(|error| BadRequest::about(name, error.to_string()))?;
    }
    if config.cells_x > MAX_CELLS_PER_AXIS {
        return Err(BadRequest::about(
            "cells_x",
            format!("cells_x must be at most {MAX_CELLS_PER_AXIS}"),
        ));
    }
    if config.cells_y > MAX_CELLS_PER_AXIS {
        return Err(BadRequest::about(
            "cells_y",
            format!("cells_y must be at most {MAX_CELLS_PER_AXIS}"),
        ));
    }
    config
        .validate()
        .map_err(|error| BadRequest::new(error.to_string()))?;
    Ok(config)
}

/// `GET /api/schema`: the presets, every settable field with its type and
/// per-preset default, and the server's limits.
pub fn schema_json(max_worlds: usize) -> String {
    let mut body = String::from("{\"presets\":[");
    for (index, (name, description)) in PRESETS.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str(&format!(
            "{{\"name\":\"{name}\",\"description\":\"{}\"}}",
            escape(description)
        ));
    }
    body.push_str("],\"fields\":[");
    let defaults: Vec<(&str, SimConfig)> = PRESETS
        .iter()
        .map(|(name, _)| (*name, preset_config(name, 0).expect("known preset")))
        .collect();
    for (index, name) in FIELD_NAMES.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        let kind = field_kind(name).expect("registered field has a kind");
        body.push_str(&format!(
            "{{\"name\":\"{name}\",\"type\":\"{}\"",
            kind.name()
        ));
        if let Some(choices) = field_choices(name) {
            body.push_str(",\"choices\":[");
            for (index, choice) in choices.iter().enumerate() {
                if index > 0 {
                    body.push(',');
                }
                body.push_str(&format!("\"{choice}\""));
            }
            body.push(']');
        }
        body.push_str(",\"defaults\":{");
        for (index, (preset, config)) in defaults.iter().enumerate() {
            if index > 0 {
                body.push(',');
            }
            let value = read_field(config, name).expect("registered field reads");
            body.push_str(&format!("\"{preset}\":\"{}\"", escape(&value.to_string())));
        }
        body.push_str("}}");
    }
    body.push_str(&format!(
        "],\"limits\":{{\"max_worlds\":{max_worlds},\"max_cells_x\":{MAX_CELLS_PER_AXIS},\
         \"max_cells_y\":{MAX_CELLS_PER_AXIS}}}}}"
    ));
    body
}

/// `POST /api/schema/preview`: the config hash a creation with these
/// inputs would produce, and every reason it would be refused. Creates
/// nothing, so a body that names an impossible world answers 200 with
/// `valid` false rather than an error status.
pub fn preview_json(body: &str, fallback_seed: u64) -> Result<String, BadRequest> {
    let object = parse_object(body).map_err(BadRequest::new)?;
    let preset = read_preset(&object)?;
    let seed = read_seed(&object, fallback_seed)?;
    let settings = read_settings(&object)?;

    let mut config = preset_config(&preset, seed).expect("preset checked");
    let mut errors: Vec<String> = Vec::new();
    for (name, value) in &settings {
        if let Err(error) = set_field(&mut config, name, value) {
            errors.push(error.to_string());
        }
    }
    if config.cells_x > MAX_CELLS_PER_AXIS {
        errors.push(format!("cells_x must be at most {MAX_CELLS_PER_AXIS}"));
    }
    if config.cells_y > MAX_CELLS_PER_AXIS {
        errors.push(format!("cells_y must be at most {MAX_CELLS_PER_AXIS}"));
    }
    if let Err(error) = config.validate() {
        errors.push(error.to_string());
    }
    let mut rendered = String::from("[");
    for (index, error) in errors.iter().enumerate() {
        if index > 0 {
            rendered.push(',');
        }
        rendered.push_str(&format!("\"{}\"", escape(error)));
    }
    rendered.push(']');
    Ok(format!(
        concat!(
            "{{\"preset\":\"{}\",\"seed\":\"0x{:016x}\",\"config_hash\":\"0x{:016x}\",",
            "\"valid\":{},\"errors\":{}}}"
        ),
        escape(&preset),
        seed,
        config.stable_hash(),
        errors.is_empty(),
        rendered
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_created_config_hashes_the_same_as_its_preview() {
        let body = "{\"preset\":\"phase2\",\"seed\":\"0x2a\",\"settings\":{\"cells_x\":64,\"cells_y\":64}}";
        let request = parse_create(body, 0).expect("parses");
        let config = build_config(&request.preset, request.seed, &request.settings).expect("builds");
        let preview = preview_json(body, 0).expect("previews");
        assert!(
            preview.contains(&format!("\"config_hash\":\"0x{:016x}\"", config.stable_hash())),
            "preview {preview} disagrees with the built config"
        );
        assert!(preview.contains("\"valid\":true"));
    }

    #[test]
    fn a_bad_setting_names_the_field_it_blames() {
        let unknown = build_config("phase2", 1, &[("not_a_field".to_owned(), "1".to_owned())])
            .expect_err("refused");
        assert_eq!(unknown.field.as_deref(), Some("not_a_field"));
        let bad_value = build_config("phase2", 1, &[("cells_x".to_owned(), "wide".to_owned())])
            .expect_err("refused");
        assert_eq!(bad_value.field.as_deref(), Some("cells_x"));
        let too_wide = build_config("phase2", 1, &[("cells_x".to_owned(), "4096".to_owned())])
            .expect_err("refused");
        assert_eq!(too_wide.field.as_deref(), Some("cells_x"));
    }

    #[test]
    fn seeds_are_read_as_decimal_hex_or_a_json_number() {
        assert_eq!(parse_seed("42"), Some(42));
        assert_eq!(parse_seed("0x2a"), Some(42));
        assert_eq!(parse_seed("18446744073709551615"), Some(u64::MAX));
        assert_eq!(parse_seed("nope"), None);
        let request = parse_create("{\"preset\":\"phase1\",\"seed\":42}", 7).expect("parses");
        assert_eq!(request.seed, 42);
        let request = parse_create("{\"preset\":\"phase1\"}", 7).expect("parses");
        assert_eq!(request.seed, 7);
    }

    #[test]
    fn names_are_bounded_and_stripped_to_a_label() {
        assert_eq!(sanitize_name("  Marsh trial-2  "), "Marsh trial-2");
        assert_eq!(sanitize_name("../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_name(""), "unnamed");
        assert_eq!(sanitize_name(&"x".repeat(200)).len(), 64);
    }

    #[test]
    fn the_schema_offers_every_registered_field_with_its_type() {
        let schema = schema_json(8);
        for name in FIELD_NAMES {
            assert!(schema.contains(&format!("\"name\":\"{name}\"")), "{name}");
        }
        assert!(schema.contains("\"choices\":[\"random\",\"seeded\",\"scratch\"]"));
        assert!(schema.contains("\"max_cells_x\":512"));
    }
}
