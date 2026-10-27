use kawi_tui::frame::{Overlay, OverlayList, OverlayPanel, Rect};
use kawi_tui::markdown::{self, MarkdownLine};
use kawi_tui::overlay::{self as placement, DocsSide, PlacementInput, Size, COMPLETION_MAX_ROWS};

use self::size::{
    cursor_vertical_capacity, side_vertical_capacity, size_panel, PanelLimits, SizedPanel,
};
use super::completion::CompletionSession;
use super::App;

pub(super) mod code;
mod diagnostic;
pub(crate) mod document;
mod hover;
mod leader;
pub(super) mod options;
mod signature;
mod size;

const MAX_LIST_WIDTH: usize = 50;

pub(super) struct OverlayData<'a> {
    list: Option<ListData<'a>>,
    panel: Option<PanelData>,
    key_menu: Option<leader::KeyMenuData>,
}

struct ListData<'a> {
    rect: Rect,
    items: &'a [String],
    selected: Option<usize>,
}

struct PanelData {
    rect: Rect,
    lines: Vec<MarkdownLine>,
    scroll: usize,
}

struct PanelLayout {
    rect: Rect,
    lines: Vec<MarkdownLine>,
    max_scroll: usize,
}

pub(super) struct CompletionGeometry {
    list: Rect,
    docs: Option<Rect>,
    docs_side: Option<DocsSide>,
    docs_lines: Vec<MarkdownLine>,
    docs_width: usize,
    max_scroll: usize,
    page: usize,
}

impl OverlayData<'_> {
    pub(super) fn view(&self) -> Overlay<'_> {
        Overlay {
            list: self.list.as_ref().map(|list| OverlayList {
                rect: list.rect,
                items: list.items,
                selected: list.selected,
            }),
            panel: self.panel.as_ref().map(|panel| OverlayPanel {
                rect: panel.rect,
                lines: &panel.lines,
                scroll: panel.scroll,
            }),
            key_menu: self.key_menu.as_ref().map(leader::KeyMenuData::view),
        }
    }
}

impl App {
    pub(super) fn prepare_completion_overlay(&mut self) {
        let Some(geometry) = self.completion_geometry() else {
            return;
        };
        if geometry.docs.is_none() {
            return;
        }
        let docs_side = geometry.docs_side;
        let docs_width = geometry.docs_width;
        let max_scroll = geometry.max_scroll;
        let Some(session) = self.completion.as_mut() else {
            return;
        };
        if session.docs_side().is_none() {
            if let Some(side) = docs_side {
                session.set_docs_side(side);
            }
        }
        session.grow_docs_width(docs_width);
        session.clamp_docs_scroll(max_scroll);
    }

