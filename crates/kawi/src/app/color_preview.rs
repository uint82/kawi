mod color;

pub(crate) use color::Rgba;
mod context;
mod contrast;
mod hex;
mod lsp;
mod mode;
mod named;
mod rgb;
mod scan;
#[cfg(test)]
mod tests;

use std::path::Path;

use kawi_tui::frame::DecorationSpan;

pub(crate) use mode::PreviewMode;

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
        let Some(mode) = self.lsp.language_color_preview_mode(path) else {
            return Vec::new();
        };
        if let Some(colors) = path.and_then(|path| {
            self.lsp
                .color_preview_colors(path, self.editor.buffer_revision())
        }) {
            return lsp::spans(colors, lines, rows, self.theme.base.background);
        }
        scan::spans(lines, rows, self.theme.base.background, mode)
    }
}
