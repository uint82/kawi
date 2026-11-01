use std::borrow::Cow;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::{BracketMatch, Mode, Viewport, HIGHLIGHT_MATCH_STEP_BUDGET};
use kawi_syntax::{SyntaxKey, SyntaxSpan};
use kawi_tui::frame::{
    DecorationSpan, DiagnosticSpan, Rect, SearchSpan, Selection, Sign, VirtualText,
};
use kawi_tui::statusbar::ModeKind;

use super::{
    is_window_prefix, BracketMatchCache, BracketMatchKey, FocusDirection, ResizeAxis,
    SplitDirection, WindowId, WindowLayout, WindowView,
};
use crate::keymap::Keymap;

use super::super::status::StatusDiagnostic;
use super::super::{frame_search_spans, App};

pub(crate) struct PaneData<'a> {
    pub rect: Rect,
    pub active: bool,
    pub lines: Vec<String>,
    pub rows: Vec<usize>,
    pub gutter: Vec<String>,
    pub signs: Option<Vec<Option<Sign>>>,
    pub diagnostic_spans: Vec<DiagnosticSpan>,
    pub decorations: Vec<DecorationSpan>,
    pub virtual_texts: Vec<Option<VirtualText>>,
    pub plugin_virtual_texts: Vec<Vec<VirtualText>>,
    pub left_width: usize,
    pub search_spans: Vec<SearchSpan>,
    pub preview_spans: Vec<SearchSpan>,
    pub syntax_spans: Cow<'a, [SyntaxSpan]>,
    pub rulers: Vec<usize>,
    pub selection: Option<Selection>,
    pub bracket_match: Option<BracketMatch>,
    pub viewport: Viewport,
    pub cursor_col: usize,
    pub cursor_screen_row: Option<usize>,
    pub row: usize,
    pub tabstop: usize,
    pub filename: String,
    pub dirty: bool,
    pub readonly: bool,
    pub col: usize,
    pub virtcol: usize,
    pub line_empty: bool,
    pub empty_buffer: bool,
    pub mode: &'static str,
    pub mode_kind: ModeKind,
    pub lsp: Option<String>,
    pub diagnostics: Option<StatusDiagnostic>,
    pub workspace_diagnostics: Option<StatusDiagnostic>,
    pub external: Option<&'static str>,
    pub filetype: Option<&'static str>,
    pub indent: Option<String>,
    pub encoding: Option<&'static str>,
    pub eol: Option<&'static str>,
    pub selection_status: Option<String>,
    pub macro_recording: Option<char>,
    pub percent: Option<usize>,
}

impl App {
    pub(crate) fn sync_active_window(&mut self) {
        self.sync_directory_metadata_width();
        let view = self.editor.view_state();
        let buffer = self.editor.buffer_number();
        let preserve_view = self.directory_loading.is_loading(buffer);
        let options = self.editor.window_options();
        let window = self.windows.active_view_mut();
        window.buffer = buffer;
        if !preserve_view {
            window.view = view;
        }
        window.options = options;
    }

    pub(crate) fn sync_active_window_options(&mut self) {
        self.windows.active_view_mut().options = self.editor.window_options();
    }

    pub(crate) fn refresh_bracket_match(&mut self) {
        let key = BracketMatchKey {
            buffer: self.editor.buffer_number(),
            revision: self.editor.buffer_revision(),
            language_generation: self.editor.language_generation(),
            row: self.editor.row(),
            col: self.editor.col(),
            insert: self.editor.mode().is_insert(),
            top: self.editor.view_top(),
            leftcol: self.editor.view_leftcol(),
            width: self.editor.viewport().width(),
            height: self.editor.view_height(),
            enabled: self.editor.match_brackets(),
        };
        if self
            .windows
            .active_view()
            .bracket_match
            .is_some_and(|cache| cache.key == key)
        {
            return;
        }
        let matched = self.bracket_match_for_key(key);
        self.windows.active_view_mut().bracket_match = Some(BracketMatchCache { key, matched });
    }

    fn bracket_match_for_key(&self, key: BracketMatchKey) -> Option<BracketMatch> {
        if !key.enabled {
            return None;
        }
        let viewport = key
            .width
            .saturating_mul(key.height)
            .saturating_add(HIGHLIGHT_MATCH_STEP_BUDGET)
            .saturating_add(key.leftcol);
        self.editor.bracket_match(viewport)
    }