    pub(super) fn overlay_data(&self) -> Option<OverlayData<'_>> {
        if let Some(overlay) = self.leader_overlay() {
            return Some(overlay);
        }
        if self.completion.is_some() {
            return self.completion_overlay();
        }
        if self.signature.is_some() {
            return self.signature_overlay();
        }
        if self.diagnostic_popup.is_some() {
            return self.diagnostic_overlay();
        }
        if self.hover.is_some() {
            return self.hover_overlay();
        }
        None
    }

    fn completion_overlay(&self) -> Option<OverlayData<'_>> {
        let geometry = self.completion_geometry()?;
        let max_scroll = geometry.max_scroll;
        let session = self.completion.as_ref()?;
        let list = ListData {
            rect: geometry.list,
            items: session.items(),
            selected: session.selected(),
        };
        let panel = geometry.docs.map(|rect| PanelData {
            rect,
            lines: geometry.docs_lines,
            scroll: session.docs_scroll().min(max_scroll),
        });
        Some(OverlayData {
            list: Some(list),
            panel,
            key_menu: None,
        })
    }

    pub(super) fn completion_geometry(&self) -> Option<CompletionGeometry> {
        let session = self.completion.as_ref()?;
        let area = self.window_area();
        let anchor = self.overlay_anchor(area)?;
        let list_size = list_size(session.list_width(), session.items().len(), area);
        let list_rect = placement::place(PlacementInput {
            area,
            anchor,
            list: list_size,
            docs: None,
            preferred_docs_side: None,
        })
        .list;
        let (docs_lines, docs_size, docs_width) = self.completion_docs(session, area, list_rect.y);
        let placement = placement::place(PlacementInput {
            area,
            anchor,
            list: list_size,
            docs: docs_size,
            preferred_docs_side: session.docs_side(),
        });
        let (max_scroll, page) = docs_scroll_limits(placement.docs, docs_lines.len());
        Some(CompletionGeometry {
            list: placement.list,
            docs: placement.docs,
            docs_side: placement.docs_side,
            docs_lines,
            docs_width,
            max_scroll,
            page,
        })
    }

    fn completion_docs(
        &self,
        session: &CompletionSession,
        area: Rect,
        list_top: usize,
    ) -> (Vec<MarkdownLine>, Option<Size>, usize) {
        let Some(document) = session.selected_document() else {
            return (Vec::new(), None, 0);
        };
        let min_width = self.overlay_options.min_width.max(session.docs_width());
        let available = side_vertical_capacity(area, list_top);
        let limits = self.overlay_options.limits(
            area,
            self.overlay_options.completion_docs_max_width,
            min_width,
            available,
        );
        let panel = size_panel(limits, |width| markdown::render_document(document, width));
        let size = Size::new(panel.width, panel.height);
        (panel.lines, Some(size), panel.width)
    }

    pub(super) fn completion_docs_scroll(&self) -> Option<(usize, usize)> {
        self.completion_geometry()
            .filter(|geometry| geometry.docs.is_some())
            .map(|geometry| (geometry.max_scroll, geometry.page))
    }

    fn panel_layout(&self, panel: SizedPanel) -> PanelLayout {
        let area = self.window_area();
        let anchor = self.overlay_anchor(area).unwrap_or((area.x, area.y));
        let placement = placement::place(PlacementInput {
            area,
            anchor,
            list: Size::new(panel.width, panel.height),
            docs: None,
            preferred_docs_side: None,
        });
        let rect = placement.list;
        let max_scroll = panel
            .lines
            .len()
            .saturating_sub(rect.height.saturating_sub(2));
        PanelLayout {
            rect,
            lines: panel.lines,
            max_scroll,
        }
    }

    fn cursor_limits(&self, max_width: usize, min_width: usize) -> PanelLimits {
        let area = self.window_area();
        let anchor = self.overlay_anchor(area).unwrap_or((area.x, area.y));
        let available = cursor_vertical_capacity(area, anchor);
        self.overlay_options
            .limits(area, max_width, min_width, available)
    }

    fn overlay_anchor(&self, area: Rect) -> Option<(usize, usize)> {
        let layout = self.window_layout();
        let (_, rect) = layout
            .panes
            .iter()
            .find(|(id, _)| *id == self.windows.active())?;
        let x = rect.x.saturating_add(
            self.editor
                .cursor_wcol()
                .saturating_sub(self.editor.view_leftcol()),
        );
        let y = rect
            .y
            .saturating_add(self.editor.cursor_screen_row().unwrap_or(0));
        Some((
            x.min(area.right().saturating_sub(1)).max(area.x),
            y.min(area.bottom().saturating_sub(1)).max(area.y),
        ))
    }
}

fn list_size(width: usize, items: usize, area: Rect) -> Size {
    let width = width.min(MAX_LIST_WIDTH).min(area.width).max(1);
    let height = items
        .saturating_add(2)
        .min(COMPLETION_MAX_ROWS.saturating_add(2))
        .min(area.height)
        .max(1);
    Size::new(width, height)
}

fn page_height(rect: Rect) -> usize {
    rect.height.saturating_sub(4).max(1)
}

fn docs_scroll_limits(docs: Option<Rect>, lines: usize) -> (usize, usize) {
    let Some(rect) = docs else {
        return (0, 0);
    };
    (
        lines.saturating_sub(rect.height.saturating_sub(2)),
        page_height(rect),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_size_keeps_the_stored_width_and_caps_the_height() {
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(list_size(80, 20, area), Size::new(40, 10));
        assert_eq!(list_size(12, 3, area), Size::new(12, 5));
    }

    #[test]
    fn list_size_caps_the_width() {
        let area = Rect::new(0, 0, 80, 30);
        assert_eq!(list_size(80, 3, area), Size::new(MAX_LIST_WIDTH, 5));
    }

    #[test]
    fn list_size_caps_completion_rows() {
        let area = Rect::new(0, 0, 40, 30);
        assert_eq!(list_size(80, 20, area), Size::new(40, 12));
        assert_eq!(list_size(80, COMPLETION_MAX_ROWS, area), Size::new(40, 12));
    }

    #[test]
    fn page_height_leaves_two_lines_of_overlap() {
        assert_eq!(page_height(Rect::new(0, 0, 20, 12)), 8);
        assert_eq!(page_height(Rect::new(0, 0, 20, 3)), 1);
    }
}
