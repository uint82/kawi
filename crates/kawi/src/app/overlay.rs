use kawi_tui::frame::{Overlay, OverlayList, OverlayPanel, Rect};
use kawi_tui::markdown::{self, MarkdownDocument, MarkdownLine};
use kawi_tui::overlay::{self as placement, DocsSide, PlacementInput, Size};

use super::App;

pub(super) mod code;
pub(crate) mod document;
mod hover;
mod signature;

const DOCS_WIDTH: usize = 42;
const MAX_LIST_WIDTH: usize = 60;
const MAX_PANEL_WIDTH: usize = 64;
const MAX_DOCS_HEIGHT: usize = 24;

pub(super) struct OverlayData<'a> {
    list: Option<ListData<'a>>,
    panel: Option<PanelData>,
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
        let max_scroll = geometry.max_scroll;
        let Some(session) = self.completion.as_mut() else {
            return;
        };
        if session.docs_side().is_none() {
            if let Some(side) = docs_side {
                session.set_docs_side(side);
            }
        }
        session.clamp_docs_scroll(max_scroll);
    }

    pub(super) fn overlay_data(&self) -> Option<OverlayData<'_>> {
        if self.completion.is_some() {
            return self.completion_overlay();
        }
        if self.signature.is_some() {
            return self.signature_overlay();
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
        })
    }

    pub(super) fn completion_geometry(&self) -> Option<CompletionGeometry> {
        let session = self.completion.as_ref()?;
        let area = self.window_area();
        let anchor = self.overlay_anchor(area)?;
        let list_size = list_size(session.list_width(), session.items().len(), area);
        let documentation = session.selected_document();
        let initial_width = docs_width(area).saturating_sub(2);
        let (mut docs_lines, docs_size) = docs_preview(documentation, area);
        let placement = placement::place(PlacementInput {
            area,
            anchor,
            list: list_size,
            docs: docs_size,
            preferred_docs_side: session.docs_side(),
        });
        if let Some(rect) = placement.docs {
            let width = rect.width.saturating_sub(2);
            if width != initial_width {
                docs_lines = render_docs(documentation, width);
            }
        }
        let (max_scroll, page) = docs_scroll_limits(placement.docs, docs_lines.len());
        Some(CompletionGeometry {
            list: placement.list,
            docs: placement.docs,
            docs_side: placement.docs_side,
            docs_lines,
            max_scroll,
            page,
        })
    }

    pub(super) fn completion_docs_scroll(&self) -> Option<(usize, usize)> {
        self.completion_geometry()
            .filter(|geometry| geometry.docs.is_some())
            .map(|geometry| (geometry.max_scroll, geometry.page))
    }

    fn panel_layout(&self, lines: Vec<MarkdownLine>) -> PanelLayout {
        let area = self.window_area();
        let anchor = self.overlay_anchor(area).unwrap_or((area.x, area.y));
        let width = MAX_PANEL_WIDTH.min(area.width);
        let height = lines.len().saturating_add(2).min(area.height).max(1);
        let placement = placement::place(PlacementInput {
            area,
            anchor,
            list: Size::new(width, height),
            docs: None,
            preferred_docs_side: None,
        });
        let rect = placement.list;
        let max_scroll = lines.len().saturating_sub(rect.height.saturating_sub(2));
        PanelLayout {
            rect,
            lines,
            max_scroll,
        }
    }

    fn overlay_anchor(&self, area: Rect) -> Option<(usize, usize)> {
        let layout = self.window_layout();
        let (_, rect) = layout
            .panes
            .iter()
            .find(|(id, _)| *id == self.windows.active())?;
        let row = self.editor.row();
        let top = self.editor.view_top();
        let x = rect.x.saturating_add(
            self.editor
                .cursor_wcol()
                .saturating_sub(self.editor.view_leftcol()),
        );
        let y = rect.y.saturating_add(row.saturating_sub(top));
        Some((
            x.min(area.right().saturating_sub(1)).max(area.x),
            y.min(area.bottom().saturating_sub(1)).max(area.y),
        ))
    }
}

fn list_size(width: usize, items: usize, area: Rect) -> Size {
    let width = width.min(MAX_LIST_WIDTH).min(area.width).max(1);
    let height = items.saturating_add(2).min(area.height).max(1);
    Size::new(width, height)
}

fn panel_content_width(area_width: usize) -> usize {
    MAX_PANEL_WIDTH.min(area_width).saturating_sub(2)
}

fn page_height(rect: Rect) -> usize {
    rect.height.saturating_sub(4).max(1)
}

fn docs_width(area: Rect) -> usize {
    DOCS_WIDTH.min(area.width)
}

fn docs_preview(
    documentation: Option<&MarkdownDocument>,
    area: Rect,
) -> (Vec<MarkdownLine>, Option<Size>) {
    let width = docs_width(area);
    let lines = render_docs(documentation, width.saturating_sub(2));
    let size = documentation.map(|_| Size::new(width, docs_height(lines.len(), area)));
    (lines, size)
}

fn render_docs(documentation: Option<&MarkdownDocument>, width: usize) -> Vec<MarkdownLine> {
    documentation
        .map(|documentation| markdown::render_document(documentation, width.max(1)))
        .unwrap_or_default()
}

fn docs_height(lines: usize, area: Rect) -> usize {
    lines
        .saturating_add(2)
        .min(MAX_DOCS_HEIGHT.saturating_add(2))
        .min(area.height)
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
    fn page_height_leaves_two_lines_of_overlap() {
        assert_eq!(page_height(Rect::new(0, 0, 20, 12)), 8);
        assert_eq!(page_height(Rect::new(0, 0, 20, 3)), 1);
    }
}
