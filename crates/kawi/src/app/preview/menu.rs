use kawi_tui::frame::PickerPreview;

use super::PreviewState;

const PREVIEW_LOADING: &str = "Loading preview...";

pub(crate) fn preview_menu(preview: &PreviewState, tabstop: usize) -> Option<PickerPreview<'_>> {
    Some(PickerPreview {
        title: preview
            .title()
            .or_else(|| preview.is_loading().then_some(PREVIEW_LOADING))?,
        lines: preview.lines(),
        first_line: preview.first_line(),
        total_lines: preview.total_lines(),
        focus_line: preview.focus_line(),
        syntax_spans: preview.syntax(),
        tabstop,
    })
}
