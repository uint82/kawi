use kawi_tui::frame::VirtualText;

use crate::app::diagnostic_ui::VirtualTextRequest;
use crate::app::App;

const SYMLINK_ARROW: &str = " -> ";

impl App {
    pub(super) fn directory_metadata_width(&self, number: usize) -> usize {
        let Some(state) = self.directory.get(number) else {
            return 0;
        };
        state.metadata_width(self.directory_options.metadata)
    }

    pub(in crate::app) fn directory_virtual_texts(
        &self,
        number: usize,
        rows: &[usize],
    ) -> Vec<Vec<VirtualText>> {
        let Some(state) = self.directory.get(number) else {
            return empty_virtual_texts(rows.len());
        };
        let width = state.metadata_width(self.directory_options.metadata);
        let columns = self.directory_options.metadata;
        rows.iter()
            .map(|row| {
                let mut texts = Vec::with_capacity(2);
                if let Some(text) = metadata_virtual_text(state, *row, columns, width) {
                    texts.push(text);
                }
                if let Some(text) = symlink_virtual_text(state, *row) {
                    texts.push(text);
                }
                texts
            })
            .collect()
    }

    pub(super) fn pane_virtual_texts(
        &self,
        number: usize,
        request: VirtualTextRequest<'_>,
    ) -> Vec<Vec<VirtualText>> {
        if self.directory.contains(number) {
            self.directory_virtual_texts(number, request.rows)
        } else {
            self.virtual_texts(request)
                .into_iter()
                .map(|text| text.into_iter().collect())
                .collect()
        }
    }

    pub(super) fn sync_directory_metadata_width(&mut self) {
        let width = self.directory_metadata_width(self.editor.buffer_number());
        if self.editor.gutter_extra_width() == width {
            return;
        }
        self.editor.set_gutter_extra_width(width);
        self.editor.reveal_cursor_horizontally();
    }
}

fn metadata_virtual_text(
    state: &crate::app::directory::DirectoryBuffer,
    row: usize,
    columns: crate::app::directory::MetadataColumns,
    width: usize,
) -> Option<VirtualText> {
    if width == 0 {
        return None;
    }
    let text = state
        .metadata_text(row, columns)
        .unwrap_or_else(|| " ".repeat(width));
    Some(VirtualText::metadata(text))
}

fn symlink_virtual_text(
    state: &crate::app::directory::DirectoryBuffer,
    row: usize,
) -> Option<VirtualText> {
    let target = state.symlink_target(row)?;
    Some(VirtualText::metadata_suffix(format!(
        "{SYMLINK_ARROW}{target}"
    )))
}

fn empty_virtual_texts(count: usize) -> Vec<Vec<VirtualText>> {
    std::iter::repeat_with(Vec::new).take(count).collect()
}
