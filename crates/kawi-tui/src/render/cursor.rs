use anyhow::{Context, Result};
use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    execute,
};
use std::io::Stdout;

use kawi_editor::Viewport;

use crate::frame::{CursorStyle, Frame, PromptMenu};

use super::furniture::{char_len, saturate_u16};
use super::pager::PAGER_PROMPT;

pub(super) fn apply_cursor_style(out: &mut Stdout, style: CursorStyle) -> Result<()> {
    execute!(out, cursor_style(style)).context("set cursor style")?;
    Ok(())
}

fn cursor_style(style: CursorStyle) -> SetCursorStyle {
    match style {
        CursorStyle::Block => SetCursorStyle::SteadyBlock,
        CursorStyle::Bar => SetCursorStyle::SteadyBar,
        CursorStyle::Underline => SetCursorStyle::SteadyUnderScore,
    }
}

pub(super) fn place_cursor(out: &mut Stdout, frame: &Frame) -> Result<()> {
    if frame.pager.is_some() {
        return place_pager_cursor(out, frame);
    }
    if let Some(col) = prompt_cursor_col_with_prompt(
        frame.prompt.as_ref(),
        frame.command,
        frame.search,
        frame.viewport.width(),
        frame.showcmd.as_deref(),
    ) {
        return place_prompt_cursor(out, col, frame.viewport);
    }
    let Some(pane) = frame.panes.iter().find(|pane| pane.active) else {
        return Ok(());
    };
    let height = pane.rect.height.saturating_sub(1);
    let Some(screen_row) = pane.cursor_screen_row else {
        return Ok(());
    };
    if screen_row >= height {
        return Ok(());
    }
    let x = pane
        .rect
        .x
        .saturating_add(pane.cursor_col.min(pane.rect.width.saturating_sub(1)));
    let y = pane.rect.y.saturating_add(screen_row);
    execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("place cursor")?;
    Ok(())
}

fn place_pager_cursor(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let width = frame.viewport.width().max(1);
    let x = PAGER_PROMPT.chars().count().min(width - 1);
    execute!(
        out,
        MoveTo(
            saturate_u16(x),
            saturate_u16(frame.viewport.height().saturating_add(1))
        )
    )
    .context("place pager cursor")?;
    Ok(())
}

fn place_prompt_cursor(out: &mut Stdout, col: usize, viewport: &Viewport) -> Result<()> {
    execute!(
        out,
        MoveTo(
            saturate_u16(col),
            saturate_u16(viewport.height().saturating_add(1))
        )
    )
    .context("place prompt cursor")?;
    Ok(())
}

fn prompt_cursor_col_with_prompt(
    prompt: Option<&PromptMenu<'_>>,
    command: Option<&str>,
    search: Option<&str>,
    width: usize,
    showcmd: Option<&str>,
) -> Option<usize> {
    if let Some(prompt) = prompt {
        let prefix = char_len(prompt.label).saturating_add(2);
        return Some(
            prefix
                .saturating_add(prompt.cursor)
                .min(prompt_edge(width, showcmd)),
        );
    }
    let text = prompt_text(command, search)?;
    Some(prompt_col(text, width, showcmd))
}

fn prompt_text<'a>(command: Option<&'a str>, search: Option<&'a str>) -> Option<&'a str> {
    match (command, search) {
        (Some(text), _) => Some(text),
        (None, Some(text)) => Some(text),
        (None, None) => None,
    }
}

fn prompt_col(text: &str, width: usize, showcmd: Option<&str>) -> usize {
    prompt_home()
        .saturating_add(char_len(text))
        .min(prompt_edge(width, showcmd))
}

fn prompt_home() -> usize {
    1
}

fn prompt_edge(width: usize, showcmd: Option<&str>) -> usize {
    width
        .saturating_sub(char_len(showcmd.unwrap_or("")))
        .saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_style_follows_frame_kind() {
        assert_eq!(
            cursor_style(CursorStyle::Block),
            SetCursorStyle::SteadyBlock
        );
        assert_eq!(cursor_style(CursorStyle::Bar), SetCursorStyle::SteadyBar);
        assert_eq!(
            cursor_style(CursorStyle::Underline),
            SetCursorStyle::SteadyUnderScore
        );
    }

    #[test]
    fn prompt_cursor_follows_the_prompt_value() {
        let prompt = PromptMenu {
            label: "rename",
            value: "value",
            cursor: 5,
        };
        assert_eq!(
            prompt_cursor_col_with_prompt(Some(&prompt), Some("w"), None, 30, None),
            Some(13)
        );
    }

    #[test]
    fn prompt_cursor_absent_without_modes() {
        assert_eq!(
            prompt_cursor_col_with_prompt(None, None, None, 20, None),
            None
        );
    }

    #[test]
    fn prompt_cursor_sits_after_prefix_when_empty() {
        assert_eq!(
            prompt_cursor_col_with_prompt(None, Some(""), None, 20, None),
            Some(1)
        );
        assert_eq!(
            prompt_cursor_col_with_prompt(None, None, Some(""), 20, None),
            Some(1)
        );
    }

    #[test]
    fn prompt_cursor_follows_typed_text() {
        assert_eq!(
            prompt_cursor_col_with_prompt(None, None, Some("ar"), 20, None),
            Some(3)
        );
        assert_eq!(
            prompt_cursor_col_with_prompt(None, Some("wq"), None, 20, None),
            Some(3)
        );
    }

    #[test]
    fn prompt_cursor_counts_chars_not_bytes() {
        assert_eq!(
            prompt_cursor_col_with_prompt(None, None, Some("éx"), 20, None),
            Some(3)
        );
    }

    #[test]
    fn prompt_cursor_prefers_command() {
        assert_eq!(
            prompt_cursor_col_with_prompt(None, Some("w"), Some("ar"), 20, None),
            Some(2)
        );
    }

    #[test]
    fn prompt_cursor_clamps_to_visible_row() {
        assert_eq!(
            prompt_cursor_col_with_prompt(None, None, Some("abcdef"), 5, None),
            Some(4)
        );
        assert_eq!(
            prompt_cursor_col_with_prompt(None, Some(""), None, 5, Some("dd")),
            Some(1)
        );
    }
}