    fn stash_active_syntax(&mut self, id: WindowId) {
        let key = self.syntax_key.take();
        let spans = std::mem::take(&mut self.syntax_spans);
        let window = std::mem::replace(&mut self.syntax_window, 0..0);
        if let Some(key) = key {
            self.window_syntax
                .insert(id, super::super::SyntaxCache { spans, window, key });
        }
        self.syntax_pending = false;
        self.syntax_latest = 0;
        self.syntax_applied = 0;
    }

    fn stash_active_syntax_copy(&mut self, id: WindowId) {
        let Some(key) = self.syntax_key else {
            return;
        };
        self.window_syntax.insert(
            id,
            super::super::SyntaxCache {
                spans: self.syntax_spans.clone(),
                window: self.syntax_window.clone(),
                key,
            },
        );
    }

    fn restore_window_syntax(&mut self, id: WindowId) {
        self.syntax_spans.clear();
        self.syntax_window = 0..0;
        self.syntax_key = None;
        self.syntax_pending = false;
        self.syntax_latest = 0;
        self.syntax_applied = 0;
        if let Some(cache) = self.window_syntax.remove(&id) {
            self.syntax_spans = cache.spans;
            self.syntax_window = cache.window;
            self.syntax_key = Some(cache.key);
        }
    }

    pub(crate) fn window_area(&self) -> Rect {
        let task_height = self.task.panel_height(self.screen_height);
        let terminal_height = self.terminal.panel_height(self.screen_height, task_height);
        let terminal_width = self.terminal_panel_width();
        let explorer_width = self.explorer_width();
        let height = self
            .screen_height
            .saturating_sub(1)
            .saturating_sub(task_height)
            .saturating_sub(terminal_height)
            .max(1);
        let width = self
            .screen_width
            .saturating_sub(explorer_width)
            .saturating_sub(terminal_width)
            .max(1);
        Rect::new(explorer_width, 0, width, height)
    }

    pub(crate) fn window_layout(&self) -> WindowLayout {
        self.windows.layout(self.window_area())
    }

    pub(crate) fn set_screen_size(&mut self, cols: u16, rows: u16) {
        self.screen_width = usize::from(cols).max(1);
        self.screen_height = usize::from(rows).max(2);
        self.close_explorer_if_unfit();
        self.refresh_window_sizes();
    }

    pub(crate) fn refresh_window_sizes(&mut self) {
        let layout = self.window_layout();
        let active = self.windows.active();
        let Some((_, rect)) = layout.panes.iter().find(|(id, _)| *id == active) else {
            return;
        };
        self.editor
            .set_view_size(rect.width.max(1), pane_content_height(*rect).max(1));
    }

    pub(crate) fn split_window(&mut self, vertical: bool, name: Option<String>) -> Result<bool> {
        self.open_split(vertical, name, false)
    }

    pub(crate) fn new_window(&mut self, vertical: bool, name: Option<String>) -> Result<bool> {
        self.open_split(vertical, name, true)
    }

    pub(crate) fn open_readonly_window(&mut self, name: &str, text: &str) -> Result<bool> {
        self.begin_split();
        self.editor.open_scratch_buffer(name, text, true);
        self.complete_split(false)
    }

    pub(crate) fn open_internal_window(&mut self, name: &str, text: &str) -> Result<usize> {
        self.begin_split();
        let number = self.editor.open_internal_buffer(name, text);
        self.complete_split(false)?;
        Ok(number)
    }

    pub(crate) fn open_buffer_window(&mut self, number: usize) -> Result<bool> {
        self.begin_split();
        if !self.editor.buffer_select(number) {
            return Ok(false);
        }
        self.complete_split(false)
    }

    fn open_split(
        &mut self,
        vertical: bool,
        name: Option<String>,
        new_buffer: bool,
    ) -> Result<bool> {
        let previous = self.begin_split();
        if let Some(name) = name {
            if let Err(error) = self.editor.edit_file(Some(&name), false) {
                self.window_syntax.remove(&previous);
                self.show_message(super::super::file_error_message(error));
                return Ok(false);
            }
        } else if new_buffer {
            self.editor.open_empty_buffer();
        }
        self.complete_split(vertical)
    }

    fn begin_split(&mut self) -> WindowId {
        self.sync_active_window();
        let previous = self.windows.active();
        self.stash_active_syntax_copy(previous);
        self.exit_visual_if_active();
        previous
    }

