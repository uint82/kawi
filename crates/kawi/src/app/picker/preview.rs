use std::path::PathBuf;
use std::time::{Duration, Instant};

use kawi_tui::frame::PickerPreview;

use crate::app::preview::{picker_preview_target, preview_menu, PickerPreviewTarget, PreviewState};
use crate::app::App;

impl App {
    pub(in crate::app) fn picker_preview_tick(&mut self) -> bool {
        let now = Instant::now();
        self.sync_picker_preview_target(now);
        self.picker_preview
            .as_mut()
            .is_some_and(|preview| preview.tick(now) | preview.poll())
    }

    pub(in crate::app) fn picker_preview_wait_timeout(&self) -> Option<Duration> {
        self.picker_preview
            .as_ref()
            .and_then(PreviewState::deadline)
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    pub(in crate::app) fn picker_preview_menu(&self) -> Option<PickerPreview<'_>> {
        let preview = self.picker_preview.as_ref()?;
        preview_menu(preview, self.editor.tabstop())
    }

    fn sync_picker_preview_target(&mut self, now: Instant) {
        let Some((target, root)) = self.current_picker_preview_target() else {
            self.clear_picker_preview();
            return;
        };
        if self.picker_preview_key.as_ref() == Some(&target.key) {
            return;
        }
        let wake = self.events.wake();
        let preview = self
            .picker_preview
            .get_or_insert_with(|| PreviewState::new(root, wake));
        preview.schedule(&target.path, target.text, target.focus_line, now);
        self.picker_preview_key = Some(target.key);
    }

    fn current_picker_preview_target(&self) -> Option<(PickerPreviewTarget, PathBuf)> {
        let session = self.picker.as_ref()?;
        let target = picker_preview_target(session, &self.editor)?;
        Some((target, session.root().to_path_buf()))
    }

    fn clear_picker_preview(&mut self) {
        self.picker_preview_key = None;
        if let Some(preview) = self.picker_preview.as_mut() {
            preview.clear();
        }
    }
}
