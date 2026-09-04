//! A bounded reader for exactly the two request bodies this server
//! accepts.
//!
//! The workspace has no serde and this increment does not add one, so the
//! shapes the console posts (`{name, preset, seed?, settings:{...},
//! paused?, speed?}`) are parsed here. It is deliberately not a general
//! JSON parser: arrays are refused, nesting stops at one object inside the
//! top-level object, and every length is bounded before anything is
//! allocated, so a hostile body costs a constant amount of memory and a
//! single pass. Anything outside that shape is an error the route turns
//! into a 400 rather than a partial parse.

/// Largest request body read from a socket, before parsing.
pub const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_KEYS: usize = 512;
const MAX_KEY_BYTES: usize = 128;
const MAX_STRING_BYTES: usize = 1_024;
const MAX_NUMBER_BYTES: usize = 40;
/// The top-level object is depth 1; one nested object is depth 2.
const MAX_DEPTH: usize = 2;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Str(String),
    /// The number's literal text. Kept as text because a `u64` seed does
    /// not survive a round trip through `f64`.
    Num(String),
    Bool(bool),
    Null,
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// The value as the text `set_field` and the seed parser consume.
    /// Objects and nulls have no scalar text.
    pub fn scalar_text(&self) -> Option<String> {
        match self {
            Json::Str(value) => Some(value.clone()),
            Json::Num(value) => Some(value.clone()),
            Json::Bool(value) => Some(value.to_string()),
            Json::Null | Json::Obj(_) => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(members) => Some(members),
            _ => None,
        }
    }
}

/// Escape a value for use inside a JSON string literal. Responses are
/// built as text, so this is the one place that keeps a name or an error
/// message from ending the string it is written into.
pub fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Look a member up in a parsed object.
pub fn field<'a>(object: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    object
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

/// Parse a request body as a JSON object. The error text is what the route
/// returns to the client, so it names what was wrong rather than an offset.
pub fn parse_object(text: &str) -> Result<Vec<(String, Json)>, String> {
    if text.len() > MAX_BODY_BYTES {
        return Err(format!("body larger than {MAX_BODY_BYTES} bytes"));
    }
    let bytes = text.as_bytes();
    let mut cursor = 0;
    skip_space(bytes, &mut cursor);
    let value = parse_value(bytes, &mut cursor, 1)?;
    skip_space(bytes, &mut cursor);
    if cursor != bytes.len() {
        return Err("trailing content after the JSON value".to_owned());
    }
    match value {
        Json::Obj(members) => Ok(members),
        _ => Err("body must be a JSON object".to_owned()),
    }
}

fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while *cursor < bytes.len() && matches!(bytes[*cursor], b' ' | b'\t' | b'\n' | b'\r') {
        *cursor += 1;
    }
}

fn parse_value(bytes: &[u8], cursor: &mut usize, depth: usize) -> Result<Json, String> {
    match bytes.get(*cursor) {
        None => Err("unexpected end of body".to_owned()),
        Some(b'{') => parse_obj(bytes, cursor, depth),
        Some(b'"') => Ok(Json::Str(parse_string(bytes, cursor, MAX_STRING_BYTES)?)),
        Some(b'[') => Err("arrays are not accepted in a request body".to_owned()),
        Some(b't') => literal(bytes, cursor, "true", Json::Bool(true)),
        Some(b'f') => literal(bytes, cursor, "false", Json::Bool(false)),
        Some(b'n') => literal(bytes, cursor, "null", Json::Null),
        Some(byte) if *byte == b'-' || byte.is_ascii_digit() => parse_number(bytes, cursor),
        Some(byte) => Err(format!("unexpected character '{}'", *byte as char)),
    }
}

fn literal(bytes: &[u8], cursor: &mut usize, text: &str, value: Json) -> Result<Json, String> {
    if bytes[*cursor..].starts_with(text.as_bytes()) {
        *cursor += text.len();
        Ok(value)
    } else {
        Err(format!("expected {text}"))
    }
}

