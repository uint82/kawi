use anyhow::{anyhow, Result};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use kawi_buffer::Buffer;
use std::collections::BTreeMap;
use std::io::stdout;
use std::path::{Path, PathBuf};

mod code_action;
mod completion;
mod config;
mod formatting;
mod frame;
mod git;
mod lsp;
mod lsp_diagnostics;
mod lsp_menu;
mod lsp_navigation;
mod lsp_text;
mod macro_codec;
mod macro_ops;
mod options;
mod picker;
mod recovery;
mod rename;
mod replace;
mod replace_apply;
mod restore;
mod window;
mod workspace_edit;

#[derive(Clone, Copy)]
enum MarkOperator {
    Delete,
    Change,
    Yank,
}

enum Redraw {
    Quit,
    Redraw,
}

struct SyntaxCache {
    spans: Vec<SyntaxSpan>,
    window: std::ops::Range<usize>,
    key: SyntaxKey,
}

fn preferred_sign(
    git: Option<kawi_tui::frame::Sign>,
    diagnostic: Option<kawi_tui::frame::DiagnosticSeverity>,
) -> Option<kawi_tui::frame::Sign> {
    diagnostic.map(kawi_tui::frame::Sign::Diagnostic).or(git)
}

fn diagnostic_label(severity: kawi_lsp::DiagnosticSeverity) -> &'static str {
    match severity {
        kawi_lsp::DiagnosticSeverity::Error => "Error",
        kawi_lsp::DiagnosticSeverity::Warning => "Warning",
        kawi_lsp::DiagnosticSeverity::Information => "Information",
        kawi_lsp::DiagnosticSeverity::Hint => "Hint",
    }
}

fn ui_diagnostic_severity(
    severity: kawi_lsp::DiagnosticSeverity,
) -> kawi_tui::frame::DiagnosticSeverity {
    match severity {
        kawi_lsp::DiagnosticSeverity::Error => kawi_tui::frame::DiagnosticSeverity::Error,
        kawi_lsp::DiagnosticSeverity::Warning => kawi_tui::frame::DiagnosticSeverity::Warning,
        kawi_lsp::DiagnosticSeverity::Information => {
            kawi_tui::frame::DiagnosticSeverity::Information
        }
        kawi_lsp::DiagnosticSeverity::Hint => kawi_tui::frame::DiagnosticSeverity::Hint,
    }
}

use crate::event::EventQueue;
use crate::keymap::{ActiveKeymap, Flavor, Keymap};
use crate::mouse::{horizontal_wheel_delta, wheel_delta};
use crate::state::{self, StatePaths};
use git::{GitSession, GitStage};
use kawi_editor::{
    parse, Action, Command, Editor, GitCommand, MatchSpan, PickerCommand, SearchOutcome,
    WindowOptions, FALLBACK_HEIGHT, FALLBACK_WIDTH,
};
use kawi_picker::PickerKind;
use kawi_syntax::{SyntaxKey, SyntaxResult, SyntaxService, SyntaxSpan};
use kawi_tui::{frame::SearchSpan, terminal, theme::Theme};
use lsp::LspUi;
use picker::PickerSession;
use replace::{is_replace_key, ReplaceOutcome, ReplaceSession, ReplaceStage};
use window::Windows;

pub struct App {
    editor: Editor,
    keymap: ActiveKeymap,
    flavor: Flavor,
    theme: Theme,
    message: Option<String>,
    pager: Option<Vec<String>>,
    macros: macro_ops::MacroEngine,
    last_command: Option<String>,
    syntax: Option<SyntaxService>,
    syntax_spans: Vec<SyntaxSpan>,
    syntax_window: std::ops::Range<usize>,
    syntax_key: Option<SyntaxKey>,
    syntax_pending: bool,
    syntax_latest: u64,
    syntax_applied: u64,
    window_syntax: BTreeMap<window::WindowId, SyntaxCache>,
    picker: Option<PickerSession>,
    replace: Option<ReplaceSession>,
    rename: Option<rename::RenameSession>,
    completion: Option<completion::CompletionSession>,
    code_actions: Option<code_action::CodeActionSession>,
    git: Option<GitSession>,
    git_path: Option<PathBuf>,
    git_error: Option<String>,
    lsp: LspUi,
    windows: Windows,
    screen_width: usize,
    screen_height: usize,
    window_prefix: bool,
    events: EventQueue,
    recovery: Option<crate::recovery::RecoveryManager>,
    recovery_prompt: Option<recovery::RecoveryPrompt>,
    window_defaults: WindowOptions,
}

impl App {
    pub fn new(buffer: Buffer, flavor: Flavor) -> Self {
        let mut editor = Editor::new(buffer);
        editor.set_clipboard(Box::new(kawi_tui::clipboard::SystemClipboard::new()));
        let windows = Windows::new(editor.buffer_number(), editor.view_state());
        Self {
            editor,
            keymap: ActiveKeymap::for_flavor(flavor),
            flavor,
            theme: Theme::default(),
            message: None,
            pager: None,
            macros: macro_ops::MacroEngine::new(),
            last_command: None,
            syntax: None,
            syntax_spans: Vec::new(),
            syntax_window: 0..0,
            syntax_key: None,
            syntax_pending: false,
            syntax_latest: 0,
            syntax_applied: 0,
            window_syntax: BTreeMap::new(),
            picker: None,
            replace: None,
            rename: None,
            completion: None,
            code_actions: None,
            git: None,
            git_path: None,
            git_error: None,
            lsp: LspUi::default(),
            windows,
            screen_width: FALLBACK_WIDTH,
            screen_height: FALLBACK_HEIGHT.saturating_add(2),
            window_prefix: false,
            events: EventQueue::new(),
            recovery: None,
            recovery_prompt: None,
            window_defaults: WindowOptions::default(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        crate::recovery::install_panic_hook();
        self.sync_viewport_size();
        self.events.start_input_thread()?;
        self.events.start_signal_handlers()?;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_loop()));
        match outcome {
            Ok(result) => {
                if result.is_err() || self.events.shutdown_requested() {
                    self.finish_recovery_abnormal();
                } else {
                    self.finish_recovery_normal();
                }
                result
            }
            Err(payload) => {
                self.finish_recovery_abnormal();
                std::panic::resume_unwind(payload);
            }
        }
    }

    pub fn save_state(&self) {
        let Some(paths) = StatePaths::resolve() else {
            return;
        };
        let _ = state::save(&self.editor, &paths);
    }

    pub(crate) fn show_startup_warning(&mut self, warning: String) {
        let warning = match self.message.take() {
            Some(existing) if !existing.is_empty() => format!("{existing}\n{warning}"),
            _ => warning,
        };
        self.show_message(warning);
    }

    #[cfg(test)]
    pub fn editor_mode(&self) -> kawi_editor::Mode {
        self.editor.mode()
    }

