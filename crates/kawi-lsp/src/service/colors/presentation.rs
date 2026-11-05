use super::super::completion_edit::convert_edit;
use super::super::text_requests::{PendingTextRequest, TextRequestKind, TextResponse};
use super::super::*;
use crate::text;
use lsp_types::{ColorPresentation, PositionEncodingKind, Range};
use ropey::Rope;
use serde_json::Value;

const MAX_PRESENTATIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPresentationEntry {
    pub label: String,
    pub edit: TextChange,
    pub additional: Vec<TextChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPresentationEvent {
    pub request: u64,
    pub path: PathBuf,
    pub presentations: Vec<ColorPresentationEntry>,
}

pub(crate) fn event(
    pending: &PendingTextRequest,
    responses: &[TextResponse],
    text: &Rope,
) -> Result<ColorPresentationEvent, String> {
    let TextRequestKind::ColorPresentation { range, .. } = &pending.kind else {
        return Err("invalid color presentation request".to_string());
    };
    let mut presentations = Vec::new();
    let mut error = None;
    let mut parsed = false;
    for response in responses {
        match convert(&response.value, text, &response.encoding, *range) {
            Ok(mut entries) => {
                parsed = true;
                presentations.append(&mut entries);
            }
            Err(message) => {
                if error.is_none() {
                    error = Some(message);
                }
            }
        }
    }
    if !parsed {
        return Err(error.unwrap_or_else(|| "No color presentations found".to_string()));
    }
    merge(&mut presentations);
    Ok(ColorPresentationEvent {
        request: pending.request,
        path: pending.path.clone(),
        presentations,
    })
}

fn convert(
    value: &Value,
    text: &Rope,
    encoding: &PositionEncodingKind,
    range: (CursorPosition, CursorPosition),
) -> Result<Vec<ColorPresentationEntry>, String> {
    let presentations = serde_json::from_value::<Vec<ColorPresentation>>(value.clone())
        .map_err(|error| format!("Invalid color presentation response: {error}"))?;
    Ok(presentations
        .into_iter()
        .filter_map(|presentation| convert_presentation(text, encoding, range, presentation))
        .collect())
}

fn convert_presentation(
    text: &Rope,
    encoding: &PositionEncodingKind,
    range: (CursorPosition, CursorPosition),
    presentation: ColorPresentation,
) -> Option<ColorPresentationEntry> {
    let edit = match presentation.text_edit {
        Some(edit) => convert_edit(text, &edit.range, &edit.new_text, encoding)?,
        None => range_edit(text, encoding, range, &presentation.label)?,
    };
    let additional = presentation
        .additional_text_edits
        .unwrap_or_default()
        .iter()
        .map(|edit| convert_edit(text, &edit.range, &edit.new_text, encoding))
        .collect::<Option<Vec<_>>>()?;
    Some(ColorPresentationEntry {
        label: presentation.label,
        edit,
        additional,
    })
}

fn range_edit(
    text: &Rope,
    encoding: &PositionEncodingKind,
    range: (CursorPosition, CursorPosition),
    value: &str,
) -> Option<TextChange> {
    let start = text::position_from_column(text, range.0.line, range.0.column, encoding);
    let end = text::position_from_column(text, range.1.line, range.1.column, encoding);
    convert_edit(text, &Range::new(start, end), value, encoding)
}

fn merge(presentations: &mut Vec<ColorPresentationEntry>) {
    presentations.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.edit.start.cmp(&right.edit.start))
            .then_with(|| left.edit.end.cmp(&right.edit.end))
            .then_with(|| left.edit.text.cmp(&right.edit.text))
    });
    presentations.dedup();
    presentations.truncate(MAX_PRESENTATIONS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pending(range: (CursorPosition, CursorPosition)) -> PendingTextRequest {
        PendingTextRequest::new(
            7,
            TextRequestKind::ColorPresentation {
                range,
                color: ColorValue::new(255, 0, 0, 255),
            },
            PathBuf::from("/tmp/color.css"),
            range.0,
            1,
            1,
        )
    }

    fn response(value: Value) -> TextResponse {
        TextResponse {
            server: "fake".to_string(),
            key: ClientKey {
                server: "fake".to_string(),
                root: PathBuf::from("/tmp"),
            },
            value,
            encoding: PositionEncodingKind::UTF16,
            resolve: false,
        }
    }

    fn range(start: u32, end: u32) -> Value {
        json!({
            "start": {"line": 0, "character": start},
            "end": {"line": 0, "character": end}
        })
    }

    #[test]
    fn converts_edits_and_sorts_presentations() {
        let text = Rope::from_str("color: #ff0000;\n");
        let range = (
            CursorPosition { line: 0, column: 7 },
            CursorPosition {
                line: 0,
                column: 14,
            },
        );
        let value = json!([
            {
                "label": "rgb(255, 0, 0)",
                "textEdit": {"range": self::range(7, 14), "newText": "rgb(255, 0, 0)"}
            },
            {
                "label": "#ff0000",
                "textEdit": {"range": self::range(7, 14), "newText": "#ff0000"}
            },
            {
                "label": "red"
            }
        ]);
        let event = event(&pending(range), &[response(value)], &text).expect("presentations");
        assert_eq!(event.presentations.len(), 3);
        assert_eq!(event.presentations[0].label, "#ff0000");
        assert_eq!(event.presentations[1].label, "red");
        assert_eq!(event.presentations[1].edit.start, 7);
        assert_eq!(event.presentations[1].edit.end, 14);
        assert_eq!(event.presentations[1].edit.text, "red");
        assert_eq!(event.presentations[2].label, "rgb(255, 0, 0)");
    }

    #[test]
    fn rejects_invalid_responses() {
        let text = Rope::from_str("color: #ff0000;\n");
        let range = (
            CursorPosition { line: 0, column: 7 },
            CursorPosition {
                line: 0,
                column: 14,
            },
        );
        let result = event(
            &pending(range),
            &[response(json!({"presentations": []}))],
            &text,
        );
        assert!(result.is_err());
    }
}
