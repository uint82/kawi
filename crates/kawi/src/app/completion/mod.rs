mod auto;
mod config;
mod lsp_source;
mod prefix;
mod session;
mod triggers;

pub(super) use auto::AutoTrigger;
pub(super) use config::CompletionOptions;
pub(super) use session::CompletionSession;
pub(super) use triggers::is_completion_key;

use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;

use kawi_completion::{Request, ResolveRequest, Source, Trigger, TriggerKind};
use kawi_lsp::{CompletionEvent, CompletionResolveEvent};

use super::lsp::editor_path;
use super::App;
use prefix::completion_prefix;
use triggers::{keeps_completion_open, trigger_for_key};

const AUTO_COMPLETION_DELAY: Duration = Duration::from_millis(120);

impl App {
    pub(super) fn request_completion(&mut self) -> anyhow::Result<bool> {
        self.start_completion(Trigger::invoked(), true);
        Ok(false)
    }

    fn start_completion(&mut self, trigger: Trigger, report_errors: bool) {
        self.completion_auto.cancel();
        let Some(path) = editor_path(&self.editor) else {
            if report_errors {
                self.show_message("E32: No file name".to_string());
            }
            return;
        };
        let position = super::lsp_navigation::cursor_position(&self.editor);
        let request = Request {
            path,
            line: position.line,
            column: position.column,
            trigger,
        };
        match Source::request(&mut self.lsp, request) {
            Ok(request) => {
                self.lsp
                    .set_completion_report_errors(request, report_errors);
                self.code_actions = None;
            }
            Err(message) if report_errors => self.show_message(message),
            Err(_) => {}
        }
    }

    pub(super) fn handle_completion_key(&mut self, key: KeyEvent) -> anyhow::Result<Option<bool>> {
        let Some(mut session) = self.completion.take() else {
            return Ok(None);
        };
        let docs_up = self.completion_options.keys.up.event();
        let docs_down = self.completion_options.keys.down.event();
        let docs_scroll = if key == docs_up || key == docs_down {
            self.completion_docs_scroll()
        } else {
            None
        };
        match session.handle_key(key, docs_up, docs_down, docs_scroll) {
            session::CompletionOutcome::Consumed => {
                self.completion = Some(session);
                self.resolve_completion_docs();
                Ok(Some(false))
            }
            session::CompletionOutcome::Cancel => {
                self.completion_auto.cancel();
                self.lsp.cancel_completion();
                self.lsp.cancel_completion_resolve();
                Ok(Some(false))
            }
            session::CompletionOutcome::Accept => {
                self.accept_completion(&session);
                self.completion_auto.cancel();
                self.lsp.cancel_completion();
                self.lsp.cancel_completion_resolve();
                Ok(Some(false))
            }
            session::CompletionOutcome::Fallthrough => {
                if keeps_completion_open(&self.lsp, key) {
                    self.completion = Some(session);
                } else {
                    self.completion_auto.cancel();
                    self.lsp.cancel_completion();
                    self.lsp.cancel_completion_resolve();
                }
                Ok(None)
            }
        }
    }

    pub(super) fn handle_completion_event(&mut self, event: CompletionEvent) {
        let event = lsp_source::neutral_event(event);
        if event.items.is_empty() {
            self.completion = None;
            return;
        }
        let prefix = completion_prefix(&self.editor);
        if event.cursor != prefix.cursor {
            return;
        }
        match self.completion.as_mut() {
            Some(session) => session.update(event, &prefix.text),
            None => {
                self.completion = Some(CompletionSession::new(event, &prefix.text));
            }
        }
        if self
            .completion
            .as_ref()
            .is_some_and(|session| session.items().is_empty())
        {
            self.completion = None;
            return;
        }
        self.resolve_completion_docs();
    }

    pub(super) fn handle_completion_resolved(&mut self, event: CompletionResolveEvent) {
        let Some(session) = self.completion.as_mut() else {
            return;
        };
        session.apply_resolved(lsp_source::neutral_resolved(event));
    }

    pub(super) fn handle_completion_resolve_failed(&mut self, request: u64) {
        if let Some(session) = self.completion.as_mut() {
            session.resolve_failed(request);
        }
    }

