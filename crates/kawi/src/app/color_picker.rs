mod apply;
mod hsla;
mod keys;
pub(super) mod render;
pub(super) mod state;
pub(super) use state::ColorPicker;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use super::App;
use state::contains_position;

impl App {
    pub(super) fn open_color_picker(&mut self) -> anyhow::Result<bool> {
        match self.color_picker_target() {
            Ok((path, revision, color)) => {
                self.lsp.cancel_color_presentations();
                self.color_picker = Some(ColorPicker::new(&color, path, revision));
            }
            Err(message) => self.show_message(message.to_string()),
        }
        Ok(false)
    }

    fn color_picker_target(&self) -> Result<(PathBuf, u64, kawi_lsp::DocumentColor), &'static str> {
        if !self.color_preview {
            return Err("Color previews are disabled");
        }
        let path = self
            .editor
            .buffer_path(self.editor.buffer_number())
            .map(PathBuf::from)
            .ok_or("E32: No file name")?;
        let revision = self.editor.buffer_revision();
        let position = super::lsp_navigation::cursor_position(&self.editor);
        let color = self
            .lsp
            .color_preview_colors(&path, revision)
            .and_then(|colors| {
                colors
                    .iter()
                    .find(|color| contains_position(color, position))
            })
            .cloned()
            .ok_or("No language server color under cursor")?;
        Ok((path, revision, color))
    }

    pub(super) fn poll_color_picker(&mut self) {
        if let Some(event) = self.lsp.take_color_presentations() {
            if let Some(picker) = self.color_picker.as_mut() {
                picker.apply_event(event);
            }
        }
        if let Some((request, message)) = self.lsp.take_color_presentation_failure() {
            if let Some(picker) = self.color_picker.as_mut() {
                picker.fail_request(request, &message);
            }
        }
    }

    pub(super) fn close_color_picker(&mut self) {
        self.lsp.cancel_color_presentations();
        self.color_picker = None;
    }

    fn request_color_presentations(&mut self) {
        let Some((path, range, color)) = self
            .color_picker
            .as_ref()
            .map(|picker| (picker.path().clone(), picker.range(), picker.color()))
        else {
            return;
        };
        let color = kawi_lsp::ColorValue::new(color.red, color.green, color.blue, color.alpha);
        match self.lsp.request_color_presentations(&path, range, color) {
            Ok(request) => {
                if let Some(picker) = self.color_picker.as_mut() {
                    picker.begin_request(request);
                }
            }
            Err(message) => {
                if let Some(picker) = self.color_picker.as_mut() {
                    picker.set_message(message);
                }
            }
        }
    }

    fn cancel_color_presentation_request(&mut self) {
        self.lsp.cancel_color_presentations();
        if let Some(picker) = self.color_picker.as_mut() {
            picker.cancel_request();
        }
    }
}
