use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

use crate::paint;
use crate::theme::Theme;

pub struct StatusInfo<'a> {
    pub filename: &'a str,
    pub dirty: bool,
    pub row: usize,
    pub col: usize,
    pub virtcol: usize,
    pub line_empty: bool,
    pub empty_buffer: bool,
    pub quit_armed: bool,
    pub mode: &'a str,
}

pub fn draw_status(
    out: &mut Stdout,
    info: &StatusInfo,
    width: usize,
    y: usize,
    theme: &Theme,
) -> Result<()> {
    let line = build_line(info, width);
    paint::apply_match(out, &theme.status)?;
    write_status_line(out, &line, y)?;
    paint::restore_base(out, theme)?;
    Ok(())
}

fn build_line(info: &StatusInfo, width: usize) -> String {
    join_padded(&left_text(info), &right_text(info), width)
}

fn left_text(info: &StatusInfo) -> String {
    if info.quit_armed {
        return "Press :q again to quit".to_string();
    }
    format!("-- {} -- {}", info.mode, file_part(info))
}

fn file_part(info: &StatusInfo) -> String {
    if info.dirty {
        format!("{} [+]", info.filename)
    } else {
        info.filename.to_string()
    }
}

fn right_text(info: &StatusInfo) -> String {
    let line = if info.empty_buffer {
        0
    } else {
        info.row.saturating_add(1)
    };
    let col = if info.line_empty {
        0
    } else {
        info.col.saturating_add(1)
    };
    let vcol = info.virtcol.saturating_add(1);
    if col == vcol {
        format!("{line}:{col}")
    } else {
        format!("{line}:{col}-{vcol}")
    }
}

fn join_padded(left: &str, right: &str, width: usize) -> String {
    let gap = width.saturating_sub(char_len(left).saturating_add(char_len(right)));
    let mut line = String::with_capacity(width);
    line.push_str(left);
    line.push_str(&" ".repeat(gap));
    line.push_str(right);
    truncate_to_width(&line, width)
}

fn write_status_line(out: &mut Stdout, text: &str, y: usize) -> Result<()> {
    execute!(out, MoveTo(0, saturate_u16(y))).context("move to status row")?;
    write!(out, "{text}").context("write status")?;
    Ok(())
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn saturate_u16(v: usize) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info<'a>(filename: &'a str, dirty: bool, mode: &'a str) -> StatusInfo<'a> {
        StatusInfo {
            filename,
            dirty,
            row: 0,
            col: 0,
            virtcol: 0,
            line_empty: false,
            empty_buffer: false,
            quit_armed: false,
            mode,
        }
    }

    #[test]
    fn shows_dirty_mark() {
        assert!(build_line(&info("a.txt", true, "NORMAL"), 40).contains("[+]"));
    }

    #[test]
    fn shows_quit_warning_when_armed() {
        let mut status = info("a.txt", true, "NORMAL");
        status.quit_armed = true;
        assert!(build_line(&status, 40).contains(":q again"));
    }

    #[test]
    fn pads_to_width() {
        assert_eq!(
            build_line(&info("a.txt", false, "INSERT"), 20)
                .chars()
                .count(),
            20
        );
    }

    #[test]
    fn shows_mode() {
        assert!(build_line(&info("a.txt", false, "NORMAL"), 40).contains("NORMAL"));
    }

    #[test]
    fn keeps_status_in_command_mode() {
        let line = build_line(&info("a.txt", false, "COMMAND"), 40);
        assert!(line.contains("COMMAND"));
        assert!(line.contains("a.txt"));
    }

    #[test]
    fn position_uses_ruler_format() {
        let mut status = info("a.txt", false, "NORMAL");
        status.row = 2;
        status.col = 4;
        status.virtcol = 4;
        assert!(build_line(&status, 40).contains("3:5"));
    }

    #[test]
    fn position_shows_virtual_column_range_on_tab() {
        let mut status = info("a.txt", false, "NORMAL");
        status.col = 0;
        status.virtcol = 7;
        assert!(build_line(&status, 40).contains("1:1-8"));
    }

    #[test]
    fn position_on_empty_line_starts_at_zero() {
        let mut status = info("a.txt", false, "NORMAL");
        status.line_empty = true;
        assert!(build_line(&status, 40).contains("1:0-1"));
    }

    #[test]
    fn position_on_empty_buffer_starts_at_zero() {
        let mut status = info("a.txt", false, "NORMAL");
        status.empty_buffer = true;
        status.line_empty = true;
        assert!(build_line(&status, 40).contains("0:0-1"));
    }
}
