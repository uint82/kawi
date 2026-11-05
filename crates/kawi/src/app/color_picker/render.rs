use kawi_tui::markdown::{MarkdownLine, MarkdownSpan, MarkdownStyle};
use kawi_tui::text::sanitize_char;
use kawi_tui::theme::Theme;

use super::super::color_preview::Rgba;
use super::state::{ColorPicker, Field, Mode};

const MAX_PRESENTATION_ROWS: usize = 8;
const SLIDER_WIDTH: usize = 18;
const SWATCH_WIDTH: usize = 24;
const LABEL_WIDTH: usize = 10;
const SWATCH_PREFIX: &str = "Preview  ";
const OPAQUE: u8 = 255;

pub(crate) fn lines(picker: &ColorPicker, theme: &Theme, width: usize) -> Vec<MarkdownLine> {
    match picker.mode() {
        Mode::Adjust => adjust_lines(picker, theme, width),
        Mode::Loading => loading_lines(picker, theme, width),
        Mode::Present => present_lines(picker),
    }
}

fn adjust_lines(picker: &ColorPicker, theme: &Theme, width: usize) -> Vec<MarkdownLine> {
    let mut lines = vec![
        header("Color picker"),
        text(&hex(picker.color())),
        MarkdownLine::blank(),
    ];
    for field in Field::ALL {
        lines.push(field_line(picker, field));
    }
    lines.push(MarkdownLine::blank());
    lines.push(swatch_line(picker.color(), theme, width));
    if let Some(message) = picker.message() {
        lines.push(MarkdownLine::blank());
        lines.push(text(message));
    }
    lines.push(MarkdownLine::blank());
    lines.push(text(
        "h/l adjust  H/L fast  j/k field  Enter formats  Esc close",
    ));
    lines
}

fn loading_lines(picker: &ColorPicker, theme: &Theme, width: usize) -> Vec<MarkdownLine> {
    vec![
        header("Color picker"),
        text(&hex(picker.color())),
        MarkdownLine::blank(),
        swatch_line(picker.color(), theme, width),
        MarkdownLine::blank(),
        text("Requesting color formats..."),
        text("Esc cancel"),
    ]
}

fn present_lines(picker: &ColorPicker) -> Vec<MarkdownLine> {
    let mut lines = vec![
        header("Color formats"),
        text(&hex(picker.color())),
        MarkdownLine::blank(),
    ];
    let presentations = picker.presentations();
    let selected = picker.selected();
    for index in visible_presentations(presentations.len(), selected) {
        let presentation = &presentations[index];
        let marker = if index == selected { "> " } else { "  " };
        let style = if index == selected {
            MarkdownStyle::Strong
        } else {
            MarkdownStyle::Normal
        };
        lines.push(spans(vec![
            span(marker, MarkdownStyle::Normal),
            span(&presentation.label, style),
        ]));
    }
    lines.push(MarkdownLine::blank());
    lines.push(text("j/k select  Enter apply  Esc back"));
    lines
}

fn field_line(picker: &ColorPicker, field: Field) -> MarkdownLine {
    let selected = picker.field() == field;
    let marker = if selected { "> " } else { "  " };
    let value = picker.value(field);
    let label = format!("{:<LABEL_WIDTH$}", field.label());
    let slider = slider(value, field.maximum(), SLIDER_WIDTH);
    let value = field_text(field, value);
    let style = if selected {
        MarkdownStyle::Strong
    } else {
        MarkdownStyle::Normal
    };
    spans(vec![
        span(marker, MarkdownStyle::Normal),
        span(&label, style),
        span(&slider, style),
        span(" ", MarkdownStyle::Normal),
        span(&value, style),
    ])
}

fn swatch_line(color: Rgba, theme: &Theme, width: usize) -> MarkdownLine {
    let style = color.to_style(theme.base.background);
    spans(vec![
        span(SWATCH_PREFIX, MarkdownStyle::Normal),
        span(
            &" ".repeat(
                SWATCH_WIDTH
                    .min(width.saturating_sub(SWATCH_PREFIX.len()))
                    .max(1),
            ),
            MarkdownStyle::Rgb {
                foreground: style.foreground,
                background: style.background,
            },
        ),
    ])
}