fn parse_obj(bytes: &[u8], cursor: &mut usize, depth: usize) -> Result<Json, String> {
    if depth > MAX_DEPTH {
        return Err(format!("objects nested deeper than {MAX_DEPTH}"));
    }
    *cursor += 1; // '{'
    let mut members: Vec<(String, Json)> = Vec::new();
    skip_space(bytes, cursor);
    if bytes.get(*cursor) == Some(&b'}') {
        *cursor += 1;
        return Ok(Json::Obj(members));
    }
    loop {
        skip_space(bytes, cursor);
        if bytes.get(*cursor) != Some(&b'"') {
            return Err("expected a quoted member name".to_owned());
        }
        let key = parse_string(bytes, cursor, MAX_KEY_BYTES)?;
        skip_space(bytes, cursor);
        if bytes.get(*cursor) != Some(&b':') {
            return Err(format!("expected ':' after member '{key}'"));
        }
        *cursor += 1;
        skip_space(bytes, cursor);
        let value = parse_value(bytes, cursor, depth + 1)?;
        if members.iter().any(|(existing, _)| existing == &key) {
            return Err(format!("member '{key}' appears twice"));
        }
        if members.len() >= MAX_KEYS {
            return Err(format!("more than {MAX_KEYS} members in one object"));
        }
        members.push((key, value));
        skip_space(bytes, cursor);
        match bytes.get(*cursor) {
            Some(b',') => *cursor += 1,
            Some(b'}') => {
                *cursor += 1;
                return Ok(Json::Obj(members));
            }
            _ => return Err("expected ',' or '}'".to_owned()),
        }
    }
}

fn parse_string(bytes: &[u8], cursor: &mut usize, limit: usize) -> Result<String, String> {
    *cursor += 1; // opening quote
    let mut out = String::new();
    loop {
        let Some(byte) = bytes.get(*cursor).copied() else {
            return Err("unterminated string".to_owned());
        };
        *cursor += 1;
        match byte {
            b'"' => return Ok(out),
            b'\\' => {
                let Some(escape) = bytes.get(*cursor).copied() else {
                    return Err("unterminated escape".to_owned());
                };
                *cursor += 1;
                let decoded = match escape {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    // No field name or preset name in this server's
                    // vocabulary needs one, and a half-implemented \u
                    // (surrogate pairs, lone halves) is a decoder bug
                    // waiting to happen. Refused rather than approximated.
                    b'u' => return Err("\\u escapes are not accepted".to_owned()),
                    other => return Err(format!("unknown escape '\\{}'", other as char)),
                };
                out.push(decoded);
            }
            // Control characters must be escaped in JSON; refusing them
            // keeps a raw newline out of a name that is echoed back.
            0x00..=0x1f => return Err("unescaped control character in a string".to_owned()),
            byte if byte < 0x80 => out.push(byte as char),
            // A multi-byte code point is copied from the source text: one
            // of its bytes is not a `char`, and casting it to one would
            // turn "café" into mojibake in the name echoed back.
            _ => {
                let start = *cursor - 1;
                let end = next_char_end(bytes, start);
                let slice = std::str::from_utf8(&bytes[start..end])
                    .map_err(|_| "invalid UTF-8 in a string".to_owned())?;
                out.push_str(slice);
                *cursor = end;
            }
        }
        if out.len() > limit {
            return Err(format!("string longer than {limit} bytes"));
        }
    }
}

/// End index of the UTF-8 sequence starting at `start`.
fn next_char_end(bytes: &[u8], start: usize) -> usize {
    let width = match bytes[start] {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    };
    (start + width).min(bytes.len())
}

