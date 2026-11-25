use std::path::Path;

use super::App;

impl App {
    pub(crate) fn sync_language_indent(&mut self) {
        let indent = if self.editor.has_file() && !self.editor.current_buffer_is_directory() {
            self.lsp
                .language_indent(Some(Path::new(self.editor.filename())))
        } else {
            Default::default()
        };
        self.editor.set_language_indent(indent);
    }

    pub(crate) fn sync_language_editing(&mut self) {
        let editing = if self.editor.has_file() && !self.editor.current_buffer_is_directory() {
            self.lsp
                .language_editing(Some(Path::new(self.editor.filename())))
        } else {
            Default::default()
        };
        self.editor.set_language_editing(editing);
    }

    pub(crate) fn sync_language_settings(&mut self) {
        self.sync_language_indent();
        self.sync_language_editing();
    }
}

#[cfg(test)]
mod tests;
