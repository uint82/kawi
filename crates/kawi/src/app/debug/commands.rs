use super::*;

impl DebugUi {
    pub(super) fn start(&mut self, editor: &Editor, name: Option<&str>) -> bool {
        let Some(service) = self.service.as_ref() else {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        };
        let name = match name {
            Some(name) => name.to_string(),
            None => match self.config.default_for(language_id(editor)) {
                Some(configuration) => configuration.name.clone(),
                None => {
                    self.message = Some(
                        "debug: no configuration for this buffer; create ~/.config/kawi/debug.toml"
                            .to_string(),
                    );
                    return false;
                }
            },
        };
        let context = DebugContext {
            workspace: self.workspace.clone(),
            file: editor_path(editor),
        };
        let configuration = match self.config.expand(&name, &context) {
            Ok(configuration) => configuration,
            Err(error) => {
                self.message = Some(format!("debug: {error}"));
                return false;
            }
        };
        if !service.start(configuration, self.breakpoint_sets()) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        self.status = DebugStatus::Starting;
        self.configuration = Some(name.clone());
        self.message = Some(format!("debug: starting {name}"));
        self.reset_session_state();
        true
    }

    pub(super) fn stop(&mut self) -> bool {
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        if matches!(self.status, DebugStatus::Idle | DebugStatus::Terminated) {
            self.message = Some("debug: not running".to_string());
            return false;
        }
        self.message = Some("debug: stopping".to_string());
        service.stop()
    }

    pub(super) fn continue_execution(&mut self) -> bool {
        self.request_thread(StepKind::Next, true)
    }

    pub(super) fn step(&mut self, kind: StepKind) -> bool {
        self.request_thread(kind, false)
    }

    pub(super) fn stack_trace(&mut self) -> bool {
        if self.status != DebugStatus::Stopped {
            self.message = Some("debug: not stopped".to_string());
            return false;
        }
        let Some(thread_id) = self.thread_id else {
            self.message = Some("debug: no stopped thread".to_string());
            return false;
        };
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.pending_report = Some(ReportKind::Stack);
        service.stack_trace(thread_id)
    }

    pub(super) fn variables(&mut self) -> bool {
        if self.status != DebugStatus::Stopped {
            self.message = Some("debug: not stopped".to_string());
            return false;
        }
        let Some(frame) = self.current_frame.as_ref() else {
            self.message = Some("debug: no current frame".to_string());
            return false;
        };
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.pending_report = Some(ReportKind::Variables);
        service.scopes(frame.id)
    }

    pub(crate) fn toggle_breakpoint(&mut self, editor: &Editor) -> bool {
        let Some(path) = editor_path(editor) else {
            self.message = Some("debug: no file name".to_string());
            return false;
        };
        let line = editor.row() as u32;
        let lines = self.breakpoints.entry(path.clone()).or_default();
        let enabled = if lines.remove(&line) {
            false
        } else {
            lines.insert(line);
            true
        };
        if lines.is_empty() {
            self.breakpoints.remove(&path);
        }
        self.sync_breakpoints(&path);
        self.message = Some(format!(
            "debug: breakpoint {} at {}:{}",
            if enabled { "set" } else { "removed" },
            path.display(),
            line.saturating_add(1)
        ));
        true
    }

    pub(super) fn show_breakpoints(&mut self) {
        self.report = Some(report::breakpoints(&self.breakpoints));
    }

    pub(super) fn show_status(&mut self) {
        self.report = Some(report::status(
            self.status,
            self.configuration.as_deref(),
            self.thread_id,
            self.current_frame.as_ref(),
            self.breakpoint_count(),
        ));
    }

    pub(super) fn show_configurations(&mut self) {
        self.report = Some(report::configurations(&self.config));
    }

    pub(super) fn show_output(&mut self) {
        self.report = Some(report::output(&self.output));
    }

    fn request_thread(&mut self, kind: StepKind, is_continue: bool) -> bool {
        if self.status != DebugStatus::Stopped {
            self.message = Some("debug: not stopped".to_string());
            return false;
        }
        let Some(thread_id) = self.thread_id else {
            self.message = Some("debug: no stopped thread".to_string());
            return false;
        };
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        let sent = if is_continue {
            service.continue_thread(thread_id)
        } else {
            service.step(thread_id, kind)
        };
        if sent {
            self.status = DebugStatus::Running;
            self.current_frame = None;
        }
        sent
    }

    fn breakpoint_sets(&self) -> Vec<BreakpointSet> {
        self.breakpoints
            .iter()
            .map(|(path, lines)| BreakpointSet {
                path: path.clone(),
                breakpoints: lines.iter().map(|line| source_breakpoint(*line)).collect(),
            })
            .collect()
    }

    fn sync_breakpoints(&self, path: &Path) {
        let Some(service) = self.service.as_ref() else {
            return;
        };
        let breakpoints = self
            .breakpoints
            .get(path)
            .map(|lines| lines.iter().map(|line| source_breakpoint(*line)).collect())
            .unwrap_or_default();
        service.set_breakpoints(path.to_path_buf(), breakpoints);
    }

    fn breakpoint_count(&self) -> usize {
        self.breakpoints.values().map(BTreeSet::len).sum()
    }
}

fn source_breakpoint(line: u32) -> SourceBreakpoint {
    SourceBreakpoint {
        line: line.saturating_add(1),
        condition: None,
        hit_condition: None,
        log_message: None,
    }
}

fn language_id(editor: &Editor) -> Option<&str> {
    editor.language_id().map(|language| language.name())
}
