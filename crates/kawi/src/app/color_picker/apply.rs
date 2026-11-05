use kawi_lsp::ColorPresentationEntry;

use super::super::App;

impl App {
    pub(super) fn apply_selected_color_presentation(&mut self) {
        let Some((path, revision, presentation)) = self.color_picker.as_ref().and_then(|picker| {
            Some((
                picker.path().clone(),
                picker.revision(),
                picker.selected_presentation()?,
            ))
        }) else {
            return;
        };
        if self.editor.buffer_revision() != revision || !self.editor.file_is_current(&path) {
            self.close_color_picker();
            self.show_message("Buffer changed while choosing color".to_string());
            return;
        }
        match self.apply_color_presentation(&path, presentation) {
            Ok(()) => {
                self.close_color_picker();
                self.show_message("Color updated".to_string());
            }
            Err(message) => {
                self.close_color_picker();
                self.show_message(message);
            }
        }
    }

    fn apply_color_presentation(
        &mut self,
        path: &std::path::Path,
        presentation: ColorPresentationEntry,
    ) -> Result<(), String> {
        let ColorPresentationEntry {
            edit, additional, ..
        } = presentation;
        let mut edits = additional;
        edits.push(edit);
        self.editor
            .apply_text_edits(path, &edits)
            .map_err(super::super::file_error_message)?;
        Ok(())
    }
}
