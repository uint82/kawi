use anyhow::{Context, Result};
use crossterm::style::Color;
use std::io::Write;

use super::paint;
use super::picker::write_padding;
use crate::frame::{
    DiagnosticSeverity, ExplorerGit, ExplorerGitKind, ExplorerPanel, ExplorerRow, ExplorerRowKind,
};
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::{MatchColors, Theme};

pub(super) fn draw_explorer_panel(out: &mut Surface, frame: &crate::frame::Frame) -> Result<()> {
    let Some(panel) = frame.explorer.as_ref() else {
        return Ok(());
    };
    draw_header(out, frame.theme, panel)?;
    draw_rows(out, frame.theme, panel)?;
    Ok(())
}

fn draw_header(out: &mut Surface, theme: &Theme, panel: &ExplorerPanel<'_>) -> Result<()> {
    line_start(out, panel.rect.x, panel.rect.y)?;
    let colors = if panel.focused {
        theme.status.clone()
    } else {
        MatchColors {
            foreground: theme.non_text,
            background: theme.base.background,
        }
    };
    paint::apply_match(out, &colors);
    let header = format!("{}  {}", panel.title, panel.status);
    let header = truncate_to_display_width(&header, panel.rect.width);
    write!(out, "{header}").context("write explorer header")?;
    write_padding(out, display_width(header), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

struct RowSlot<'a> {
    entry: &'a ExplorerRow<'a>,
    index: usize,
    y: usize,
}

fn draw_rows(out: &mut Surface, theme: &Theme, panel: &ExplorerPanel<'_>) -> Result<()> {
    let error_rows = usize::from(panel.error.is_some());
    let rows_height = panel
        .rect
        .height
        .saturating_sub(1)
        .saturating_sub(error_rows);
    for offset in 0..rows_height {
        let index = panel.offset.saturating_add(offset);
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        if let Some(entry) = panel.rows.get(index) {
            draw_row(
                out,
                theme,
                panel,
                RowSlot {
                    entry,
                    index,
                    y: row,
                },
            )?;
        } else {
            blank_line(out, panel.rect.x, row, panel.rect.width)?;
        }
    }
    if rows_height == 0 && panel.error.is_none() {
        return Ok(());
    }
    if panel.rows.is_empty() && panel.error.is_none() {
        draw_empty(out, theme, panel)?;
    }
    if let Some(error) = panel.error {
        draw_error(out, theme, panel, error)?;
    }
    Ok(())
}

fn draw_row(
    out: &mut Surface,
    theme: &Theme,
    panel: &ExplorerPanel<'_>,
    slot: RowSlot<'_>,
) -> Result<()> {
    line_start(out, panel.rect.x, slot.y)?;
    let selected = panel.selected == Some(slot.index);
    if selected {
        paint::apply_match(out, &theme.selection);
    } else {
        paint::apply_base(out, theme);
        apply_label_color(out, theme, slot.entry);
    }
    let label = row_label(slot.entry);
    let markers = row_markers(theme, slot.entry);
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
    paint::restore_base(out, theme);
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

fn apply_label_color(out: &mut Surface, theme: &Theme, row: &ExplorerRow<'_>) {
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
    out.set_foreground(color);
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

fn write_markers(out: &mut Surface, markers: &[(char, Color)]) -> Result<()> {
    for (character, color) in markers {
        out.set_foreground(*color);
        write!(out, "{character}").context("write explorer marker")?;
    }
    Ok(())
}

fn draw_empty(out: &mut Surface, theme: &Theme, panel: &ExplorerPanel<'_>) -> Result<()> {
    let row = panel.rect.y.saturating_add(1);
    line_start(out, panel.rect.x, row)?;
    out.set_foreground(theme.non_text);
    let label = "(empty)";
    write!(out, "{label}").context("write empty explorer")?;
    write_padding(out, display_width(label), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn draw_error(
    out: &mut Surface,
    theme: &Theme,
    panel: &ExplorerPanel<'_>,
    error: &str,
) -> Result<()> {
    let row = panel
        .rect
        .y
        .saturating_add(panel.rect.height.saturating_sub(1));
    line_start(out, panel.rect.x, row)?;
    out.set_foreground(theme.diagnostic_error);
    let error = truncate_to_display_width(error, panel.rect.width);
    write!(out, "{error}").context("write explorer error")?;
    write_padding(out, display_width(error), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn line_start(out: &mut Surface, x: usize, y: usize) -> Result<()> {
    out.move_to(x, y);
    Ok(())
}

fn blank_line(out: &mut Surface, x: usize, y: usize, width: usize) -> Result<()> {
    line_start(out, x, y)?;
    write_padding(out, 0, width)
}

#[cfg(test)]
mod tests;
