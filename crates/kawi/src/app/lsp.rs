use super::lsp_diagnostics::{DiagnosticState, Navigation};
use super::lsp_navigation::{
    HoverResult, LocationKind, LocationResult, LspNavigation, NavigationRequest,
};
use super::lsp_text::LspText;
use kawi_editor::{Editor, LanguageIndent};
use kawi_lsp::{
    CodeActionEvent, CompletionEvent, CursorPosition, Diagnostic, DiagnosticSeverity,
    FormattingEvent, FormattingSettings, LanguageWarning, Languages, LspCommand, LspEvent,
    LspService, MessageLevel, ProgressEvent, RenameEvent, RenameReady, ServerState, StatusEvent,
};
use kawi_tui::frame::DiagnosticSpan;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

#[derive(Default)]
pub struct LspUi {
    service: Option<LspService>,
    languages: Option<Languages>,
    path: Option<PathBuf>,
    revision: Option<u64>,
    statuses: BTreeMap<String, ServerState>,
    status_messages: BTreeMap<String, String>,
    progress: BTreeMap<(String, String), ProgressSnapshot>,
    diagnostics: DiagnosticState,
    navigation: LspNavigation,
    text: LspText,
    rename_request: Option<u64>,
    rename_ready: Option<RenameReady>,
    rename_edit: Option<RenameEvent>,
    rename_error: Option<u64>,
    display: Option<String>,
    message: Option<String>,
}

#[derive(Clone)]
struct ProgressSnapshot {
    title: Option<String>,
    message: Option<String>,
    percentage: Option<u32>,
}

impl LspUi {
    pub fn load(&mut self, path: &Path, wake: Sender<()>) -> Vec<LanguageWarning> {
        let languages = Languages::load(path);
        let warnings = languages.warnings().to_vec();
        match LspService::new(languages.clone(), wake) {
            Ok(service) => {
                self.languages = Some(languages);
                self.service = Some(service);
            }
            Err(error) => {
                self.languages = Some(languages);
                self.service = None;
                self.message = Some(format!("lsp: {error}"));
            }
        }
        self.update_display();
        warnings
    }

    pub fn refresh(&mut self, editor: &Editor) -> bool {
        let document = self.sync_document(editor);
        let events = self.poll_events(editor);
        let display = self.update_display();
        events || document || display
    }

    pub fn poll_events(&mut self, editor: &Editor) -> bool {
        self.navigation.sync_editor(editor, self.service.as_ref());
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        let events: Vec<LspEvent> = std::iter::from_fn(|| service.try_recv()).collect();
        let mut changed = false;
        for event in events {
            changed |= self.apply_event(event);
        }
        changed
    }

    pub fn definition(&mut self, editor: &Editor) -> Option<String> {
        self.navigation
            .request(NavigationRequest::Definition, editor, self.service.as_ref())
    }

    pub fn references(&mut self, editor: &Editor) -> Option<String> {
        self.navigation
            .request(NavigationRequest::References, editor, self.service.as_ref())
    }

    pub fn hover(&mut self, editor: &Editor) -> Option<String> {
        self.navigation
            .request(NavigationRequest::Hover, editor, self.service.as_ref())
    }

    pub fn take_location(&mut self) -> Option<LocationResult> {
        self.navigation.take_location()
    }

    pub fn take_hover(&mut self) -> Option<HoverResult> {
        self.navigation.take_hover()
    }

    pub fn completion(&mut self, editor: &Editor) -> Result<u64, String> {
        let Some(path) = editor_path(editor) else {
            return Err("E32: No file name".to_string());
        };
        self.text.completion(
            self.service.as_ref(),
            &path,
            super::lsp_navigation::cursor_position(editor),
        )
    }

    pub fn code_actions(
        &mut self,
        editor: &Editor,
        range: (CursorPosition, CursorPosition),
        diagnostics: Vec<Diagnostic>,
    ) -> Result<u64, String> {
        let Some(path) = editor_path(editor) else {
            return Err("E32: No file name".to_string());
        };
        self.text
            .code_actions(self.service.as_ref(), &path, range, diagnostics)
    }

    pub fn formatting(
        &mut self,
        editor: &Editor,
        range: Option<(CursorPosition, CursorPosition)>,
    ) -> Result<u64, String> {
        let Some(path) = editor_path(editor) else {
            return Err("E32: No file name".to_string());
        };
        let settings = FormattingSettings {
            tab_size: u32::try_from(editor.tabstop()).unwrap_or(u32::MAX),
            insert_spaces: editor.expandtab(),
        };
        self.text
            .formatting(self.service.as_ref(), &path, range, settings)
    }

