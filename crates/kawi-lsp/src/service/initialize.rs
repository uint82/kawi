use super::to_params;
use super::*;
use crate::protocol::{Message, RequestId, ResponseError};
use crate::workspace::file_uri;
use crate::ServerConfig;
use lsp_types::{
    ClientCapabilities, GeneralClientCapabilities, InitializeParams, InitializeResult,
    PositionEncodingKind, PublishDiagnosticsClientCapabilities, TextDocumentClientCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncSaveOptions, WindowClientCapabilities,
    WorkspaceClientCapabilities, WorkspaceFolder,
};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

impl Manager {
    pub(super) fn handle_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some(client) = self.clients.get_mut(&key) else {
            return;
        };
        if client.pending_initialize.as_ref() != Some(&id) {
            return;
        }
        client.pending_initialize = None;
        client.initialize_deadline = None;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.client_failed(key, format!("initialize failed: {}", error.message));
                return;
            }
        };
        let result = match serde_json::from_value::<InitializeResult>(result) {
            Ok(result) => result,
            Err(error) => {
                self.client_failed(key, format!("invalid initialize result: {error}"));
                return;
            }
        };
        self.finish_initialize(key, result);
    }

    fn finish_initialize(&mut self, key: ClientKey, result: InitializeResult) {
        let Some(client) = self.clients.get_mut(&key) else {
            return;
        };
        client.sync = sync_config(result.capabilities.text_document_sync);
        client.encoding = result
            .capabilities
            .position_encoding
            .unwrap_or(PositionEncodingKind::UTF16);
        client.status = ServerState::Ready;
        let initialized = Message::notification("initialized", Some(serde_json::json!({})));
        client.client.send(initialized);
        self.emit_status(&key.server, ServerState::Ready, None);
        self.open_documents_for(&key);
    }

    fn open_documents_for(&mut self, key: &ClientKey) {
        let documents: Vec<(PathBuf, Rope, i32)> = self
            .documents
            .iter()
            .filter(|(_, document)| document.clients.contains(key))
            .map(|(path, document)| (path.clone(), document.text.clone(), document.version))
            .collect();
        for (path, text, version) in documents {
            self.send_open(key, &path, &text, version);
        }
    }

    pub(super) fn start_client(&mut self, key: ClientKey, config: &ServerConfig, root: &Path) {
        self.emit_status(&key.server, ServerState::Starting, None);
        let client = Client::spawn(key.clone(), config, self.input_tx.clone());
        let client = match client {
            Ok(client) => client,
            Err(error) => {
                self.emit_status(&key.server, ServerState::Failed, Some(error.to_string()));
                return;
            }
        };
        let settings = config
            .settings
            .as_ref()
            .and_then(|settings| serde_json::to_value(settings).ok())
            .unwrap_or(Value::Null);
        let initialization_options = config
            .initialization_options
            .as_ref()
            .and_then(|options| serde_json::to_value(options).ok());
        self.clients.insert(
            key.clone(),
            ClientState {
                client,
                status: ServerState::Initializing,
                sync: SyncConfig::default(),
                encoding: PositionEncodingKind::UTF16,
                initialization_options,
                settings,
                pending_initialize: None,
                initialize_deadline: None,
                opened: HashSet::new(),
            },
        );
        self.send_initialize(key, root);
    }

    fn send_initialize(&mut self, key: ClientKey, root: &Path) {
        let id = self.request_id();
        let initialization_options = self
            .clients
            .get(&key)
            .and_then(|client| client.initialization_options.clone());
        let params =
            match initialize_params(root, initialization_options, initialize_capabilities()) {
                Ok(params) => params,
                Err(error) => {
                    self.client_failed(key, format!("initialize params: {error}"));
                    return;
                }
            };
        let Some(params) = to_params(params) else {
            self.client_failed(key, "initialize params could not be serialized".to_string());
            return;
        };
        let Some(client) = self.clients.get_mut(&key) else {
            return;
        };
        let request = Message::request(id.clone(), "initialize", Some(params));
        if !client.client.send(request) {
            let key_for_error = key;
            self.client_failed(
                key_for_error,
                "server is not accepting messages".to_string(),
            );
            return;
        }
        client.pending_initialize = Some(id);
        client.initialize_deadline = Some(Instant::now() + self.initialize_timeout);
        client.status = ServerState::Initializing;
        self.emit_status(&key.server, ServerState::Initializing, None);
    }

    pub(super) fn request_id(&mut self) -> RequestId {
        self.next_request = self.next_request.wrapping_add(1);
        RequestId::Number(self.next_request)
    }
}

