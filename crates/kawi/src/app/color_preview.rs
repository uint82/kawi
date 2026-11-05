mod color;
mod scan;
#[cfg(test)]
mod tests;

use std::path::Path;

use kawi_tui::frame::DecorationSpan;

use super::App;

impl App {
    pub(crate) fn color_preview_spans(
        &self,
        buffer: usize,
        lines: &[String],
        rows: &[usize],
    ) -> Vec<DecorationSpan> {
        if !self.color_preview {
            return Vec::new();
        }
        let path = self.editor.buffer_path(buffer).map(Path::new);
        if !self.lsp.language_color_preview(path) {
            return Vec::new();
        }
        scan::spans(lines, rows, self.theme.base.background)
    }
}
