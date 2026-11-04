use anyhow::{Context, Result};
use std::io::Write;

use super::paint;
use super::picker::write_padding;
use crate::frame::{PluginPanel, PluginPanelLine};
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::{MatchColors, Theme};

pub(super) fn draw_plugin_panel(out: &mut Surface, frame: &crate::frame::Frame) -> Result<()> {
    let Some(panel) = frame.plugin_panel.as_ref() else {
        return Ok(());
    };
    draw_header(out, frame.theme, panel)?;
    draw_lines(out, frame.theme, panel)
}

fn draw_header(out: &mut Surface, theme: &Theme, panel: &PluginPanel<'_>) -> Result<()> {
    prepare_line(out, panel.rect.x, panel.rect.y);
    let colors = if panel.focused {
        theme.status.clone()
    } else {
        MatchColors {
            foreground: theme.non_text,
            background: theme.base.background,
        }
    };
    paint::apply_match(out, &colors);
    let header = header_text(panel);
    let header = truncate_to_display_width(&header, panel.rect.width);
    write!(out, "{header}").context("write plugin panel header")?;
    write_padding(out, display_width(header), panel.rect.width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn draw_lines(out: &mut Surface, theme: &Theme, panel: &PluginPanel<'_>) -> Result<()> {
    for offset in 0..panel.rect.height.saturating_sub(1) {
        let index = panel.offset.saturating_add(offset);
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        prepare_line(out, panel.rect.x, row);
        let Some(line) = panel.lines.get(index) else {
            paint::apply_base(out, theme);
            write_padding(out, 0, panel.rect.width)?;
            continue;
        };
        paint::apply_base(out, theme);
        apply_style(out, theme, line);
        let text = truncate_to_display_width(&line.text, panel.rect.width);
        write!(out, "{text}").context("write plugin panel line")?;
        write_padding(out, display_width(text), panel.rect.width)?;
        paint::restore_base(out, theme);
    }
    Ok(())
}

fn apply_style(out: &mut Surface, theme: &Theme, line: &PluginPanelLine) {
    let Some(style) = line.style else {
        return;
    };
    out.set_foreground(theme.decoration_colors(style).foreground);
}

fn header_text(panel: &PluginPanel<'_>) -> String {
    match (panel.title.is_empty(), panel.status.is_empty()) {
        (true, true) => "Panel".to_string(),
        (true, false) => panel.status.to_string(),
        (false, true) => panel.title.to_string(),
        (false, false) => format!("{}  {}", panel.title, panel.status),
    }
}

fn prepare_line(out: &mut Surface, x: usize, y: usize) {
    out.move_to(x, y);
    out.clear_line();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(title: &'static str, status: &'static str) -> PluginPanel<'static> {
        PluginPanel {
            rect: crate::frame::Rect::new(0, 0, 10, 2),
            title,
            status,
            focused: false,
            lines: &[],
            offset: 0,
        }
    }

    #[test]
    fn header_combines_title_and_status() {
        assert_eq!(header_text(&panel("Index", "done")), "Index  done");
        assert_eq!(header_text(&panel("Index", "")), "Index");
        assert_eq!(header_text(&panel("", "done")), "done");
        assert_eq!(header_text(&panel("", "")), "Panel");
    }
}