    pub fn execute_command(
        &mut self,
        path: &std::path::Path,
        server: &str,
        command: &LspCommand,
    ) -> bool {
        self.service
            .as_ref()
            .is_some_and(|service| service.execute_command(path, server, command))
    }

    pub fn take_completion(&mut self) -> Option<CompletionEvent> {
        self.text.take_completion()
    }

    pub fn take_code_actions(&mut self) -> Option<CodeActionEvent> {
        self.text.take_code_actions()
    }

    pub fn take_formatting(&mut self) -> Option<FormattingEvent> {
        self.text.take_formatting()
    }

    pub fn cancel_completion(&mut self) -> bool {
        self.text.cancel_completion(self.service.as_ref())
    }

    pub fn cancel_code_actions(&mut self) -> bool {
        self.text.cancel_code_actions(self.service.as_ref())
    }

    pub fn diagnostics_in_range(
        &self,
        start: CursorPosition,
        end: CursorPosition,
    ) -> Vec<Diagnostic> {
        self.diagnostics.in_range(start, end)
    }

    pub fn prepare_rename(&mut self, editor: &Editor) -> Result<u64, String> {
        let Some(service) = self.service.as_ref() else {
            return Err("lsp unavailable".to_string());
        };
        let Some(path) = editor_path(editor) else {
            return Err("E32: No file name".to_string());
        };
        if let Some(request) = self.rename_request.take() {
            service.cancel(request);
        }
        self.rename_ready = None;
        self.rename_edit = None;
        self.rename_error = None;
        let request = service.prepare_rename(&path, super::lsp_navigation::cursor_position(editor));
        self.rename_request = Some(request);
        Ok(request)
    }

    pub fn rename(&mut self, request: u64, name: &str) -> bool {
        self.service
            .as_ref()
            .is_some_and(|service| service.rename(request, name))
    }

    pub fn cancel_rename(&mut self, request: u64) {
        if let Some(service) = self.service.as_ref() {
            service.cancel(request);
        }
        if self.rename_request == Some(request) {
            self.rename_request = None;
        }
        self.rename_ready = None;
        self.rename_edit = None;
        self.rename_error = None;
    }

    pub fn take_rename_ready(&mut self) -> Option<RenameReady> {
        self.rename_ready.take()
    }

    pub fn take_rename_edit(&mut self) -> Option<RenameEvent> {
        self.rename_edit.take()
    }

    pub fn language_indent(&self, path: Option<&Path>) -> LanguageIndent {
        let Some(languages) = self.languages.as_ref() else {
            return LanguageIndent::default();
        };
        let Some(language) = path.and_then(|path| languages.language_for_path(path)) else {
            return LanguageIndent::default();
        };
        LanguageIndent {
            shiftwidth: language.shiftwidth,
            expandtab: language.expandtab,
        }
    }

    pub fn take_rename_error(&mut self) -> Option<u64> {
        self.rename_error.take()
    }

    pub fn cancel_pending(&mut self) -> bool {
        let navigation = self.navigation.cancel(self.service.as_ref());
        let text = self.text.cancel_all(self.service.as_ref());
        let Some(request) = self.rename_request.take() else {
            return navigation || text;
        };
        if let Some(service) = self.service.as_ref() {
            service.cancel(request);
        }
        self.rename_ready = None;
        self.rename_edit = None;
        self.rename_error = None;
        true
    }

    pub fn update_display(&mut self) -> bool {
        let display = self.compute_display();
        if display == self.display {
            return false;
        }
        self.display = display;
        true
    }

    pub fn save(&mut self) {
        let (Some(service), Some(path)) = (self.service.as_ref(), self.path.as_ref()) else {
            return;
        };
        service.save(path);
    }

    pub fn status(&self) -> Option<&str> {
        self.display.as_deref()
    }

    pub fn has_diagnostics(&self) -> bool {
        self.diagnostics.has_diagnostics()
    }

    pub fn diagnostic_severity(&self, row: usize) -> Option<DiagnosticSeverity> {
        self.diagnostics.severity_at(row)
    }

    pub fn diagnostic_spans(&self, window: &[String], top: usize) -> Vec<DiagnosticSpan> {
        self.diagnostics.spans(window, top)
    }

    pub fn navigate_diagnostics(
        &self,
        row: usize,
        column: usize,
        forward: bool,
    ) -> Option<Navigation> {
        self.diagnostics.navigate(row, column, forward)
    }

    pub fn take_message(&mut self) -> Option<String> {
        self.message.take()
    }

