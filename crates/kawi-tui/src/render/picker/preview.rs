use anyhow::{Context, Result};
use std::io::Write;

use super::super::highlight::{paint_span_at, syntax_highlight_spans, SpanView};
use super::super::special_key::{draw_special_keys, SpecialKeyArea, SpecialKeyDraw};
use super::write_padding;
use crate::display;
use crate::frame::{Frame, PickerPreview, Rect};
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, sanitize_text, truncate_to_display_width};
use kawi_editor::Viewport;

pub(super) fn draw_preview(
    out: &mut Surface,
    frame: &Frame,
    preview: Option<&PickerPreview<'_>>,
    rect: Option<Rect>,
) -> Result<()> {
    let (Some(preview), Some(rect)) = (preview, rect) else {
        return Ok(());
    };
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    draw_title(out, frame, preview, rect)?;
    if rect.height < 2 || preview.lines.is_empty() {
        return Ok(());
    }
    let body = Rect::new(
        rect.x,
        rect.y.saturating_add(1),
        rect.width,
        rect.height.saturating_sub(1),
    );
    let first = preview_first_line(preview, body.height);
    let view = PreviewView {
        preview,
        lines: &preview.lines[first..],
        first,
        rect: body,
        gutter_width: preview_gutter_width(preview, first),
    };
    draw_lines(out, frame, &view)?;
    draw_highlights(out, frame, &view)
}

fn draw_title(
    out: &mut Surface,
    frame: &Frame,
    preview: &PickerPreview<'_>,
    rect: Rect,
) -> Result<()> {
    out.move_to(rect.x, rect.y);
    out.set_foreground(frame.theme.line_number);
    let title = sanitize_text(preview.title);
    let title = truncate_to_display_width(&title, rect.width);
    write!(out, "{title}").context("write preview title")?;
    write_padding(out, display_width(title), rect.width)
}

struct PreviewView<'a, 'b> {
    preview: &'b PickerPreview<'a>,
    lines: &'a [String],
    first: usize,
    rect: Rect,
    gutter_width: usize,
}

fn preview_first_line(preview: &PickerPreview<'_>, body_height: usize) -> usize {
    let focus = preview.focus_line.unwrap_or(preview.first_line);
    let offset = focus.saturating_sub(preview.first_line);
    offset
        .saturating_sub(body_height / 2)
        .min(preview.lines.len().saturating_sub(body_height))
}

fn preview_line_number(preview: &PickerPreview<'_>, first: usize, offset: usize) -> usize {
    preview
        .first_line
        .saturating_add(first)
        .saturating_add(offset)
        .saturating_add(1)
}

fn preview_gutter_width(preview: &PickerPreview<'_>, first: usize) -> usize {
    let last = preview
        .first_line
        .saturating_add(first)
        .saturating_add(preview.lines.len());
    last.to_string().len().saturating_add(1)
}

fn draw_lines(out: &mut Surface, frame: &Frame, view: &PreviewView<'_, '_>) -> Result<()> {
    let text_width = view.rect.width.saturating_sub(view.gutter_width);
    for (offset, line) in view.lines.iter().take(view.rect.height).enumerate() {
        let row = view.rect.y.saturating_add(offset);
        out.move_to(view.rect.x, row);
        write_line_number(out, frame, view, offset)?;
        paint::apply_colors(
            out,
            frame.theme.base.foreground,
            frame.theme.float_background,
        );
        let text = display::slice_screen(line, 0, text_width, view.preview.tabstop);
        write!(out, "{text}").context("write preview line")?;
        write_padding(out, display_width(&text), text_width)?;
    }
    Ok(())
}

fn write_line_number(
    out: &mut Surface,
    frame: &Frame,
    view: &PreviewView<'_, '_>,
    offset: usize,
) -> Result<()> {
    let number = preview_line_number(view.preview, view.first, offset);
    out.set_colors(frame.theme.line_number, frame.theme.float_background);
    let width = view.gutter_width.saturating_sub(1);
    write!(out, "{number:>width$} ").context("write preview line number")
}

