use super::*;
use crate::frame::Rect;

fn row(kind: ExplorerRowKind) -> ExplorerRow<'static> {
    ExplorerRow {
        depth: 1,
        name: "src",
        kind,
        hidden: false,
        active: false,
        dirty: false,
        git: None,
        diagnostic: None,
    }
}

#[test]
fn headers_do_not_clear_outside_the_panel() {
    let mut out = Vec::new();
    let panel = ExplorerPanel {
        rect: Rect::new(2, 1, 10, 4),
        title: "Explorer",
        status: "2 items",
        focused: false,
        rows: &[],
        selected: None,
        offset: 0,
        error: None,
    };
    draw_header(&mut out, &Theme::default(), &panel).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(!text.contains("\u{1b}[K"));
    assert!(text.contains("Explorer"));
}

#[test]
fn blank_lines_fill_only_the_panel_width() {
    let mut out = Vec::new();
    blank_line(&mut out, 2, 3, 5).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "\u{1b}[4;3H     ");
}

#[test]
fn row_labels_show_depth_and_directory_state() {
    assert_eq!(
        row_label(&row(ExplorerRowKind::Directory { expanded: true })),
        "  \u{25be} src"
    );
    assert_eq!(
        row_label(&row(ExplorerRowKind::Directory { expanded: false })),
        "  \u{25b8} src"
    );
    assert_eq!(row_label(&row(ExplorerRowKind::File)), "    src");
    assert_eq!(row_label(&row(ExplorerRowKind::Symlink)), "  @ src");
}

#[test]
fn row_markers_order_dirty_diagnostic_and_git_state() {
    let theme = Theme::default();
    let mut entry = row(ExplorerRowKind::File);
    entry.dirty = true;
    entry.diagnostic = Some(DiagnosticSeverity::Warning);
    entry.git = Some(ExplorerGit {
        code: 'M',
        kind: ExplorerGitKind::Change,
    });
    let markers: Vec<char> = row_markers(&theme, &entry)
        .iter()
        .map(|(marker, _)| *marker)
        .collect();
    assert_eq!(markers, ['*', 'W', 'M']);
}
