use crate::protocol::{
    Decoration, DecorationStyle as ProtocolDecorationStyle, RpcError, StatuslineSegment,
};

use super::super::bindings::kawi::plugin::editor_signs::Sign as WitSign;
use super::super::bindings::kawi::plugin::types::{
    Decoration as WitDecoration, DecorationStyle as WitDecorationStyle,
};
use super::super::bindings::kawi::plugin::ui_statusline::StatuslineSegment as WitStatuslineSegment;
use super::position_to_protocol;

pub fn decorations_to_protocol(
    decorations: Vec<WitDecoration>,
) -> Result<Vec<Decoration>, RpcError> {
    decorations
        .into_iter()
        .map(|decoration| match decoration {
            WitDecoration::Highlight(highlight) => Ok(Decoration::Highlight {
                start: position_to_protocol(highlight.start)?,
                end: position_to_protocol(highlight.end)?,
                group: Some(highlight.group),
                style: None,
            }),
            WitDecoration::VirtualText(text) => Ok(Decoration::VirtualText {
                line: text.line as usize,
                text: text.text,
                group: Some(text.group),
                style: None,
            }),
        })
        .collect()
}

pub fn signs_to_protocol(signs: Vec<WitSign>) -> Vec<Decoration> {
    signs
        .into_iter()
        .map(|sign| Decoration::Sign {
            line: sign.line as usize,
            text: sign.text,
            group: Some(sign.group),
            style: None,
        })
        .collect()
}

pub fn statusline_to_protocol(segments: Vec<WitStatuslineSegment>) -> Vec<StatuslineSegment> {
    segments
        .into_iter()
        .map(|segment| StatuslineSegment {
            text: segment.text,
            style: decoration_style_to_protocol(segment.style),
            priority: segment.priority,
        })
        .collect()
}

fn decoration_style_to_protocol(style: WitDecorationStyle) -> ProtocolDecorationStyle {
    match style {
        WitDecorationStyle::Accent => ProtocolDecorationStyle::Accent,
        WitDecorationStyle::Info => ProtocolDecorationStyle::Info,
        WitDecorationStyle::Success => ProtocolDecorationStyle::Success,
        WitDecorationStyle::Warning => ProtocolDecorationStyle::Warning,
        WitDecorationStyle::Error => ProtocolDecorationStyle::Error,
        WitDecorationStyle::Muted => ProtocolDecorationStyle::Muted,
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::bindings::kawi::plugin::types::Highlight;
    use super::super::super::bindings::kawi::plugin::types::Position as WitPosition;
    use super::super::super::bindings::kawi::plugin::types::VirtualText;
    use super::*;
    use crate::protocol::{DecorationStyle, Position};

    fn position(line: u32, byte_column: u32) -> WitPosition {
        WitPosition { line, byte_column }
    }

    #[test]
    fn converts_highlights_and_virtual_text() {
        let decorations = decorations_to_protocol(vec![
            WitDecoration::Highlight(Highlight {
                start: position(1, 2),
                end: position(1, 4),
                group: "todo".to_string(),
            }),
            WitDecoration::VirtualText(VirtualText {
                line: 3,
                text: "note".to_string(),
                group: "note".to_string(),
            }),
        ])
        .unwrap();
        assert_eq!(
            decorations[0],
            Decoration::Highlight {
                start: Position {
                    line: 1,
                    byte_column: 2,
                },
                end: Position {
                    line: 1,
                    byte_column: 4,
                },
                group: Some("todo".to_string()),
                style: None,
            }
        );
        assert_eq!(
            decorations[1],
            Decoration::VirtualText {
                line: 3,
                text: "note".to_string(),
                group: Some("note".to_string()),
                style: None,
            }
        );
    }

    #[test]
    fn converts_signs() {
        let decorations = signs_to_protocol(vec![WitSign {
            line: 2,
            text: "!".to_string(),
            group: "fixme".to_string(),
        }]);
        assert_eq!(
            decorations,
            vec![Decoration::Sign {
                line: 2,
                text: "!".to_string(),
                group: Some("fixme".to_string()),
                style: None,
            }]
        );
    }

    #[test]
    fn converts_statusline_segments() {
        let segments = statusline_to_protocol(vec![WitStatuslineSegment {
            text: "indexed".to_string(),
            style: WitDecorationStyle::Success,
            priority: 9,
        }]);
        assert_eq!(
            segments,
            vec![StatuslineSegment {
                text: "indexed".to_string(),
                style: DecorationStyle::Success,
                priority: 9,
            }]
        );
    }
}
