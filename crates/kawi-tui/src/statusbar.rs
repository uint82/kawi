use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

pub struct StatusInfo<'a> {
    pub filename: &'a str,
    pub dirty: bool,
    pub row: usize,
    pub col: usize,
    pub quit_armed: bool,
}

pub fn draw_status(out: &mut Stdout, info: &StatusInfo, width: usize, y: usize) -> Result<()> {
    let line = build_line(info, width);
    write_status_line(out, &line, y)?;
    Ok(())
}

fn build_line(info: &StatusInfo, width: usize) -> String {
    join_padded(&left_text(info), &right_text(info), width)
}

fn left_text(info: &StatusInfo) -> String {
    if info.quit_armed {
        return "Press Ctrl-Q again to quit".to_string();
    }
    if info.dirty {
        format!("{} [+]", info.filename)
    } else {
        info.filename.to_string()
    }
}

fn right_text(info: &StatusInfo) -> String {
    format!(
        "{}:{}",
        info.row.saturating_add(1),
        info.col.saturating_add(1)
    )
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

    #[test]
    fn shows_dirty_mark() {
        let info = StatusInfo {
            filename: "a.txt",
            dirty: true,
            row: 0,
            col: 0,
            quit_armed: false,
        };
        assert!(build_line(&info, 40).contains("[+]"));
    }

    #[test]
    fn shows_quit_warning_when_armed() {
        let info = StatusInfo {
            filename: "a.txt",
            dirty: true,
            row: 0,
            col: 0,
            quit_armed: true,
        };
        assert!(build_line(&info, 40).contains("Ctrl-Q"));
    }

    #[test]
    fn pads_to_width() {
        let info = StatusInfo {
            filename: "a.txt",
            dirty: false,
            row: 0,
            col: 0,
            quit_armed: false,
        };
        assert_eq!(build_line(&info, 20).chars().count(), 20);
    }
}
