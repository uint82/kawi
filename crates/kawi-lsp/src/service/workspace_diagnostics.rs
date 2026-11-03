use super::diagnostics::convert_diagnostics;
use super::*;
use crate::protocol::{Message, RequestId, ResponseError};
use crate::workspace::{self, file_uri, path_from_uri};
use lsp_types::{
    PartialResultParams, PositionEncodingKind, PreviousResultId, WorkDoneProgressParams,
    WorkspaceDiagnosticParams, WorkspaceDiagnosticReportResult, WorkspaceDocumentDiagnosticReport,
};
use serde_json::Value;

#[derive(Default)]
pub(super) struct WorkspaceDiagnostics {
    pending: HashMap<RequestId, ClientKey>,
    latest: HashMap<ClientKey, RequestId>,
    result_ids: HashMap<(ClientKey, PathBuf), String>,
}

impl WorkspaceDiagnostics {
    pub(super) fn is_pending(&self, id: &RequestId) -> bool {
        self.pending.contains_key(id)
    }

    pub(super) fn fail_client(&mut self, key: &ClientKey) {
        self.pending.retain(|_, client| client != key);
        self.latest.remove(key);
        self.result_ids.retain(|(client, _), _| client != key);
    }
}

impl Manager {
    pub(super) fn request_workspace_diagnostics(&mut self) {
        let keys: Vec<ClientKey> = self
            .clients
            .iter()
            .filter(|(_, client)| {
                client.status == ServerState::Ready && client.support.workspace_diagnostics
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            self.start_workspace_diagnostics(&key);
        }
    }

    fn start_workspace_diagnostics(&mut self, key: &ClientKey) {
        if let Some(previous) = self.workspace_diagnostics.latest.remove(key) {
            if self
                .workspace_diagnostics
                .pending
                .remove(&previous)
                .is_some()
            {
                self.cancel_workspace_request(key, &previous);
            }
        }
        let previous_result_ids = self
            .workspace_diagnostics
            .result_ids
            .iter()
            .filter(|((client, _), _)| client == key)
            .filter_map(|((_, path), value)| {
                file_uri(path).ok().map(|uri| PreviousResultId {
                    uri,
                    value: value.clone(),
                })
            })
            .collect();
        let identifier = self.diagnostic_identifier(key);
        let params = WorkspaceDiagnosticParams {
            identifier,
            previous_result_ids,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let Some(params) = to_params(params) else {
            return;
        };
        let id = self.request_id();
        self.workspace_diagnostics
            .pending
            .insert(id.clone(), key.clone());
        self.workspace_diagnostics
            .latest
            .insert(key.clone(), id.clone());
        self.emit(LspEvent::WorkspaceDiagnostics(WorkspaceDiagnosticEvent {
            server: key.server.clone(),
            state: WorkspaceDiagnosticState::Started,
        }));
        let sent = self.clients.get(key).is_some_and(|client| {
            client.client.send(Message::request(
                id.clone(),
                "workspace/diagnostic",
                Some(params),
            ))
        });
        if !sent {
            self.workspace_diagnostics.pending.remove(&id);
            self.workspace_diagnostics.latest.remove(key);
            self.finish_workspace_diagnostics(key);
        }
    }

    fn cancel_workspace_request(&self, key: &ClientKey, id: &RequestId) {
        if let Some(client) = self.clients.get(key) {
            client.client.send(Message::notification(
                "$/cancelRequest",
                Some(serde_json::json!({"id": id.to_value()})),
            ));
        }
    }

    pub(super) fn handle_workspace_diagnostic_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some(pending) = self.workspace_diagnostics.pending.remove(&id) else {
            return;
        };
        if pending != key {
            return;
        }
        if self.workspace_diagnostics.latest.get(&key) != Some(&id) {
            return;
        }
        self.workspace_diagnostics.latest.remove(&key);
        let report = match result {
            Ok(value) => value,
            Err(error) => {
                self.emit(LspEvent::Log {
                    server: key.server.clone(),
                    message: format!(
                        "workspace diagnostic pull failed: {}",
                        one_line_message(&error.message)
                    ),
                });
                self.finish_workspace_diagnostics(&key);
                return;
            }
        };
        let report = match serde_json::from_value::<WorkspaceDiagnosticReportResult>(report) {
            Ok(report) => report,
            Err(error) => {
                self.emit(LspEvent::Log {
                    server: key.server.clone(),
                    message: format!("invalid workspace diagnostic report: {error}"),
                });
                self.finish_workspace_diagnostics(&key);
                return;
            }
        };
        let (items, complete) = match report {
            WorkspaceDiagnosticReportResult::Report(report) => (report.items, true),
            WorkspaceDiagnosticReportResult::Partial(report) => (report.items, false),
        };
        let known: Vec<PathBuf> = self
            .workspace_diagnostics
            .result_ids
            .keys()
            .filter(|(client, _)| client == &key)
            .map(|(_, path)| path.clone())
            .collect();
        let encoding = self
            .clients
            .get(&key)
            .map_or(PositionEncodingKind::UTF16, |client| {
                client.encoding.clone()
            });
        let mut seen = HashSet::new();
        for item in items {
            self.apply_workspace_report(&key, &encoding, item, &mut seen);
        }
        if complete {
            for path in known {
                if !seen.contains(&path) {
                    self.clear_workspace_path(&key, path);
                }
            }
        }
        self.finish_workspace_diagnostics(&key);
    }

    fn apply_workspace_report(
        &mut self,
        key: &ClientKey,
        encoding: &PositionEncodingKind,
        report: WorkspaceDocumentDiagnosticReport,
        seen: &mut HashSet<PathBuf>,
    ) {
        match report {
            WorkspaceDocumentDiagnosticReport::Full(report) => {
                let Some(path) = path_from_uri(&report.uri) else {
                    return;
                };
                let path = workspace::absolute(&path);
                seen.insert(path.clone());
                if !self.workspace_version_matches(&path, report.version) {
                    return;
                }
                let Some(text) = self.text_for_path(&path) else {
                    self.emit(LspEvent::Log {
                        server: key.server.clone(),
                        message: format!(
                            "cannot read {} for workspace diagnostics",
                            path.display()
                        ),
                    });
                    return;
                };
                let diagnostics = convert_diagnostics(
                    &report.full_document_diagnostic_report.items,
                    &text,
                    encoding,
                );
                self.record_workspace_result_id(
                    key,
                    &path,
                    report.full_document_diagnostic_report.result_id,
                );
                let identifier = self.diagnostic_identifier(key);
                self.emit(LspEvent::Diagnostics(DiagnosticsEvent {
                    server: key.server.clone(),
                    identifier,
                    path,
                    version: None,
                    diagnostics,
                }));
            }
            WorkspaceDocumentDiagnosticReport::Unchanged(report) => {
                let Some(path) = path_from_uri(&report.uri) else {
                    return;
                };
                let path = workspace::absolute(&path);
                seen.insert(path.clone());
                self.record_workspace_result_id(
                    key,
                    &path,
                    Some(report.unchanged_document_diagnostic_report.result_id),
                );
            }
        }
    }

    fn workspace_version_matches(&self, path: &Path, version: Option<i64>) -> bool {
        let Some(version) = version else {
            return true;
        };
        self.documents
            .get(path)
            .is_none_or(|document| i64::from(document.version) == version)
    }

    fn text_for_path(&self, path: &Path) -> Option<Rope> {
        if let Some(document) = self.documents.get(path) {
            return Some(document.text.clone());
        }
        std::fs::read_to_string(path)
            .ok()
            .map(|text| Rope::from_str(&text))
    }

    fn record_workspace_result_id(
        &mut self,
        key: &ClientKey,
        path: &Path,
        result_id: Option<String>,
    ) {
        let cache_key = (key.clone(), path.to_path_buf());
        match result_id {
            Some(result_id) => {
                self.workspace_diagnostics
                    .result_ids
                    .insert(cache_key, result_id);
            }
            None => {
                self.workspace_diagnostics.result_ids.remove(&cache_key);
            }
        }
    }

    fn clear_workspace_path(&mut self, key: &ClientKey, path: PathBuf) {
        self.workspace_diagnostics
            .result_ids
            .remove(&(key.clone(), path.clone()));
        let identifier = self.diagnostic_identifier(key);
        self.emit(LspEvent::Diagnostics(DiagnosticsEvent {
            server: key.server.clone(),
            identifier,
            path,
            version: None,
            diagnostics: Vec::new(),
        }));
    }

    fn finish_workspace_diagnostics(&mut self, key: &ClientKey) {
        self.emit(LspEvent::WorkspaceDiagnostics(WorkspaceDiagnosticEvent {
            server: key.server.clone(),
            state: WorkspaceDiagnosticState::Finished,
        }));
    }
}
