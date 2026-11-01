use kawi_plugin::{
    Decoration, DecorationRef, DecorationScope, DecorationStyle as ProtocolStyle, PluginCatalog,
    Position,
};
use kawi_tui::frame::PluginStyle;
use kawi_tui::theme::Theme;
use ropey::Rope;

use super::*;
use crate::app::plugin::style::PluginStyles;

fn position(line: usize, byte_column: usize) -> Position {
    Position { line, byte_column }
}

fn styles() -> PluginStyles {
    PluginStyles::new(&PluginCatalog::default(), &Theme::default())
}

fn ui_style(style: ProtocolStyle) -> PluginStyle {
    styles()
        .resolve("demo", &DecorationRef::Style(style))
        .expect("style")
}

fn highlight(start: Position, end: Position, style: ProtocolStyle) -> Decoration {
    Decoration::Highlight {
        start,
        end,
        group: None,
        style: Some(style),
    }
}

fn virtual_text(line: usize, text: &str, style: ProtocolStyle) -> Decoration {
    Decoration::VirtualText {
        line,
        text: text.to_string(),
        group: None,
        style: Some(style),
    }
}

fn sign(line: usize, text: &str, style: ProtocolStyle) -> Decoration {
    Decoration::Sign {
        line,
        text: text.to_string(),
        group: None,
        style: Some(style),
    }
}

fn set(
    decorations: &mut PluginDecorations,
    decorations_list: &[Decoration],
) -> Result<(), RpcError> {
    decorations.set(
        "demo",
        1,
        DecorationScope::All,
        decorations_list,
        &Rope::from_str("alpha\nbeta\ngamma\n"),
        &styles(),
    )
}

#[test]
fn highlights_are_clipped_to_visible_rows() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 1),
            position(2, 2),
            ProtocolStyle::Info,
        )],
    )
    .unwrap();
    let spans = decorations.spans(1, &[0, 2, 4]);
    assert_eq!(spans.len(), 2);
    assert_eq!(
        (spans[0].row, spans[0].start, spans[0].end, spans[0].style),
        (0, 1, usize::MAX, ui_style(ProtocolStyle::Info))
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
            ProtocolStyle::Muted,
        )],
    )
    .unwrap();
    assert!(decorations.spans(1, &[1]).is_empty());
}

#[test]
fn virtual_texts_are_grouped_by_row_and_sanitized() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[
            virtual_text(1, "bad\u{1b}[31m", ProtocolStyle::Warning),
            virtual_text(3, "tail", ProtocolStyle::Muted),
        ],
    )
    .unwrap();
    let rows = decorations.virtual_texts(1, &[0, 1, 2]);
    assert_eq!(rows[0].len(), 0);
    assert_eq!(rows[1].len(), 1);
    assert_eq!(rows[1][0].text, "bad");
    assert_eq!(
        rows[1][0].kind,
        kawi_tui::frame::VirtualTextKind::Plugin(ui_style(ProtocolStyle::Warning))
    );
}

#[test]
fn decorations_survive_buffer_revision_changes() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 0),
            position(0, 1),
            ProtocolStyle::Error,
        )],
    )
    .unwrap();
    assert_eq!(decorations.spans(1, &[0]).len(), 1);
    assert_eq!(decorations.virtual_texts(1, &[0])[0].len(), 0);
}

#[test]
fn retain_live_keeps_decorations_after_a_revision_change() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 0),
            position(0, 1),
            ProtocolStyle::Info,
        )],
    )
    .unwrap();
    let mut editor = kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("alpha\n"));
    editor.paste("x");
    decorations.retain_live(&editor);
    assert_eq!(decorations.spans(1, &[0]).len(), 1);
}

#[test]
fn retain_live_drops_closed_buffers_only() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 0),
            position(0, 1),
            ProtocolStyle::Accent,
        )],
    )
    .unwrap();
    decorations
        .set(
            "demo",
            2,
            DecorationScope::All,
            &[highlight(
                position(0, 0),
                position(0, 1),
                ProtocolStyle::Info,
            )],
            &Rope::from_str("alpha\n"),
            &styles(),
        )
        .unwrap();
    let editor = kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("alpha\n"));
    decorations.retain_live(&editor);
    assert_eq!(decorations.spans(1, &[0]).len(), 1);
    assert!(decorations.spans(2, &[0]).is_empty());
}

