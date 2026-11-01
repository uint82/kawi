use std::io::Stdout;

use anyhow::Result;
use kawi_editor::Mode;
use kawi_picker::PickerKind;
use kawi_tui::{
    frame::{CompletionMenu, Frame, PaneFrame, TaskPanel, TerminalPanel},
    render,
    statusbar::StatusInfo,
    CursorStyle, Selection,
};

use super::{preview, rename, replace_stage, App};

fn cursor_style_for(mode: Mode) -> CursorStyle {
    match mode {
        Mode::Insert => CursorStyle::Bar,
        Mode::Replace => CursorStyle::Underline,
        _ => CursorStyle::Block,
    }
}

impl App {
    pub(crate) fn draw_frame(&mut self, out: &mut Stdout) -> Result<()> {
        self.refresh_bracket_match();
        self.prepare_completion_overlay();
        self.sync_explorer_marks();
        let overlay_data = self.overlay_data();
        let overlay = overlay_data.as_ref().map(|data| data.view());
        let layout = self.window_layout();
        let explorer_view = self.explorer_view();
        let explorer = explorer_view
            .as_ref()
            .map(|view| kawi_tui::frame::ExplorerPanel {
                rect: view.rect,
                title: view.title,
                status: &view.status,
                focused: view.focused,
                rows: &view.rows,
                selected: view.selected,
                offset: view.offset,
                error: view.error,
            });
        let data = self.frame_panes(&layout);
        let git = self.git_status.display();
        let statuses: Vec<StatusInfo> = data
            .iter()
            .map(|pane| StatusInfo {
                filename: &pane.filename,
                dirty: pane.dirty,
                readonly: pane.readonly,
                row: pane.row,
                col: pane.col,
                virtcol: pane.virtcol,
                line_empty: pane.line_empty,
                empty_buffer: pane.empty_buffer,
                mode: pane.mode,
                mode_kind: pane.mode_kind,
                lsp: pane.lsp.as_deref(),
                diagnostics: pane.diagnostics.as_ref().map(|value| value.text.as_str()),
                diagnostics_severity: pane.diagnostics.as_ref().map(|value| value.severity),
                workspace_diagnostics: pane
                    .workspace_diagnostics
                    .as_ref()
                    .map(|value| value.text.as_str()),
                workspace_diagnostics_severity: pane
                    .workspace_diagnostics
                    .as_ref()
                    .map(|value| value.severity),
                external: pane.external,
                git,
                filetype: pane.filetype,
                indent: pane.indent.as_deref(),
                encoding: pane.encoding,
                eol: pane.eol,
                selection: pane.selection_status.as_deref(),
                macro_recording: pane.macro_recording,
                percent: pane.percent,
            })
            .collect();
        let explorer_focused = self.explorer.is_focused();
        let panes: Vec<PaneFrame<'_>> = data
            .iter()
            .zip(statuses.iter())
            .map(|(pane, status)| PaneFrame {
                rect: pane.rect,
                active: pane.active && !explorer_focused,
                lines: &pane.lines,
                rows: &pane.rows,
                gutter: &pane.gutter,
                signs: pane.signs.as_deref(),
                diagnostic_spans: &pane.diagnostic_spans,
                virtual_texts: &pane.virtual_texts,
                left_width: pane.left_width,
                search_spans: &pane.search_spans,
                preview_spans: &pane.preview_spans,
                syntax_spans: pane.syntax_spans.as_ref(),
                rulers: &pane.rulers,
                selection: pane.selection,
                bracket_match: pane.bracket_match,
                viewport: pane.viewport,
                cursor_col: pane.cursor_col,
                cursor_screen_row: pane.cursor_screen_row,
                row: pane.row,
                tabstop: pane.tabstop,
                status,
            })
            .collect();
        let search_status = self.editor.search_status_text();
        let symbol_items = self.symbol_picker_lines();
        let symbol_preview = self.symbol_preview();
        let picker_preview = self.picker_preview_menu();
        let picker_items: Vec<kawi_tui::frame::PickerLine<'_>> = self
            .picker
            .as_ref()
            .map(|session| {
                session
                    .picker()
                    .rows()
                    .iter()
                    .map(|row| kawi_tui::frame::PickerLine {
                        primary: &row.display,
                        detail: row.secondary.as_deref(),
                        deprecated: row.deprecated,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let plugin_picker = self.plugin_ui.picker();
        let plugin_picker_items: Vec<kawi_tui::frame::PickerLine<'_>> = plugin_picker
            .map(|state| {
                state
                    .rows()
                    .iter()
                    .map(|row| kawi_tui::frame::PickerLine {
                        primary: &row.display,
                        detail: row.secondary.as_deref(),
                        deprecated: row.deprecated,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let plugin_picker_menu = plugin_picker.map(|state| kawi_tui::frame::PickerMenu {
            title: state.title(),
            query: state.query(),
            cursor: state.cursor(),
            items: &plugin_picker_items,
            selected: state.selected(),
            loading: state.loading(),
            status: state.status(),
            replace_hint: false,
            file_hint: false,
            preview_enabled: false,
            preview: None,
        });
        let git_lines = self.git_status.view_lines();
        let git_picker_lines: Vec<kawi_tui::frame::PickerLine<'_>> = git_lines
            .iter()
            .map(|line| kawi_tui::frame::PickerLine {
                primary: line,
                detail: None,
                deprecated: false,
            })
            .collect();
        let picker = plugin_picker_menu
            .or_else(|| {
                self.symbol_picker.as_ref().map(|session| {
                    let state = session.picker();
                    kawi_tui::frame::PickerMenu {
                        title: state.title(),
                        query: state.query(),
                        cursor: state.cursor(),
                        items: &symbol_items,
                        selected: state.selected(),
                        loading: state.loading(),
                        status: state.status(),
                        replace_hint: false,
                        file_hint: false,
                        preview_enabled: true,
                        preview: symbol_preview,
                    }
                })
            })
            .or_else(|| {
                self.picker.as_ref().map(|session| {
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
                        file_hint: state.kind() == PickerKind::Files,
                        preview_enabled: preview::has_file_preview(state.kind()),
                        preview: picker_preview,
                    }
                })
            })
            .or_else(|| {
                self.git_status
                    .has_view()
                    .then(|| kawi_tui::frame::PickerMenu {
                        title: "Git status",
                        query: "",
                        cursor: 0,
                        items: &git_picker_lines,
                        selected: self.git_status.view_selected(),
                        loading: false,
                        status: self.git_status.view_status(),
                        replace_hint: false,
                        file_hint: false,
                        preview_enabled: false,
                        preview: None,
                    })
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
        let task_lines = self
            .task_panel_rect()
            .map(|rect| self.task_panel_lines(rect.height.saturating_sub(1)))
            .unwrap_or_default();
        let task_title = self.task_panel_title();
        let task = self.task_panel_rect().map(|rect| TaskPanel {
            rect,
            title: task_title.as_deref().unwrap_or("Task"),
            status: self.task_panel_status().unwrap_or_default(),
            lines: &task_lines,
        });
        let terminal_lines = if self.pager.is_some() {
            Vec::new()
        } else {
            self.terminal_panel_rect()
                .map(|rect| {
                    self.terminal
                        .lines(rect.width, rect.height.saturating_sub(1))
                })
                .unwrap_or_default()
        };
        let terminal_title = self.terminal.title();
        let terminal_status = self.terminal.status();
        let terminal = if self.pager.is_some() {
            None
        } else {
            self.terminal_panel_rect().map(|rect| TerminalPanel {
                rect,
                title: terminal_title,
                status: terminal_status,
                lines: &terminal_lines,
                cursor: self.terminal.cursor(),
                focused: self.terminal_focus,
            })
        };
        let viewport = kawi_editor::Viewport::new(
            self.screen_height.saturating_sub(2).max(1),
            self.screen_width.max(1),
        );
        let plugin_progress = self.plugin_ui.progress_text();
        let frame = Frame {
            panes: &panes,
            separators: &layout.separators,
            pager: self.pager.as_deref(),
            completion: self.completion_menu(),
            overlay,
            picker,
            replace,
            explorer,
            task,
            terminal,
            viewport: &viewport,
            message: plugin_progress.as_deref().or(self.message.as_deref()),
            prompt: self.plugin_ui.prompt().or_else(|| {
                self.file_ops_prompt()
                    .or_else(|| self.rename.as_ref().and_then(rename::RenameSession::prompt))
                    .or_else(|| {
                        self.branch_prompt
                            .as_ref()
                            .and_then(super::git_branch::BranchPrompt::prompt)
                    })
            }),
            command: self.editor.command_prompt(),
            search: self.editor.search_prompt(),
            search_delimiter: self.editor.search_prompt_delimiter(),
            search_status: search_status.as_deref(),
            showcmd: self.showcmd_text(),
            cursor_style: cursor_style_for(self.editor.mode()),
            theme: &self.theme,
            statusline: &self.statusline,
        };
        render::draw(out, &frame)
    }

    fn completion_menu(&self) -> Option<CompletionMenu<'_>> {
        self.plugin_prompt_items()
            .map(|(items, selected)| CompletionMenu {
                items,
                selected: Some(selected),
            })
            .or_else(|| {
                self.conflict.as_ref().map(|session| CompletionMenu {
                    items: session.items(),
                    selected: session.selected(),
                })
            })
            .or_else(|| {
                self.code_actions.as_ref().map(|session| CompletionMenu {
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