    fn complete_split(&mut self, vertical: bool) -> Result<bool> {
        let view = self.editor.view_state();
        let buffer = self.editor.buffer_number();
        let direction = if vertical {
            SplitDirection::Vertical
        } else {
            SplitDirection::Horizontal
        };
        let options = self.editor.window_options();
        self.windows
            .split_active(direction, WindowView::with_options(buffer, view, options));
        self.refresh_window_sizes();
        self.reveal_cursor();
        Ok(false)
    }

    pub(crate) fn focus_window(&mut self, id: WindowId) -> bool {
        if id == self.windows.active() {
            return false;
        }
        let Some(target) = self.windows.view(id).copied() else {
            return false;
        };
        self.sync_active_window();
        let previous = self.windows.active();
        self.exit_visual_if_active();
        self.dismiss_lsp_menu();
        self.stash_active_syntax(previous);
        if target.buffer != self.editor.buffer_number() && !self.editor.buffer_select(target.buffer)
        {
            self.restore_window_syntax(previous);
            return false;
        }
        self.sync_directory_metadata_width();
        if !self.windows.set_active(id) {
            self.restore_window_syntax(previous);
            return false;
        }
        self.restore_window_syntax(id);
        self.refresh_window_sizes();
        self.editor.restore_view_state(target.view);
        self.editor.set_window_options(target.options);
        self.reveal_cursor();
        true
    }

    fn restore_active_window(&mut self) {
        let id = self.windows.active();
        let target = *self.windows.active_view();
        self.restore_window_syntax(id);
        if target.buffer != self.editor.buffer_number() {
            let _ = self.editor.buffer_select(target.buffer);
        }
        self.sync_directory_metadata_width();
        self.refresh_window_sizes();
        self.editor.restore_view_state(target.view);
        self.editor.set_window_options(target.options);
        self.reveal_cursor();
    }

    pub(crate) fn focus_window_direction(&mut self, direction: FocusDirection) -> bool {
        let area = self.window_area();
        let Some(target) = self.windows.focus_direction(direction, area) else {
            return false;
        };
        self.focus_window(target)
    }

    pub(crate) fn focus_next_window(&mut self, reverse: bool) -> bool {
        let area = self.window_area();
        let Some(target) = self.windows.focus_next(area, reverse) else {
            return false;
        };
        self.focus_window(target)
    }

    pub(crate) fn close_active_window(&mut self, force: bool) -> Result<bool> {
        if self.windows.len() <= 1 {
            self.show_message("E444: Cannot close last window".to_string());
            return Ok(false);
        }
        if self.editor.is_dirty() && !force {
            self.show_message("E37: No write since last change (add ! to override)".to_string());
            return Ok(false);
        }
        self.sync_active_window();
        let closing = self.windows.active();
        self.exit_visual_if_active();
        let buffer = self.editor.buffer_number();
        self.finish_git_editor_for_buffer(buffer);
        let remaining = self.windows.count_buffer(buffer).saturating_sub(1);
        if force && remaining == 0 && self.editor.is_dirty() {
            self.editor.discard_current_changes();
            self.clear_recovery_buffer(buffer);
        }
        if self.windows.remove_active().is_none() {
            return Ok(false);
        }
        self.window_syntax.remove(&closing);
        if self
            .help
            .as_ref()
            .is_some_and(|help| help.window == Some(closing))
        {
            if let Some(help) = self.help.as_mut() {
                help.window = None;
            }
        }
        self.restore_active_window();
        Ok(false)
    }

    pub(crate) fn only_window(&mut self) -> Result<bool> {
        self.windows.only_active();
        self.window_syntax.clear();
        if let Some(help) = self.help.as_mut() {
            help.window = self
                .windows
                .active_view()
                .buffer
                .eq(&help.buffer)
                .then_some(self.windows.active());
        }
        self.refresh_window_sizes();
        self.reveal_cursor();
        Ok(false)
    }

    pub(crate) fn resize_active_window(&mut self, axis: ResizeAxis, grow: bool) {
        if self.windows.resize_active(axis, grow) {
            self.refresh_window_sizes();
            self.reveal_cursor();
        }
    }

    pub(crate) fn equalize_windows(&mut self) {
        if self.windows.equalize() {
            self.refresh_window_sizes();
            self.reveal_cursor();
        }
    }

