use super::super::position::cursor_position;
use super::super::*;
use super::DocumentColor;
use lsp_types::{ColorInformation, PositionEncodingKind};
use ropey::Rope;
use serde_json::Value;

const MAX_COLORS: usize = 4096;
const NORMALIZED_MAX: f32 = 1.0;
const CHANNEL_SCALE: f32 = 255.0;
const ROUNDING_BIAS: f32 = 0.5;

pub(super) fn document_colors(
    value: &Value,
    text: &Rope,
    encoding: &PositionEncodingKind,
) -> Result<Vec<DocumentColor>, String> {
    let information = serde_json::from_value::<Vec<ColorInformation>>(value.clone())
        .map_err(|error| format!("Invalid document color response: {error}"))?;
    let mut colors: Vec<DocumentColor> = information
        .into_iter()
        .filter_map(|information| convert_color(text, information, encoding))
        .collect();
    merge(&mut colors);
    Ok(colors)
}

pub(super) fn merge(colors: &mut Vec<DocumentColor>) {
    colors.sort_by_key(color_key);
    colors.dedup();
    colors.truncate(MAX_COLORS);
}

fn convert_color(
    text: &Rope,
    information: ColorInformation,
    encoding: &PositionEncodingKind,
) -> Option<DocumentColor> {
    let start = cursor_position(text, information.range.start, encoding);
    let end = cursor_position(text, information.range.end, encoding);
    if !is_forward(start, end) {
        return None;
    }
    Some(DocumentColor {
        start,
        end,
        red: channel(information.color.red),
        green: channel(information.color.green),
        blue: channel(information.color.blue),
        alpha: channel(information.color.alpha),
    })
}

fn is_forward(start: CursorPosition, end: CursorPosition) -> bool {
    (start.line, start.column) < (end.line, end.column)
}

fn channel(value: f32) -> u8 {
    let value = if value.is_finite() {
        value.clamp(0.0, NORMALIZED_MAX)
    } else {
        0.0
    };
    (value * CHANNEL_SCALE + ROUNDING_BIAS) as u8
}

fn color_key(color: &DocumentColor) -> (u32, u32, u32, u32, u8, u8, u8, u8) {
    (
        color.start.line,
        color.start.column,
        color.end.line,
        color.end.column,
        color.red,
        color.green,
        color.blue,
        color.alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn color(start: u32, end: u32, red: f32, green: f32, blue: f32, alpha: f32) -> Value {
        json!({
            "range": {
                "start": {"line": 0, "character": start},
                "end": {"line": 0, "character": end}
            },
            "color": {
                "red": red,
                "green": green,
                "blue": blue,
                "alpha": alpha
            }
        })
    }

    #[test]
    fn converts_clamps_and_merges_colors() {
        let text = Rope::from_str("abcdef\n");
        let value = json!([
            color(0, 3, 1.0, 0.0, 0.0, 1.0),
            color(0, 3, 1.0, 0.0, 0.0, 1.0),
            color(3, 6, 1.5, -1.0, 0.5, 0.5),
            color(6, 6, 0.0, 1.0, 0.0, 1.0),
        ]);
        let colors = document_colors(&value, &text, &PositionEncodingKind::UTF16).expect("colors");
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0].start.column, 0);
        assert_eq!(colors[0].end.column, 3);
        assert_eq!(colors[0].alpha, 255);
        assert_eq!(colors[1].start.column, 3);
        assert_eq!(colors[1].red, 255);
        assert_eq!(colors[1].green, 0);
        assert_eq!(colors[1].blue, 128);
        assert_eq!(colors[1].alpha, 128);
    }

    #[test]
    fn converts_negotiated_character_offsets() {
        let text = Rope::from_str("𝄞 red\n");
        let value = json!([color(3, 6, 0.0, 0.0, 1.0, 1.0)]);
        let utf16 = document_colors(&value, &text, &PositionEncodingKind::UTF16).expect("colors");
        assert_eq!(utf16[0].start.column, 2);
        assert_eq!(utf16[0].end.column, 5);
        let value = json!([color(5, 8, 0.0, 0.0, 1.0, 1.0)]);
        let utf8 = document_colors(&value, &text, &PositionEncodingKind::UTF8).expect("colors");
        assert_eq!(utf8[0].start.column, 2);
        assert_eq!(utf8[0].end.column, 5);
    }

    #[test]
    fn rejects_invalid_responses() {
        let text = Rope::from_str("color\n");
        assert!(document_colors(
            &json!({"color": "red"}),
            &text,
            &PositionEncodingKind::UTF16
        )
        .is_err());
    }
}
