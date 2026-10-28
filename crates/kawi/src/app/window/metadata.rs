use kawi_tui::frame::VirtualText;

use crate::app::diagnostic_ui::VirtualTextRequest;
use crate::app::App;

impl App {
    pub(in crate::app) fn directory_metadata_texts(
        &self,
        number: usize,
        rows: &[usize],
    ) -> Vec<Option<VirtualText>> {
        let Some(state) = self.directory.get(number) else {
            return std::iter::repeat_with(|| None).take(rows.len()).collect();
        };
        let columns = self.directory_options.metadata;
        if columns.is_empty() {
            return std::iter::repeat_with(|| None).take(rows.len()).collect();
        }
        rows.iter()
            .map(|row| {
                state
                    .metadata_text(*row, columns)
                    .map(VirtualText::metadata)
            })
            .collect()
    }

    pub(super) fn pane_virtual_texts(
        &self,
        number: usize,
        request: VirtualTextRequest<'_>,
    ) -> Vec<Option<VirtualText>> {
        let metadata = self.directory_metadata_texts(number, request.rows);
        if metadata.iter().any(Option::is_some) {
            metadata
        } else {
            self.virtual_texts(request)
        }
    }
}
