use kawi_tui::markdown::{self, MarkdownDocument, MarkdownLine};

use super::super::App;
use super::{page_height, panel_content_width, OverlayData, PanelData};

impl App {
    pub(super) fn hover_overlay(&self) -> Option<OverlayData<'_>> {
        let session = self.hover.as_ref()?;
        let layout = self.panel_layout(self.hover_lines(session.documents()));
        Some(OverlayData {
            list: None,
            panel: Some(PanelData {
                rect: layout.rect,
                lines: layout.lines,
                scroll: session.scroll().min(layout.max_scroll),
            }),
        })
    }

    pub(crate) fn hover_scroll(&self) -> Option<(usize, usize)> {
        let session = self.hover.as_ref()?;
        let layout = self.panel_layout(self.hover_lines(session.documents()));
        Some((layout.max_scroll, page_height(layout.rect)))
    }

    fn hover_lines(&self, documents: &[MarkdownDocument]) -> Vec<MarkdownLine> {
        let width = panel_content_width(self.window_area().width);
        let mut lines = Vec::new();
        for (index, document) in documents.iter().enumerate() {
            if index > 0 {
                lines.push(MarkdownLine::blank());
            }
            lines.extend(markdown::render_document(document, width));
        }
        lines
    }
}
