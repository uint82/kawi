use kawi_lsp::SignatureEntry;
use kawi_tui::markdown::{self, MarkdownLine, MarkdownSpan, MarkdownStyle};

use super::super::{signature::SignatureSession, App};
use super::size::size_panel;
use super::{page_height, OverlayData, PanelData};

impl App {
    pub(super) fn signature_overlay(&self) -> Option<OverlayData<'_>> {
        let session = self.signature.as_ref()?;
        let layout = self.panel_layout(self.signature_panel(session));
        Some(OverlayData {
            list: None,
            panel: Some(PanelData {
                rect: layout.rect,
                lines: layout.lines,
                scroll: session.scroll().min(layout.max_scroll),
            }),
        })
    }

    pub(crate) fn signature_scroll(&self) -> Option<(usize, usize)> {
        let session = self.signature.as_ref()?;
        let panel = self.signature_panel(session);
        let layout = self.panel_layout(panel);
        Some((layout.max_scroll, page_height(layout.rect)))
    }

    fn signature_panel(&self, session: &SignatureSession) -> super::size::SizedPanel {
        let limits = self.cursor_limits(
            self.overlay_options.hover_max_width,
            self.overlay_options.min_width,
        );
        size_panel(limits, |width| signature_lines(session, width))
    }
}

fn signature_lines(session: &SignatureSession, width: usize) -> Vec<MarkdownLine> {
    let width = width.max(1);
    let event = session.event();
    let mut lines = Vec::new();
    for (index, signature) in event.signatures.iter().enumerate() {
        if index > 0 {
            lines.push(MarkdownLine::blank());
        }
        let active = event.active_signature == Some(index);
        let marker = if active { "> " } else { "  " };
        let range = active
            .then(|| active_parameter_range(event.active_parameter, signature))
            .flatten();
        lines.extend(markdown::render_spans(
            &signature_spans(marker, &signature.label, range),
            width,
        ));
        if let Some(documentation) = session.document(index) {
            lines.extend(markdown::render_document(documentation, width));
        }
    }
    lines
}

fn signature_spans(marker: &str, label: &str, range: Option<(usize, usize)>) -> Vec<MarkdownSpan> {
    let chars: Vec<char> = label.chars().collect();
    let Some((start, end)) = range.filter(|(start, end)| start < end && *end <= chars.len()) else {
        return vec![normal_span(format!("{marker}{label}"))];
    };
    vec![
        normal_span(format!(
            "{marker}{}",
            chars[..start].iter().collect::<String>()
        )),
        MarkdownSpan {
            text: chars[start..end].iter().collect(),
            style: MarkdownStyle::Strong,
        },
        normal_span(chars[end..].iter().collect()),
    ]
}

fn normal_span(text: String) -> MarkdownSpan {
    MarkdownSpan {
        text,
        style: MarkdownStyle::Normal,
    }
}

fn active_parameter_range(
    index: Option<usize>,
    signature: &SignatureEntry,
) -> Option<(usize, usize)> {
    let (start, end) = index
        .and_then(|index| signature.parameters.get(index))
        .and_then(|parameter| parameter.range)?;
    (start < end).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::SignatureParameter;

    #[test]
    fn active_parameter_is_marked_for_rendering() {
        let signature = SignatureEntry {
            label: "fake(value: i32, other: i32)".to_string(),
            documentation: None,
            parameters: vec![
                SignatureParameter {
                    label: "value".to_string(),
                    documentation: None,
                    range: Some((5, 10)),
                },
                SignatureParameter {
                    label: "other".to_string(),
                    documentation: None,
                    range: Some((17, 22)),
                },
            ],
            active_parameter: None,
        };
        let range = active_parameter_range(Some(1), &signature);
        let spans = signature_spans("> ", &signature.label, range);
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "> fake(value: i32, other: i32)"
        );
        assert_eq!(spans[1].text, "other");
        assert_eq!(spans[1].style, MarkdownStyle::Strong);
    }

    #[test]
    fn invalid_parameter_ranges_leave_the_label_unchanged() {
        let signature = SignatureEntry {
            label: "fake(value)".to_string(),
            documentation: None,
            parameters: vec![SignatureParameter {
                label: "value".to_string(),
                documentation: None,
                range: Some((20, 30)),
            }],
            active_parameter: None,
        };
        let range = active_parameter_range(Some(0), &signature);
        let spans = signature_spans("  ", &signature.label, range);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "  fake(value)");
    }

    #[test]
    fn signature_labels_are_not_interpreted_as_markdown() {
        let spans = signature_spans("  ", "fn value(x: *const u8)", None);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "  fn value(x: *const u8)");
        assert_eq!(spans[0].style, MarkdownStyle::Normal);
    }
}
