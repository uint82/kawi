use std::io::Stdout;

use anyhow::Result;
use kawi_editor::Mode;
use kawi_picker::PickerKind;
use kawi_tui::{
    frame::{CompletionMenu, Frame, PaneFrame},
    render,
    statusbar::StatusInfo,
    CursorStyle, Selection,
};

use super::{rename, replace_stage, App};

fn cursor_style_for(mode: Mode) -> CursorStyle {
    match mode {
        Mode::Insert => CursorStyle::Bar,
        Mode::Replace => CursorStyle::Underline,
        _ => CursorStyle::Block,
    }
}

impl App {
    pub(crate) fn draw_frame(&self, out: &mut Stdout) -> Result<()> {
        let layout = self.window_layout();
        let data = self.frame_panes(&layout);
        let statuses: Vec<StatusInfo> = data
            .iter()
            .map(|pane| StatusInfo {
                filename: &pane.filename,
                dirty: pane.dirty,
                row: pane.row,
                col: pane.col,
                virtcol: pane.virtcol,
                line_empty: pane.line_empty,
                empty_buffer: pane.empty_buffer,
                mode: pane.mode,
                lsp: pane.lsp.as_deref(),
            })
            .collect();
        let panes: Vec<PaneFrame<'_>> = data
            .iter()
            .zip(statuses.iter())
            .map(|(pane, status)| PaneFrame {
                rect: pane.rect,
                active: pane.active,
                lines: &pane.lines,
                gutter: &pane.gutter,
                signs: pane.signs.as_deref(),
                diagnostic_spans: &pane.diagnostic_spans,
                search_spans: &pane.search_spans,
                preview_spans: &pane.preview_spans,
                syntax_spans: pane.syntax_spans.as_ref(),
                selection: pane.selection,
                viewport: pane.viewport,
                cursor_col: pane.cursor_col,
                row: pane.row,
                tabstop: pane.tabstop,
                status,
            })
            .collect();
        let search_status = self.editor.search_status_text();
        let picker_items: Vec<String> = self
            .picker
            .as_ref()
            .map(|session| {
                session
                    .picker()
                    .rows()
                    .iter()
                    .map(|row| row.display.clone())
                    .collect()
            })
            .unwrap_or_default();
        let picker = self.picker.as_ref().map(|session| {
            let state = session.picker();
            kawi_tui::frame::PickerMenu {
                title: state.title(),
                query: state.query(),
                cursor: state.cursor(),
                items: &picker_items,
                selected: state.selected(),
                loading: state.loading(),
                status: state.status(),
                replace_hint: state.kind() == PickerKind::Grep,
            }
        });
        let replace_state = self.replace.as_ref();
        let replace_rows: Vec<String> = replace_state
            .map(|session| session.rows().to_vec())
            .unwrap_or_default();
        let replace = replace_state.map(|session| kawi_tui::frame::ReplaceMenu {
            stage: replace_stage(session.stage()),
            query: session.query(),
            replacement: session.replacement(),
            cursor: session.cursor(),
            items: &replace_rows,
            selected: session.selected(),
            loading: session.loading(),
            status: session.status(),
            matches: session.matches(),
            files: session.files(),
        });
        let viewport = kawi_editor::Viewport::new(
            self.screen_height.saturating_sub(2).max(1),
            self.screen_width.max(1),
        );
        let frame = Frame {
            panes: &panes,
            separators: &layout.separators,
            pager: self.pager.as_deref(),
            completion: self.completion_menu(),
            picker,
            replace,
            viewport: &viewport,
            message: self.message.as_deref(),
            prompt: self.rename.as_ref().and_then(rename::RenameSession::prompt),
            command: self.editor.command_prompt(),
            search: self.editor.search_prompt(),
            search_delimiter: self.editor.search_prompt_delimiter(),
            search_status: search_status.as_deref(),
            showcmd: self.showcmd_text(),
            cursor_style: cursor_style_for(self.editor.mode()),
            theme: &self.theme,
        };
        render::draw(out, &frame)
    }

    fn completion_menu(&self) -> Option<CompletionMenu<'_>> {
        self.code_actions
            .as_ref()
            .map(|session| CompletionMenu {
                items: session.items(),
                selected: session.selected(),
            })
            .or_else(|| {
                self.completion.as_ref().map(|session| CompletionMenu {
                    items: session.items(),
                    selected: session.selected(),
                })
            })
            .or_else(|| {
                self.editor
                    .completion_view()
                    .map(|(items, selected)| CompletionMenu { items, selected })
            })
    }

    pub(crate) fn frame_selection(&self) -> Option<Selection> {
        if self.editor.mode() == Mode::VisualBlock {
            let (top, left, bottom, right, ragged) = self.editor.block_bounds()?;
            return Some(Selection::Block {
                top,
                left,
                bottom,
                right,
                ragged,
            });
        }
        self.editor
            .selection_range()
            .map(|(start, end)| Selection::Range { start, end })
    }
}
