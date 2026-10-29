use serde_json::Value;

pub(crate) fn payload_too_large(value: &Value) -> bool {
    serialized_size(value, kawi_jsonrpc::framing::MAX_MESSAGE_BYTES).is_none()
}

fn serialized_size(value: &Value, limit: usize) -> Option<usize> {
    match value {
        Value::Null => Some(4),
        Value::Bool(true) => Some(4),
        Value::Bool(false) => Some(5),
        Value::Number(number) => Some(number.to_string().len()),
        Value::String(text) => string_size(text, limit),
        Value::Array(items) => sequence_size(items.iter(), limit),
        Value::Object(entries) => object_size(entries.iter(), limit),
    }
}

fn string_size(text: &str, limit: usize) -> Option<usize> {
    if text.len() > limit {
        return None;
    }
    let mut size = 2;
    for character in text.chars() {
        size += match character {
            '"' | '\\' | '\u{08}' | '\u{0c}' | '\n' | '\r' | '\t' => 2,
            character if character < '\u{20}' => 6,
            character => character.len_utf8(),
        };
        if size > limit {
            return None;
        }
    }
    Some(size)
}

fn sequence_size<'a>(items: impl Iterator<Item = &'a Value>, limit: usize) -> Option<usize> {
    let mut size = 2;
    for (index, item) in items.enumerate() {
        if index > 0 {
            size += 1;
        }
        size = add_size(size, serialized_size(item, limit)?, limit)?;
    }
    Some(size)
}

fn object_size<'a>(
    entries: impl Iterator<Item = (&'a String, &'a Value)>,
    limit: usize,
) -> Option<usize> {
    let mut size = 2;
    for (index, (key, value)) in entries.enumerate() {
        if index > 0 {
            size += 1;
        }
        size = add_size(size, string_size(key, limit)?, limit)?;
        size = add_size(size, 1, limit)?;
        size = add_size(size, serialized_size(value, limit)?, limit)?;
    }
    Some(size)
}

fn add_size(size: usize, amount: usize, limit: usize) -> Option<usize> {
    let size = size.checked_add(amount)?;
    (size <= limit).then_some(size)
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Map, Value};

    use super::payload_too_large;

    const LIMIT: usize = kawi_jsonrpc::framing::MAX_MESSAGE_BYTES;

    #[test]
    fn accepts_small_payloads() {
        assert!(!payload_too_large(&json!({"text": "ok"})));
    }

    #[test]
    fn rejects_long_strings() {
        assert!(payload_too_large(&json!({"text": "x".repeat(LIMIT + 1)})));
    }

    #[test]
    fn counts_escaped_control_characters() {
        let text = "\u{1}".repeat(LIMIT / 6 + 1);
        assert!(payload_too_large(&json!({"text": text})));
    }

    #[test]
    fn counts_object_keys() {
        let mut object = Map::new();
        object.insert("x".repeat(LIMIT), json!(0));
        assert!(payload_too_large(&Value::Object(object)));
    }
}