#[test]
fn removing_a_plugin_drops_all_of_its_sets() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[highlight(
            position(0, 0),
            position(0, 1),
            ProtocolStyle::Accent,
        )],
    )
    .unwrap();
    decorations.remove_plugin("demo");
    assert!(decorations.spans(1, &[0]).is_empty());
}

#[test]
fn invalid_ranges_and_lines_are_rejected() {
    let mut decorations = PluginDecorations::default();
    let error = set(
        &mut decorations,
        &[highlight(
            position(1, 2),
            position(0, 1),
            ProtocolStyle::Info,
        )],
    )
    .unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_POSITION);
    let error = set(
        &mut decorations,
        &[virtual_text(9, "missing", ProtocolStyle::Info)],
    )
    .unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_POSITION);
}

#[test]
fn signs_are_placed_on_visible_rows() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[
            sign(1, "!", ProtocolStyle::Error),
            sign(3, "?", ProtocolStyle::Info),
        ],
    )
    .unwrap();
    let signs = decorations.signs(1, &[0, 1, 2]);
    assert_eq!(signs.len(), 3);
    assert!(signs[0].is_none());
    let sign = signs[1].as_ref().expect("sign");
    assert_eq!(sign.text(), "!");
    assert_eq!(
        sign,
        &Sign::Plugin(PluginSign {
            text: "!".to_string(),
            style: ui_style(ProtocolStyle::Error),
        })
    );
    assert!(signs[2].is_none());
}

#[test]
fn sign_text_is_sanitized_and_limited_to_one_cell() {
    let mut decorations = PluginDecorations::default();
    set(
        &mut decorations,
        &[sign(0, "\u{1b}[31m!", ProtocolStyle::Warning)],
    )
    .unwrap();
    assert_eq!(
        decorations.signs(1, &[0])[0].as_ref().expect("sign").text(),
        "!"
    );
    let error = set(&mut decorations, &[sign(0, "ab", ProtocolStyle::Warning)]).unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_PARAMS);
    let error = set(&mut decorations, &[sign(9, "x", ProtocolStyle::Warning)]).unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_POSITION);
}

#[test]
fn signs_report_whether_a_buffer_has_any() {
    let mut decorations = PluginDecorations::default();
    assert!(!decorations.has_signs(1));
    set(&mut decorations, &[sign(0, "!", ProtocolStyle::Info)]).unwrap();
    assert!(decorations.has_signs(1));
    assert!(!decorations.has_signs(2));
}

#[test]
fn named_groups_resolve_through_the_plugin_catalog() {
    let manifest = toml::from_str(
        r#"name = "demo"
version = "0.1.0"
protocol = 1

[[decorations]]
id = "todo"
color = "info"
italic = true

[[decorations]]
id = "todo_text"
color = "info"
fill = "foreground"
italic = true

[[decorations]]
id = "todo_colon"
color = "info"
hidden = true
"#,
    )
    .expect("manifest");
    let catalog = PluginCatalog {
        plugins: vec![kawi_plugin::LoadedPlugin {
            manifest,
            dir: std::path::PathBuf::new(),
            path: std::path::PathBuf::new(),
            computed_hash: None,
            disabled_reason: None,
        }],
        warnings: Vec::new(),
    };
    let styles = PluginStyles::new(&catalog, &Theme::default());
    let style = styles
        .resolve("demo", &DecorationRef::Group("todo".to_string()))
        .expect("group");
    assert!(style.italic);
    assert!(!style.bold);
    let text = styles
        .resolve("demo", &DecorationRef::Group("todo_text".to_string()))
        .expect("text group");
    assert_eq!(text.foreground, text.accent);
    assert_eq!(text.background, Theme::default().base.background);
    let colon = styles
        .resolve("demo", &DecorationRef::Group("todo_colon".to_string()))
        .expect("colon group");
    assert!(colon.hidden);
}
