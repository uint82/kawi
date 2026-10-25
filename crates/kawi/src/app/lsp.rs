use super::lsp_diagnostics::{DiagnosticState, Navigation};
use super::lsp_navigation::{
    HoverResult, LocationKind, LocationResult, LspNavigation, NavigationRequest,
};
use super::lsp_text::{LspText, TextFailure};
use kawi_completion::Capabilities;
use kawi_editor::{
    default_pairs, BlockComment, CommentTokens, Editor, LanguageEditing, LanguageIndent, Pair,
};
use kawi_lsp::{
    CodeActionEvent, CompletionCapabilitiesEvent, CompletionEvent, CompletionResolveEvent,
    CompletionTrigger, CursorPosition, Diagnostic, DiagnosticMode, DiagnosticSeverity,
    DiagnosticsEvent, FileOperationEvent, FormattingEvent, FormattingSettings, LanguageWarning,
    Languages, LspCommand, LspEvent, LspService, MessageLevel, ProgressEvent, RenameEvent,
    RenameReady, ServerState, SignatureHelpEvent, StatusEvent, WatchedFileChange,
    WorkspaceDiagnosticState,
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
    diagnostic_modes: BTreeMap<String, DiagnosticMode>,
    workspace_diagnostics_loading: std::collections::BTreeSet<String>,
    workspace_diagnostics_requested: bool,
    diagnostics_activity: bool,
    navigation: LspNavigation,
    text: LspText,
    rename_request: Option<u64>,
    rename_ready: Option<RenameReady>,
    rename_edit: Option<RenameEvent>,
    rename_error: Option<u64>,
    display: Option<String>,
    message: Option<String>,
    formatting_failure: Option<(u64, String)>,
    diagnostic_updates: Vec<DiagnosticUpdate>,
    completion_capabilities: BTreeMap<String, CompletionCapabilitiesEvent>,
    rename_files: bool,
    delete_files: bool,
    file_operation: Option<FileOperationEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DiagnosticUpdate {
    Replace(DiagnosticsEvent),
    Clear(String),
}

#[derive(Clone)]
struct ProgressSnapshot {
    title: Option<String>,
    message: Option<String>,
    percentage: Option<u32>,
}

impl LspUi {
    pub fn load(&mut self, path: &Path, wake: Sender<()>) -> Vec<LanguageWarning> {
        self.rename_files = false;
        self.delete_files = false;
        self.file_operation = None;
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
        let document = self.sync_editor(editor);
        let events = self.poll_events(editor);
        let display = self.update_display();
        events || document || display
    }

    pub fn sync_editor(&mut self, editor: &Editor) -> bool {
        self.sync_document(editor)
    }

    pub fn did_change_watched_files(&self, files: &[(PathBuf, WatchedFileChange)]) {
        if let Some(service) = self.service.as_ref() {
            service.did_change_watched_files(files);
        }
    }

    pub fn supports_file_rename(&self) -> bool {
        self.rename_files
    }

    pub fn supports_file_delete(&self) -> bool {
        self.delete_files
    }

    pub fn will_rename_files(&self, files: &[(PathBuf, PathBuf)]) -> u64 {
        self.service
            .as_ref()
            .map_or(0, |service| service.will_rename_files(files))
    }

    pub fn will_delete_files(&self, files: &[PathBuf]) -> u64 {
        self.service
            .as_ref()
            .map_or(0, |service| service.will_delete_files(files))
    }

    pub fn take_file_operation(&mut self) -> Option<FileOperationEvent> {
        self.file_operation.take()
    }

    pub fn did_rename_files(&mut self, files: &[(PathBuf, PathBuf)]) {
        if let Some(service) = self.service.as_ref() {
            service.did_rename_files(files);
        }
        let Some(path) = self.path.as_mut() else {
            return;
        };
        let Some(rewritten) = files
            .iter()
            .find_map(|(old, new)| super::file_ops::rewrite_path(path, old, new))
        else {
            return;
        };
        *path = rewritten;
        self.text.cancel_all(self.service.as_ref());
        self.diagnostics.reset();
    }

    pub fn did_delete_files(&mut self, files: &[PathBuf]) {
        if let Some(service) = self.service.as_ref() {
            service.did_delete_files(files);
        }
        let removed = self.path.as_ref().is_some_and(|path| {
            files
                .iter()
                .any(|target| super::file_ops::path_under(path, target))
        });
        if !removed {
            return;
        }
        self.path = None;
        self.revision = None;
        self.text.cancel_all(self.service.as_ref());
        self.diagnostics.reset();
    }

    pub fn detach(&mut self) -> bool {
        self.text.cancel_all(self.service.as_ref());
        self.close_current();
        let changed = self.path.take().is_some();
        self.revision = None;
        self.statuses.clear();
        self.status_messages.clear();
        self.progress.clear();
        self.diagnostics.reset();
        self.update_display();
        changed
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

    pub fn completion_at(
        &mut self,
        path: &Path,
        position: CursorPosition,
        trigger: CompletionTrigger,
    ) -> Result<u64, String> {
        self.text
            .completion(self.service.as_ref(), path, position, trigger)
    }

    pub fn resolve_completion(&mut self, item_id: u64) -> Result<u64, String> {
        self.text.resolve_completion(self.service.as_ref(), item_id)
    }

    pub fn set_completion_report_errors(&mut self, request: u64, report: bool) {
        self.text.set_completion_report_errors(request, report);
    }

    pub fn signature_help(
        &mut self,
        editor: &Editor,
        trigger: CompletionTrigger,
    ) -> Result<u64, String> {
        let Some(path) = editor_path(editor) else {
            return Err("E32: No file name".to_string());
        };
        self.text.signature_help(
            self.service.as_ref(),
            &path,
            super::lsp_navigation::cursor_position(editor),
            trigger,
        )
    }

    pub fn completion_capabilities(&self) -> Capabilities {
        let servers: Vec<&CompletionCapabilitiesEvent> = self
            .path
            .as_deref()
            .and_then(|path| self.languages.as_ref()?.language_for_path(path))
            .map(|language| {
                language
                    .servers
                    .iter()
                    .filter_map(|server| self.completion_capabilities.get(server))
                    .collect()
            })
            .unwrap_or_else(|| self.completion_capabilities.values().collect());
        let mut capabilities = Capabilities::default();
        for server in servers {
            capabilities.completion |= server.completion;
            capabilities.resolve |= server.completion_resolve;
            capabilities.signature_help |= server.signature_help;
            capabilities
                .completion_triggers
                .extend(server.completion_triggers.iter().cloned());
            capabilities
                .signature_triggers
                .extend(server.signature_triggers.iter().cloned());
        }
        capabilities.completion_triggers.sort();
        capabilities.completion_triggers.dedup();
        capabilities.signature_triggers.sort();
        capabilities.signature_triggers.dedup();
        capabilities
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
        self.formatting_failure = None;
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

    pub fn take_completion_resolved(&mut self) -> Option<CompletionResolveEvent> {
        self.text.take_completion_resolved()
    }

    pub fn take_completion_resolve_failed(&mut self) -> Option<u64> {
        self.text.take_completion_resolve_failed()
    }

    pub fn take_signature(&mut self) -> Option<SignatureHelpEvent> {
        self.text.take_signature()
    }

    pub fn take_code_actions(&mut self) -> Option<CodeActionEvent> {
        self.text.take_code_actions()
    }

    pub fn take_formatting(&mut self) -> Option<FormattingEvent> {
        self.text.take_formatting()
    }

    pub fn take_formatting_failure(&mut self) -> Option<(u64, String)> {
        self.formatting_failure.take()
    }

    pub fn cancel_formatting(&mut self) {
        self.text.cancel_formatting(self.service.as_ref());
    }

    pub fn cancel_completion(&mut self) -> bool {
        self.text.cancel_completion(self.service.as_ref())
    }

    pub fn cancel_request(&mut self, request: u64) {
        if let Some(service) = self.service.as_ref() {
            service.cancel(request);
        }
    }

    pub fn cancel_completion_resolve(&mut self) -> bool {
        self.text.cancel_completion_resolve(self.service.as_ref())
    }

    pub fn cancel_signature(&mut self) -> bool {
        self.text.cancel_signature(self.service.as_ref())
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

    pub(super) fn take_diagnostic_updates(&mut self) -> Vec<DiagnosticUpdate> {
        std::mem::take(&mut self.diagnostic_updates)
    }

    pub(super) fn request_workspace_diagnostics(&mut self) -> bool {
        self.workspace_diagnostics_requested = true;
        self.service
            .as_ref()
            .is_some_and(LspService::workspace_diagnostics)
    }

    pub(super) fn diagnostics_loading(&self) -> bool {
        !self.workspace_diagnostics_loading.is_empty()
    }

    pub(super) fn take_diagnostics_activity(&mut self) -> bool {
        std::mem::take(&mut self.diagnostics_activity)
    }

    pub(super) fn diagnostic_mode_label(&self) -> &'static str {
        let modes: Vec<DiagnosticMode> = self.diagnostic_modes.values().copied().collect();
        if modes.contains(&DiagnosticMode::Workspace) {
            "workspace pull"
        } else if modes.contains(&DiagnosticMode::Pull) {
            "single-file pull"
        } else if modes.contains(&DiagnosticMode::Push) {
            "server push"
        } else {
            "off"
        }
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
        let Some(language) = self.language_config(path) else {
            return LanguageIndent::default();
        };
        LanguageIndent {
            shiftwidth: language.shiftwidth,
            expandtab: language.expandtab,
        }
    }

    pub fn language_editing(&self, path: Option<&Path>) -> LanguageEditing {
        let Some(language) = self.language_config(path) else {
            return LanguageEditing::default();
        };
        let comments = CommentTokens {
            line: language.line_comment.clone(),
            block: language
                .block_comment
                .as_ref()
                .map(|block| BlockComment::new(block.open.clone(), block.close.clone())),
        };
        let pairs = language
            .auto_pairs
            .as_ref()
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| Pair::new(pair.open, pair.close))
                    .collect()
            })
            .unwrap_or_else(default_pairs);
        LanguageEditing { comments, pairs }
    }

    fn language_config(&self, path: Option<&Path>) -> Option<&kawi_lsp::LanguageConfig> {
        self.languages.as_ref()?.language_for_path(path?)
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
        let Some(path) = self.path.clone() else {
            return;
        };
        self.save_path(&path);
    }

    pub fn save_path(&mut self, path: &Path) {
        let Some(service) = self.service.as_ref() else {
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
        self.text.cancel_document_requests(self.service.as_ref());
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
            LspEvent::DiagnosticMode(event) => {
                let workspace = event.mode == DiagnosticMode::Workspace;
                self.diagnostic_modes.insert(event.server, event.mode);
                if workspace && self.workspace_diagnostics_requested {
                    let _ = self
                        .service
                        .as_ref()
                        .is_some_and(LspService::workspace_diagnostics);
                }
                true
            }
            LspEvent::WorkspaceDiagnostics(event) => {
                self.diagnostics_activity = true;
                match event.state {
                    WorkspaceDiagnosticState::Started => {
                        self.workspace_diagnostics_loading.insert(event.server);
                    }
                    WorkspaceDiagnosticState::Finished => {
                        self.workspace_diagnostics_loading.remove(&event.server);
                    }
                }
                true
            }
            LspEvent::Diagnostics(diagnostics) => {
                let active = self.path.as_deref() == Some(diagnostics.path.as_path());
                let accepted = !active
                    || self
                        .diagnostics
                        .apply(diagnostics.clone(), &diagnostics.path);
                if accepted {
                    self.diagnostic_updates
                        .push(DiagnosticUpdate::Replace(diagnostics));
                }
                accepted
            }
            LspEvent::Definition(location) => self
                .navigation
                .apply_location(&location, LocationKind::Definition),
            LspEvent::References(location) => self
                .navigation
                .apply_location(&location, LocationKind::References),
            LspEvent::Hover(hover) => self.navigation.apply_hover(&hover),
            LspEvent::Completion(event) => self.text.apply_completion(event),
            LspEvent::CompletionResolved(event) => self.text.apply_completion_resolved(event),
            LspEvent::CompletionCapabilities(event) => {
                self.completion_capabilities
                    .insert(event.server.clone(), event);
                true
            }
            LspEvent::FileOperationCapabilities(event) => {
                self.rename_files |= event.rename;
                self.delete_files |= event.delete;
                true
            }
            LspEvent::FileOperation(event) => {
                self.file_operation = Some(event);
                true
            }
            LspEvent::SignatureHelp(event) => self.text.apply_signature(event),
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
                let formatting = self.text.formatting_pending(request);
                match self.text.fail(request) {
                    Some(TextFailure::Report) => {
                        if formatting {
                            self.formatting_failure = Some((request, message.clone()));
                        }
                        self.message = Some(message);
                        true
                    }
                    Some(TextFailure::Silent) => true,
                    None => false,
                }
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
            self.diagnostic_modes.remove(&server);
            self.workspace_diagnostics_loading.remove(&server);
            self.diagnostic_updates
                .push(DiagnosticUpdate::Clear(server.clone()));
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
    fn inactive_workspace_diagnostics_reach_the_problem_store() {
        let mut ui = LspUi::default();
        let path = PathBuf::from("/tmp/workspace-diagnostic.rs");
        let event = DiagnosticsEvent {
            server: "fake".to_string(),
            path: path.clone(),
            version: None,
            diagnostics: vec![Diagnostic {
                range: kawi_lsp::DiagnosticRange {
                    start: kawi_lsp::DiagnosticPosition {
                        line: 0,
                        character: 0,
                    },
                    end: kawi_lsp::DiagnosticPosition {
                        line: 0,
                        character: 1,
                    },
                },
                severity: DiagnosticSeverity::Error,
                message: "workspace".to_string(),
                source: None,
                code: None,
                tags: Vec::new(),
                related: Vec::new(),
            }],
        };
        assert!(ui.apply_event(LspEvent::Diagnostics(event)));
        let updates = ui.take_diagnostic_updates();
        assert!(matches!(
            updates.as_slice(),
            [DiagnosticUpdate::Replace(event)] if event.path == path
        ));
    }

    #[test]
    fn workspace_loading_and_mode_are_visible_to_the_pane() {
        let mut ui = LspUi::default();
        ui.apply_event(LspEvent::DiagnosticMode(kawi_lsp::DiagnosticModeEvent {
            server: "fake".to_string(),
            mode: DiagnosticMode::Workspace,
        }));
        assert_eq!(ui.diagnostic_mode_label(), "workspace pull");
        ui.apply_event(LspEvent::WorkspaceDiagnostics(
            kawi_lsp::WorkspaceDiagnosticEvent {
                server: "fake".to_string(),
                state: kawi_lsp::WorkspaceDiagnosticState::Started,
            },
        ));
        assert!(ui.diagnostics_loading());
        assert!(ui.take_diagnostics_activity());
        ui.apply_event(LspEvent::WorkspaceDiagnostics(
            kawi_lsp::WorkspaceDiagnosticEvent {
                server: "fake".to_string(),
                state: kawi_lsp::WorkspaceDiagnosticState::Finished,
            },
        ));
        assert!(!ui.diagnostics_loading());
        assert!(ui.take_diagnostics_activity());
    }

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
