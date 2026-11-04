use super::*;
use crate::frame::Rect;
use crate::surface::Surface;

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
    let theme = Theme::default();
    let mut out = Surface::new(14, 6, theme.base.foreground, theme.base.background);
    fill(&mut out, 'x');
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
    draw_header(&mut out, &theme, &panel).unwrap();
    assert_eq!(out.cell(0, 1).expect("outside cell").text(), "x");
    assert_eq!(out.cell(13, 1).expect("outside cell").text(), "x");
    assert_eq!(out.cell(2, 1).expect("header cell").text(), "E");
}

#[test]
fn blank_lines_fill_only_the_panel_width() {
    let theme = Theme::default();
    let mut out = Surface::new(10, 4, theme.base.foreground, theme.base.background);
    out.move_to(0, 3);
    out.write_text("xxxxxxxxxx");
    blank_line(&mut out, 2, 3, 5).unwrap();
    assert_eq!(out.cell(1, 3).expect("outside cell").text(), "x");
    assert_eq!(out.cell(2, 3).expect("blank cell").text(), " ");
    assert_eq!(out.cell(6, 3).expect("blank cell").text(), " ");
    assert_eq!(out.cell(7, 3).expect("outside cell").text(), "x");
}

fn fill(out: &mut Surface, character: char) {
    let text = character.to_string().repeat(out.width());
    for y in 0..out.height() {
        out.move_to(0, y);
        out.write_text(&text);
    }
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
