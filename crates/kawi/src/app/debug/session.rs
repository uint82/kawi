use super::*;
use kawi_dap::{ExpandedConfiguration, StartRequest};

impl DebugUi {
    pub(super) fn start(&mut self, editor: &Editor, name: Option<&str>) -> bool {
        let (name, configuration) = match self.resolve_configuration(editor, name) {
            Ok(configuration) => configuration,
            Err(message) => {
                self.message = Some(message);
                return false;
            }
        };
        let Some(service) = self.service.as_ref() else {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        };
        let request = StartRequest {
            config: configuration,
            breakpoints: self.breakpoint_sets(),
            function_breakpoints: self.function_breakpoint_values(),
            exception_filters: self.exception_filters_for_start(),
        };
        if !service.start_request(request) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        self.status = DebugStatus::Starting;
        self.configuration = Some(name.clone());
        self.show_panel();
        self.message = Some(format!("debug: starting {name}"));
        self.reset_session_state();
        true
    }

    fn resolve_configuration(
        &self,
        editor: &Editor,
        name: Option<&str>,
    ) -> Result<(String, ExpandedConfiguration), String> {
        let name = match name {
            Some(name) => name.to_string(),
            None => self
                .config
                .default_for(language_id(editor))
                .map(|configuration| configuration.name.clone())
                .ok_or_else(|| {
                    "debug: no configuration for this buffer; create ~/.config/kawi/debug.toml"
                        .to_string()
                })?,
        };
        let context = DebugContext {
            workspace: self.workspace.clone(),
            file: editor_path(editor),
        };
        let configuration = self
            .config
            .expand(&name, &context)
            .map_err(|error| format!("debug: {error}"))?;
        Ok((name, configuration))
    }

    pub(super) fn restart(&mut self, editor: &Editor) -> bool {
        if self.supports_restart_request
            && matches!(self.status, DebugStatus::Running | DebugStatus::Stopped)
        {
            let Some(service) = self.service.as_ref() else {
                return false;
            };
            self.message = Some("debug: restarting adapter".to_string());
            return service.restart();
        }
        let Some(name) = self.configuration.clone() else {
            self.message = Some("debug: no previous configuration".to_string());
            return false;
        };
        self.start(editor, Some(&name))
    }

    pub(super) fn terminate(&mut self) -> bool {
        if !self.supports_terminate_request {
            self.message = Some("debug: adapter does not support terminate".to_string());
            return false;
        }
        if !matches!(self.status, DebugStatus::Running | DebugStatus::Stopped) {
            self.message = Some("debug: not running".to_string());
            return false;
        }
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.message = Some("debug: terminating".to_string());
        service.terminate()
    }

    pub(super) fn set_variable(&mut self, reference: i64, name: String, value: String) -> bool {
        if reference <= 0 || name.is_empty() {
            self.message = Some("debug: invalid variable reference or name".to_string());
            return false;
        }
        if self.status != DebugStatus::Stopped {
            self.message = Some("debug: not stopped".to_string());
            return false;
        }
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        if !service.set_variable(reference, name, value) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
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
        if !service.stack_trace(thread_id) {
            self.pending_report = None;
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn show_status(&mut self) {
        self.set_report(report::status(
            self.status,
            self.configuration.as_deref(),
            self.thread_id,
            self.current_frame.as_ref(),
            self.breakpoint_count(),
            self.function_breakpoints.len(),
            self.enabled_exception_filters.len(),
        ));
    }

    pub(super) fn show_configurations(&mut self) {
        self.set_report(report::configurations(&self.config));
    }

    pub(super) fn show_output(&mut self) {
        self.set_report(report::output(&self.output));
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
            self.clear_frame_state();
        }
        sent
    }
}

fn language_id(editor: &Editor) -> Option<&str> {
    editor.language_id().map(|language| language.name())
}
