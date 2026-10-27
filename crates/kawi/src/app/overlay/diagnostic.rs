use std::path::Path;

use kawi_tui::markdown::{self, MarkdownLine, MarkdownSpan, MarkdownStyle};

use super::super::diagnostic_popup::DiagnosticPopup;
use super::size::{size_panel, SizedPanel};
use super::{page_height, OverlayData, PanelData};
use crate::app::App;

impl App {
    pub(super) fn diagnostic_overlay(&self) -> Option<OverlayData<'_>> {
        let popup = self.diagnostic_popup.as_ref()?;
        let layout = self.panel_layout(self.diagnostic_panel(popup));
        Some(OverlayData {
            list: None,
            panel: Some(PanelData {
                rect: layout.rect,
                lines: layout.lines,
                scroll: popup.scroll().min(layout.max_scroll),
            }),
            key_menu: None,
        })
    }

    pub(crate) fn diagnostic_scroll(&self) -> Option<(usize, usize)> {
        let popup = self.diagnostic_popup.as_ref()?;
        let layout = self.panel_layout(self.diagnostic_panel(popup));
        Some((layout.max_scroll, page_height(layout.rect)))
    }

    fn diagnostic_panel(&self, popup: &DiagnosticPopup) -> SizedPanel {
        let limits = self.cursor_limits(
            self.overlay_options.diagnostic_popup_max_width,
            self.overlay_options.min_width,
        );
        size_panel(limits, |width| diagnostic_lines(popup, width))
    }
}

fn diagnostic_lines(popup: &DiagnosticPopup, width: usize) -> Vec<MarkdownLine> {
    let mut lines = Vec::new();
    for (index, entry) in popup.entries().iter().enumerate() {
        if index > 0 {
            lines.push(MarkdownLine::blank());
        }
        push_entry(&mut lines, entry, popup.show_source(), width);
    }
    lines
}

fn push_entry(
    lines: &mut Vec<MarkdownLine>,
    entry: &crate::app::problems::Problem,
    show_source: bool,
    width: usize,
) {
    let mut text = entry.severity.label().to_string();
    if show_source {
        text.push_str(" [");
        text.push_str(&entry.source);
        text.push(']');
    }
    if let Some(code) = entry.code.as_deref() {
        text.push(' ');
        text.push_str(code);
    }
    text.push_str(": ");
    let mut message = entry.message.lines();
    text.push_str(message.next().unwrap_or_default());
    push_plain(lines, &text, width);
    for line in message {
        push_plain(lines, &format!("  {line}"), width);
    }
    if let Some(href) = entry.code_description.as_deref() {
        push_plain(lines, &format!("  code: {href}"), width);
    }
    for related in &entry.related {
        let path = short_path(&related.path);
        push_plain(
            lines,
            &format!("  related: {path}:{}: {}", related.line, related.message),
            width,
        );
    }
}

fn push_plain(lines: &mut Vec<MarkdownLine>, text: &str, width: usize) {
    lines.extend(markdown::render_spans(
        &[MarkdownSpan {
            text: text.to_string(),
            style: MarkdownStyle::Normal,
            link: None,
        }],
        width,
    ));
}

fn short_path(path: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    kawi_editor::paths::shorten(path, &cwd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::diagnostic_popup::DiagnosticPopup;
    use crate::app::problems::{Problem, ProblemSeverity, RelatedProblem};
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use std::path::PathBuf;

    fn entry(message: &str) -> Problem {
        Problem {
            source: "lsp:rust-analyzer".to_string(),
            path: PathBuf::from("/tmp/diagnostic.rs"),
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 2,
            severity: ProblemSeverity::Error,
            message: message.to_string(),
            code: Some("E0308".to_string()),
            code_description: Some("https://example.com/error".to_string()),
            provider: Some("rustc".to_string()),
            tags: Vec::new(),
            related: Vec::new(),
        }
    }

    fn popup(entries: Vec<Problem>, show_source: bool) -> DiagnosticPopup {
        DiagnosticPopup::new(entries, show_source)
    }

    #[test]
    fn entries_render_as_plain_text() {
        let popup = popup(vec![entry("**not markdown**")], false);
        let lines = diagnostic_lines(&popup, 80);
        let text = lines
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("**not markdown**"));
        assert!(!text.contains("\\*"));
        assert!(text.contains("code: https://example.com/error"));
    }

    #[test]
    fn source_is_hidden_when_not_requested() {
        let popup = self::popup(vec![entry("message")], false);
        let text = diagnostic_lines(&popup, 80)
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("lsp:rust-analyzer"));
        let popup = self::popup(vec![entry("message")], true);
        let text = diagnostic_lines(&popup, 80)
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("lsp:rust-analyzer"));
    }

    #[test]
    fn popup_near_the_bottom_stays_visible() {
        let mut app = App::new(Buffer::from_string("one\ntwo\nthree\n"), Flavor::Vim);
        app.set_screen_size(40, 10);
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('G'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .expect("bottom");
        app.diagnostic_popup = Some(popup(vec![entry("message")], false));
        let overlay = app.diagnostic_overlay().expect("overlay");
        let panel = overlay.panel.expect("panel");
        assert!(panel.rect.y + panel.rect.height <= 10);
        assert!(panel.rect.height > 0);
    }

    #[test]
    fn related_information_and_control_characters_are_rendered_safely() {
        let mut item = entry("bad\u{1b}[31mmessage");
        item.related = vec![RelatedProblem {
            path: PathBuf::from("/tmp/outside/related.rs"),
            line: 7,
            message: "related\u{7}text".to_string(),
        }];
        let popup = self::popup(vec![item], false);
        let text = diagnostic_lines(&popup, 80)
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains('\u{1b}'));
        assert!(!text.contains('\u{7}'));
        assert!(text.contains("related.rs:7: related text"));
    }

    #[test]
    fn popup_placement_stays_inside_the_window() {
        let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
        app.set_screen_size(40, 12);
        app.diagnostic_popup = Some(popup(vec![entry("message")], false));
        let overlay = app.diagnostic_overlay().expect("overlay");
        let panel = overlay.panel.expect("panel");
        assert!(panel.rect.x + panel.rect.width <= 40);
        assert!(panel.rect.y + panel.rect.height <= 12);
    }
}
