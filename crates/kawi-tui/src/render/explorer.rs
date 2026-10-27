use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write};

use super::picker::write_padding;
use super::{paint, saturate_u16};
use crate::frame::{
    DiagnosticSeverity, ExplorerGit, ExplorerGitKind, ExplorerPanel, ExplorerRow, ExplorerRowKind,
};
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::{MatchColors, Theme};

pub(super) fn draw_explorer_panel(out: &mut Stdout, frame: &crate::frame::Frame) -> Result<()> {
    let Some(panel) = frame.explorer.as_ref() else {
        return Ok(());
    };
    draw_header(out, frame, panel)?;
    draw_rows(out, frame, panel)?;
    Ok(())
}

fn draw_header(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &ExplorerPanel<'_>,
) -> Result<()> {
    clear_line(out, panel.rect.x, panel.rect.y)?;
    let colors = if panel.focused {
        frame.theme.status.clone()
    } else {
        MatchColors {
            foreground: frame.theme.non_text,
            background: frame.theme.base.background,
        }
    };
    paint::apply_match(out, &colors)?;
    let header = format!("{}  {}", panel.title, panel.status);
    let header = truncate_to_display_width(&header, panel.rect.width);
    write!(out, "{header}").context("write explorer header")?;
    write_padding(out, display_width(header), panel.rect.width)?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn draw_rows(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &ExplorerPanel<'_>,
) -> Result<()> {
    let error_rows = usize::from(panel.error.is_some());
    let rows_height = panel
        .rect
        .height
        .saturating_sub(1)
        .saturating_sub(error_rows);
    for offset in 0..rows_height {
        let index = panel.offset.saturating_add(offset);
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        clear_line(out, panel.rect.x, row)?;
        if let Some(entry) = panel.rows.get(index) {
            draw_row(out, frame, panel, entry, index)?;
        }
    }
    if rows_height == 0 && panel.error.is_none() {
        return Ok(());
    }
    if panel.rows.is_empty() && panel.error.is_none() {
        draw_empty(out, frame, panel)?;
    }
    if let Some(error) = panel.error {
        draw_error(out, frame, panel, error)?;
    }
    Ok(())
}

fn draw_row(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &ExplorerPanel<'_>,
    row: &ExplorerRow<'_>,
    index: usize,
) -> Result<()> {
    let selected = panel.selected == Some(index);
    if selected {
        paint::apply_match(out, &frame.theme.selection)?;
    } else {
        paint::apply_base(out, frame.theme)?;
        apply_label_color(out, frame.theme, row)?;
    }
    let label = row_label(row);
    let markers = row_markers(frame.theme, row);
    let marker_width = markers.len();
    let gap = usize::from(marker_width > 0);
    let label_width = panel
        .rect
        .width
        .saturating_sub(marker_width.saturating_add(gap));
    let label = truncate_to_display_width(&label, label_width);
    write!(out, "{label}").context("write explorer label")?;
    write_padding(out, display_width(label), label_width)?;
    if marker_width > 0 {
        write!(out, " ").context("write explorer marker gap")?;
        write_markers(out, &markers)?;
    }
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn row_label(row: &ExplorerRow<'_>) -> String {
    let indent = "  ".repeat(row.depth);
    let marker = match row.kind {
        ExplorerRowKind::Directory { expanded: true } => "▾ ",
        ExplorerRowKind::Directory { expanded: false } => "▸ ",
        ExplorerRowKind::File => "  ",
        ExplorerRowKind::Symlink => "@ ",
    };
    format!("{indent}{marker}{}", row.name)
}

fn apply_label_color(out: &mut Stdout, theme: &Theme, row: &ExplorerRow<'_>) -> Result<()> {
    let color = if row.hidden || matches!(row.kind, ExplorerRowKind::Symlink) {
        theme.non_text
    } else if matches!(row.kind, ExplorerRowKind::Directory { .. }) {
        theme
            .syntax
            .named("type")
            .and_then(|style| style.foreground)
            .unwrap_or(theme.base.foreground)
    } else if row.active {
        theme.status.foreground
    } else {
        theme.base.foreground
    };
    execute!(out, SetForegroundColor(color)).context("apply explorer label color")?;
    Ok(())
}

fn row_markers(theme: &Theme, row: &ExplorerRow<'_>) -> Vec<(char, Color)> {
    let mut markers = Vec::new();
    if row.dirty {
        markers.push(('*', theme.git_change));
    }
    if let Some(severity) = row.diagnostic {
        markers.push(diagnostic_marker(theme, severity));
    }
    if let Some(git) = row.git {
        markers.push(git_marker(theme, git));
    }
    markers
}

fn diagnostic_marker(theme: &Theme, severity: DiagnosticSeverity) -> (char, Color) {
    match severity {
        DiagnosticSeverity::Error => ('E', theme.diagnostic_error),
        DiagnosticSeverity::Warning => ('W', theme.diagnostic_warning),
        DiagnosticSeverity::Information => ('I', theme.diagnostic_information),
        DiagnosticSeverity::Hint => ('H', theme.diagnostic_hint),
    }
}

fn git_marker(theme: &Theme, git: ExplorerGit) -> (char, Color) {
    let color = match git.kind {
        ExplorerGitKind::Add | ExplorerGitKind::Untracked => theme.git_add,
        ExplorerGitKind::Change => theme.git_change,
        ExplorerGitKind::Delete => theme.git_delete,
        ExplorerGitKind::Conflict => theme.diagnostic_error,
    };
    (git.code, color)
}

fn write_markers(out: &mut Stdout, markers: &[(char, Color)]) -> Result<()> {
    for (character, color) in markers {
        execute!(out, SetForegroundColor(*color)).context("apply explorer marker color")?;
        write!(out, "{character}").context("write explorer marker")?;
    }
    Ok(())
}

fn draw_empty(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &ExplorerPanel<'_>,
) -> Result<()> {
    let row = panel.rect.y.saturating_add(1);
    clear_line(out, panel.rect.x, row)?;
    execute!(out, SetForegroundColor(frame.theme.non_text))
        .context("apply empty explorer color")?;
    write!(out, "(empty)").context("write empty explorer")?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn draw_error(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &ExplorerPanel<'_>,
    error: &str,
) -> Result<()> {
    let row = panel
        .rect
        .y
        .saturating_add(panel.rect.height.saturating_sub(1));
    clear_line(out, panel.rect.x, row)?;
    execute!(out, SetForegroundColor(frame.theme.diagnostic_error))
        .context("apply explorer error color")?;
    let error = truncate_to_display_width(error, panel.rect.width);
    write!(out, "{error}").context("write explorer error")?;
    write_padding(out, display_width(error), panel.rect.width)?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn clear_line(out: &mut Stdout, x: usize, y: usize) -> Result<()> {
    execute!(
        out,
        MoveTo(saturate_u16(x), saturate_u16(y)),
        Clear(ClearType::UntilNewLine)
    )
    .context("clear explorer line")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
