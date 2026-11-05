use kawi_lsp::{DocumentColor, DocumentColorsEvent, LspService};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) const COLOR_REQUEST_DELAY: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    path: PathBuf,
    revision: u64,
}

#[derive(Debug)]
pub(super) struct DocumentColorState {
    capabilities: BTreeMap<String, bool>,
    request: Option<u64>,
    deadline: Option<Instant>,
    target: Option<Target>,
    colors: Option<Vec<DocumentColor>>,
    enabled: bool,
}

impl Default for DocumentColorState {
    fn default() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            request: None,
            deadline: None,
            target: None,
            colors: None,
            enabled: true,
        }
    }
}

impl DocumentColorState {
    pub(super) fn supports(&self, server: &str) -> bool {
        self.capabilities.get(server).copied().unwrap_or(false)
    }

    pub(super) fn set_enabled(
        &mut self,
        enabled: bool,
        service: Option<&LspService>,
        supported: bool,
        now: Instant,
    ) -> bool {
        if self.enabled == enabled {
            return false;
        }
        self.enabled = enabled;
        if enabled {
            self.schedule(now, supported);
        } else {
            self.cancel(service);
            self.deadline = None;
            self.colors = None;
        }
        true
    }

    pub(super) fn document_changed(
        &mut self,
        service: Option<&LspService>,
        path: Option<PathBuf>,
        revision: u64,
        supported: bool,
        now: Instant,
    ) {
        self.cancel(service);
        self.target = path.map(|path| Target { path, revision });
        self.deadline = None;
        self.colors = None;
        self.schedule(now, supported);
    }

    pub(super) fn apply_capability(&mut self, server: String, supported: bool, now: Instant) {
        self.capabilities.insert(server, supported);
        if supported {
            self.schedule(now, true);
        }
    }

    pub(super) fn remove_server(&mut self, server: &str) {
        self.capabilities.remove(server);
        self.colors = None;
        self.deadline = Some(Instant::now());
    }

    pub(super) fn request_if_due(
        &mut self,
        now: Instant,
        supported: bool,
        request: impl FnOnce(&Path) -> u64,
    ) -> bool {
        let Some(deadline) = self.deadline else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.deadline = None;
        if !self.enabled || !supported || self.colors.is_some() || self.request.is_some() {
            return false;
        }
        let Some(target) = self.target.as_ref() else {
            return false;
        };
        let id = request(&target.path);
        if id == 0 {
            return false;
        }
        self.request = Some(id);
        true
    }

    pub(super) fn wait_timeout(&self, now: Instant) -> Option<Duration> {
        if !self.enabled || self.target.is_none() || self.request.is_some() || self.colors.is_some()
        {
            return None;
        }
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    pub(super) fn colors(&self, path: &Path, revision: u64) -> Option<&[DocumentColor]> {
        let target = self.target.as_ref()?;
        if target.path != path || target.revision != revision {
            return None;
        }
        self.colors.as_deref()
    }

    pub(super) fn apply_event(
        &mut self,
        event: DocumentColorsEvent,
        path: Option<&Path>,
        revision: Option<u64>,
    ) -> bool {
        if self.request != Some(event.request) {
            return false;
        }
        self.request = None;
        let Some(target) = self.target.as_ref() else {
            return false;
        };
        if target.path != event.path
            || path != Some(target.path.as_path())
            || revision != Some(target.revision)
        {
            return false;
        }
        self.colors = Some(event.colors);
        true
    }

    pub(super) fn fail(&mut self, request: u64) -> bool {
        if self.request != Some(request) {
            return false;
        }
        self.request = None;
        self.deadline = None;
        self.colors = None;
        true
    }

    pub(super) fn reset(&mut self, service: Option<&LspService>) {
        self.cancel(service);
        self.deadline = None;
        self.target = None;
        self.colors = None;
    }

    fn schedule(&mut self, now: Instant, supported: bool) {
        if !self.enabled
            || !supported
            || self.target.is_none()
            || self.request.is_some()
            || self.colors.is_some()
        {
            return;
        }
        self.deadline = Some(now + COLOR_REQUEST_DELAY);
    }

    fn cancel(&mut self, service: Option<&LspService>) {
        if let (Some(service), Some(request)) = (service, self.request.take()) {
            service.cancel(request);
        }
    }
}