fn sync_config(capability: Option<TextDocumentSyncCapability>) -> SyncConfig {
    match capability {
        Some(TextDocumentSyncCapability::Kind(kind)) => SyncConfig {
            open_close: true,
            change: Some(kind),
            save: None,
        },
        Some(TextDocumentSyncCapability::Options(options)) => SyncConfig {
            open_close: options.open_close.unwrap_or(false),
            change: options.change,
            save: options.save,
        },
        None => SyncConfig::default(),
    }
}

pub(super) fn save_enabled(save: &TextDocumentSyncSaveOptions) -> bool {
    match save {
        TextDocumentSyncSaveOptions::Supported(supported) => *supported,
        TextDocumentSyncSaveOptions::SaveOptions(_) => true,
    }
}

pub(super) fn include_save_text(save: &TextDocumentSyncSaveOptions) -> bool {
    match save {
        TextDocumentSyncSaveOptions::Supported(_) => false,
        TextDocumentSyncSaveOptions::SaveOptions(options) => options.include_text.unwrap_or(false),
    }
}

fn initialize_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            workspace_folders: Some(true),
            configuration: Some(true),
            ..WorkspaceClientCapabilities::default()
        }),
        window: Some(WindowClientCapabilities {
            work_done_progress: Some(true),
            ..WindowClientCapabilities::default()
        }),
        text_document: Some(TextDocumentClientCapabilities {
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                version_support: Some(true),
                ..PublishDiagnosticsClientCapabilities::default()
            }),
            ..TextDocumentClientCapabilities::default()
        }),
        general: Some(GeneralClientCapabilities {
            position_encodings: Some(vec![
                PositionEncodingKind::UTF16,
                PositionEncodingKind::UTF8,
            ]),
            ..GeneralClientCapabilities::default()
        }),
        ..ClientCapabilities::default()
    }
}

#[allow(deprecated)]
fn initialize_params(
    root: &Path,
    initialization_options: Option<Value>,
    capabilities: ClientCapabilities,
) -> Result<InitializeParams> {
    let root_uri = file_uri(root)?;
    let name = root.file_name().map_or_else(
        || root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Ok(InitializeParams {
        process_id: Some(std::process::id()),
        root_uri: Some(root_uri.clone()),
        capabilities,
        initialization_options,
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: root_uri,
            name,
        }]),
        client_info: Some(lsp_types::ClientInfo {
            name: "kawi".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
        ..InitializeParams::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::TextDocumentSyncKind;

    #[test]
    fn incremental_sync_is_supported() {
        let sync = sync_config(Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )));
        assert!(sync.open_close);
        assert_eq!(sync.change, Some(TextDocumentSyncKind::INCREMENTAL));
    }

    #[test]
    fn advertises_versioned_diagnostics() {
        let capabilities = initialize_capabilities();
        let diagnostics = capabilities
            .text_document
            .and_then(|text_document| text_document.publish_diagnostics)
            .expect("diagnostics capability");
        assert_eq!(diagnostics.version_support, Some(true));
    }

    #[test]
    fn save_options_control_text_inclusion() {
        let save = TextDocumentSyncSaveOptions::SaveOptions(lsp_types::SaveOptions {
            include_text: Some(true),
        });
        assert!(save_enabled(&save));
        assert!(include_save_text(&save));
        let supported = TextDocumentSyncSaveOptions::Supported(true);
        assert!(save_enabled(&supported));
        assert!(!include_save_text(&supported));
    }
}
