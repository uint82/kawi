use super::diagnostics::convert_diagnostics;
use super::*;
use crate::protocol::{Message, RequestId, ResponseError};
use crate::workspace;
use lsp_types::{
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    TextDocumentIdentifier,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) const DIAGNOSTIC_DEBOUNCE: Duration = Duration::from_millis(180);

#[derive(Default)]
pub(super) struct PullDiagnostics {
    deadlines: HashMap<PathBuf, Instant>,
    pending: HashMap<RequestId, PendingDiagnostic>,
    latest: HashMap<(ClientKey, PathBuf), RequestId>,
    result_ids: HashMap<(ClientKey, PathBuf), String>,
}

struct PendingDiagnostic {
    key: ClientKey,
    path: PathBuf,
    version: i32,
    encoding: PositionEncodingKind,
}

impl PullDiagnostics {
    pub(super) fn is_pending(&self, id: &RequestId) -> bool {
        self.pending.contains_key(id)
    }

    pub(super) fn next_deadline(&self) -> Option<Duration> {
        self.deadlines
            .values()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .min()
    }

    fn forget_path(&mut self, path: &Path) -> Vec<(ClientKey, RequestId)> {
        self.deadlines.remove(path);
        self.result_ids
            .retain(|(_, candidate), _| candidate != path);
        self.latest.retain(|(_, candidate), _| candidate != path);
        self.take_pending(path)
    }

    fn take_pending(&mut self, path: &Path) -> Vec<(ClientKey, RequestId)> {
        let ids: Vec<(ClientKey, RequestId)> = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.path == path)
            .map(|(id, pending)| (pending.key.clone(), id.clone()))
            .collect();
        for (_, id) in &ids {
            self.pending.remove(id);
        }
        ids
    }

    pub(super) fn fail_client(&mut self, key: &ClientKey) {
        self.pending.retain(|_, pending| &pending.key != key);
        self.latest.retain(|(client, _), _| client != key);
        self.result_ids.retain(|(client, _), _| client != key);
    }
}

impl Manager {
    pub(super) fn schedule_pull_diagnostics(&mut self, path: &Path, delay: Duration) {
        let path = workspace::absolute(path);
        if !self.has_pull_diagnostics(&path) {
            self.pull_diagnostics.deadlines.remove(&path);
            return;
        }
        self.cancel_pending_pull(&path);
        self.pull_diagnostics.deadlines.insert(
            path,
            Instant::now()
                .checked_add(delay)
                .unwrap_or_else(Instant::now),
        );
    }

    pub(super) fn schedule_all_diagnostics(&mut self) {
        let paths: Vec<PathBuf> = self.documents.keys().cloned().collect();
        for path in paths {
            self.schedule_pull_diagnostics(&path, Duration::ZERO);
        }
    }

