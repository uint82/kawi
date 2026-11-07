use super::worker::{PreviewFile, PreviewService};
use kawi_syntax::{LanguageId, SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxService, SyntaxSpan};
use ropey::Rope;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(80);

#[derive(Default)]
pub(crate) struct PreviewState {
    service: Option<PreviewService>,
    syntax: Option<SyntaxService>,
    root: PathBuf,
    target: Option<PreviewTarget>,
    deadline: Option<Instant>,
    request: Option<u64>,
    syntax_request: Option<u64>,
    next_document: u64,
    entry: Option<PreviewEntry>,
    status: Option<String>,
    loading: bool,
}

struct PreviewTarget {
    path: PathBuf,
    text: Option<Rope>,
    focus_line: usize,
}

struct PreviewEntry {
    title: String,
    lines: Vec<String>,
    first_line: usize,
    total_lines: usize,
    focus_line: usize,
    syntax: Vec<SyntaxSpan>,
}

impl PreviewState {
    pub(crate) fn new(root: PathBuf, wake: Sender<()>) -> Self {
        Self {
            service: Some(PreviewService::with_wake(wake.clone())),
            syntax: Some(SyntaxService::with_wake(wake)),
            root,
            ..Self::default()
        }
    }

    pub(crate) fn detach(&mut self) {
        if let Some(service) = self.service.as_mut() {
            service.detach();
        }
        if let Some(syntax) = self.syntax.as_mut() {
            syntax.detach();
        }
    }

    pub(crate) fn schedule(
        &mut self,
        path: &Path,
        text: Option<Rope>,
        focus_line: usize,
        now: Instant,
    ) {
        self.target = Some(PreviewTarget {
            path: path.to_path_buf(),
            text,
            focus_line,
        });
        self.deadline = Some(now.checked_add(PREVIEW_DEBOUNCE).unwrap_or(now));
        self.request = None;
        self.syntax_request = None;
        self.entry = None;
        self.status = None;
        self.loading = true;
    }

    pub(crate) fn clear(&mut self) {
        self.target = None;
        self.deadline = None;
        self.request = None;
        self.syntax_request = None;
        self.entry = None;
        self.status = None;
        self.loading = false;
    }

    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub(crate) fn tick(&mut self, now: Instant) -> bool {
        let Some(deadline) = self.deadline else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.deadline = None;
        let Some(target) = self.target.take() else {
            return false;
        };
        let Some(service) = self.service.as_ref() else {
            self.status = Some("preview unavailable".to_string());
            self.loading = false;
            return true;
        };
        self.request = Some(service.submit(target.path, target.text, target.focus_line));
        self.loading = true;
        true
    }

    pub(crate) fn poll(&mut self) -> bool {
        let preview_changed = self.poll_preview();
        let syntax_changed = self.poll_syntax();
        preview_changed || syntax_changed
    }

    fn poll_preview(&mut self) -> bool {
        let mut changed = false;
        while let Some(result) = self.service.as_ref().and_then(PreviewService::try_recv) {
            if Some(result.id) != self.request {
                continue;
            }
            self.request = None;
            changed = true;
            if let Some(error) = result.error {
                self.status = Some(error);
                self.loading = false;
                continue;
            }
            if result.text.is_none() {
                continue;
            }
            self.apply_preview(result);
        }
        changed
    }

    fn apply_preview(&mut self, result: PreviewFile) {
        let Some(text) = result.text else {
            return;
        };
        self.entry = Some(PreviewEntry {
            title: self.title_for(&result.path),
            lines: result.lines,
            first_line: result.first_line,
            total_lines: result.total_lines,
            focus_line: result.focus_line,
            syntax: Vec::new(),
        });
        self.submit_syntax(result.path, text, result.first_line, result.total_lines);
    }

    fn submit_syntax(&mut self, path: PathBuf, text: Rope, first_line: usize, total_lines: usize) {
        let Some(language) = LanguageId::from_path(&path.to_string_lossy()) else {
            self.loading = false;
            return;
        };
        let Some(entry) = self.entry.as_mut() else {
            self.loading = false;
            return;
        };
        let end = first_line
            .saturating_add(entry.lines.len())
            .min(total_lines);
        entry.syntax.clear();
        self.next_document = self.next_document.saturating_add(1);
        let key = SyntaxKey {
            document: self.next_document,
            revision: self.next_document,
            top: first_line,
            end,
            language,
        };
        let Some(syntax) = self.syntax.as_ref() else {
            self.loading = false;
            return;
        };
        self.syntax_request = Some(syntax.submit(SyntaxRequest { key, text }));
    }

    fn poll_syntax(&mut self) -> bool {
        let mut changed = false;
        while let Some(result) = self.syntax.as_ref().and_then(SyntaxService::try_recv) {
            if Some(result.generation) != self.syntax_request {
                continue;
            }
            self.syntax_request = None;
            if self.apply_syntax(result) {
                changed = true;
            }
        }
        changed
    }

    fn apply_syntax(&mut self, result: SyntaxResult) -> bool {
        let Some(entry) = self.entry.as_mut() else {
            return false;
        };
        entry.syntax = result.spans;
        self.loading = false;
        true
    }

    fn title_for(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }

    pub(crate) fn title(&self) -> Option<&str> {
        if let Some(error) = self.status.as_deref() {
            return Some(error);
        }
        self.entry.as_ref().map(|entry| entry.title.as_str())
    }

    pub(crate) fn lines(&self) -> &[String] {
        self.entry
            .as_ref()
            .map_or(&[], |entry| entry.lines.as_slice())
    }

    pub(crate) fn first_line(&self) -> usize {
        self.entry.as_ref().map_or(0, |entry| entry.first_line)
    }

    pub(crate) fn total_lines(&self) -> usize {
        self.entry.as_ref().map_or(0, |entry| entry.total_lines)
    }

    pub(crate) fn focus_line(&self) -> Option<usize> {
        self.entry.as_ref().map(|entry| entry.focus_line)
    }

    pub(crate) fn syntax(&self) -> &[SyntaxSpan] {
        self.entry
            .as_ref()
            .map_or(&[], |entry| entry.syntax.as_slice())
    }

    pub(crate) fn is_loading(&self) -> bool {
        self.loading
    }
}
