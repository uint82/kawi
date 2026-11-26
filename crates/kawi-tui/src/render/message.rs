use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::{Frame, MessageSeverity, PromptMenu};
use crate::paint;
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
    let is_notice = is_notice(frame);
    if is_notice {
        apply_severity(out, frame);
    }
    move_to_message_row(out, frame.viewport)?;
    write!(out, "{}", layout.line).context("write message row")?;
    if is_notice {
        paint::clear_decorations(out);
        paint::restore_base(out, frame.theme);
    }
    erase_row_tail(out)?;
    if let Some(column) = layout.showcmd_col {
        move_to_showcmd(out, column, frame.viewport)?;
        write!(out, "{}", layout.showcmd).context("write showcmd")?;
    }
    Ok(())
}

fn is_notice(frame: &Frame<'_>) -> bool {
    frame.message.is_some()
        && frame.prompt.is_none()
        && frame.command.is_none()
        && frame.search.is_none()
}

fn apply_severity(out: &mut Surface, frame: &Frame<'_>) {
    match frame.message_severity {
        MessageSeverity::Error => out.set_foreground(frame.theme.diagnostic_error),
        MessageSeverity::Warning => out.set_foreground(frame.theme.diagnostic_warning),
        MessageSeverity::Success => out.set_bold(true),
        MessageSeverity::Information => {}
    }
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
mod tests;