    fn sync_document(&mut self, editor: &Editor) -> bool {
        let path = editor_path(editor);
        let revision = editor.buffer_revision();
        if path != self.path {
            self.text.cancel_all(self.service.as_ref());
            self.close_current();
            self.statuses.clear();
            self.status_messages.clear();
            self.progress.clear();
            self.diagnostics.reset();
            self.path = path;
            self.revision = Some(revision);
            if let (Some(service), Some(path)) = (self.service.as_ref(), self.path.as_ref()) {
                service.open(path, editor.document_snapshot());
            }
            return true;
        }
        if Some(revision) == self.revision {
            return false;
        }
        self.revision = Some(revision);
        self.text.cancel_all(self.service.as_ref());
        if let (Some(service), Some(path)) = (self.service.as_ref(), self.path.as_ref()) {
            self.diagnostics.advance();
            service.change(path, editor.document_snapshot());
            return true;
        }
        false
    }

    fn close_current(&mut self) {
        let (Some(service), Some(path)) = (self.service.as_ref(), self.path.as_ref()) else {
            return;
        };
        service.close(path);
    }

    fn apply_event(&mut self, event: LspEvent) -> bool {
        match event {
            LspEvent::Status(status) => {
                self.apply_status(status);
                true
            }
            LspEvent::Progress(progress) => {
                self.apply_progress(progress);
                true
            }
            LspEvent::Diagnostics(diagnostics) => self
                .path
                .as_deref()
                .is_some_and(|path| self.diagnostics.apply(diagnostics, path)),
            LspEvent::Definition(location) => self
                .navigation
                .apply_location(&location, LocationKind::Definition),
            LspEvent::References(location) => self
                .navigation
                .apply_location(&location, LocationKind::References),
            LspEvent::Hover(hover) => self.navigation.apply_hover(&hover),
            LspEvent::Completion(event) => self.text.apply_completion(event),
            LspEvent::CodeActions(event) => self.text.apply_code_actions(event),
            LspEvent::Formatting(event) => self.text.apply_formatting(event),
            LspEvent::RenameReady(ready) => {
                if self.rename_request != Some(ready.request) {
                    return false;
                }
                self.rename_ready = Some(ready);
                true
            }
            LspEvent::Rename(event) => {
                if self.rename_request != Some(event.request) {
                    return false;
                }
                self.rename_request = None;
                self.rename_edit = Some(event);
                true
            }
            LspEvent::RequestError { request, message } => {
                if self.rename_request == Some(request) {
                    self.rename_request = None;
                    self.rename_ready = None;
                    self.rename_error = Some(request);
                    self.message = Some(message);
                    return true;
                }
                if self.navigation.apply_error(request) {
                    self.message = Some(message);
                    return true;
                }
                if self.text.fail(request) {
                    self.message = Some(message);
                    return true;
                }
                false
            }
            LspEvent::Message {
                server,
                level,
                message,
            } => {
                self.message = Some(format_message(&server, level, &message));
                true
            }
            LspEvent::Log { .. } => false,
        }
    }

    fn apply_status(&mut self, status: StatusEvent) {
        let server = status.server;
        self.statuses.insert(server.clone(), status.state);
        if matches!(status.state, ServerState::Failed | ServerState::Stopped) {
            self.diagnostics.remove_server(&server);
        }
        match status.message {
            Some(message) => {
                self.status_messages.insert(server, message);
            }
            None => {
                self.status_messages.remove(&server);
            }
        }
    }

    fn apply_progress(&mut self, progress: ProgressEvent) {
        let key = (progress.server, progress.token);
        if progress.done {
            self.progress.remove(&key);
            return;
        }
        let snapshot = self
            .progress
            .entry(key)
            .or_insert_with(|| ProgressSnapshot {
                title: None,
                message: None,
                percentage: None,
            });
        if progress.title.is_some() {
            snapshot.title = progress.title;
        }
        if progress.message.is_some() {
            snapshot.message = progress.message;
        }
        if progress.percentage.is_some() {
            snapshot.percentage = progress.percentage;
        }
    }

    fn compute_display(&self) -> Option<String> {
        let path = self.path.as_ref()?;
        let languages = self.languages.as_ref()?;
        if let Some(missing) = languages.missing_binary_status(Some(path)) {
            return Some(missing);
        }
        let language = languages.language_for_path(path)?;
        let progress: Vec<String> = self
            .progress
            .iter()
            .filter(|((server, _), _)| language.servers.contains(server))
            .map(|((server, _), snapshot)| format_progress(server, snapshot))
            .collect();
        if !progress.is_empty() {
            return Some(progress.join("; "));
        }
        let statuses: Vec<String> = language
            .servers
            .iter()
            .filter_map(|server| {
                let state = self.statuses.get(server)?;
                Some(format_status(
                    server,
                    *state,
                    self.status_messages.get(server),
                ))
            })
            .collect();
        (!statuses.is_empty()).then(|| statuses.join("; "))
    }
}

