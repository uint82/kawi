use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::{Frame, PromptMenu};
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use kawi_editor::Viewport;

use super::furniture::erase_row_tail;

pub(super) fn draw_message_row(out: &mut Surface, frame: &Frame) -> Result<()> {
    let layout = layout_message_row_with_prompt(
        frame.prompt.as_ref(),
        MessageParts {
            command: frame.command,
            search: frame.search,
            delimiter: frame.search_delimiter,
            message: frame.message,
            search_status: frame.search_status,
        },
        frame.showcmd.as_deref(),
        frame.viewport.width(),
    );
    move_to_message_row(out, frame.viewport)?;
    write!(out, "{}", layout.line).context("write message row")?;
    erase_row_tail(out)?;
    if let Some(column) = layout.showcmd_col {
        move_to_showcmd(out, column, frame.viewport)?;
        write!(out, "{}", layout.showcmd).context("write showcmd")?;
    }
    Ok(())
}

struct MessageLayout {
    line: String,
    showcmd: String,
    showcmd_col: Option<usize>,
}

struct MessageParts<'a> {
    command: Option<&'a str>,
    search: Option<&'a str>,
    delimiter: char,
    message: Option<&'a str>,
    search_status: Option<&'a str>,
}

#[cfg(test)]
fn layout_message_row(
    command: Option<&str>,
    search: Option<&str>,
    search_delimiter: char,
    message: Option<&str>,
    search_status: Option<&str>,
    showcmd: Option<&str>,
    width: usize,
) -> MessageLayout {
    layout_message_row_with_prompt(
        None,
        MessageParts {
            command,
            search,
            delimiter: search_delimiter,
            message,
            search_status,
        },
        showcmd,
        width,
    )
}

fn layout_message_row_with_prompt(
    prompt: Option<&PromptMenu<'_>>,
    parts: MessageParts<'_>,
    showcmd: Option<&str>,
    width: usize,
) -> MessageLayout {
    let show = showcmd.unwrap_or("");
    let left = message_left_with_prompt(prompt, &parts);
    MessageLayout {
        line: truncate_to_display_width(&left, left_width(width, show)).to_string(),
        showcmd: truncate_to_display_width(show, width).to_string(),
        showcmd_col: showcmd_column(show, width),
    }
}

fn message_left_with_prompt(prompt: Option<&PromptMenu<'_>>, parts: &MessageParts<'_>) -> String {
    if let Some(prompt) = prompt {
        return format!("{}: {}", prompt.label, prompt.value);
    }
    if let Some(text) = parts.command {
        return format!(":{text}");
    }
    if let Some(text) = parts.search {
        return format!("{}{text}", parts.delimiter);
    }
    parts
        .message
        .or(parts.search_status)
        .unwrap_or("")
        .to_string()
}

fn left_width(width: usize, showcmd: &str) -> usize {
    width.saturating_sub(display_width(showcmd))
}

fn showcmd_column(showcmd: &str, width: usize) -> Option<usize> {
    if showcmd.is_empty() {
        return None;
    }
    Some(width.saturating_sub(display_width(showcmd)))
}

fn move_to_showcmd(out: &mut Surface, col: usize, viewport: &Viewport) -> Result<()> {
    out.move_to(col, viewport.height().saturating_add(1));
    Ok(())
}

fn move_to_message_row(out: &mut Surface, viewport: &Viewport) -> Result<()> {
    out.move_to(0, viewport.height().saturating_add(1));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_prefers_prompt_over_command() {
        let prompt = PromptMenu {
            label: "rename",
            value: "value",
            cursor: 5,
        };
        let layout = layout_message_row_with_prompt(
            Some(&prompt),
            MessageParts {
                command: Some("w"),
                search: None,
                delimiter: '/',
                message: Some("saved"),
                search_status: None,
            },
            None,
            20,
        );
        assert_eq!(layout.line, "rename: value");
    }

    #[test]
    fn layout_prefers_command_over_message() {
        let layout = layout_message_row(Some("w"), None, '/', Some("saved"), None, None, 20);
        assert_eq!(layout.line, ":w");
        assert_eq!(layout.showcmd_col, None);
    }

    #[test]
    fn layout_right_aligns_showcmd() {
        let layout = layout_message_row(None, None, '/', Some("saved"), None, Some("d"), 20);
        assert_eq!(layout.line, "saved");
        assert_eq!(layout.showcmd, "d");
        assert_eq!(layout.showcmd_col, Some(19));
    }

    #[test]
    fn layout_prefixes_backward_search_prompt() {
        let layout = layout_message_row(None, Some("foo"), '?', None, None, None, 20);
        assert_eq!(layout.line, "?foo");
    }

    #[test]
    fn layout_truncates_by_display_width() {
        let layout = layout_message_row(
            None,
            None,
            '/',
            Some("\u{4e2d}\u{4e2d}\u{4e2d}"),
            None,
            None,
            4,
        );
        assert_eq!(layout.line, "\u{4e2d}\u{4e2d}");
    }

    #[test]
    fn layout_truncates_left_for_showcmd() {
        let layout = layout_message_row(
            None,
            None,
            '/',
            Some("12345678901234567890"),
            None,
            Some("dd"),
            20,
        );
        assert_eq!(layout.line.chars().count(), 18);
        assert_eq!(layout.showcmd_col, Some(18));
    }

    #[test]
    fn layout_empty_row_has_no_column() {
        let layout = layout_message_row(None, None, '/', None, None, None, 20);
        assert_eq!(layout.line, "");
        assert_eq!(layout.showcmd_col, None);
    }

    #[test]
    fn layout_prefers_search_over_message() {
        let layout = layout_message_row(None, Some("ar"), '/', Some("saved"), None, None, 20);
        assert_eq!(layout.line, "/ar");
    }

    #[test]
    fn layout_shows_search_status_when_idle() {
        let layout = layout_message_row(None, None, '/', None, Some("/ar [1/3]"), None, 20);
        assert_eq!(layout.line, "/ar [1/3]");
    }

    #[test]
    fn layout_prefers_message_over_search_status() {
        let layout =
            layout_message_row(None, None, '/', Some("saved"), Some("/ar [1/3]"), None, 20);
        assert_eq!(layout.line, "saved");
    }
}
