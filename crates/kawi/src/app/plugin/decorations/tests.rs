use kawi_plugin::{Decoration, DecorationStyle as PluginStyle, Position};
use kawi_tui::frame::DecorationStyle;
use ropey::Rope;

use super::*;

fn position(line: usize, byte_column: usize) -> Position {
    Position { line, byte_column }
}

fn highlight(start: Position, end: Position, style: PluginStyle) -> Decoration {
    Decoration::Highlight { start, end, style }
}

fn virtual_text(line: usize, text: &str, style: PluginStyle) -> Decoration {
    Decoration::VirtualText {
        line,
        text: text.to_string(),
        style,
    }
}

fn set(
    decorations: &mut PluginDecorations,
    decorations_list: &[Decoration],
) -> Result<(), RpcError> {
    decorations.set(
        "demo",
        1,
        4,
        decorations_list,
        &Rope::from_str("alpha\nbeta\ngamma\n"),
    )
}

#[test]
fn highlights_are_clipped_to_visible_rows() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(position(0, 1), position(2, 2), PluginStyle::Info)],
    )
    .unwrap();
    let spans = decorations.spans(1, 4, &[0, 2, 4]);
    assert_eq!(spans.len(), 2);
    assert_eq!(
        (spans[0].row, spans[0].start, spans[0].end, spans[0].style),
        (0, 1, usize::MAX, DecorationStyle::Info)
    );
    assert_eq!((spans[1].row, spans[1].start, spans[1].end), (2, 0, 2));
}

#[test]
fn zero_length_highlights_are_ignored() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(1, 2),
            position(1, 2),
            PluginStyle::Muted,
        )],
    )
    .unwrap();
    assert!(decorations.spans(1, 4, &[1]).is_empty());
}

#[test]
fn virtual_texts_are_grouped_by_row_and_sanitized() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[
            virtual_text(1, "bad\u{1b}[31m", PluginStyle::Warning),
            virtual_text(3, "tail", PluginStyle::Muted),
        ],
    )
    .unwrap();
    let rows = decorations.virtual_texts(1, 4, &[0, 1, 2]);
    assert_eq!(rows[0].len(), 0);
    assert_eq!(rows[1].len(), 1);
    assert_eq!(rows[1][0].text, "bad");
    assert_eq!(
        rows[1][0].kind,
        kawi_tui::frame::VirtualTextKind::Plugin(DecorationStyle::Warning)
    );
}

#[test]
fn stale_revisions_are_not_rendered() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 0),
            position(0, 1),
            PluginStyle::Error,
        )],
    )
    .unwrap();
    assert!(decorations.spans(1, 5, &[0]).is_empty());
    assert!(decorations
        .virtual_texts(1, 5, &[0])
        .iter()
        .all(Vec::is_empty));
}

#[test]
fn removing_a_plugin_drops_all_of_its_sets() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 0),
            position(0, 1),
            PluginStyle::Accent,
        )],
    )
    .unwrap();
    decorations.remove_plugin("demo");
    assert!(decorations.spans(1, 4, &[0]).is_empty());
}

#[test]
fn invalid_ranges_and_lines_are_rejected() {
    let mut decorations = PluginDecorations::default();
    let error = set(
        &mut decorations,
        &[highlight(position(1, 2), position(0, 1), PluginStyle::Info)],
    )
    .unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_POSITION);
    let error = set(
        &mut decorations,
        &[virtual_text(9, "missing", PluginStyle::Info)],
    )
    .unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_POSITION);
}

fn sign(line: usize, text: &str, style: PluginStyle) -> Decoration {
    Decoration::Sign {
        line,
        text: text.to_string(),
        style,
    }
}

#[test]
fn signs_are_placed_on_visible_rows() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[
            sign(1, "!", PluginStyle::Error),
            sign(3, "?", PluginStyle::Info),
        ],
    )
    .unwrap();
    let signs = decorations.signs(1, 4, &[0, 1, 2]);
    assert_eq!(signs.len(), 3);
    assert!(signs[0].is_none());
    let sign = signs[1].as_ref().expect("sign");
    assert_eq!(sign.text(), "!");
    assert_eq!(
        sign,
        &Sign::Plugin(PluginSign {
            text: "!".to_string(),
            style: DecorationStyle::Error,
        })
    );
    assert!(signs[2].is_none());
}

#[test]
fn sign_text_is_sanitized_and_limited_to_one_cell() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[sign(0, "\u{1b}[31m!", PluginStyle::Warning)],
    )
    .unwrap();
    assert_eq!(
        decorations.signs(1, 4, &[0])[0]
            .as_ref()
            .expect("sign")
            .text(),
        "!"
    );
    let error = set(&mut decorations, &[sign(0, "ab", PluginStyle::Warning)]).unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_PARAMS);
    let error = set(&mut decorations, &[sign(9, "x", PluginStyle::Warning)]).unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_POSITION);
}

#[test]
fn signs_report_whether_a_buffer_has_any() {
    let mut decorations = PluginDecorations::default();
    assert!(!decorations.has_signs(1, 4));
    set(&mut decorations, &[sign(0, "!", PluginStyle::Info)]).unwrap();
    assert!(decorations.has_signs(1, 4));
    assert!(!decorations.has_signs(1, 5));
}
