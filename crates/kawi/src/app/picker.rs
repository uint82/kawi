use std::path::{Path, PathBuf};

use crossterm::event::KeyEvent;
use kawi_picker::{
    Picker, PickerKind, PickerOutcome, ProjectRequest, ProjectResult, ProjectService,
};

pub struct PickerSession {
    picker: Picker,
    service: Option<ProjectService>,
    root: PathBuf,
    latest: u64,
}

impl PickerSession {
    pub fn files(root: PathBuf) -> Self {
        let mut picker = Picker::files();
        picker.set_loading(true);
        let service = ProjectService::new();
        let latest = service.submit(ProjectRequest::Files { root: root.clone() });
        Self {
            picker,
            service: Some(service),
            root,
            latest,
        }
    }

    pub fn grep(root: PathBuf, query: Option<&str>) -> Self {
        let picker = Picker::grep();
        let mut session = Self {
            picker,
            service: Some(ProjectService::new()),
            root,
            latest: 0,
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
        }
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

    pub fn pending(&self) -> bool {
        self.service.is_some() && self.picker.loading()
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
