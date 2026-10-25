use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

use crate::paint;
use crate::theme::{MatchColors, Theme};

#[derive(Clone, Copy)]
pub struct GitStatusInfo<'a> {
    pub branch: &'a str,
    pub operation: Option<&'a str>,
    pub ahead: usize,
    pub behind: usize,
}

pub struct StatusInfo<'a> {
    pub filename: &'a str,
    pub dirty: bool,
    pub row: usize,
    pub col: usize,
    pub virtcol: usize,
    pub line_empty: bool,
    pub empty_buffer: bool,
    pub mode: &'a str,
    pub lsp: Option<&'a str>,
    pub external: Option<&'a str>,
    pub git: Option<GitStatusInfo<'a>>,
}

pub fn draw_status(
    out: &mut Stdout,
    info: &StatusInfo,
    x: usize,
    y: usize,
    width: usize,
    active: bool,
    theme: &Theme,
) -> Result<()> {
    let line = build_line(info, width);
    let colors = status_colors(theme, active);
    paint::apply_match(out, &colors)?;
    write_status_line(out, &line, x, y)?;
    paint::restore_base(out, theme)?;
    Ok(())
}

fn status_colors(theme: &Theme, active: bool) -> MatchColors {
    if active {
        return MatchColors {
            foreground: theme.status.foreground,
            background: theme.status.background,
        };
    }
    MatchColors {
        foreground: theme.line_number,
        background: theme.base.background,
    }
}

fn build_line(info: &StatusInfo, width: usize) -> String {
    join_padded(&left_text(info), &right_text(info), width)
}

fn left_text(info: &StatusInfo) -> String {
    let mut text = format!("-- {} -- {}", info.mode, file_part(info));
    if let Some(lsp) = info.lsp {
        text.push_str("  ");
        text.push_str(lsp);
    }
    text
}

fn file_part(info: &StatusInfo) -> String {
    let mut text = info.filename.to_string();
    if info.dirty {
        text.push_str(" [+]");
    }
    if let Some(external) = info.external {
        text.push_str(" [");
        text.push_str(external);
        text.push(']');
    }
    text
}

fn right_text(info: &StatusInfo) -> String {
    let mut text = String::new();
    if let Some(git) = info.git.as_ref() {
        text.push_str(&git_text(git));
        text.push_str("  ");
    }
    text.push_str(&position_text(info));
    text
}

fn git_text(git: &GitStatusInfo<'_>) -> String {
    let mut text = String::new();
    if let Some(operation) = git.operation {
        text.push_str(operation);
        text.push(' ');
    }
    text.push_str(git.branch);
    if git.ahead > 0 {
        text.push_str(&format!(" +{}", git.ahead));
    }
    if git.behind > 0 {
        text.push_str(&format!(" -{}", git.behind));
    }
    text
}

fn position_text(info: &StatusInfo) -> String {
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

fn write_status_line(out: &mut Stdout, text: &str, x: usize, y: usize) -> Result<()> {
    execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("move to status row")?;
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
            mode,
            lsp: None,
            external: None,
            git: None,
        }
    }

    #[test]
    fn shows_dirty_mark() {
        assert!(build_line(&info("a.txt", true, "NORMAL"), 40).contains("[+]"));
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
    fn shows_external_file_state() {
        let mut status = info("a.txt", false, "NORMAL");
        status.external = Some("changed");
        assert!(build_line(&status, 40).contains("[changed]"));
        status.external = Some("deleted");
        assert!(build_line(&status, 40).contains("[deleted]"));
    }

    #[test]
    fn shows_lsp_status() {
        let mut status = info("a.rs", false, "NORMAL");
        status.lsp = Some("lsp: rust-analyzer not found in PATH");
        assert!(build_line(&status, 80).contains("lsp: rust-analyzer not found in PATH"));
    }

    #[test]
    fn shows_git_branch_and_divergence() {
        let mut status = info("a.txt", false, "NORMAL");
        status.git = Some(GitStatusInfo {
            branch: "main",
            operation: None,
            ahead: 2,
            behind: 1,
        });
        let line = build_line(&status, 60);
        assert!(line.contains("main +2 -1"));
    }

    #[test]
    fn shows_git_operation_and_detached_branch() {
        let mut status = info("a.txt", false, "NORMAL");
        status.git = Some(GitStatusInfo {
            branch: "abc1234",
            operation: Some("REBASING"),
            ahead: 0,
            behind: 0,
        });
        let line = build_line(&status, 60);
        assert!(line.contains("REBASING abc1234"));
    }

    #[test]
    fn hides_git_when_no_repository_is_known() {
        let line = build_line(&info("a.txt", false, "NORMAL"), 60);
        assert!(!line.contains("main"));
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
