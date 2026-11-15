use kawi_plugin::{DecorationStyle, OverlayAnchor, OverlayLine};
use kawi_tui::frame::Rect;
use kawi_tui::markdown::{MarkdownLine, MarkdownSpan, MarkdownStyle};
use kawi_tui::overlay::{self as placement, PlacementInput, Size};

use super::super::super::overlay::{OverlayData, PanelData, PluginPanelView};
use super::super::style::frame_style;
use super::state::OverlayView;
use crate::app::App;

impl App {
    pub(crate) fn plugin_overlay_data(&self) -> Option<OverlayData<'_>> {
        let view = self.plugin_overlays.view()?;
        let rect = self.plugin_overlay_rect();
        let content_height = rect.height.saturating_sub(2).max(1);
        let scroll = view
            .scroll
            .min(view.lines.len().saturating_sub(content_height));
        let lines = view
            .lines
            .iter()
            .map(markdown_line)
            .collect::<Vec<MarkdownLine>>();
        let panel = PanelData::plugin(
            rect,
            lines,
            scroll,
            PluginPanelView {
                selected: view.selected,
                title: view.title,
                status: view.status,
                focused: view.focus,
            },
        );
        Some(OverlayData {
            list: None,
            panel: Some(panel),
            key_menu: None,
        })
    }

    pub(crate) fn plugin_overlay_rect(&self) -> Rect {
        let Some(view) = self.plugin_overlays.view() else {
            return Rect::new(0, 0, 0, 0);
        };
        let area = self.window_area();
        let (width, height) = overlay_size(&view, area);
        self.place_overlay(area, &view, width, height)
    }

    fn place_overlay(
        &self,
        area: Rect,
        view: &OverlayView<'_>,
        width: usize,
        height: usize,
    ) -> Rect {
        match view.anchor {
            OverlayAnchor::Cursor => {
                let anchor = self.overlay_anchor(area).unwrap_or((area.x, area.y));
                placement::place(PlacementInput {
                    area,
                    anchor,
                    list: Size::new(width, height),
                    docs: None,
                    preferred_docs_side: None,
                })
                .list
            }
            OverlayAnchor::Center => centered(area, width, height),
            OverlayAnchor::TopLeft => corner(area, width, height, false, false),
            OverlayAnchor::TopRight => corner(area, width, height, true, false),
            OverlayAnchor::BottomLeft => corner(area, width, height, false, true),
            OverlayAnchor::BottomRight => corner(area, width, height, true, true),
        }
    }
}

fn overlay_size(view: &OverlayView<'_>, area: Rect) -> (usize, usize) {
    let width = view
        .width
        .map(|value| value as usize)
        .unwrap_or(kawi_plugin::DEFAULT_OVERLAY_WIDTH as usize)
        .min(area.width)
        .max(1);
    let height = view
        .height
        .map(|value| value as usize)
        .unwrap_or_else(|| {
            view.lines
                .len()
                .saturating_add(2)
                .min(kawi_plugin::DEFAULT_OVERLAY_HEIGHT as usize)
        })
        .min(area.height)
        .max(1);
    (width, height)
}

fn centered(area: Rect, width: usize, height: usize) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area
        .y
        .saturating_add(area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width, height)
}

fn corner(area: Rect, width: usize, height: usize, right: bool, bottom: bool) -> Rect {
    let x = if right {
        area.right().saturating_sub(width)
    } else {
        area.x
    };
    let y = if bottom {
        area.bottom().saturating_sub(height)
    } else {
        area.y
    };
    Rect::new(x, y, width, height)
}

fn markdown_line(line: &OverlayLine) -> MarkdownLine {
    MarkdownLine {
        spans: line
            .spans
            .iter()
            .map(|span| MarkdownSpan {
                text: span.text.clone(),
                style: span
                    .style
                    .map(decoration_style)
                    .unwrap_or(MarkdownStyle::Normal),
                link: None,
            })
            .collect(),
        code: false,
    }
}

fn decoration_style(style: DecorationStyle) -> MarkdownStyle {
    MarkdownStyle::Decoration(frame_style(style))
}
