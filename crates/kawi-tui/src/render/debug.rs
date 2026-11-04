use anyhow::{Context, Result};
use std::io::Write;

use super::paint;
use super::picker::write_padding;
use crate::frame::{DebugPanel, DebugRow, DebugRowKind};
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::{MatchColors, Theme};

pub(super) fn draw_debug_panel(out: &mut Surface, frame: &crate::frame::Frame) -> Result<()> {
    let Some(panel) = frame.debug.as_ref() else {
        return Ok(());
    };
    draw_header(out, frame.theme, panel)?;
    draw_rows(out, frame.theme, panel)
}

fn draw_header(out: &mut Surface, theme: &Theme, panel: &DebugPanel<'_>) -> Result<()> {
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
    write!(out, "{header}").context("write debug header")?;
    write_padding(out, display_width(header), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn draw_rows(out: &mut Surface, theme: &Theme, panel: &DebugPanel<'_>) -> Result<()> {
    for offset in 0..panel.rect.height.saturating_sub(1) {
        let index = panel.offset.saturating_add(offset);
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        match panel.rows.get(index) {
            Some(entry) => draw_row(out, theme, panel, entry, index, row)?,
            None => blank_line(out, panel.rect.x, row, panel.rect.width)?,
        }
    }
    if panel.rows.is_empty() {
        draw_empty(out, theme, panel)?;
    }
    Ok(())
}

fn draw_row(
    out: &mut Surface,
    theme: &Theme,
    panel: &DebugPanel<'_>,
    row: &DebugRow,
    index: usize,
    y: usize,
) -> Result<()> {
    line_start(out, panel.rect.x, y)?;
    if panel.selected == Some(index) {
        paint::apply_match(out, &theme.selection);
    } else {
        paint::apply_base(out, theme);
        apply_color(out, theme, row.kind);
    }
    let label = row_label(row);
    let label = truncate_to_display_width(&label, panel.rect.width);
    write!(out, "{label}").context("write debug row")?;
    write_padding(out, display_width(label), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn row_label(row: &DebugRow) -> String {
    let indent = "  ".repeat(row.depth);
    format!("{indent}{}{}", marker(row.kind), row.label)
}

fn marker(kind: DebugRowKind) -> &'static str {
    match kind {
        DebugRowKind::Section => "",
        DebugRowKind::Thread { current: true } | DebugRowKind::Frame { current: true } => "> ",
        DebugRowKind::Thread { current: false } | DebugRowKind::Frame { current: false } => "  ",
        DebugRowKind::Variable {
            expandable: true,
            expanded: true,
        } => "- ",
        DebugRowKind::Variable {
            expandable: true,
            expanded: false,
        } => "+ ",
        DebugRowKind::Variable {
            expandable: false, ..
        } => "  ",
        DebugRowKind::Watch => "= ",
        DebugRowKind::Breakpoint => "B ",
        DebugRowKind::FunctionBreakpoint => "F ",
        DebugRowKind::ExceptionFilter { enabled: true } => "[x] ",
        DebugRowKind::ExceptionFilter { enabled: false } => "[ ] ",
        DebugRowKind::Console { .. } => "  ",
        DebugRowKind::Action => "> ",
    }
}

fn apply_color(out: &mut Surface, theme: &Theme, kind: DebugRowKind) {
    let color = match kind {
        DebugRowKind::Section => theme
            .syntax
            .named("type")
            .and_then(|style| style.foreground)
            .unwrap_or(theme.status.foreground),
        DebugRowKind::Thread { current: true } | DebugRowKind::Frame { current: true } => {
            theme.status.foreground
        }
        DebugRowKind::Breakpoint | DebugRowKind::FunctionBreakpoint => theme.diagnostic_warning,
        DebugRowKind::ExceptionFilter { enabled: true } => theme.git_add,
        DebugRowKind::Console { error: true } => theme.diagnostic_error,
        DebugRowKind::Action => theme.status.foreground,
        _ => theme.base.foreground,
    };
    out.set_foreground(color);
}

fn draw_empty(out: &mut Surface, theme: &Theme, panel: &DebugPanel<'_>) -> Result<()> {
    if panel.rect.height < 2 {
        return Ok(());
    }
    line_start(out, panel.rect.x, panel.rect.y.saturating_add(1))?;
    out.set_foreground(theme.non_text);
    let label = truncate_to_display_width("No debug session", panel.rect.width);
    write!(out, "{label}").context("write debug empty")?;
    write_padding(out, display_width(label), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn blank_line(out: &mut Surface, x: usize, y: usize, width: usize) -> Result<()> {
    line_start(out, x, y)?;
    write_padding(out, 0, width)
}

fn line_start(out: &mut Surface, x: usize, y: usize) -> Result<()> {
    out.move_to(x, y);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_distinguish_tree_state() {
        assert_eq!(
            marker(DebugRowKind::Variable {
                expandable: true,
                expanded: false,
            }),
            "+ "
        );
        assert_eq!(
            marker(DebugRowKind::Variable {
                expandable: true,
                expanded: true,
            }),
            "- "
        );
        assert_eq!(marker(DebugRowKind::Thread { current: true }), "> ");
        assert_eq!(marker(DebugRowKind::Breakpoint), "B ");
        assert_eq!(
            marker(DebugRowKind::ExceptionFilter { enabled: true }),
            "[x] "
        );
    }

    #[test]
    fn labels_apply_depth_and_kind() {
        let row = DebugRow {
            depth: 2,
            label: "x: int = 42".to_string(),
            kind: DebugRowKind::Watch,
        };
        assert_eq!(row_label(&row), "    = x: int = 42");
    }
}
