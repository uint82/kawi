use super::lsp_diagnostics::{DiagnosticState, Navigation};
use kawi_editor::Editor;
use kawi_lsp::{
    DiagnosticSeverity, Languages, LspEvent, LspService, MessageLevel, ProgressEvent, ServerState,
    StatusEvent,
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
    pub fn load(&mut self, path: &Path, wake: Sender<()>) -> Option<String> {
        let languages = Languages::load(path);
        let warning = languages.warning().map(str::to_string);
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
        warning
    }

    pub fn refresh(&mut self, editor: &Editor) -> bool {
        let events = self.poll_events();
        let document = self.sync_document(editor);
        let display = self.update_display();
        events || document || display
    }

    pub fn poll_events(&mut self) -> bool {
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
            self.close_current();
            self.statuses.clear();
            self.status_messages.clear();
            self.progress.clear();
            self.diagnostics.reset();
            self.path = path;
            self.revision = Some(revision);
            if let (Some(service), Some(path)) = (self.service.as_ref(), self.path.as_ref()) {
                service.open(path, &editor.document_text());
            }
            return true;
        }
        if Some(revision) == self.revision {
            return false;
        }
        self.revision = Some(revision);
        if let (Some(service), Some(path)) = (self.service.as_ref(), self.path.as_ref()) {
            self.diagnostics.advance();
            service.change(path, &editor.document_text());
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

fn editor_path(editor: &Editor) -> Option<PathBuf> {
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