    pub(super) fn expire_pull_diagnostics(&mut self) {
        let now = Instant::now();
        let paths: Vec<PathBuf> = self
            .pull_diagnostics
            .deadlines
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(path, _)| path.clone())
            .collect();
        for path in paths {
            self.pull_diagnostics.deadlines.remove(&path);
            self.send_pull_diagnostics(&path);
        }
    }

    pub(super) fn forget_pull_diagnostics(&mut self, path: &Path) {
        let pending = self.pull_diagnostics.forget_path(path);
        self.cancel_pull_requests(pending);
    }

    fn cancel_pending_pull(&mut self, path: &Path) {
        let pending = self.pull_diagnostics.take_pending(path);
        self.cancel_pull_requests(pending);
    }

    fn cancel_pull_requests(&self, pending: Vec<(ClientKey, RequestId)>) {
        for (key, id) in pending {
            if let Some(client) = self.clients.get(&key) {
                client.client.send(Message::notification(
                    "$/cancelRequest",
                    Some(serde_json::json!({"id": id.to_value()})),
                ));
            }
        }
    }

    fn has_pull_diagnostics(&self, path: &Path) -> bool {
        let Some(document) = self.documents.get(path) else {
            return false;
        };
        document.clients.iter().any(|key| {
            self.clients.get(key).is_some_and(|client| {
                client.status == ServerState::Ready
                    && client.opened.contains(path)
                    && client.support.pull_diagnostics
            })
        })
    }

    fn send_pull_diagnostics(&mut self, path: &Path) {
        let Some(document) = self.documents.get(path) else {
            return;
        };
        let uri = document.uri.clone();
        let version = document.version;
        let keys: Vec<ClientKey> = document
            .clients
            .iter()
            .filter(|key| {
                self.clients.get(*key).is_some_and(|client| {
                    client.status == ServerState::Ready
                        && client.opened.contains(path)
                        && client.support.pull_diagnostics
                })
            })
            .cloned()
            .collect();
        for key in keys {
            let Some(encoding) = self.clients.get(&key).map(|client| client.encoding.clone())
            else {
                continue;
            };
            let previous = self
                .pull_diagnostics
                .result_ids
                .get(&(key.clone(), path.to_path_buf()))
                .cloned();
            let params = DocumentDiagnosticParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                identifier: None,
                previous_result_id: previous,
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            let Some(params) = to_params(params) else {
                continue;
            };
            let id = self.request_id();
            self.pull_diagnostics.pending.insert(
                id.clone(),
                PendingDiagnostic {
                    key: key.clone(),
                    path: path.to_path_buf(),
                    version,
                    encoding,
                },
            );
            self.pull_diagnostics
                .latest
                .insert((key.clone(), path.to_path_buf()), id.clone());
            let sent = self.clients.get(&key).is_some_and(|client| {
                client.client.send(Message::request(
                    id.clone(),
                    "textDocument/diagnostic",
                    Some(params),
                ))
            });
            if !sent {
                self.pull_diagnostics.pending.remove(&id);
                self.pull_diagnostics
                    .latest
                    .remove(&(key, path.to_path_buf()));
            }
        }
    }

    pub(super) fn handle_pull_diagnostic_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some(pending) = self.pull_diagnostics.pending.remove(&id) else {
            return;
        };
        if pending.key != key {
            return;
        }
        let latest_key = (key.clone(), pending.path.clone());
        if self.pull_diagnostics.latest.get(&latest_key) != Some(&id) {
            return;
        }
        self.pull_diagnostics.latest.remove(&latest_key);
        let report = match result {
            Ok(value) => value,
            Err(error) => {
                self.emit(LspEvent::Log {
                    server: key.server,
                    message: format!(
                        "diagnostic pull failed: {}",
                        one_line_message(&error.message)
                    ),
                });
                return;
            }
        };
        let report = match serde_json::from_value::<DocumentDiagnosticReportResult>(report) {
            Ok(report) => report,
            Err(error) => {
                self.emit(LspEvent::Log {
                    server: key.server,
                    message: format!("invalid diagnostic report: {error}"),
                });
                return;
            }
        };
        let DocumentDiagnosticReportResult::Report(report) = report else {
            return;
        };
        let (version, text) = {
            let Some(document) = self.documents.get(&pending.path) else {
                return;
            };
            if document.version != pending.version {
                return;
            }
            (document.version, document.text.clone())
        };
        let diagnostics = match report {
            DocumentDiagnosticReport::Full(report) => {
                self.record_result_id(&pending, report.full_document_diagnostic_report.result_id);
                convert_diagnostics(
                    &report.full_document_diagnostic_report.items,
                    &text,
                    &pending.encoding,
                )
            }
            DocumentDiagnosticReport::Unchanged(report) => {
                self.record_result_id(
                    &pending,
                    Some(report.unchanged_document_diagnostic_report.result_id),
                );
                return;
            }
        };
        self.emit(LspEvent::Diagnostics(DiagnosticsEvent {
            server: key.server,
            path: pending.path,
            version: Some(version),
            diagnostics,
        }));
    }

    fn record_result_id(&mut self, pending: &PendingDiagnostic, result_id: Option<String>) {
        let key = (pending.key.clone(), pending.path.clone());
        match result_id {
            Some(result_id) => {
                self.pull_diagnostics.result_ids.insert(key, result_id);
            }
            None => {
                self.pull_diagnostics.result_ids.remove(&key);
            }
        }
    }
}