    fn resolve_completion_docs(&mut self) {
        let Some(item) = self
            .completion
            .as_ref()
            .and_then(CompletionSession::needs_resolve)
            .cloned()
        else {
            return;
        };
        let request = ResolveRequest { item: item.clone() };
        if let Ok(request) = Source::resolve(&mut self.lsp, request) {
            if let Some(session) = self.completion.as_mut() {
                session.start_resolve(request, &item);
            }
        }
    }

    fn accept_completion(&mut self, session: &CompletionSession) {
        let had_snippet = self.snippet.is_some();
        let cursor = completion_prefix(&self.editor).cursor;
        let Some(edit) = session.workspace_edit(cursor) else {
            return;
        };
        let snippet = session
            .selected_item()
            .and_then(|item| item.snippet.clone());
        if snippet.is_some() {
            self.end_snippet();
        }
        let command = session.command().map(|command| {
            (
                session.selected_item().map(|item| item.source.clone()),
                command.clone(),
            )
        });
        match self.apply_workspace_edit(&edit) {
            Ok(_) => {
                if let Some((Some(server), command)) = command {
                    let command = kawi_lsp::LspCommand {
                        title: command.title,
                        command: command.command,
                        arguments: match command.arguments {
                            Some(serde_json::Value::Array(arguments)) => Some(arguments),
                            _ => None,
                        },
                    };
                    if !self.lsp.execute_command(session.path(), &server, &command) {
                        self.show_message("lsp unavailable".to_string());
                    }
                }
                if let Some(snippet) = snippet {
                    self.start_snippet(snippet);
                } else if had_snippet {
                    self.update_snippet();
                    self.reselect_snippet_stop();
                }
                self.reveal_cursor();
            }
            Err(message) => self.show_message(message),
        }
    }

    pub(super) fn completion_wait_timeout(&self) -> Option<Duration> {
        self.completion_auto.wait_timeout()
    }

    pub(super) fn completion_tick(&mut self) -> bool {
        let Some(trigger) = self.completion_auto.take_due(Instant::now()) else {
            return false;
        };
        if !self.completion_request_allowed() {
            return false;
        }
        self.start_completion(trigger, false);
        true
    }

    pub(super) fn after_buffer_key(&mut self, key: KeyEvent) {
        self.snippet_choices = None;
        self.update_snippet();
        let prefix = completion_prefix(&self.editor);
        let trigger = trigger_for_key(&self.lsp, key, &prefix.text);
        let keep_open = keeps_completion_open(&self.lsp, key);
        if let Some(session) = self.completion.as_mut() {
            if prefix.text.is_empty() || !keep_open {
                self.completion = None;
                self.completion_auto.cancel();
                self.lsp.cancel_completion();
            } else {
                session.refilter(&prefix.text, None);
                if session.items().is_empty() {
                    self.completion = None;
                    self.completion_auto.cancel();
                    self.lsp.cancel_completion();
                }
            }
        }
        self.after_signature_key(key);
        if !self.completion_options.auto || !self.completion_request_allowed() {
            return;
        }
        if let Some(trigger) = trigger {
            let delay = match trigger.kind {
                TriggerKind::TriggerCharacter => Duration::ZERO,
                _ => AUTO_COMPLETION_DELAY,
            };
            self.completion_auto.schedule(trigger, delay);
            self.lsp.cancel_completion();
        } else if self.completion.is_none() {
            self.completion_auto.cancel();
        }
        self.resolve_completion_docs();
    }

    pub(super) fn close_completion_on_mode_change(&mut self) {
        if self.editor.mode().is_insert() {
            return;
        }
        self.end_snippet();
        self.completion_auto.cancel();
        self.completion = None;
        self.lsp.cancel_completion();
        self.lsp.cancel_completion_resolve();
        self.signature = None;
        self.lsp.cancel_signature();
    }

    fn completion_request_allowed(&self) -> bool {
        self.editor.mode().is_insert()
            && self.picker.is_none()
            && self.replace.is_none()
            && self.pager.is_none()
    }
}

#[cfg(test)]
mod tests;