fn slider(value: f32, maximum: f32, width: usize) -> String {
    let ratio = (value / maximum).clamp(0.0, 1.0);
    let filled = (ratio * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
}

fn field_text(field: Field, value: f32) -> String {
    match field {
        Field::Hue => format!("{value:.0}deg"),
        Field::Saturation | Field::Lightness | Field::Alpha => format!("{value:.0}%"),
    }
}

fn visible_presentations(len: usize, selected: usize) -> std::ops::Range<usize> {
    if len == 0 {
        return 0..0;
    }
    let start = selected
        .saturating_sub(MAX_PRESENTATION_ROWS - 1)
        .min(len.saturating_sub(MAX_PRESENTATION_ROWS));
    start..(start + MAX_PRESENTATION_ROWS).min(len)
}

fn hex(color: Rgba) -> String {
    if color.alpha == OPAQUE {
        format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.red, color.green, color.blue, color.alpha
        )
    }
}

fn header(value: &str) -> MarkdownLine {
    spans(vec![span(value, MarkdownStyle::Strong)])
}

fn text(value: &str) -> MarkdownLine {
    spans(vec![span(value, MarkdownStyle::Normal)])
}

fn spans(spans: Vec<MarkdownSpan>) -> MarkdownLine {
    MarkdownLine { spans, code: false }
}

fn span(text: &str, style: MarkdownStyle) -> MarkdownSpan {
    let text = text
        .chars()
        .map(|character| match sanitize_char(character) {
            '\t' => ' ',
            character => character,
        })
        .collect();
    MarkdownSpan {
        text,
        style,
        link: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::{CursorPosition, DocumentColor};
    use std::path::PathBuf;

    fn picker() -> ColorPicker {
        ColorPicker::new(
            &DocumentColor {
                start: CursorPosition { line: 0, column: 0 },
                end: CursorPosition { line: 0, column: 7 },
                red: 204,
                green: 34,
                blue: 51,
                alpha: 255,
            },
            PathBuf::from("/tmp/color.css"),
            1,
        )
    }

    #[test]
    fn adjust_view_shows_fields_and_rgb_swatch() {
        let picker = picker();
        let theme = Theme::default();
        let lines = lines(&picker, &theme, 40);
        assert!(lines.iter().any(|line| line.text().contains("Hue")));
        assert!(lines.iter().any(|line| line.text().contains("Saturation")));
        assert!(lines.iter().any(|line| line.text().contains("#cc2233")));
        assert!(lines.iter().flat_map(|line| &line.spans).any(|span| {
            matches!(span.style, MarkdownStyle::Rgb { background, .. } if background == crossterm::style::Color::Rgb { r: 204, g: 34, b: 51 })
        }));
    }

    #[test]
    fn slider_clamps_and_rounds() {
        assert_eq!(slider(0.0, 100.0, 10), "[----------]");
        assert_eq!(slider(100.0, 100.0, 10), "[##########]");
        assert_eq!(slider(50.0, 100.0, 10), "[#####-----]");
    }

    #[test]
    fn visible_window_keeps_selection_in_view() {
        assert_eq!(visible_presentations(20, 0), 0..8);
        assert_eq!(visible_presentations(20, 10), 3..11);
        assert_eq!(visible_presentations(20, 19), 12..20);
        assert_eq!(visible_presentations(0, 0), 0..0);
    }

    #[test]
    fn spans_sanitize_control_characters() {
        let span = span("a\tb\u{1b}[31m", MarkdownStyle::Normal);
        assert_eq!(span.text, "a b [31m");
    }

    #[test]
    fn alpha_hex_includes_alpha_channel() {
        assert_eq!(hex(Rgba::new(204, 34, 51, 255)), "#cc2233");
        assert_eq!(hex(Rgba::new(204, 34, 51, 128)), "#cc223380");
    }
}