    pub(crate) fn process_replayed_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.window_prefix {
            self.window_prefix = false;
            return self.handle_window_key(key);
        }
        if is_window_prefix(key, self.editor.mode()) {
            self.window_prefix = true;
            self.keymap.reset_pending();
            self.editor.apply(kawi_editor::Action::CancelPending);
            return Ok(false);
        }
        self.process_key(key)
    }

    pub(crate) fn handle_window_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('h') => {
                self.focus_window_direction(FocusDirection::Left);
                Ok(false)
            }
            KeyCode::Char('j') => {
                self.focus_window_direction(FocusDirection::Down);
                Ok(false)
            }
            KeyCode::Char('k') => {
                self.focus_window_direction(FocusDirection::Up);
                Ok(false)
            }
            KeyCode::Char('l') => {
                self.focus_window_direction(FocusDirection::Right);
                Ok(false)
            }
            KeyCode::Char('w') => {
                self.focus_next_window(false);
                Ok(false)
            }
            KeyCode::Char('W' | 'p') => {
                self.focus_next_window(true);
                Ok(false)
            }
            KeyCode::Char('s') => self.split_window(false, None),
            KeyCode::Char('n') => self.new_window(false, None),
            KeyCode::Char('v') => self.split_window(true, None),
            KeyCode::Char('c') => self.close_active_window(false),
            KeyCode::Char('q') => self.quit_or_close(false),
            KeyCode::Char('o') => self.only_window(),
            KeyCode::Char('=') => {
                self.equalize_windows();
                Ok(false)
            }
            KeyCode::Char('+') => {
                self.resize_active_window(ResizeAxis::Height, true);
                Ok(false)
            }
            KeyCode::Char('-') => {
                self.resize_active_window(ResizeAxis::Height, false);
                Ok(false)
            }
            KeyCode::Char('>') => {
                self.resize_active_window(ResizeAxis::Width, true);
                Ok(false)
            }
            KeyCode::Char('<') => {
                self.resize_active_window(ResizeAxis::Width, false);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn pane_at(&self, column: usize, row: usize) -> Option<(WindowId, Rect)> {
        self.window_layout()
            .panes
            .into_iter()
            .find(|(_, rect)| rect.contains(column, row))
    }

    fn commit_rulers(&self, buffer: usize) -> Vec<usize> {
        if self.git_commit.editor_buffer() == Some(buffer) {
            vec![50, 72]
        } else {
            Vec::new()
        }
    }

    pub(crate) fn frame_panes<'a>(&'a self, layout: &WindowLayout) -> Vec<PaneData<'a>> {
        let active = self.windows.active();
        layout
            .panes
            .iter()
            .map(|(id, rect)| {
                if *id == active {
                    self.active_pane_data(*rect)
                } else {
                    self.inactive_pane_data(*id, *rect)
                }
            })
            .collect()
    }

    fn active_syntax_spans(&self, top: usize, end: usize) -> Cow<'_, [SyntaxSpan]> {
        let buffer = self.editor.buffer_number();
        let Some(key) = self.syntax_key else {
            return Cow::Owned(Vec::new());
        };
        if syntax_key_matches(key, buffer, self.editor.buffer_revision_at(buffer))
            && syntax_window_covers(&self.syntax_window, top, end)
        {
            return Cow::Borrowed(self.syntax_spans.as_slice());
        }
        Cow::Owned(Vec::new())
    }

    fn active_pane_data<'a>(&'a self, rect: Rect) -> PaneData<'a> {
        let height = pane_content_height(rect);
        let top = self.editor.visible_top();
        let window = self.editor.visible_window(top, height);
        let rows = window.rows;
        let lines = window.lines;
        let end = rows.last().map(|row| row.saturating_add(1)).unwrap_or(top);
        let search_height = end.saturating_sub(top).max(1);
        let gutter = self.editor.gutter_lines_at(&rows);
        let preview_active = self.editor.command_preview_active();
        let search_spans = if preview_active {
            Vec::new()
        } else {
            frame_search_spans(&self.editor.search_spans(top, search_height))
        };
        let preview_spans = if preview_active {
            frame_search_spans(&self.editor.preview_spans(top, search_height))
        } else {
            Vec::new()
        };
        let path = self.problem_path();
        let left_width = self.directory_metadata_width(self.editor.buffer_number());
        let virtual_texts = self.pane_virtual_texts(
            self.editor.buffer_number(),
            crate::app::diagnostic_ui::VirtualTextRequest {
                path: path.as_deref(),
                rows: &rows,
                cursor_row: self.editor.row(),
                cursor_column: self.editor.col(),
                suppress: self.virtual_text_suppressed(),
            },
        );
        let buffer = self.editor.buffer_number();
        let decorations = self.plugin_decorations.spans(buffer, &rows);
        let plugin_virtual_texts = self.plugin_decorations.virtual_texts(buffer, &rows);
        PaneData {
            rect,
            active: true,
            signs: self.view_signs_for_rows(&rows),
            diagnostic_spans: self.lsp.diagnostic_spans(&lines, &rows),
            decorations,
            virtual_texts,
            plugin_virtual_texts,
            left_width,
            lines,
            gutter,
            search_spans,
            preview_spans,
            syntax_spans: self.active_syntax_spans(top, end),
            rulers: self.commit_rulers(self.editor.buffer_number()),
            selection: self.frame_selection(),
            bracket_match: self
                .windows
                .active_view()
                .bracket_match
                .and_then(|cache| cache.matched),
            viewport: pane_viewport(
                rect,
                self.editor.display_row(top),
                self.editor.view_leftcol(),
                self.editor.visible_line_count(),
            ),
            cursor_col: self
                .editor
                .cursor_wcol()
                .saturating_sub(self.editor.view_leftcol()),
            cursor_screen_row: self.editor.cursor_screen_row(),
            rows,
            row: self.editor.row(),
            tabstop: self.editor.tabstop(),
            filename: self.editor.filename().to_string(),
            dirty: self.editor.is_dirty(),
            readonly: self.editor.current_buffer_is_readonly(),
            col: self.editor.col(),
            virtcol: self.editor.virtcol(),
            line_empty: !self.editor.mode().is_insert() && self.editor.cursor_line_empty(),
            empty_buffer: self.editor.is_empty(),
            mode: self.keymap.mode_label(self.editor.mode()),
            mode_kind: self.status_mode_kind(),
            lsp: self.status_lsp(),
            diagnostics: self.status_diagnostics(self.problem_path().as_deref()),
            workspace_diagnostics: self.status_workspace_diagnostics(),
            external: self.disk_status(self.editor.buffer_number()),
            filetype: self.status_filetype(),
            indent: Some(self.status_indent()),
            encoding: None,
            eol: None,
            selection_status: self.status_selection(),
            macro_recording: self.macros.recording_register(),
            percent: self.status_percent(),
        }
    }

    fn inactive_syntax_spans<'a>(
        &'a self,
        id: WindowId,
        buffer: usize,
        top: usize,
        end: usize,
    ) -> Cow<'a, [SyntaxSpan]> {
        let revision = self.editor.buffer_revision_at(buffer);
        if let Some(key) = self.syntax_key {
            if syntax_key_matches(key, buffer, revision)
                && syntax_window_covers(&self.syntax_window, top, end)
            {
                return Cow::Borrowed(self.syntax_spans.as_slice());
            }
        }
        if let Some(cache) = self.window_syntax.get(&id) {
            if syntax_key_matches(cache.key, buffer, revision)
                && syntax_window_covers(&cache.window, top, end)
            {
                return Cow::Borrowed(cache.spans.as_slice());
            }
        }
        for cache in self.window_syntax.values() {
            if syntax_key_matches(cache.key, buffer, revision)
                && syntax_window_covers(&cache.window, top, end)
            {
                return Cow::Borrowed(cache.spans.as_slice());
            }
        }
        Cow::Owned(Vec::new())
    }

    fn inactive_pane_data<'a>(&'a self, id: WindowId, rect: Rect) -> PaneData<'a> {
        let window = self
            .windows
            .view(id)
            .copied()
            .expect("pane windows carry a view");
        let height = pane_content_height(rect);
        let buffer_lines = self
            .editor
            .buffer_line_count(window.buffer)
            .unwrap_or(1)
            .max(1);
        let line_count = self
            .editor
            .buffer_visible_line_count(window.buffer)
            .unwrap_or(1)
            .max(1);
        let top = self
            .editor
            .buffer_visible_top(window.buffer, window.view.top)
            .unwrap_or(0);
        let window_data = self
            .editor
            .buffer_visible_window(window.buffer, top, height)
            .unwrap_or_default();
        let rows = window_data.rows;
        let lines = window_data.lines;
        let end = rows.last().map(|row| row.saturating_add(1)).unwrap_or(top);
        let row = self
            .editor
            .buffer_visible_top(window.buffer, window.view.row)
            .unwrap_or(0)
            .min(buffer_lines.saturating_sub(1));
        let line_len = self.editor.buffer_line_len(window.buffer, row).unwrap_or(0);
        let col = window.view.col.min(line_len.saturating_sub(1));
        let gutter = self
            .editor
            .buffer_gutter_lines_for_rows(window.buffer, &rows, row, height, window.options)
            .unwrap_or_default();
        let shared_document = window.buffer == self.editor.buffer_number();
        let diagnostic_spans = if shared_document {
            self.lsp.diagnostic_spans(&lines, &rows)
        } else {
            Vec::new()
        };
        let syntax_spans = self.inactive_syntax_spans(id, window.buffer, top, end);
        let left_width = self.directory_metadata_width(window.buffer);
        let virtual_texts = self.pane_virtual_texts(
            window.buffer,
            crate::app::diagnostic_ui::VirtualTextRequest {
                path: self.buffer_problem_path(window.buffer).as_deref(),
                rows: &rows,
                cursor_row: row,
                cursor_column: col,
                suppress: false,
            },
        );
        let decorations = self.plugin_decorations.spans(window.buffer, &rows);
        let plugin_virtual_texts = self.plugin_decorations.virtual_texts(window.buffer, &rows);
        PaneData {
            rect,
            active: false,
            signs: None,
            diagnostic_spans,
            decorations,
            virtual_texts,
            plugin_virtual_texts,
            left_width,
            lines,
            gutter,
            search_spans: Vec::new(),
            preview_spans: Vec::new(),
            syntax_spans,
            rulers: self.commit_rulers(window.buffer),
            selection: None,
            bracket_match: None,
            viewport: pane_viewport(
                rect,
                self.editor
                    .buffer_display_row(window.buffer, top)
                    .unwrap_or(0),
                window.view.leftcol,
                line_count,
            ),
            cursor_col: 0,
            cursor_screen_row: None,
            rows,
            row,
            tabstop: self
                .editor
                .buffer_tabstop(window.buffer)
                .unwrap_or_else(|| self.editor.tabstop()),
            filename: self
                .editor
                .buffer_name(window.buffer)
                .unwrap_or("[No Name]")
                .to_string(),
            dirty: self.editor.buffer_is_dirty(window.buffer).unwrap_or(false),
            readonly: self
                .editor
                .buffer_is_readonly(window.buffer)
                .unwrap_or(false),
            col,
            virtcol: self
                .editor
                .buffer_virtcol(window.buffer, row, col)
                .unwrap_or(0),
            line_empty: line_len == 0,
            empty_buffer: self.editor.buffer_is_empty(window.buffer).unwrap_or(false),
            mode: Mode::Normal.label(),
            mode_kind: ModeKind::Normal,
            lsp: None,
            diagnostics: self
                .status_diagnostics(self.buffer_problem_path(window.buffer).as_deref()),
            workspace_diagnostics: None,
            external: self.disk_status(window.buffer),
            filetype: if self.directory.contains(window.buffer) {
                Some("directory")
            } else {
                self.editor
                    .buffer_name(window.buffer)
                    .and_then(kawi_syntax::LanguageId::from_path)
                    .map(|language| language.name())
            },
            indent: None,
            encoding: None,
            eol: None,
            selection_status: None,
            macro_recording: None,
            percent: None,
        }
    }
}

fn pane_content_height(rect: Rect) -> usize {
    rect.height.saturating_sub(1)
}

fn pane_viewport(rect: Rect, top: usize, leftcol: usize, line_count: usize) -> Viewport {
    let mut viewport = Viewport::new(pane_content_height(rect), rect.width);
    viewport.place_top(top, line_count);
    viewport.set_leftcol(leftcol);
    viewport
}

fn syntax_key_matches(key: SyntaxKey, buffer: usize, revision: Option<u64>) -> bool {
    key.document == buffer as u64 && Some(key.revision) == revision
}

fn syntax_window_covers(window: &std::ops::Range<usize>, top: usize, end: usize) -> bool {
    window.start <= top && end <= window.end
}