fn draw_highlights(out: &mut Surface, frame: &Frame, view: &PreviewView<'_, '_>) -> Result<()> {
    let text_width = view.rect.width.saturating_sub(view.gutter_width);
    let top = view.preview.first_line.saturating_add(view.first);
    let mut viewport = Viewport::new(view.rect.height, text_width);
    viewport.place_top(top, view.preview.total_lines.max(1));
    let span_view = SpanView {
        window: view.lines,
        rows: std::borrow::Cow::Owned((top..top.saturating_add(view.lines.len())).collect()),
        viewport: &viewport,
        gutter: view.gutter_width,
        leftcol: 0,
        tabstop: view.preview.tabstop,
    };
    for span in syntax_highlight_spans(view.preview.syntax_spans, &span_view) {
        paint_preview_span(out, frame, view, &span)?;
    }
    draw_special_keys(
        out,
        SpecialKeyDraw::from_area(
            view.lines,
            SpecialKeyArea {
                x: view.rect.x.saturating_add(view.gutter_width),
                y: view.rect.y,
                width: text_width,
                height: view.rect.height,
                leftcol: 0,
                tabstop: view.preview.tabstop,
            },
            frame.theme.special_key,
        ),
    );
    if let Some(focus) = view.preview.focus_line {
        if let Some(offset) = focus.checked_sub(top) {
            if let Some(line) = view.lines.get(offset) {
                paint_focus_line(out, frame, view, line, offset, text_width)?;
            }
        }
    }
    Ok(())
}

fn paint_focus_line(
    out: &mut Surface,
    frame: &Frame,
    view: &PreviewView<'_, '_>,
    line: &str,
    offset: usize,
    width: usize,
) -> Result<()> {
    out.move_to(
        view.rect.x.saturating_add(view.gutter_width),
        view.rect.y.saturating_add(offset),
    );
    paint::apply_match(out, &frame.theme.selection);
    let text = display::slice_screen(line, 0, width, view.preview.tabstop);
    write!(out, "{text}").context("write focused preview line")?;
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
    );
    Ok(())
}

fn paint_preview_span(
    out: &mut Surface,
    frame: &Frame,
    view: &PreviewView<'_, '_>,
    span: &super::super::highlight::HighlightSpan,
) -> Result<()> {
    paint_span_at(
        out,
        span,
        view.rect.x.saturating_add(view.gutter_width),
        view.rect.y,
        frame.theme,
        frame.theme.float_background,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::frame::CursorStyle;
    use crate::statusbar::StatusBarConfig;
    use crate::theme::Theme;

    fn preview(
        lines: &[String],
        first_line: usize,
        focus_line: Option<usize>,
    ) -> PickerPreview<'_> {
        PickerPreview {
            title: "file.rs",
            lines,
            first_line,
            total_lines: 100,
            focus_line,
            syntax_spans: &[],
            tabstop: 8,
        }
    }

    #[test]
    fn line_numbers_include_the_scroll_offset() {
        let lines = vec!["line".to_string()];
        let preview = preview(&lines, 10, None);
        assert_eq!(preview_line_number(&preview, 5, 2), 18);
    }

    #[test]
    fn centers_the_focus_line() {
        let lines = vec!["line".to_string(); 100];
        let preview = preview(&lines, 0, Some(50));
        assert_eq!(preview_first_line(&preview, 20), 40);
    }

    #[test]
    fn clamps_the_scroll_to_the_visible_lines() {
        let lines = vec!["line".to_string(); 30];
        let preview = preview(&lines, 0, Some(25));
        assert_eq!(preview_first_line(&preview, 20), 10);
    }

    #[test]
    fn colors_nonprintable_placeholders() {
        let theme = Theme::default();
        let viewport = Viewport::new(2, 20);
        let statusline = StatusBarConfig::default();
        let frame = Frame {
            panes: &[],
            separators: &[],
            pager: None,
            completion: None,
            overlay: None,
            picker: None,
            replace: None,
            explorer: None,
            debug: None,
            task: None,
            plugin_panel: None,
            terminal: None,
            viewport: &viewport,
            message: None,
            prompt: None,
            command: None,
            search: None,
            search_delimiter: '/',
            search_status: None,
            showcmd: None,
            cursor_style: CursorStyle::Block,
            theme: &theme,
            statusline: &statusline,
        };
        let lines = vec!["a\u{feff}b".to_string()];
        let preview = preview(&lines, 0, None);
        let rect = Rect::new(0, 0, 16, 2);
        let mut out = Surface::new(16, 2, theme.base.foreground, theme.base.background);
        draw_preview(&mut out, &frame, Some(&preview), Some(rect)).expect("draw preview");
        let start = preview_gutter_width(&preview, 0).saturating_add(1);
        for x in start..start + 6 {
            let style = out.cell(x, 1).expect("placeholder cell").style();
            assert_eq!(style.foreground(), theme.special_key);
        }
    }
}
