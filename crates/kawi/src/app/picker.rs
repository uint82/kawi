use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crossterm::event::KeyEvent;
use kawi_git::{LogEvent, LogOptions, LogStream};
use kawi_picker::{
    Picker, PickerKind, PickerOutcome, PickerRow, ProjectRequest, ProjectResult, ProjectService,
};

pub struct PickerSession {
    picker: Picker,
    service: Option<ProjectService>,
    root: PathBuf,
    latest: u64,
    quickfix: bool,
    log: Option<LogStream>,
    log_rows: Vec<PickerRow>,
    log_done: bool,
}

impl PickerSession {
    pub fn files(root: PathBuf, wake: Sender<()>) -> Self {
        let mut picker = Picker::files();
        picker.set_loading(true);
        let service = ProjectService::with_wake(wake);
        let latest = service.submit(ProjectRequest::Files { root: root.clone() });
        Self {
            picker,
            service: Some(service),
            root,
            latest,
            quickfix: false,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        }
    }

    pub fn grep(root: PathBuf, query: Option<&str>, wake: Sender<()>) -> Self {
        let picker = Picker::grep();
        let mut session = Self {
            picker,
            service: Some(ProjectService::with_wake(wake)),
            root,
            latest: 0,
            quickfix: false,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        };
        if let Some(query) = query {
            session.set_query(query);
        }
        session
    }

    pub fn buffers(root: PathBuf, buffers: Vec<(usize, String)>) -> Self {
        Self {
            picker: Picker::buffers(buffers),
            service: None,
            root,
            latest: 0,
            quickfix: false,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        }
    }

    pub fn locations(
        root: PathBuf,
        title: &'static str,
        rows: Vec<kawi_picker::PickerRow>,
    ) -> Self {
        Self {
            picker: Picker::locations(title, rows),
            service: None,
            root,
            latest: 0,
            quickfix: false,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        }
    }

    pub fn quickfix(root: PathBuf, rows: Vec<kawi_picker::PickerRow>) -> Self {
        Self {
            picker: Picker::locations("Problems", rows),
            service: None,
            root,
            latest: 0,
            quickfix: true,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        }
    }

    pub fn git_branches(root: PathBuf, rows: Vec<PickerRow>, status: Option<String>) -> Self {
        let mut picker = Picker::git_branches("Git branches", rows);
        if status.is_some() {
            picker.set_status(status);
        }
        Self {
            picker,
            service: None,
            root,
            latest: 0,
            quickfix: false,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        }
    }

    pub fn git_log(
        root: PathBuf,
        options: LogOptions,
        wake: std::sync::mpsc::Sender<()>,
    ) -> Result<Self, String> {
        let stream = LogStream::start(&root, &options, wake).map_err(|error| error.to_string())?;
        Ok(Self {
            picker: Picker::git_log("Git log"),
            service: None,
            root,
            latest: 0,
            quickfix: false,
            log: Some(stream),
            log_rows: Vec::new(),
            log_done: false,
        })
    }

    pub fn git_commit_files(root: PathBuf, rows: Vec<PickerRow>) -> Self {
        Self {
            picker: Picker::git_commit_files("Commit files", rows),
            service: None,
            root,
            latest: 0,
            quickfix: false,
            log: None,
            log_rows: Vec::new(),
            log_done: false,
        }
    }

    pub fn is_quickfix(&self) -> bool {
        self.quickfix
    }

    pub fn set_locations(&mut self, rows: Vec<kawi_picker::PickerRow>) {
        self.picker.set_locations(rows);
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.picker.set_status(status);
    }

    pub fn picker(&self) -> &Picker {
        &self.picker
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome {
        let outcome = self.picker.handle_key(key);
        if outcome == PickerOutcome::QueryChanged {
            self.submit_search();
        }
        outcome
    }

    pub fn set_query(&mut self, query: &str) {
        self.picker.set_query(query);
        self.submit_search();
    }

    pub fn paste(&mut self, text: &str) {
        self.picker.insert_text(text);
        self.submit_search();
    }

    pub fn refresh(&mut self) -> bool {
        let changed = self.refresh_project();
        changed | self.refresh_log()
    }

    fn refresh_project(&mut self) -> bool {
        let mut results = Vec::new();
        if let Some(service) = self.service.as_ref() {
            while let Some(result) = service.try_recv() {
                results.push(result);
            }
        }
        let mut changed = false;
        for result in results {
            if result.id() != self.latest {
                continue;
            }
            self.apply(result);
            changed = true;
        }
        changed
    }

    fn refresh_log(&mut self) -> bool {
        if self.picker.kind() != PickerKind::GitLog || self.log_done {
            return false;
        }
        let mut changed = false;
        while self.picker.needs_more_rows() {
            let Some(stream) = self.log.as_ref() else {
                break;
            };
            match stream.try_recv() {
                Some(LogEvent::Entries(entries)) => {
                    self.log_rows.extend(entries.into_iter().map(log_row));
                    self.picker.set_locations(self.log_rows.clone());
                    if !self.picker.rows().is_empty() {
                        self.picker.set_loading(false);
                    }
                    changed = true;
                }
                Some(LogEvent::Done(error)) => {
                    self.log_done = true;
                    self.finish_log(error);
                    changed = true;
                    break;
                }
                None => break,
            }
        }
        changed
    }

    fn finish_log(&mut self, error: Option<String>) {
        let status = match error {
            Some(error) => super::compact_error(&error),
            None if self.log_rows.is_empty() => "No commits".to_string(),
            None => "End of history".to_string(),
        };
        self.picker.set_status(Some(status));
    }

    fn apply(&mut self, result: ProjectResult) {
        match result {
            ProjectResult::Files { files, .. } => self.picker.set_files(files),
            ProjectResult::Grep { hits, error, .. } => match error {
                Some(error) => self.picker.set_error(super::compact_error(&error)),
                None => self.picker.set_hits(hits),
            },
            ProjectResult::Replace { .. } => {}
        }
    }

    fn submit_search(&mut self) {
        if self.picker.kind() != PickerKind::Grep {
            return;
        }
        let query = self.picker.query().to_string();
        if query.is_empty() {
            self.picker.clear_hits();
            return;
        }
        let Some(service) = self.service.as_ref() else {
            return;
        };
        self.latest = service.submit(ProjectRequest::Grep {
            root: self.root.clone(),
            query,
            limit: 2_000,
        });
        self.picker.set_loading(true);
    }
}

fn log_row(entry: kawi_git::LogEntry) -> PickerRow {
    PickerRow {
        display: entry.display(),
        path: entry.oid,
        line: None,
        column: None,
        buffer: None,
    }
}