pub(super) fn editor_path(editor: &Editor) -> Option<PathBuf> {
    let path = editor
        .has_file()
        .then(|| PathBuf::from(editor.filename()))?;
    Some(std::path::absolute(&path).unwrap_or(path))
}

fn format_progress(server: &str, progress: &ProgressSnapshot) -> String {
    let mut text = format!("lsp: {server}");
    if let Some(title) = &progress.title {
        text.push(' ');
        text.push_str(&one_line(title));
    }
    if let Some(message) = &progress.message {
        text.push(' ');
        text.push_str(&one_line(message));
    }
    if let Some(percentage) = progress.percentage {
        text.push_str(&format!(" {percentage}%"));
    }
    text
}

fn format_status(server: &str, state: ServerState, message: Option<&String>) -> String {
    match state {
        ServerState::Starting | ServerState::Initializing => {
            format!("lsp: {server} starting")
        }
        ServerState::Ready => format!("lsp: {server} ready"),
        ServerState::Missing => format!("lsp: {server} not found in PATH"),
        ServerState::Failed => message.map_or_else(
            || format!("lsp: {server} failed"),
            |message| format!("lsp: {server} failed: {}", one_line(message)),
        ),
        ServerState::Stopped => format!("lsp: {server} stopped"),
    }
}

fn format_message(server: &str, level: MessageLevel, message: &str) -> String {
    let message = one_line(message);
    match level {
        MessageLevel::Error => format!("{server}: E: {message}"),
        MessageLevel::Warning => format!("{server}: W: {message}"),
        MessageLevel::Info | MessageLevel::Log => format!("{server}: {message}"),
    }
}

pub(super) fn one_line(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_errors_are_reported_for_the_pending_request() {
        let mut ui = LspUi {
            rename_request: Some(5),
            ..LspUi::default()
        };
        assert!(ui.apply_event(LspEvent::RequestError {
            request: 5,
            message: "failed".to_string(),
        }));
        assert_eq!(ui.take_rename_error(), Some(5));
        assert_eq!(ui.take_message().as_deref(), Some("failed"));
        assert!(ui.rename_request.is_none());
    }

    #[test]
    fn cancel_pending_cancels_a_rename_request() {
        let mut ui = LspUi {
            rename_request: Some(7),
            ..LspUi::default()
        };
        assert!(ui.cancel_pending());
        assert!(ui.rename_request.is_none());
        assert!(!ui.cancel_pending());
    }

    #[test]
    fn progress_format_includes_percentage() {
        let progress = ProgressSnapshot {
            title: Some("Indexing".to_string()),
            message: Some("half".to_string()),
            percentage: Some(50),
        };
        assert_eq!(
            format_progress("rust-analyzer", &progress),
            "lsp: rust-analyzer Indexing half 50%"
        );
    }

    #[test]
    fn status_format_reports_missing_binary() {
        assert_eq!(
            format_status("rust-analyzer", ServerState::Missing, None),
            "lsp: rust-analyzer not found in PATH"
        );
    }

    #[test]
    fn status_format_includes_failure_details() {
        let message = "crashed".to_string();
        assert_eq!(
            format_status("rust-analyzer", ServerState::Failed, Some(&message)),
            "lsp: rust-analyzer failed: crashed"
        );
    }

    #[test]
    #[cfg(unix)]
    fn display_prefers_progress_over_ready_status() {
        let root = std::env::temp_dir().join(format!("kawi_lsp_ui_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("languages.toml");
        std::fs::write(
            &config,
            "[server.fake]\ncommand = \"/bin/true\"\n\n[[language]]\nname = \"rust\"\nservers = [\"fake\"]\n",
        )
        .unwrap();
        let mut ui = LspUi {
            languages: Some(Languages::load(&config)),
            path: Some(root.join("main.rs")),
            ..LspUi::default()
        };
        ui.statuses.insert("fake".to_string(), ServerState::Ready);
        assert!(ui.update_display());
        assert_eq!(ui.status(), Some("lsp: fake ready"));
        ui.progress.insert(
            ("fake".to_string(), "token".to_string()),
            ProgressSnapshot {
                title: Some("Indexing".to_string()),
                message: None,
                percentage: Some(50),
            },
        );
        assert!(ui.update_display());
        assert_eq!(ui.status(), Some("lsp: fake Indexing 50%"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