    #[cfg(test)]
    pub fn editor_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.editor.selection_range()
    }

    #[cfg(test)]
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.editor.row(), self.editor.col())
    }

    #[cfg(test)]
    pub fn viewport_top(&self) -> usize {
        self.editor.view_top()
    }

    #[cfg(test)]
    pub fn last_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    #[cfg(test)]
    pub fn pager_lines(&self) -> Option<&[String]> {
        self.pager.as_deref()
    }

    #[cfg(test)]
    pub fn search_status_line(&self) -> Option<String> {
        self.editor.search_status_text()
    }

    #[cfg(test)]
    pub fn theme_name(&self) -> &str {
        self.theme.name
    }

    fn run_loop(&mut self) -> Result<()> {
        let mut out = stdout();
        loop {
            self.sync_active_window();
            self.show_recovery_prompt_if_ready();
            self.sync_git();
            self.sync_lsp();
            if self.picker.is_none() && self.replace.is_none() {
                self.refresh_syntax();
            }
            self.refresh_picker();
            self.refresh_replace();
            self.draw_frame(&mut out)?;
            if matches!(self.wait_for_redraw()?, Redraw::Quit) {
                return Ok(());
            }
        }
    }

    fn wait_for_redraw(&mut self) -> Result<Redraw> {
        loop {
            if self.events.shutdown_requested() {
                return Ok(Redraw::Quit);
            }
            let mut redraw = false;
            for event in self.events.take_input() {
                let needs_redraw = event_needs_redraw(&event);
                if self.update(event)? {
                    return Ok(Redraw::Quit);
                }
                redraw |= needs_redraw;
            }
            redraw |= self.lsp.poll_events(&self.editor);
            self.update_signcolumn_width();
            redraw |= self.lsp.update_display();
            if self.picker.is_some() || self.replace.is_some() {
                redraw |= self.refresh_picker() | self.refresh_replace();
            } else {
                redraw |= self.apply_syntax_results();
            }
            redraw |= self.recovery_tick();
            if redraw {
                return Ok(Redraw::Redraw);
            }
            if self.events.input_closed() {
                return Err(anyhow!("terminal input closed"));
            }
            match self.recovery_wait_timeout() {
                Some(timeout) => self.events.wait_timeout(timeout)?,
                None => self.events.wait()?,
            }
        }
    }

    fn refresh_syntax(&mut self) {
        let Some(key) = self.editor.syntax_key() else {
            if self.syntax_key.is_some() {
                self.syntax_spans.clear();
                self.syntax_window = 0..0;
            }
            self.syntax_key = None;
            self.syntax_pending = false;
            self.apply_syntax_results();
            return;
        };
        let same_request = self.syntax_key.is_some_and(|submitted| {
            submitted.document == key.document
                && submitted.revision == key.revision
                && submitted.language == key.language
                && submitted.top == key.top
                && submitted.end == key.end
        });
        let same_document = self.syntax_key.is_some_and(|submitted| {
            submitted.document == key.document
                && submitted.revision == key.revision
                && submitted.language == key.language
        });
        let covered = self.syntax_window.start <= key.top && key.end <= self.syntax_window.end;
        if (self.syntax_pending && same_request) || (same_document && covered) {
            self.apply_syntax_results();
            return;
        }
        let document_changed = self.syntax_key.is_none_or(|submitted| {
            submitted.document != key.document || submitted.language != key.language
        });
        if document_changed {
            self.syntax_spans.clear();
            self.syntax_window = 0..0;
        }
        if let Some(request) = self.editor.syntax_request() {
            let wake = self.events.wake();
            let service = self
                .syntax
                .get_or_insert_with(|| SyntaxService::with_wake(wake));
            self.syntax_latest = service.submit(request);
            self.syntax_pending = true;
            self.syntax_key = Some(key);
        } else {
            self.syntax_pending = false;
        }
        self.apply_syntax_results();
    }

    fn apply_syntax_results(&mut self) -> bool {
        let mut results = Vec::new();
        if let Some(service) = self.syntax.as_ref() {
            while let Some(result) = service.try_recv() {
                results.push(result);
            }
        }
        let mut changed = false;
        for result in results {
            changed |= self.apply_syntax_result(result);
        }
        changed
    }

    fn apply_syntax_result(&mut self, result: SyntaxResult) -> bool {
        let latest = result.generation == self.syntax_latest;
        let mut changed = latest;
        if latest {
            self.syntax_pending = false;
        }
        let Some(current) = self.editor.syntax_key() else {
            return changed;
        };
        if result.key.document == current.document
            && result.key.revision == current.revision
            && result.key.language == current.language
            && result.generation >= self.syntax_applied
            && result.window.start <= current.top
            && current.end <= result.window.end
        {
            self.syntax_spans = result.spans;
            self.syntax_window = result.window;
            self.syntax_applied = result.generation;
            changed = true;
        }
        changed
    }

    fn view_signs(&self) -> Option<Vec<Option<kawi_tui::frame::Sign>>> {
        let top = self.editor.view_top();
        let height = self.editor.view_height();
        let git = self.git_view_signs(top, height);
        if git.is_none() && !self.lsp.has_diagnostics() {
            return None;
        }
        let mut signs = git.unwrap_or_else(|| vec![None; height]);
        self.apply_diagnostic_signs(&mut signs, top);
        Some(signs)
    }

    fn git_view_signs(
        &self,
        top: usize,
        height: usize,
    ) -> Option<Vec<Option<kawi_tui::frame::Sign>>> {
        self.git.as_ref().map(|session| {
            session
                .visible_signs(top, height)
                .into_iter()
                .map(|sign| sign.map(kawi_tui::frame::Sign::Git))
                .collect()
        })
    }

    fn apply_diagnostic_signs(&self, signs: &mut [Option<kawi_tui::frame::Sign>], top: usize) {
        for (offset, sign) in signs.iter_mut().enumerate() {
            let diagnostic = self
                .lsp
                .diagnostic_severity(top.saturating_add(offset))
                .map(ui_diagnostic_severity);
            *sign = preferred_sign(*sign, diagnostic);
        }
    }

    fn showcmd_text(&self) -> Option<String> {
        let pending = if self.window_prefix {
            Some("^W")
        } else {
            self.keymap.pending_text()
        };
        match (self.macros.recording_register(), pending) {
            (Some(register), Some(pending)) => Some(format!("recording @{register} {pending}")),
            (Some(register), None) => Some(format!("recording @{register}")),
            (None, pending) => pending.map(str::to_string),
        }
    }

    fn show_message(&mut self, text: String) {
        let lines = pager_lines(&text, self.screen_width);
        if lines.len() > 1 {
            self.show_pager(lines);
        } else {
            self.pager = None;
            self.message = Some(text);
        }
    }

    fn show_pager(&mut self, lines: Vec<String>) {
        self.message = None;
        self.pager = Some(lines);
    }

    fn clear_message(&mut self) {
        self.message = None;
        self.pager = None;
    }

    #[cfg(test)]
    fn status_snapshot(&self) -> kawi_tui::statusbar::StatusInfo<'_> {
        kawi_tui::statusbar::StatusInfo {
            filename: self.editor.filename(),
            dirty: self.editor.is_dirty(),
            row: self.editor.row(),
            col: self.editor.col(),
            virtcol: self.editor.virtcol(),
            line_empty: !self.editor.mode().is_insert() && self.editor.cursor_line_empty(),
            empty_buffer: self.editor.is_empty(),
            mode: self.keymap.mode_label(self.editor.mode()),
            lsp: self.lsp.status(),
        }
    }

    fn sync_lsp(&mut self) {
        self.lsp.refresh(&self.editor);
        self.update_signcolumn_width();
        if let Some(message) = self.lsp.take_message() {
            self.show_message(message);
        }
        if let Some(result) = self.lsp.take_location() {
            self.handle_location_result(result);
        }
        if let Some(hover) = self.lsp.take_hover() {
            self.handle_hover_result(hover);
        }
        if let Some(event) = self.lsp.take_completion() {
            self.handle_completion_event(event);
        }
        if let Some(event) = self.lsp.take_code_actions() {
            self.handle_code_action_event(event);
        }
        self.handle_formatting_result();
        self.sync_rename();
    }

    fn update_signcolumn_width(&mut self) {
        let width = usize::from(self.git.is_some() || self.lsp.has_diagnostics());
        self.editor.set_signcolumn_width(width);
    }

    fn update(&mut self, event: Event) -> Result<bool> {
        let result = match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(cols, rows) => self.handle_resize(cols, rows),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::Paste(text) => self.handle_paste(text),
            _ => Ok(false),
        };
        self.sync_active_window();
        result
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<bool> {
        self.window_prefix = false;
        if self.picker.is_some()
            || self.replace.is_some()
            || self.rename.is_some()
            || self.completion.is_some()
            || self.code_actions.is_some()
        {
            self.dismiss_lsp_menu();
            return self.handle_noop();
        }
        if self.pager.is_some() {
            return self.handle_noop();
        }
        let column = usize::from(mouse.column);
        let screen_row = usize::from(mouse.row);
        let Some((id, rect)) = self.pane_at(column, screen_row) else {
            return self.handle_noop();
        };
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !self.focus_mouse_pane(id) {
                    return self.handle_noop();
                }
                if screen_row >= rect.y.saturating_add(rect.height.saturating_sub(1)) {
                    return self.handle_noop();
                }
                let (row, col) = self.mouse_position(rect, column, screen_row);
                self.jump_to(row, col)
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if id != self.windows.active()
                    || screen_row >= rect.y.saturating_add(rect.height.saturating_sub(1))
                {
                    return self.handle_noop();
                }
                let (row, col) = self.mouse_position(rect, column, screen_row);
                self.drag_to(row, col)
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                if !self.focus_mouse_pane(id) {
                    return self.handle_noop();
                }
                match wheel_delta(&mouse) {
                    Some(delta) => self.scroll_by(delta),
                    None => self.handle_noop(),
                }
            }
            MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => {
                if !self.focus_mouse_pane(id) {
                    return self.handle_noop();
                }
                let page = mouse
                    .modifiers
                    .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL);
                match horizontal_wheel_delta(&mouse) {
                    Some(delta) => self.scroll_horizontally(delta, page),
                    None => self.handle_noop(),
                }
            }
            _ => self.handle_noop(),
        }
    }

    fn focus_mouse_pane(&mut self, id: window::WindowId) -> bool {
        if id == self.windows.active() {
            return true;
        }
        self.focus_window(id)
    }

    fn mouse_position(
        &self,
        rect: kawi_tui::frame::Rect,
        column: usize,
        screen_row: usize,
    ) -> (usize, usize) {
        let local_row = screen_row
            .saturating_sub(rect.y)
            .min(rect.height.saturating_sub(2));
        let row = self.editor.view_top().saturating_add(local_row);
        let screen_col = column.saturating_sub(rect.x);
        let col = self.editor.buffer_col_at_screen(row, screen_col);
        (row, col)
    }

    fn jump_to(&mut self, row: usize, col: usize) -> Result<bool> {
        self.exit_visual_if_active();
        self.editor.apply(Action::JumpTo(row, col));
        self.reveal_cursor();
        Ok(false)
    }

    fn exit_visual_if_active(&mut self) {
        if self.editor.mode().is_visual() {
            self.editor.apply(Action::ExitVisual);
        }
    }

    fn drag_to(&mut self, row: usize, col: usize) -> Result<bool> {
        self.ensure_visual();
        self.editor.apply(Action::JumpTo(row, col));
        self.reveal_cursor();
        Ok(false)
    }

    fn scroll_by(&mut self, delta: isize) -> Result<bool> {
        self.editor.scroll_wheel(delta);
        Ok(false)
    }

    fn scroll_horizontally(&mut self, delta: isize, page: bool) -> Result<bool> {
        self.editor.scroll_wheel_horizontal(delta, page);
        Ok(false)
    }

    fn ensure_visual(&mut self) {
        if !self.editor.mode().is_visual() {
            self.editor.apply(Action::EnterVisual);
        }
    }

    fn handle_resize(&mut self, cols: u16, rows: u16) -> Result<bool> {
        self.set_viewport_size(cols, rows);
        self.reveal_cursor();
        Ok(false)
    }

    fn set_viewport_size(&mut self, cols: u16, rows: u16) {
        self.set_screen_size(cols, rows);
    }

    fn sync_viewport_size(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.set_viewport_size(cols, rows);
        }
    }

    fn reveal_cursor(&mut self) {
        self.editor.reveal_cursor();
        self.editor.reveal_cursor_horizontally();
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        let result = self.handle_key_inner(key);
        self.sync_active_window();
        result
    }

    fn handle_key_inner(&mut self, key: KeyEvent) -> Result<bool> {
        if self.recovery_prompt_visible() {
            return self.handle_recovery_prompt_key(key);
        }
        if self.rename.is_some() {
            return self.handle_rename_key(key);
        }
        if self.replace.is_some() {
            return self.handle_replace_key(key);
        }
        if self.picker.is_some() {
            return self.handle_picker_key(key);
        }
        if self.code_actions.is_some() {
            return self.handle_code_action_key(key);
        }
        if self.completion.is_some() {
            if let Some(result) = self.handle_completion_key(key)? {
                return Ok(result);
            }
        }
        if self.pager.is_none() && completion::is_completion_key(key) {
            return self.request_completion();
        }
        if self.window_prefix {
            self.window_prefix = false;
            self.macros.record_key(key);
            return self.handle_window_key(key);
        }
        if self.pager.is_none() && window::is_window_prefix(key, self.editor.mode()) {
            self.window_prefix = true;
            self.keymap.reset_pending();
            self.editor.apply(Action::CancelPending);
            self.macros.record_key(key);
            return Ok(false);
        }
        if !self.stops_macro_recording(key) {
            self.macros.record_key(key);
        }
        self.process_key(key)
    }

    fn handle_picker_key(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(mut session) = self.picker.take() else {
            return Ok(false);
        };
        if is_replace_key(key) && session.picker().kind() == PickerKind::Grep {
            let query = session.picker().query().to_string();
            if query.is_empty() {
                self.show_message("Type a search pattern first".to_string());
                self.picker = Some(session);
                return Ok(false);
            }
            self.replace = Some(ReplaceSession::from_query(
                session.root().to_path_buf(),
                &query,
                self.events.wake(),
            ));
            return Ok(false);
        }
        let outcome = session.handle_key(key);
        match outcome {
            kawi_picker::PickerOutcome::Cancel => Ok(false),
            kawi_picker::PickerOutcome::Open(row) => {
                match self.open_picker_row(&row, session.root()) {
                    Ok(()) => Ok(false),
                    Err(message) => {
                        self.show_message(message);
                        self.picker = Some(session);
                        Ok(false)
                    }
                }
            }
            _ => {
                self.picker = Some(session);
                Ok(false)
            }
        }
    }

    fn handle_replace_key(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(mut session) = self.replace.take() else {
            return Ok(false);
        };
        match session.handle_key(key) {
            ReplaceOutcome::Cancel => Ok(false),
            ReplaceOutcome::Consumed => {
                self.replace = Some(session);
                Ok(false)
            }
            ReplaceOutcome::Apply(preview) => {
                let root = session.root().clone();
                match self.apply_replace_preview(&preview, &root) {
                    Ok(()) => {
                        self.show_message(format!(
                            "Replaced {} in {}",
                            kawi_tui::text::count_label(preview.matches, "match", "matches"),
                            kawi_tui::text::count_label(preview.files.len(), "file", "files")
                        ));
                        self.reveal_cursor();
                        Ok(false)
                    }
                    Err(message) => {
                        session.set_status(message);
                        self.replace = Some(session);
                        Ok(false)
                    }
                }
            }
        }
    }

    fn open_picker_row(&mut self, row: &kawi_picker::PickerRow, root: &Path) -> Result<(), String> {
        if let Some(number) = row.buffer {
            if !self.editor.buffer_select(number) {
                return Err(format!("E86: Buffer {number} does not exist"));
            }
            self.reveal_cursor();
            return Ok(());
        }
        let path = root.join(&row.path);
        self.open_file_location(
            &path,
            row.line.map(|line| line.saturating_sub(1)),
            row.column.map(|column| column.saturating_sub(1)),
        )
    }

    fn process_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.pager.is_some() {
            return self.handle_pager_key(key);
        }
        if key.code == KeyCode::Esc
            && self.editor.mode() == kawi_editor::Mode::Normal
            && self.lsp.cancel_pending()
        {
            return Ok(false);
        }
        if self.stops_macro_recording(key) {
            self.consume_macro_stop();
            return Ok(false);
        }
        let action = self.keymap.map(key, self.editor.mode());
        self.dispatch_action(action)
    }

    fn handle_pager_key(&mut self, key: KeyEvent) -> Result<bool> {
        self.pager = None;
        if is_pager_dismiss(key) {
            return Ok(false);
        }
        let action = self.keymap.map(key, self.editor.mode());
        self.dispatch_action(action)
    }

    fn dispatch_action(&mut self, action: Action) -> Result<bool> {
        match action {
            Action::Quit => self.handle_quit(),
            Action::Save => self.handle_save(false),
            Action::ExecuteCommand => self.handle_execute(),
            Action::Undo => self.handle_undo(),
            Action::Redo => self.handle_redo(),
            Action::NextMatch => self.handle_next_match(),
            Action::PrevMatch => self.handle_prev_match(),
            Action::ConfirmSearch => self.handle_confirm_search(),
            Action::SearchWordForward => self.handle_search_word(true),
            Action::SearchWordBackward => self.handle_search_word(false),
            Action::JumpOlder => self.handle_jump(true),
            Action::JumpNewer => self.handle_jump(false),
            Action::JumpToMark { name, line_wise } => self.handle_mark_jump(name, line_wise),
            Action::JumpToPrevious { line_wise } => self.handle_previous_jump(line_wise),
            Action::DeleteToMark { name, line_wise } => {
                self.handle_mark_operator(MarkOperator::Delete, name, line_wise)
            }
            Action::ChangeToMark { name, line_wise } => {
                self.handle_mark_operator(MarkOperator::Change, name, line_wise)
            }
            Action::YankToMark { name, line_wise } => {
                self.handle_mark_operator(MarkOperator::Yank, name, line_wise)
            }
            Action::EditAlternate => self.handle_alternate(),
            Action::OpenFilePicker => self.open_picker(PickerKind::Files),
            Action::OpenGrepPicker => self.open_picker(PickerKind::Grep),
            Action::OpenGrepWordPicker => self.open_grep_word_picker(),
            Action::OpenBufferPicker => self.open_picker(PickerKind::Buffers),
            Action::OpenReplacePicker => self.open_replace_picker(),
            Action::GitNextHunk => self.handle_git_navigation(true),
            Action::GitPrevHunk => self.handle_git_navigation(false),
            Action::DiagnosticNext => self.handle_diagnostic_navigation(true),
            Action::DiagnosticPrev => self.handle_diagnostic_navigation(false),
            Action::GotoDefinition => self.request_definition(),
            Action::ShowReferences => self.request_references(),
            Action::Hover => self.request_hover(),
            Action::Rename => self.request_rename(),
            Action::CodeAction => self.request_code_actions(),
            Action::Format => self.request_format(),
            Action::GitStageHunk => self.handle_git_stage_hunk(),
            Action::GitUndoStage => self.handle_git_undo_stage(),
            Action::GitStageBuffer => self.handle_git_stage_buffer(),
            Action::GitPreviewHunk => self.handle_git_preview_hunk(),
            Action::GitDiff => self.handle_git_diff(),
            Action::Noop => self.handle_noop(),
            Action::StartMacroRecording(register) => {
                self.start_macro_recording(register);
                Ok(false)
            }
            Action::RunMacro(register) => self.run_macro(register),
            Action::RepeatLastMacro => self.repeat_last_macro(),
            Action::RepeatRecordedMacro => self.repeat_recorded_macro(),
            Action::ExecuteLastCommand => self.execute_last_command(),
            Action::MacroRegisterError(register) => {
                self.report_macro_register_error(register);
                Ok(false)
            }
            _ => self.handle_engine_action(action),
        }
    }

    fn handle_save(&mut self, force: bool) -> Result<bool> {
        if !self.editor.has_file() {
            self.show_message("E32: No file name".to_string());
            return Ok(false);
        }
        self.save_and_report(force);
        Ok(false)
    }

    fn save_and_report(&mut self, force: bool) -> bool {
        match self.editor.save(force) {
            Ok(outcome) => {
                let report = written_report(self.editor.filename(), outcome);
                self.lsp.save();
                self.show_message(report);
                true
            }
            Err(error) => self.report_error(file_error_message(error)),
        }
    }

    fn write_as_and_report(&mut self, name: &str, force: bool) -> bool {
        match self.editor.write_as(name, force) {
            Ok(outcome) => {
                let report = written_report(name, outcome);
                self.show_message(report);
                true
            }
            Err(error) => self.report_error(file_error_message(error)),
        }
    }

    fn report_error(&mut self, text: String) -> bool {
        self.show_message(text);
        false
    }

    fn handle_undo(&mut self) -> Result<bool> {
        if !self.editor.try_undo() {
            self.show_message("Already at oldest change".to_string());
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_redo(&mut self) -> Result<bool> {
        if !self.editor.try_redo() {
            self.show_message("Already at newest change".to_string());
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_next_match(&mut self) -> Result<bool> {
        let outcome = self.editor.next_match();
        self.report_outcome(Some(outcome));
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_prev_match(&mut self) -> Result<bool> {
        let outcome = self.editor.prev_match();
        self.report_outcome(Some(outcome));
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_confirm_search(&mut self) -> Result<bool> {
        let outcome = self.editor.confirm_search();
        self.report_outcome(outcome);
        self.reveal_cursor();
        Ok(false)
    }

    fn report_outcome(&mut self, outcome: Option<SearchOutcome>) {
        match outcome {
            None | Some(SearchOutcome::Found { .. }) => self.clear_message(),
            Some(SearchOutcome::Missing) => self.show_message(missing_message(&self.editor)),
        }
    }

    fn handle_noop(&mut self) -> Result<bool> {
        Ok(false)
    }

    fn dismiss_lsp_menu(&mut self) {
        if self.completion.take().is_some() {
            self.lsp.cancel_completion();
        }
        if self.code_actions.take().is_some() {
            self.lsp.cancel_code_actions();
        }
    }

    fn refresh_picker(&mut self) -> bool {
        self.picker.as_mut().is_some_and(PickerSession::refresh)
    }

    fn refresh_replace(&mut self) -> bool {
        self.replace.as_mut().is_some_and(ReplaceSession::refresh)
    }

    fn open_picker(&mut self, kind: PickerKind) -> Result<bool> {
        let root = self.picker_root();
        let wake = self.events.wake();
        self.picker = Some(match kind {
            PickerKind::Files => PickerSession::files(root, wake),
            PickerKind::Grep => PickerSession::grep(root, None, wake),
            PickerKind::Buffers => PickerSession::buffers(root, self.buffer_picker_rows()),
            PickerKind::Locations => PickerSession::locations(root, "Locations", Vec::new()),
        });
        Ok(false)
    }

    fn open_replace_picker(&mut self) -> Result<bool> {
        let root = self.picker_root();
        self.replace = Some(ReplaceSession::new(root, self.events.wake()));
        Ok(false)
    }

    fn open_grep_word_picker(&mut self) -> Result<bool> {
        let Some(word) = self.editor.word_under_cursor() else {
            self.show_message("E348: No string under cursor".to_string());
            return Ok(false);
        };
        let root = self.picker_root();
        self.picker = Some(PickerSession::grep(root, Some(&word), self.events.wake()));
        Ok(false)
    }

    fn picker_root(&self) -> std::path::PathBuf {
        let start = if self.editor.has_file() {
            Path::new(self.editor.filename())
                .parent()
                .map(Path::to_path_buf)
        } else {
            None
        };
        let start = start.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        kawi_picker::project_root(&start)
    }

    fn buffer_picker_rows(&self) -> Vec<(usize, String)> {
        self.editor
            .buffer_numbers()
            .into_iter()
            .filter_map(|number| {
                self.editor
                    .buffer_name(number)
                    .map(|name| (number, name.to_string()))
            })
            .collect()
    }

    fn handle_pick_command(&mut self, command: PickerCommand) -> Result<bool> {
        match command {
            PickerCommand::Files => self.open_picker(PickerKind::Files),
            PickerCommand::Grep => self.open_picker(PickerKind::Grep),
            PickerCommand::Buffers => self.open_picker(PickerKind::Buffers),
            PickerCommand::Replace => self.open_replace_picker(),
        }
    }

    fn handle_paste(&mut self, text: String) -> Result<bool> {
        self.window_prefix = false;
        if self.paste_rename(&text) {
            return Ok(false);
        }
        if let Some(session) = self.replace.as_mut() {
            session.paste(&text);
            return Ok(false);
        }
        if let Some(session) = self.picker.as_mut() {
            session.paste(&text);
            return Ok(false);
        }
        if self.pager.is_some() {
            return Ok(false);
        }
        self.dismiss_lsp_menu();
        self.editor.paste(&text);
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_engine_action(&mut self, action: Action) -> Result<bool> {
        self.editor.apply(action);
        self.exit_operator_for_flavor(action);
        self.reveal_cursor();
        Ok(false)
    }

    fn exit_operator_for_flavor(&mut self, action: Action) {
        if !self.keymap.operator_exits_visual() {
            return;
        }
        if !is_operator(action) {
            return;
        }
        self.editor.apply(Action::ExitVisual);
    }

    fn handle_git_command(&mut self, command: GitCommand) -> Result<bool> {
        match command {
            GitCommand::NextHunk => self.handle_git_navigation(true),
            GitCommand::PrevHunk => self.handle_git_navigation(false),
            GitCommand::StageHunk => self.handle_git_stage_hunk(),
            GitCommand::UndoStage => self.handle_git_undo_stage(),
            GitCommand::StageBuffer => self.handle_git_stage_buffer(),
            GitCommand::PreviewHunk => self.handle_git_preview_hunk(),
            GitCommand::Diff => self.handle_git_diff(),
            GitCommand::Refresh => self.handle_git_refresh(),
        }
    }

    fn sync_git(&mut self) {
        let path = self
            .editor
            .has_file()
            .then(|| PathBuf::from(self.editor.filename()));
        if path.as_deref() != self.git_path.as_deref() {
            self.git = None;
            self.git_error = None;
            self.update_signcolumn_width();
            self.git_path = path.clone();
            let Some(path) = path else {
                return;
            };
            match kawi_git::load(&path) {
                Ok(Some(file)) => {
                    let session = GitSession::new(
                        file,
                        &self.editor.text_lines(),
                        self.editor.buffer_revision(),
                        !self.editor.text_has_final_newline(),
                    );
                    self.git = Some(session);
                    self.update_signcolumn_width();
                }
                Ok(None) => {}
                Err(error) => self.git_error = Some(error.to_string()),
            }
            return;
        }
        let revision = self.editor.buffer_revision();
        let Some(session) = self.git.as_mut() else {
            return;
        };
        let buffer_no_final_newline = !self.editor.text_has_final_newline();
        if session.revision() != revision
            || session.buffer_no_final_newline() != buffer_no_final_newline
        {
            session.update(&self.editor.text_lines(), revision, buffer_no_final_newline);
        }
    }

    fn git_unavailable_message(&self) -> String {
        if !self.editor.has_file() {
            return "E32: No file name".to_string();
        }
        self.git_error
            .clone()
            .unwrap_or_else(|| "No git repository for this file".to_string())
    }

    fn handle_git_navigation(&mut self, forward: bool) -> Result<bool> {
        let row = self.editor.row();
        let nav = self.git.as_ref().and_then(|session| {
            if forward {
                session.next(row)
            } else {
                session.prev(row)
            }
        });
        let Some(nav) = nav else {
            let message = if self.git.is_some() {
                "No hunks".to_string()
            } else {
                self.git_unavailable_message()
            };
            self.show_message(message);
            return Ok(false);
        };
        self.editor.apply(Action::JumpTo(nav.row, 0));
        self.editor.apply(Action::MoveToFirstNonBlank);
        self.reveal_cursor();
        self.show_message(format!("Hunk {} of {}", nav.index, nav.total));
        Ok(false)
    }

    fn handle_diagnostic_navigation(&mut self, forward: bool) -> Result<bool> {
        let navigation =
            self.lsp
                .navigate_diagnostics(self.editor.row(), self.editor.col(), forward);
        let Some(navigation) = navigation else {
            self.show_message("No diagnostics".to_string());
            return Ok(false);
        };
        self.editor
            .apply(Action::JumpTo(navigation.row, navigation.column));
        self.reveal_cursor();
        self.show_message(format!(
            "{} {} of {}: {}",
            diagnostic_label(navigation.severity),
            navigation.index,
            navigation.total,
            lsp::one_line(&navigation.message)
        ));
        Ok(false)
    }

    fn handle_git_stage_hunk(&mut self) -> Result<bool> {
        self.sync_git();
        let row = self.editor.row();
        let lines = self.editor.text_lines();
        let revision = self.editor.buffer_revision();
        let Some(session) = self.git.as_mut() else {
            let message = self.git_unavailable_message();
            self.show_message(message);
            return Ok(false);
        };
        match session.stage_hunk(row, &lines, revision) {
            Ok(GitStage::Staged) => self.show_message("Staged hunk".to_string()),
            Ok(GitStage::Unstaged) => self.show_message("Unstaged hunk".to_string()),
            Err(error) => self.show_message(error),
        }
        Ok(false)
    }

    fn handle_git_undo_stage(&mut self) -> Result<bool> {
        self.sync_git();
        let lines = self.editor.text_lines();
        let revision = self.editor.buffer_revision();
        let Some(session) = self.git.as_mut() else {
            let message = self.git_unavailable_message();
            self.show_message(message);
            return Ok(false);
        };
        match session.undo_stage(&lines, revision) {
            Ok(()) => self.show_message("Undid staged hunk".to_string()),
            Err(error) => self.show_message(error),
        }
        Ok(false)
    }

    fn handle_git_stage_buffer(&mut self) -> Result<bool> {
        self.sync_git();
        let lines = self.editor.text_lines();
        let revision = self.editor.buffer_revision();
        let Some(session) = self.git.as_mut() else {
            let message = self.git_unavailable_message();
            self.show_message(message);
            return Ok(false);
        };
        match session.stage_buffer(&lines, revision) {
            Ok(()) => self.show_message("Staged file".to_string()),
            Err(error) => self.show_message(error),
        }
        Ok(false)
    }

    fn handle_git_preview_hunk(&mut self) -> Result<bool> {
        let row = self.editor.row();
        let preview = self.git.as_ref().map_or_else(
            || Err(self.git_unavailable_message()),
            |session| session.preview_hunk(row),
        );
        match preview {
            Ok(lines) => self.show_pager(lines),
            Err(error) => self.show_message(error),
        }
        Ok(false)
    }

    fn handle_git_refresh(&mut self) -> Result<bool> {
        self.git = None;
        self.git_path = None;
        self.git_error = None;
        self.update_signcolumn_width();
        self.sync_git();
        Ok(false)
    }

    fn handle_git_diff(&mut self) -> Result<bool> {
        let lines = self
            .git
            .as_ref()
            .map(GitSession::diff_lines)
            .unwrap_or_default();
        if lines.is_empty() {
            let message = if self.git.is_some() {
                "No changes".to_string()
            } else {
                self.git_unavailable_message()
            };
            self.show_message(message);
        } else {
            self.show_pager(lines);
        }
        Ok(false)
    }

    fn handle_execute(&mut self) -> Result<bool> {
        let text = self.editor.command_text().to_string();
        self.last_command = Some(text.clone());
        let command = parse(&text);
        self.editor.apply(Action::CancelCommand);
        self.run_command(command)
    }

    fn run_command(&mut self, command: Command) -> Result<bool> {
        let result = self.dispatch_command(command);
        self.editor.reveal_cursor_horizontally();
        result
    }

    fn dispatch_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Write { force } => self.handle_save(force),
            Command::Quit => self.handle_quit(),
            Command::WriteQuit { force } => self.handle_write_quit(force),
            Command::ForceQuit => self.handle_force_quit(),
            Command::ForceQuitAll => self.handle_force_quit_all(),
            Command::QuitAll => self.handle_quit_all(),
            Command::WriteAll => self.handle_save_all(),
            Command::WriteQuitAll => self.handle_write_quit_all(),
            Command::Edit { name, force } => self.handle_edit(name, force),
            Command::WriteAs { name, force } => self.handle_write_as(name, force),
            Command::DeleteBuffer { spec, force } => self.handle_delete_buffer(spec, force),
            Command::ListBuffers => self.handle_list_buffers(),
            Command::Marks => self.handle_marks(),
            Command::NextBuffer => self.handle_buffer_step(true),
            Command::PreviousBuffer => self.handle_buffer_step(false),
            Command::AlternateBuffer => self.handle_alternate(),
            Command::SelectBuffer(arg) => self.handle_select_buffer(&arg),
            Command::Split { vertical, name } => self.split_window(vertical, name),
            Command::NewWindow { vertical, name } => self.new_window(vertical, name),
            Command::CloseWindow => self.close_active_window(false),
            Command::OnlyWindow => self.only_window(),
            Command::Set { name, value } => self.handle_set(&name, value),
            Command::Pick(command) => self.handle_pick_command(command),
            Command::Git(command) => self.handle_git_command(command),
            Command::Substitute(command) => match self.editor.substitute_command(&command) {
                Ok(message) => {
                    if let Some(message) = message {
                        self.show_message(message);
                    }
                    self.reveal_cursor();
                    Ok(false)
                }
                Err(message) => {
                    self.show_message(message);
                    Ok(false)
                }
            },
            Command::SetError(message) => {
                self.show_message(message);
                Ok(false)
            }
            Command::Unknown => Ok(false),
        }
    }

    fn handle_search_word(&mut self, forward: bool) -> Result<bool> {
        if let Err(error) = self.editor.search_word(forward) {
            self.show_message(file_error_message(error));
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_write_quit(&mut self, force: bool) -> Result<bool> {
        if !self.editor.has_file() {
            self.show_message("E32: No file name".to_string());
            return Ok(false);
        }
        if self.save_and_report(force) {
            return self.handle_quit();
        }
        Ok(false)
    }

    fn handle_quit(&mut self) -> Result<bool> {
        self.quit_or_close(false)
    }

    fn handle_force_quit(&mut self) -> Result<bool> {
        self.quit_or_close(true)
    }

    fn quit_or_close(&mut self, force: bool) -> Result<bool> {
        if self.windows.len() > 1 {
            return self.close_active_window(force);
        }
        if force {
            self.discard_recovery_buffers();
            return Ok(true);
        }
        if !self.editor.has_unsaved() {
            return Ok(true);
        }
        self.show_message("E37: No write since last change".to_string());
        Ok(false)
    }

    fn handle_force_quit_all(&mut self) -> Result<bool> {
        self.discard_recovery_buffers();
        Ok(true)
    }

    fn handle_quit_all(&mut self) -> Result<bool> {
        if !self.editor.has_unsaved() {
            return Ok(true);
        }
        self.show_message("E37: No write since last change".to_string());
        Ok(false)
    }

    fn handle_save_all(&mut self) -> Result<bool> {
        match self.editor.write_all() {
            Ok(()) => {
                self.lsp.save();
                Ok(false)
            }
            Err(error) => {
                self.show_message(file_error_message(error));
                Ok(false)
            }
        }
    }

    fn handle_write_quit_all(&mut self) -> Result<bool> {
        match self.editor.write_all() {
            Ok(()) => {
                self.lsp.save();
                Ok(true)
            }
            Err(error) => {
                self.show_message(file_error_message(error));
                Ok(false)
            }
        }
    }

    fn handle_edit(&mut self, name: Option<String>, force: bool) -> Result<bool> {
        if name.as_deref() == Some("#") {
            return self.handle_alternate();
        }
        match self.editor.edit_file(name.as_deref(), force) {
            Ok(()) => {
                self.reveal_cursor();
                Ok(false)
            }
            Err(error) => {
                self.show_message(file_error_message(error));
                Ok(false)
            }
        }
    }

    fn handle_list_buffers(&mut self) -> Result<bool> {
        self.show_message(self.editor.buffer_list());
        Ok(false)
    }

    fn handle_marks(&mut self) -> Result<bool> {
        let listing = self.editor.marks_listing(self.editor.viewport().width());
        self.show_message(listing);
        Ok(false)
    }

    fn handle_mark_jump(&mut self, name: char, line_wise: bool) -> Result<bool> {
        if let Err(error) = self.editor.jump_to_mark(name, line_wise) {
            self.show_message(file_error_message(error));
        }
        Ok(false)
    }

    fn handle_previous_jump(&mut self, line_wise: bool) -> Result<bool> {
        if let Err(error) = self.editor.jump_to_previous(line_wise) {
            self.show_message(file_error_message(error));
        }
        Ok(false)
    }

    fn handle_mark_operator(
        &mut self,
        operator: MarkOperator,
        name: char,
        line_wise: bool,
    ) -> Result<bool> {
        let outcome = match operator {
            MarkOperator::Delete => self.editor.delete_to_mark(name, line_wise),
            MarkOperator::Change => self.editor.change_to_mark(name, line_wise),
            MarkOperator::Yank => self.editor.yank_to_mark(name, line_wise),
        };
        if let Err(error) = outcome {
            self.show_message(file_error_message(error));
        }
        Ok(false)
    }

    fn handle_buffer_step(&mut self, forward: bool) -> Result<bool> {
        if forward {
            self.editor.buffer_next();
        } else {
            self.editor.buffer_previous();
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_alternate(&mut self) -> Result<bool> {
        self.editor.buffer_alternate();
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_select_buffer(&mut self, arg: &str) -> Result<bool> {
        let outcome = match arg.parse::<usize>() {
            Ok(number) => {
                if self.editor.buffer_select(number) {
                    Ok(())
                } else {
                    Err(kawi_editor::BufferError::Missing(number))
                }
            }
            Err(_) => self.editor.buffer_select_pattern(arg),
        };
        if let Err(error) = outcome {
            self.show_message(buffer_error_message(error));
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_write_as(&mut self, name: String, force: bool) -> Result<bool> {
        self.write_as_and_report(&name, force);
        Ok(false)
    }

    fn handle_delete_buffer(&mut self, spec: Option<String>, force: bool) -> Result<bool> {
        match self.editor.buffer_delete(spec.as_deref(), force) {
            Ok(()) => {
                self.reconcile_unlisted_buffers();
                self.reveal_cursor();
            }
            Err(error) => self.show_message(buffer_error_message(error)),
        }
        Ok(false)
    }

    fn reconcile_unlisted_buffers(&mut self) {
        let listed = self.editor.buffer_numbers();
        let fallback = self.editor.buffer_number();
        if self.windows.replace_unlisted_buffers(&listed, fallback) == 0 {
            return;
        }
        self.sync_active_window();
        self.refresh_window_sizes();
        self.reveal_cursor();
    }

    fn handle_jump(&mut self, older: bool) -> Result<bool> {
        let outcome = if older {
            self.editor.jump_older()
        } else {
            self.editor.jump_newer()
        };
        if let Err(error) = outcome {
            self.show_message(file_error_message(error));
        }
        self.reveal_cursor();
        Ok(false)
    }
}

pub(super) fn compact_error(error: &str) -> String {
    error.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn replace_stage(stage: ReplaceStage) -> kawi_tui::frame::ReplaceStage {
    match stage {
        ReplaceStage::Search => kawi_tui::frame::ReplaceStage::Search,
        ReplaceStage::Replacement => kawi_tui::frame::ReplaceStage::Replacement,
        ReplaceStage::Preview => kawi_tui::frame::ReplaceStage::Preview,
    }
}

fn event_needs_redraw(event: &Event) -> bool {
    match event {
        Event::Key(_) | Event::Resize(_, _) | Event::Paste(_) => true,
        Event::Mouse(mouse) => !matches!(mouse.kind, MouseEventKind::Moved),
        _ => false,
    }
}

fn frame_search_spans(spans: &[MatchSpan]) -> Vec<SearchSpan> {
    spans.iter().map(frame_search_span).collect()
}

fn frame_search_span(span: &MatchSpan) -> SearchSpan {
    SearchSpan {
        row: span.row,
        start: span.start,
        end: span.end,
        current: span.current,
    }
}

fn missing_message(editor: &Editor) -> String {
    if editor.search_pattern().is_empty() {
        return "E35: No previous regular expression".to_string();
    }
    format!("E486: Pattern not found: {}", editor.search_pattern())
}

fn written_report(name: &str, outcome: kawi_editor::WriteOutcome) -> String {
    let new = if outcome.created { " [New]" } else { "" };
    format!(
        "\"{name}\"{new} {}L, {}B written",
        outcome.lines, outcome.bytes
    )
}

fn is_operator(action: Action) -> bool {
    matches!(
        action,
        Action::DeleteSelection
            | Action::YankSelection
            | Action::PasteOverSelection
            | Action::DeleteLines
            | Action::YankLines
            | Action::ShiftRight
            | Action::ShiftLeft
            | Action::Reindent
            | Action::ToggleCase
            | Action::Lowercase
            | Action::Uppercase
            | Action::ReplaceSelection(_)
    )
}

fn buffer_error_message(error: kawi_editor::BufferError) -> String {
    match error {
        kawi_editor::BufferError::Missing(number) => {
            format!("E86: Buffer {number} does not exist")
        }
        kawi_editor::BufferError::NotFound(spec) => {
            format!("E94: No matching buffer for {spec}")
        }
        kawi_editor::BufferError::Ambiguous(spec) => {
            format!("E93: More than one match for {spec}")
        }
        kawi_editor::BufferError::NoWrite(number) => {
            format!("E89: No write since last change for buffer {number} (add ! to override)")
        }
    }
}

fn pager_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut len = 0;
        for ch in line.chars() {
            if len == width {
                lines.push(std::mem::take(&mut current));
                len = 0;
            }
            current.push(ch);
            len += 1;
        }
        lines.push(current);
    }
    lines
}

fn is_pager_dismiss(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Enter | KeyCode::Esc)
        || matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn file_error_message(error: kawi_editor::EditError) -> String {
    match error {
        kawi_editor::EditError::NoName => "E32: No file name".to_string(),
        kawi_editor::EditError::NoWrite => {
            "E37: No write since last change (add ! to override)".to_string()
        }
        kawi_editor::EditError::FileExists => "E13: File exists (add ! to override)".to_string(),
        kawi_editor::EditError::CannotOpen(name) => {
            format!("E484: Can't open file {name}")
        }
        kawi_editor::EditError::Io(message) => message,
        kawi_editor::EditError::InvalidMark => "E19: Mark has invalid line number".to_string(),
        kawi_editor::EditError::MarkNotSet => "E20: Mark not set".to_string(),
        kawi_editor::EditError::NoString => "E348: No string under cursor".to_string(),
        kawi_editor::EditError::InvalidEdit => "E474: Invalid text edit".to_string(),
    }
}

#[cfg(test)]
mod tests {
    // NOTE(probes): nvim verification lessons. Run one insert entry per
    // `:normal` call because later entries are dropped, end insert-ending
    // sequences with a real Esc because stopinsert also moves the cursor
    // left, and keep every motion succeeding because :normal aborts the
    // rest of the keys after a failed one.
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
    use kawi_editor::Mode;
    use std::time::Duration;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn set_command(name: &str, value: &str) -> Command {
        Command::Set {
            name: name.to_string(),
            value: kawi_editor::SetValue::Assign(Some(value.to_string())),
        }
    }

    fn press(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn drag(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn wheel_down() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn wheel_right() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::ScrollRight,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn wheel_right_page() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::ScrollRight,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    #[test]
    fn drag_selects_from_press_point() {
        let mut app = App::new(Buffer::from_string("abcdef\nghijkl"), Flavor::Vim);
        app.handle_mouse(press(5, 0)).expect("press");
        app.handle_mouse(drag(8, 1)).expect("drag");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 1), (1, 4))));
    }

    #[test]
    fn paste_event_inserts_text_in_normal_mode() {
        let mut app = App::new(Buffer::from_string("ab\ncd\n"), Flavor::Vim);
        app.handle_paste("x".to_string()).expect("paste");
        assert_eq!(app.editor.visible_lines(0, 2), vec!["axb", "cd"]);
        assert!(event_needs_redraw(&Event::Paste("x".to_string())));
    }

    #[test]
    fn paste_event_fills_command_line() {
        let mut app = App::new(Buffer::from_string("ab\ncd\n"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char(':'))).expect("command");
        app.handle_paste("e other\nignored".to_string())
            .expect("paste");
        assert_eq!(app.editor.command_text(), "e other");
    }

    #[test]
    fn ctrl_d_scrolls_half_page() {
        let text = (1..=100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = App::new(Buffer::from_string(&text), Flavor::Vim);
        app.handle_resize(80, 22).expect("resize");
        for key in [
            key(KeyCode::Char('5')),
            key(KeyCode::Char('0')),
            key(KeyCode::Char('G')),
        ] {
            app.handle_key(key).expect("goto");
        }
        assert_eq!(app.cursor_position(), (49, 0));
        assert_eq!(app.viewport_top(), 40);
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
            .expect("scroll");
        assert_eq!(app.cursor_position(), (59, 0));
        assert_eq!(app.viewport_top(), 50);
    }

    #[test]
    fn zz_centers_cursor() {
        let text = (1..=100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = App::new(Buffer::from_string(&text), Flavor::Vim);
        app.handle_resize(80, 22).expect("resize");
        for key in [
            key(KeyCode::Char('5')),
            key(KeyCode::Char('0')),
            key(KeyCode::Char('G')),
        ] {
            app.handle_key(key).expect("goto");
        }
        app.handle_key(key(KeyCode::Char('z'))).expect("arm");
        app.handle_key(key(KeyCode::Char('z'))).expect("center");
        assert_eq!(app.cursor_position(), (49, 0));
        assert_eq!(app.viewport_top(), 40);
        app.handle_key(key(KeyCode::Char('H'))).expect("top");
        assert_eq!(app.cursor_position(), (40, 0));
    }

    #[test]
    fn vim_visual_c_enters_insert() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('c'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_capital_c_enters_insert() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('C'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_line_capital_c_enters_insert() {
        let mut app = App::new(Buffer::from_string("abc\ndef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        app.handle_key(key(KeyCode::Char('C'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_capital_s_enters_insert() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('S'))).expect("substitute");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_line_capital_s_enters_insert() {
        let mut app = App::new(Buffer::from_string("abc\ndef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        app.handle_key(key(KeyCode::Char('S'))).expect("substitute");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_line_toggle() {
        let mut app = App::new(Buffer::from_string("a\nb\nc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        assert_eq!(app.editor_mode(), Mode::VisualLine);
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char('V'))).expect("exit");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_capital_d_deletes_lines() {
        let mut app = App::new(Buffer::from_string("aBcDeF"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('D'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_tilde_toggles() {
        let mut app = App::new(Buffer::from_string("aBcDeF"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('~'))).expect("toggle");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_tilde_w_toggles_word() {
        let mut app = App::new(Buffer::from_string("foo BAR baz"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('~'))).expect("tilde");
        app.handle_key(key(KeyCode::Char('w'))).expect("word");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_lower_line_toggles_line() {
        let mut app = App::new(Buffer::from_string("foo BAR"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('u'))).expect("lower");
        app.handle_key(key(KeyCode::Char('u'))).expect("line");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_goto_last_line() {
        let mut app = App::new(Buffer::from_string("abcdef\ngh"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (1, 1));
    }

    #[test]
    fn vim_normal_goto_first_line() {
        let mut app = App::new(Buffer::from_string("ab\ncd\nef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_goto_extends() {
        let mut app = App::new(Buffer::from_string("ab\ncd\nef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (2, 0))));
        assert_eq!(app.cursor_position(), (2, 0));
    }

    #[test]
    fn vim_normal_delete_to_last_line() {
        let mut app = App::new(Buffer::from_string("a\nbb\ncc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_delete_to_end_then_j_leaves_no_selection() {
        let mut app = App::new(Buffer::from_string("a\nbb\ncc\ndd"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_dgg_from_last() {
        let mut app = App::new(Buffer::from_string("a\nbb\ncc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_gv_reselects() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('v'))).expect("exit");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('v'))).expect("reselect");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn vim_visual_gv_exchanges() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('v'))).expect("exit");
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('h'))).expect("shrink");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('v'))).expect("exchange");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn vim_normal_gi_resumes_insert() {
        let mut app = App::new(Buffer::from_string("ab"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('i'))).expect("insert");
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        app.handle_key(key(KeyCode::Char('h'))).expect("move");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('i'))).expect("resume");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.cursor_position(), (0, 1));
        app.handle_key(key(KeyCode::Char('Z'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 1));
    }

    #[test]
    fn vim_normal_shift_right() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_shift_right_exits() {
        let mut app = App::new(Buffer::from_string("ab\ncd"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_reindent_exits() {
        let mut app = App::new(Buffer::from_string("    a\n    b"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char('='))).expect("reindent");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_shift_word() {
        let mut app = App::new(Buffer::from_string("foo BAR"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        app.handle_key(key(KeyCode::Char('w'))).expect("word");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_shift_find() {
        let mut app = App::new(Buffer::from_string("axbxc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        app.handle_key(key(KeyCode::Char('f'))).expect("find");
        app.handle_key(key(KeyCode::Char('x'))).expect("target");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_dot_repeats_delete() {
        let mut app = App::new(Buffer::from_string("a\nb\nc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('d'))).expect("line");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('.'))).expect("repeat");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_replace_tab_keeps_cursor_on_tab() {
        let mut app = App::new(Buffer::from_string("Hello world"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('r'))).expect("replace");
        app.handle_key(key(KeyCode::Tab)).expect("tab");
        let status = app.status_snapshot();
        assert_eq!(status.col, 0);
        assert_eq!(status.virtcol, 7);
        assert!(!status.line_empty);
    }

    #[test]
    fn changelist_walk_moves_through_changes() {
        let mut app = App::new(Buffer::from_string("aaa\nbbb\nccc\n"), Flavor::Vim);
        for k in ["x", "j", "x", "j", "x"] {
            app.handle_key(key(KeyCode::Char(k.chars().next().unwrap())))
                .expect("edit");
        }
        let mut positions = Vec::new();
        for _ in 0..3 {
            app.handle_key(key(KeyCode::Char('g'))).expect("g");
            app.handle_key(key(KeyCode::Char(';'))).expect("semicolon");
            positions.push(app.cursor_position());
        }
        assert_eq!(positions, vec![(2, 0), (1, 0), (0, 0)]);
    }

    #[test]
    fn status_reports_empty_line_position() {
        let mut app = App::new(Buffer::from_string("ab\n\ncd"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("down");
        let status = app.status_snapshot();
        assert!(status.line_empty);
        assert_eq!(status.row, 1);
    }

    #[test]
    fn vim_dot_repeats_change_text() {
        let mut app = App::new(Buffer::from_string("aaa\nbbb"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('c'))).expect("change");
        app.handle_key(key(KeyCode::Char('c'))).expect("line");
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('.'))).expect("repeat");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (1, 0));
    }

    #[test]
    fn vim_normal_ge_moves() {
        let mut app = App::new(Buffer::from_string("foo BAR baz"), Flavor::Vim);
        for _ in 0..7 {
            app.handle_key(key(KeyCode::Char('l'))).expect("move");
        }
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('e'))).expect("motion");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 6));
    }

    #[test]
    fn vim_normal_dge_deletes() {
        let mut app = App::new(Buffer::from_string("foo BAR baz"), Flavor::Vim);
        for _ in 0..7 {
            app.handle_key(key(KeyCode::Char('l'))).expect("move");
        }
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('e'))).expect("motion");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 6));
    }

    #[test]
    fn vim_normal_tilde_toggles_and_advances() {
        let mut app = App::new(Buffer::from_string("aBc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('~'))).expect("toggle");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn vim_normal_goto_column_start_inserts() {
        let mut app = App::new(Buffer::from_string("abc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('I'))).expect("insert");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.cursor_position(), (0, 0));
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_o_swaps_ends() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('o'))).expect("swap");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.cursor_position(), (0, 0));
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn press_after_drag_leaves_visual() {
        let mut app = App::new(Buffer::from_string("abcdef\nghijkl"), Flavor::Vim);
        app.handle_mouse(press(5, 0)).expect("press");
        app.handle_mouse(drag(8, 1)).expect("drag");
        app.handle_mouse(press(0, 0)).expect("second press");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn wheel_down_scrolls_and_drags_cursor() {
        let mut app = App::new(Buffer::from_string(&"line\n".repeat(30)), Flavor::Vim);
        app.handle_mouse(wheel_down()).expect("wheel");
        assert_eq!(app.viewport_top(), 3);
        assert_eq!(app.cursor_position(), (3, 0));
    }

    #[test]
    fn wheel_right_scrolls_horizontally() {
        let mut app = App::new(Buffer::from_string(&"0123456789".repeat(8)), Flavor::Vim);
        app.handle_mouse(wheel_right()).expect("wheel");
        assert_eq!(app.editor.view_leftcol(), 6);
        assert_eq!(app.cursor_position(), (0, 6));
    }

    #[test]
    fn wheel_right_page_uses_window_width() {
        let mut app = App::new(Buffer::from_string(&"0123456789".repeat(8)), Flavor::Vim);
        app.handle_mouse(wheel_right_page()).expect("wheel");
        assert!(app.editor.view_leftcol() > 6);
        assert_eq!(app.cursor_position(), (0, app.editor.view_leftcol()));
    }

    #[test]
    fn wheel_keeps_visible_cursor() {
        let mut app = App::new(Buffer::from_string(&"line\n".repeat(30)), Flavor::Vim);
        app.handle_mouse(press(0, 10)).expect("press");
        app.handle_mouse(wheel_down()).expect("wheel");
        assert_eq!(app.viewport_top(), 3);
        assert_eq!(app.cursor_position(), (10, 0));
    }

    #[test]
    fn vim_delete_leaves_visual() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_yank_leaves_visual() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('y'))).expect("yank");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn helix_starts_clean() {
        let app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn set_keymap_switches_to_helix() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.run_command(set_command("keymap", "helix"))
            .expect("switch");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn set_keymap_back_to_vim_clears() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.run_command(set_command("keymap", "vim"))
            .expect("switch");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn set_keymap_helix_preserves_range() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.run_command(set_command("keymap", "helix"))
            .expect("switch");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn set_keymap_unknown_keeps_flavor() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.run_command(set_command("keymap", "personal"))
            .expect("unknown");
        assert_eq!(app.editor_mode(), Mode::Normal);
    }

    #[test]
    fn helix_motion_moves_without_extending() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn helix_v_selects_then_motion_extends() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('v'))).expect("select");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn helix_esc_returns_to_normal() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('v'))).expect("select");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn helix_delete_acts_on_cursor_char() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 1));
    }

    #[test]
    fn change_word_enters_insert() {
        let mut app = App::new(Buffer::from_string("foo bar"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('c'))).expect("pending");
        app.handle_key(key(KeyCode::Char('w'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
    }

    #[test]
    fn save_reports_lines_and_bytes() {
        let path = std::env::temp_dir().join("kawi_msg_test.txt");
        let _ = std::fs::remove_file(&path);
        let mut buffer = Buffer::from_string("aa\nbb");
        buffer.path = Some(path.to_string_lossy().to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.handle_save(false).expect("save");
        let expected = format!("\"{}\" [New] 2L, 6B written", path.to_string_lossy());
        assert_eq!(app.last_message(), Some(expected.as_str()));
        app.handle_save(false).expect("save again");
        let expected = format!("\"{}\" 2L, 6B written", path.to_string_lossy());
        assert_eq!(app.last_message(), Some(expected.as_str()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_without_path_reports_error() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_save(false).expect("stays");
        assert_eq!(app.last_message(), Some("E32: No file name"));
    }

    #[test]
    fn save_readonly_file_reports_e45() {
        let target = temp_file("readonly_save.txt", "old\n");
        set_mode(&target, 0o444);
        let mut buffer = Buffer::from_string("new");
        buffer.path = Some(target.clone());
        let mut app = App::new(buffer, Flavor::Vim);
        app.handle_save(false).expect("stays");
        assert_eq!(
            app.last_message(),
            Some("E45: 'readonly' option is set (add ! to override)")
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old\n");
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn forced_save_overwrites_readonly_file() {
        let target = temp_file("readonly_force.txt", "old\n");
        set_mode(&target, 0o444);
        let mut buffer = Buffer::from_string("new");
        buffer.path = Some(target.clone());
        let mut app = App::new(buffer, Flavor::Vim);
        app.handle_save(true).expect("forced save");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");
        assert_eq!(mode_of(&target), 0o444);
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn undo_without_history_reports() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_undo().expect("undo");
        assert_eq!(app.last_message(), Some("Already at oldest change"));
    }

    #[test]
    fn redo_without_future_reports() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_redo().expect("redo");
        assert_eq!(app.last_message(), Some("Already at newest change"));
    }

    #[test]
    fn unknown_keymap_reports() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.run_command(set_command("keymap", "personal"))
            .expect("unknown");
        assert_eq!(app.last_message(), Some("Unknown keymap: personal"));
    }

    #[test]
    fn written_report_formats_like_editor() {
        let outcome = kawi_editor::WriteOutcome {
            lines: 3,
            bytes: 12,
            created: false,
        };
        assert_eq!(
            written_report("notes.txt", outcome),
            "\"notes.txt\" 3L, 12B written"
        );
        let created = kawi_editor::WriteOutcome {
            lines: 0,
            bytes: 0,
            created: true,
        };
        assert_eq!(
            written_report("fresh.txt", created),
            "\"fresh.txt\" [New] 0L, 0B written"
        );
    }

    #[test]
    fn search_refreshes_wanted_column_for_vertical_moves() {
        let text = "\tello world\n01234567890123456789\nxabc tail";
        let mut app = App::new(kawi_buffer::Buffer::from_string(text), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        for ch in "abc".chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.cursor_position(), (2, 1));
        app.handle_key(key(KeyCode::Char('k'))).expect("up");
        assert_eq!(app.cursor_position(), (1, 1));
    }

    #[test]
    fn search_prompt_types_and_confirms() {
        let mut app = App::new(Buffer::from_string("foo bar foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        assert_eq!(app.editor_mode(), Mode::Search);
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        assert_eq!(app.cursor_position(), (0, 5));
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 5));
    }

    #[test]
    fn substitute_command_edits_and_reports() {
        let mut app = App::new(Buffer::from_string("foo foo foo\n"), Flavor::Vim);
        type_command(&mut app, "s/foo/X/g");
        assert_eq!(app.last_message(), Some("3 substitutions on 1 line"));
        assert_eq!(app.editor.visible_lines(0, 1), vec!["X X X"]);
        assert_eq!(app.editor_mode(), Mode::Normal);
    }

    #[test]
    fn substitute_command_reports_missing_pattern() {
        let mut app = App::new(Buffer::from_string("abc\n"), Flavor::Vim);
        type_command(&mut app, "s/zzz/X/");
        assert_eq!(app.last_message(), Some("E486: Pattern not found: zzz"));
        assert_eq!(app.editor.visible_lines(0, 1), vec!["abc"]);
    }

    #[test]
    fn substitute_command_uses_visual_range() {
        let mut app = App::new(Buffer::from_string("foo\nfoo\nfoo\n"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("visual");
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char(':'))).expect("command");
        assert_eq!(app.editor_mode(), Mode::Command);
        for ch in "s/foo/X/g".chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("run");
        assert_eq!(app.editor.visible_lines(0, 3), vec!["X", "X", "foo"]);
    }

    fn type_command(app: &mut App, text: &str) {
        for ch in format!(":{text}").chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("run");
    }

    fn search_forward(app: &mut App, pattern: &str) {
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        for ch in pattern.chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
    }

    #[test]
    fn search_options_apply_to_prompt_searches() {
        let mut app = App::new(
            kawi_buffer::Buffer::from_string("foo FOO Foo foo"),
            Flavor::Vim,
        );
        search_forward(&mut app, "foo");
        assert_eq!(app.cursor_position(), (0, 12));
        type_command(&mut app, "set ignorecase");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        app.handle_key(key(KeyCode::Char('0'))).expect("start");
        search_forward(&mut app, "foo");
        assert_eq!(app.cursor_position(), (0, 4));
        type_command(&mut app, "set smartcase");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        app.handle_key(key(KeyCode::Char('0'))).expect("start");
        search_forward(&mut app, "Foo");
        assert_eq!(app.cursor_position(), (0, 8));
    }

    #[test]
    fn star_searches_word_under_cursor() {
        let mut app = App::new(
            kawi_buffer::Buffer::from_string("foo FOO Foo foo"),
            Flavor::Vim,
        );
        type_command(&mut app, "set ignorecase");
        app.handle_key(key(KeyCode::Char('*'))).expect("star");
        assert_eq!(app.cursor_position(), (0, 4));
    }

    #[test]
    fn star_on_blank_line_reports_e348() {
        let mut app = App::new(kawi_buffer::Buffer::from_string(""), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('*'))).expect("star");
        assert_eq!(app.last_message(), Some("E348: No string under cursor"));
    }

    #[test]
    fn autoindent_command_toggles() {
        let mut app = App::new(kawi_buffer::Buffer::from_string("    abc"), Flavor::Vim);
        type_command(&mut app, "set noautoindent");
        app.handle_key(key(KeyCode::Char('A'))).expect("append");
        app.handle_key(key(KeyCode::Enter)).expect("enter");
        app.handle_key(key(KeyCode::Esc)).expect("esc");
        let lines = app.editor.visible_lines(0, app.editor.line_count() + 1);
        assert_eq!(lines, vec!["    abc".to_string(), "".to_string()]);
    }

    fn temp_file(name: &str, text: &str) -> String {
        let path = temp_path(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("kawi_app_{name}"))
            .to_string_lossy()
            .into_owned()
    }

    fn set_mode(path: &str, bits: u32) {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(bits);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    fn mode_of(path: &str) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[test]
    fn write_as_command_names_unnamed_buffer() {
        let target = temp_path("write_as_named.txt");
        let _ = std::fs::remove_file(&target);
        let mut app = App::new(kawi_buffer::Buffer::from_string("abc"), Flavor::Vim);
        type_command(&mut app, &format!("w {target}"));
        assert_eq!(app.editor.filename(), target);
        assert_eq!(
            app.last_message(),
            Some(format!("\"{target}\" [New] 1L, 4B written").as_str())
        );
        assert!(!app.editor.is_dirty());
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn write_as_existing_target_reports_e13() {
        let alpha = temp_file("write_as_alpha.txt", "alpha\n");
        let exists = temp_file("write_as_exists.txt", "old\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("w {exists}"));
        assert_eq!(
            app.last_message(),
            Some("E13: File exists (add ! to override)")
        );
        type_command(&mut app, &format!("w! {exists}"));
        assert_eq!(std::fs::read_to_string(&exists).unwrap(), "alpha\n");
    }

    #[test]
    fn delete_buffer_command_works() {
        let alpha = temp_file("delete_alpha.txt", "alpha\n");
        let bravo = temp_file("delete_bravo.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, &format!("bd {alpha}"));
        assert_eq!(app.editor.listed_buffer_count(), 1);
        type_command(&mut app, "bd");
        assert_eq!(app.editor.listed_buffer_count(), 1);
        assert_eq!(app.editor.filename(), "[No Name]");
    }

    #[test]
    fn delete_modified_buffer_reports_e89() {
        let alpha = temp_file("delete_mod.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, "bd");
        assert_eq!(
            app.last_message(),
            Some("E89: No write since last change for buffer 1 (add ! to override)")
        );
    }

    #[test]
    fn select_buffer_by_partial_name() {
        let alpha = temp_file("select_name_alpha.txt", "alpha\n");
        let bravo = temp_file("select_name_bravo.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "b name_alpha");
        assert_eq!(app.editor.filename(), alpha);
        type_command(&mut app, "b zzz");
        assert_eq!(app.last_message(), Some("E94: No matching buffer for zzz"));
    }

    #[test]
    fn diagnostics_take_priority_over_git_signs() {
        let git = Some(kawi_tui::frame::Sign::Git(kawi_tui::frame::GitSign::Change));
        let diagnostic = Some(kawi_tui::frame::DiagnosticSeverity::Error);
        assert_eq!(
            preferred_sign(git, diagnostic),
            Some(kawi_tui::frame::Sign::Diagnostic(
                kawi_tui::frame::DiagnosticSeverity::Error
            ))
        );
        assert_eq!(preferred_sign(git, None), git);
    }

    #[test]
    fn marks_command_pages_listing() {
        let path = temp_file("marks_listing.txt", "  alpha\n    beta\nthird\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&path).unwrap(), Flavor::Vim);
        app.handle_resize(60, 10).expect("resize");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('m'))).expect("mark");
        app.handle_key(key(KeyCode::Char('a'))).expect("name");
        type_command(&mut app, "marks");
        let lines = app.pager_lines().expect("pager");
        assert_eq!(lines[0], "");
        assert_eq!(lines[1], "mark line  col file/text");
        assert!(lines[2].starts_with(" a      2    0"), "{:?}", lines);
    }

    #[test]
    fn unset_mark_reports_e20() {
        let mut app = App::new(Buffer::from_string("abc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('`'))).expect("prefix");
        app.handle_key(key(KeyCode::Char('z'))).expect("name");
        assert_eq!(app.last_message(), Some("E20: Mark not set"));
    }

    #[test]
    fn list_buffers_pages_when_rows_overflow() {
        let alpha = temp_file("pager_alpha_file.txt", "one\ntwo\n");
        let bravo = temp_file("pager_bravo_file.txt", "three\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_resize(40, 10).expect("resize");
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "ls");
        assert!(app.pager_lines().is_some());
        app.handle_key(key(KeyCode::Enter)).expect("dismiss");
        assert!(app.pager_lines().is_none());
    }

    #[test]
    fn pager_replays_non_dismiss_keys() {
        let alpha = temp_file("pager_replay.txt", "one\ntwo\nthree\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_resize(40, 10).expect("resize");
        type_command(&mut app, "ls");
        assert!(app.pager_lines().is_some());
        app.handle_key(key(KeyCode::Char('j'))).expect("replay");
        assert!(app.pager_lines().is_none());
        assert_eq!(app.cursor_position(), (1, 0));
    }

    #[test]
    fn edit_command_switches_files() {
        let alpha = temp_file("edit_alpha.txt", "alpha\n");
        let bravo = temp_file("edit_bravo.txt", "bravo\ncharlie\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        assert_eq!(app.editor.filename(), bravo);
        assert_eq!(app.editor.line_count(), 2);
        type_command(&mut app, "e #");
        assert_eq!(app.editor.filename(), alpha);
    }

    #[test]
    fn edit_reload_reports_e37_when_dirty() {
        let alpha = temp_file("edit_dirty.txt", "alpha\n");
        let bravo = temp_file("edit_dirty_other.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        assert_eq!(app.editor.filename(), bravo);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        assert_eq!(
            app.last_message(),
            Some("E37: No write since last change (add ! to override)")
        );
        assert_eq!(app.editor.filename(), bravo);
        type_command(&mut app, "e!");
        assert_eq!(app.editor.filename(), bravo);
        assert!(!app.editor.is_dirty());
    }

    #[test]
    fn quit_dirty_buffer_reports_e37() {
        let alpha = temp_file("quit_dirty.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, "q");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
        type_command(&mut app, "q!");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
    }

    #[test]
    fn quit_reports_hidden_changes() {
        let alpha = temp_file("quit_hidden.txt", "alpha\n");
        let bravo = temp_file("quit_hidden_other.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "q");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
    }

    #[test]
    fn quit_all_reports_hidden_changes() {
        let alpha = temp_file("quit_all.txt", "alpha\n");
        let bravo = temp_file("quit_all_other.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "qa");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
    }

    #[test]
    fn write_unnamed_buffer_reports_e32() {
        let mut app = App::new(kawi_buffer::Buffer::from_string("abc"), Flavor::Vim);
        type_command(&mut app, "w");
        assert_eq!(app.last_message(), Some("E32: No file name"));
    }

    #[test]
    fn list_buffers_shows_flags() {
        let alpha = temp_file("list_buffers.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, "ls");
        let message = app.last_message().unwrap();
        assert!(message.contains("  1 %a"));
        assert!(message.contains(&format!("\"{alpha}\"")));
        assert!(message.contains("line 1"));
    }

    #[test]
    fn alternate_key_switches_buffers() {
        let alpha = temp_file("alt_alpha.txt", "alpha\n");
        let bravo = temp_file("alt_bravo.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        app.handle_key(KeyEvent::new(KeyCode::Char('6'), KeyModifiers::CONTROL))
            .expect("alternate");
        assert_eq!(app.editor.filename(), alpha);
    }

    #[test]
    fn select_buffer_missing_reports_e86() {
        let alpha = temp_file("select_missing.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, "b 9");
        assert_eq!(app.last_message(), Some("E86: Buffer 9 does not exist"));
        type_command(&mut app, "b nope");
        assert_eq!(app.last_message(), Some("E94: No matching buffer for nope"));
    }

    #[test]
    fn numbers_show_by_default() {
        let app = App::new(Buffer::from_string("abc\ndef\nghi"), Flavor::Vim);
        assert!(app.editor.number());
        assert_eq!(app.editor.gutter_width(), 4);
        assert_eq!(
            app.editor.gutter_lines(0, 3),
            vec!["  1 ".to_string(), "  2 ".to_string(), "  3 ".to_string()]
        );
    }

    #[test]
    fn set_number_command_shows_gutter() {
        let mut app = App::new(Buffer::from_string("abc\ndef\nghi"), Flavor::Vim);
        type_command(&mut app, "set nonumber");
        assert_eq!(app.editor.gutter_width(), 0);
        type_command(&mut app, "set number");
        assert_eq!(app.editor.gutter_width(), 4);
        assert_eq!(
            app.editor.gutter_lines(0, 3),
            vec!["  1 ".to_string(), "  2 ".to_string(), "  3 ".to_string()]
        );
        assert_eq!(app.editor.cursor_wcol(), 4);
    }

    #[test]
    fn set_relativenumber_command_marks_cursor_line() {
        let mut app = App::new(Buffer::from_string("abc\ndef\nghi"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("down");
        type_command(&mut app, "set relativenumber");
        assert!(app.editor.number());
        assert_eq!(
            app.editor.gutter_lines(0, 3),
            vec!["  1 ".to_string(), "2   ".to_string(), "  1 ".to_string()]
        );
    }

    #[test]
    fn set_numberwidth_command_validates() {
        let mut app = App::new(Buffer::from_string("abc"), Flavor::Vim);
        type_command(&mut app, "set numberwidth=0");
        assert_eq!(
            app.last_message(),
            Some("E487: Argument must be positive: numberwidth=0")
        );
        assert_eq!(app.editor.numberwidth(), 4);
        type_command(&mut app, "set numberwidth=21");
        assert_eq!(
            app.last_message(),
            Some("E474: Invalid argument: numberwidth=21")
        );
        type_command(&mut app, "set numberwidth=2");
        assert_eq!(app.editor.numberwidth(), 2);
    }

    #[test]
    fn click_maps_through_gutter_and_tab() {
        let mut app = App::new(Buffer::from_string("\tello world"), Flavor::Vim);
        type_command(&mut app, "set number");
        app.handle_mouse(press(3, 0)).expect("click gutter");
        assert_eq!(app.cursor_position(), (0, 0));
        app.handle_mouse(press(12, 0)).expect("click past tab");
        assert_eq!(app.cursor_position(), (0, 1));
        app.handle_mouse(press(4, 0)).expect("click tab start");
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn search_cancel_restores_origin() {
        let mut app = App::new(Buffer::from_string("foo bar foo"), Flavor::Vim);
        for _ in 0..4 {
            app.handle_key(key(KeyCode::Char('l'))).expect("move");
        }
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 4));
    }

    #[test]
    fn event_needs_redraw_only_for_screen_events() {
        assert!(event_needs_redraw(&Event::Key(key(KeyCode::Char('j')))));
        assert!(event_needs_redraw(&Event::Resize(80, 24)));
        assert!(!event_needs_redraw(&Event::FocusGained));
        assert!(!event_needs_redraw(&Event::FocusLost));
        let moved = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 3,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!event_needs_redraw(&Event::Mouse(moved)));
        let press = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        assert!(event_needs_redraw(&Event::Mouse(press)));
    }

    #[test]
    fn search_next_reports_wrap() {
        let mut app = App::new(Buffer::from_string("ar ar ar"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.cursor_position(), (0, 3));
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.cursor_position(), (0, 6));
        assert_eq!(app.last_message(), None);
        app.handle_key(key(KeyCode::Char('n'))).expect("wrap");
        assert_eq!(app.cursor_position(), (0, 0));
        assert_eq!(app.last_message(), None);
        assert_eq!(app.search_status_line().as_deref(), Some("W /ar [1/3]"));
    }

    #[test]
    fn search_without_pattern_reports() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(
            app.last_message(),
            Some("E35: No previous regular expression")
        );
    }

    #[test]
    fn search_no_match_reports() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('z'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.last_message(), Some("E486: Pattern not found: z"));
    }

    #[test]
    fn search_confirm_keeps_status_without_message() {
        let mut app = App::new(Buffer::from_string("foo bar foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 5));
        assert_eq!(app.last_message(), None);
        assert_eq!(app.search_status_line().as_deref(), Some("/ar [1/1]"));
    }

    #[test]
    fn search_confirm_without_match_reports() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('z'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.last_message(), Some("E486: Pattern not found: z"));
        assert_eq!(app.search_status_line(), None);
    }

    #[test]
    fn set_theme_switches_palette() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        assert_eq!(app.theme_name(), "default");
        app.run_command(set_command("theme", "gruvbox-dark"))
            .expect("switch");
        assert_eq!(app.theme_name(), "gruvbox-dark");
        assert_eq!(app.last_message(), None);
    }

    #[test]
    fn set_theme_unknown_reports_and_keeps() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.run_command(set_command("theme", "nope"))
            .expect("unknown");
        assert_eq!(app.theme_name(), "default");
        assert_eq!(app.last_message(), Some("Unknown theme: nope"));
    }

    #[test]
    fn search_next_restores_status_after_wrap() {
        let mut app = App::new(Buffer::from_string("ar ar ar"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.cursor_position(), (0, 6));
        app.handle_key(key(KeyCode::Char('n'))).expect("wrap");
        assert_eq!(app.last_message(), None);
        assert_eq!(app.search_status_line().as_deref(), Some("W /ar [1/3]"));
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.last_message(), None);
        assert_eq!(app.cursor_position(), (0, 3));
        assert_eq!(app.search_status_line().as_deref(), Some("/ar [2/3]"));
    }

    #[test]
    fn command_prompts_feed_the_history() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char(':'))).expect("colon");
        for character in "set nu".chars() {
            app.handle_key(key(KeyCode::Char(character))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("execute");
        app.handle_key(key(KeyCode::Char(':'))).expect("colon");
        for character in "aborted".chars() {
            app.handle_key(key(KeyCode::Char(character))).expect("type");
        }
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        assert_eq!(
            app.editor.history_entries().command,
            vec!["set nu", "aborted"]
        );
        app.handle_key(key(KeyCode::Char(':'))).expect("colon");
        app.handle_key(key(KeyCode::Up)).expect("up");
        assert_eq!(app.editor.command_text(), "aborted");
        app.handle_key(key(KeyCode::Up)).expect("up");
        assert_eq!(app.editor.command_text(), "set nu");
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
    }

    #[test]
    fn missing_lsp_binary_reaches_the_status_line() {
        let root = std::env::temp_dir().join(format!("kawi_lsp_status_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("languages.toml");
        std::fs::write(
            &path,
            "[server.rust-analyzer]\ncommand = \"kawi-missing-rust-analyzer\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nservers = [\"rust-analyzer\"]\n",
        )
        .unwrap();
        let mut buffer = Buffer::from_string("fn main() {}\n");
        buffer.path = Some("main.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.lsp.load(&path, app.events.wake());
        app.sync_lsp();
        assert_eq!(
            app.lsp.status(),
            Some("lsp: rust-analyzer not found in PATH")
        );
        assert_eq!(
            app.status_snapshot().lsp,
            Some("lsp: rust-analyzer not found in PATH")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn syntax_results_reach_the_app() {
        let mut buffer = Buffer::from_string("// hi\nfn main() {}\n");
        buffer.path = Some("test.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.refresh_syntax();
        wait_for_syntax(&mut app);
        assert!(app
            .syntax_spans
            .iter()
            .any(|span| span.group == kawi_syntax::SyntaxGroup::Comment));
    }

    #[test]
    fn syntax_coverage_avoids_scrolling_requests() {
        let text = (0..3_000)
            .map(|index| format!("fn function_{index}() {{}}\n"))
            .collect::<String>();
        let mut buffer = Buffer::from_string(&text);
        buffer.path = Some("large.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.refresh_syntax();
        wait_for_syntax(&mut app);
        let generation = app.syntax_latest;
        let window = app.syntax_window.clone();
        let request = app.syntax_key.expect("request");
        assert!(window.start <= request.top);
        assert!(request.end <= window.end);
        assert!(window.end > 100);
        app.editor.apply(Action::ScrollHalfDown);
        app.refresh_syntax();
        assert_eq!(app.syntax_latest, generation);
        assert_eq!(app.syntax_window, window);
    }

    fn wait_for_syntax(app: &mut App) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            app.apply_syntax_results();
            if !app.syntax_spans.is_empty() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("syntax result timed out");
    }

    #[test]
    fn unnamed_buffers_do_not_start_syntax() {
        let mut app = App::new(Buffer::from_string("plain\n"), Flavor::Vim);
        app.refresh_syntax();
        assert!(app.syntax.is_none());
        assert!(app.syntax_spans.is_empty());
    }

    fn wait_for_picker(app: &mut App) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            app.refresh_picker();
            if app
                .picker
                .as_ref()
                .is_some_and(|session| !session.picker().rows().is_empty())
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("picker result timed out");
    }

    fn picker_project(name: &str) -> String {
        let root = std::env::temp_dir().join(format!("kawi_picker_app_{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("alpha.txt"), "needle one\n").unwrap();
        std::fs::write(root.join("beta.txt"), "needle two\n").unwrap();
        root.join("alpha.txt").to_string_lossy().into_owned()
    }

    #[test]
    fn pick_files_command_opens_selected_file() {
        let path = picker_project("files");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&path).unwrap(), Flavor::Vim);
        type_command(&mut app, "pick files");
        assert!(app.picker.is_some());
        wait_for_picker(&mut app);
        app.handle_key(key(KeyCode::Char('b'))).expect("filter");
        app.handle_key(key(KeyCode::Char('e'))).expect("filter");
        app.handle_key(key(KeyCode::Char('t'))).expect("filter");
        app.handle_key(key(KeyCode::Enter)).expect("open");
        assert!(app.editor.filename().ends_with("beta.txt"));
    }

    #[test]
    fn pick_grep_command_opens_match_position() {
        let path = picker_project("grep");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&path).unwrap(), Flavor::Vim);
        type_command(&mut app, "pick grep");
        for character in "needle".chars() {
            app.handle_key(key(KeyCode::Char(character)))
                .expect("query");
        }
        wait_for_picker(&mut app);
        app.handle_key(key(KeyCode::Enter)).expect("open");
        assert!(app.editor.filename().ends_with("alpha.txt"));
        assert_eq!((app.editor.row(), app.editor.col()), (0, 0));
    }

    #[test]
    fn replace_command_opens_the_replace_session() {
        let path = picker_project("replace_command");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&path).unwrap(), Flavor::Vim);
        type_command(&mut app, "replace");
        assert!(app.picker.is_none());
        assert!(app.replace.is_some());
    }

    #[test]
    fn ctrl_r_converts_a_grep_picker_into_a_replace_session() {
        let path = picker_project("replace_ctrl_r");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&path).unwrap(), Flavor::Vim);
        type_command(&mut app, "pick grep");
        for character in "needle".chars() {
            app.handle_key(key(KeyCode::Char(character)))
                .expect("query");
        }
        wait_for_picker(&mut app);
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL))
            .expect("replace");
        assert!(app.picker.is_none());
        let session = app.replace.as_ref().expect("replace session");
        assert_eq!(session.query(), "needle");
        assert_eq!(session.replacement(), "");
        assert_eq!(session.stage(), ReplaceStage::Replacement);
    }

    #[test]
    fn pick_buffers_command_selects_an_open_buffer() {
        let path = picker_project("buffers");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&path).unwrap(), Flavor::Vim);
        let beta = Path::new(&path).with_file_name("beta.txt");
        type_command(&mut app, &format!("edit {}", beta.display()));
        type_command(&mut app, "pick buffers");
        assert!(app.picker.is_some());
        app.handle_key(key(KeyCode::Enter)).expect("open");
        assert!(app.editor.filename().ends_with("alpha.txt"));
    }
}