fn parse_number(bytes: &[u8], cursor: &mut usize) -> Result<Json, String> {
    let start = *cursor;
    if bytes.get(*cursor) == Some(&b'-') {
        *cursor += 1;
    }
    let digits_start = *cursor;
    while matches!(bytes.get(*cursor), Some(byte) if byte.is_ascii_digit()) {
        *cursor += 1;
    }
    if *cursor == digits_start {
        return Err("number without digits".to_owned());
    }
    if bytes.get(*cursor) == Some(&b'.') {
        *cursor += 1;
        let fraction_start = *cursor;
        while matches!(bytes.get(*cursor), Some(byte) if byte.is_ascii_digit()) {
            *cursor += 1;
        }
        if *cursor == fraction_start {
            return Err("number with an empty fraction".to_owned());
        }
    }
    if matches!(bytes.get(*cursor), Some(b'e' | b'E')) {
        *cursor += 1;
        if matches!(bytes.get(*cursor), Some(b'+' | b'-')) {
            *cursor += 1;
        }
        let exponent_start = *cursor;
        while matches!(bytes.get(*cursor), Some(byte) if byte.is_ascii_digit()) {
            *cursor += 1;
        }
        if *cursor == exponent_start {
            return Err("number with an empty exponent".to_owned());
        }
    }
    if *cursor - start > MAX_NUMBER_BYTES {
        return Err(format!("number longer than {MAX_NUMBER_BYTES} digits"));
    }
    let text = std::str::from_utf8(&bytes[start..*cursor])
        .map_err(|_| "invalid number".to_owned())?
        .to_owned();
    Ok(Json::Num(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_create_world_shape_parses_with_its_nested_settings() {
        let body = r#"{"name":"Test world","preset":"phase2","seed":"0x2a",
            "settings":{"cells_x":64,"phase2.enabled":true,"origin.mode":"seeded"},
            "paused":true,"speed":2.5}"#;
        let object = parse_object(body).expect("parses");
        assert_eq!(
            field(&object, "name").and_then(Json::scalar_text).as_deref(),
            Some("Test world")
        );
        assert_eq!(
            field(&object, "seed").and_then(Json::scalar_text).as_deref(),
            Some("0x2a")
        );
        assert_eq!(
            field(&object, "speed")
                .and_then(Json::scalar_text)
                .as_deref(),
            Some("2.5")
        );
        assert_eq!(
            field(&object, "paused")
                .and_then(Json::scalar_text)
                .as_deref(),
            Some("true")
        );
        let settings = field(&object, "settings")
            .and_then(Json::as_object)
            .expect("settings object");
        // Values reach `set_field` as text, whatever their JSON type.
        assert_eq!(
            settings
                .iter()
                .map(|(key, value)| (key.as_str(), value.scalar_text().unwrap_or_default()))
                .collect::<Vec<_>>(),
            vec![
                ("cells_x", "64".to_owned()),
                ("phase2.enabled", "true".to_owned()),
                ("origin.mode", "seeded".to_owned()),
            ]
        );
    }

    #[test]
    fn a_u64_seed_survives_because_numbers_keep_their_text() {
        let object = parse_object("{\"seed\":18446744073709551615}").expect("parses");
        assert_eq!(
            field(&object, "seed").and_then(Json::scalar_text).as_deref(),
            Some("18446744073709551615")
        );
    }

    #[test]
    fn escapes_and_unicode_survive_a_round_trip_but_backslash_u_does_not() {
        let object = parse_object("{\"name\":\"a\\\"b\\\\c\\nd\"}").expect("parses");
        assert_eq!(
            field(&object, "name").and_then(Json::scalar_text).as_deref(),
            Some("a\"b\\c\nd")
        );
        let object = parse_object("{\"name\":\"caf\u{e9} \u{1f600}\"}").expect("parses");
        assert_eq!(
            field(&object, "name").and_then(Json::scalar_text).as_deref(),
            Some("caf\u{e9} \u{1f600}")
        );
        assert!(parse_object("{\"name\":\"\\u0041\"}").is_err());
    }

    #[test]
    fn everything_outside_the_accepted_shape_is_refused() {
        for body in [
            "[1,2,3]",
            "{\"settings\":[1]}",
            "\"just a string\"",
            "{\"a\":{\"b\":{\"c\":1}}}",
            "{\"a\":1,\"a\":2}",
            "{\"a\":}",
            "{\"a\" 1}",
            "{\"a\":1",
            "{\"a\":01x}",
            "{\"a\":1} trailing",
            "{a:1}",
            "",
        ] {
            assert!(parse_object(body).is_err(), "accepted {body}");
        }
    }

    #[test]
    fn every_length_is_bounded_before_the_value_is_used() {
        let long_key = format!("{{\"{}\":1}}", "k".repeat(MAX_KEY_BYTES + 1));
        assert!(parse_object(&long_key).is_err());
        let long_string = format!("{{\"a\":\"{}\"}}", "v".repeat(MAX_STRING_BYTES + 1));
        assert!(parse_object(&long_string).is_err());
        let many_members = format!(
            "{{{}}}",
            (0..=MAX_KEYS)
                .map(|index| format!("\"k{index}\":1"))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(parse_object(&many_members).is_err());
        let oversized = format!("{{\"a\":\"{}\"}}", "v".repeat(MAX_BODY_BYTES));
        assert!(parse_object(&oversized).is_err());
    }
}
