use super::pending::ReplyPurpose;
use super::to_params;
use super::*;
use crate::protocol::{Message, RequestId, ResponseError};
use crate::workspace::file_uri;
use crate::ServerConfig;
use lsp_types::{
    ClientCapabilities, FailureHandlingKind, GeneralClientCapabilities, InitializeParams,
    InitializeResult, PositionEncodingKind, PrepareSupportDefaultBehavior,
    PublishDiagnosticsClientCapabilities, RenameClientCapabilities, RenameOptions,
    TextDocumentClientCapabilities, TextDocumentSyncCapability, TextDocumentSyncSaveOptions,
    WindowClientCapabilities, WorkspaceClientCapabilities, WorkspaceEditClientCapabilities,
    WorkspaceFileOperationsClientCapabilities, WorkspaceFolder,
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
            if self.pending_commands.remove(&id).is_some() {
                self.handle_command_response(key, result);
                return;
            }
            if self.pull_diagnostics.is_pending(&id) {
                self.handle_pull_diagnostic_response(key, id, result);
                return;
            }
            if self.workspace_diagnostics.is_pending(&id) {
                self.handle_workspace_diagnostic_response(key, id, result);
                return;
            }
            match self.pending_replies.get(&id).map(|reply| reply.purpose) {
                Some(ReplyPurpose::Navigation) => {
                    self.handle_navigation_response(key, id, result);
                }
                Some(ReplyPurpose::Rename) => self.handle_rename_response(key, id, result),
                Some(ReplyPurpose::FileOperation) => {
                    let request = self
                        .pending_replies
                        .remove(&id)
                        .map_or(0, |reply| reply.request);
                    self.handle_file_operation_response(key, request, result);
                }
                Some(ReplyPurpose::Text) => self.handle_text_response(key, id, result),
                Some(ReplyPurpose::CompletionResolve) => {
                    self.handle_completion_resolve_response(key, id, result);
                }
                Some(ReplyPurpose::Symbols) => {
                    self.handle_symbols_response(key, id, result);
                }
                Some(ReplyPurpose::SymbolResolve) => {
                    self.handle_symbol_resolve_response(key, id, result);
                }
                Some(ReplyPurpose::Colors) => self.handle_colors_response(key, id, result),
                None => {}
            }
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
        let capabilities_event = completion_capabilities_event(&key, &result.capabilities);
        let symbol_capabilities_event = symbol_capabilities_event(&key, &result.capabilities);
        let color_capabilities_event = color_capabilities_event(&key, &result.capabilities);
        let capabilities = result.capabilities;
        let support = request_support(&capabilities);
        let sync = sync_config(capabilities.text_document_sync);
        let encoding = capabilities
            .position_encoding
            .clone()
            .unwrap_or(PositionEncodingKind::UTF16);
        let diagnostic_identifier =
            diagnostic_identifier(capabilities.diagnostic_provider.as_ref());
        let Some(client) = self.clients.get_mut(&key) else {
            return;
        };
        client.sync = sync;
        client.encoding = encoding;
        client.diagnostic_identifier = diagnostic_identifier;
        client.support = support;
        client.status = ServerState::Ready;
        let file_operations = FileOperationCapabilitiesEvent {
            server: key.server.clone(),
            rename: support.will_rename_files,
            delete: support.will_delete_files,
        };
        let initialized = Message::notification("initialized", Some(serde_json::json!({})));
        client.client.send(initialized);
        self.emit_status(&key.server, ServerState::Ready, None);
        self.emit(LspEvent::CompletionCapabilities(capabilities_event));
        self.emit(LspEvent::SymbolCapabilities(symbol_capabilities_event));
        self.emit(LspEvent::DocumentColorCapabilities(
            color_capabilities_event,
        ));
        self.emit(LspEvent::FileOperationCapabilities(file_operations));
        self.emit_diagnostic_mode(&key);
        self.open_documents_for(&key);
        self.schedule_all_diagnostics();
    }

    fn emit_diagnostic_mode(&self, key: &ClientKey) {
        let Some(client) = self.clients.get(key) else {
            return;
        };
        let mode = if client.support.workspace_diagnostics {
            DiagnosticMode::Workspace
        } else if client.support.pull_diagnostics {
            DiagnosticMode::Pull
        } else {
            DiagnosticMode::Push
        };
        self.emit(LspEvent::DiagnosticMode(DiagnosticModeEvent {
            server: key.server.clone(),
            mode,
        }));
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
        self.next_client_generation = self.next_client_generation.wrapping_add(1);
        let generation = self.next_client_generation;
        let client = Client::spawn(key.clone(), generation, config, self.input_tx.clone());
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
                generation,
                status: ServerState::Initializing,
                sync: SyncConfig::default(),
                encoding: PositionEncodingKind::UTF16,
                diagnostic_identifier: None,
                initialization_options,
                settings,
                pending_initialize: None,
                initialize_deadline: None,
                opened: HashSet::new(),
                support: RequestSupport::default(),
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

fn completion_capabilities_event(
    key: &ClientKey,
    capabilities: &lsp_types::ServerCapabilities,
) -> CompletionCapabilitiesEvent {
    let completion = capabilities.completion_provider.as_ref();
    let signature = capabilities.signature_help_provider.as_ref();
    CompletionCapabilitiesEvent {
        server: key.server.clone(),
        completion: completion.is_some(),
        completion_triggers: completion
            .and_then(|provider| provider.trigger_characters.clone())
            .unwrap_or_default(),
        completion_resolve: completion
            .and_then(|provider| provider.resolve_provider)
            .unwrap_or(false),
        signature_help: signature.is_some(),
        signature_triggers: signature
            .and_then(|provider| provider.trigger_characters.clone())
            .unwrap_or_default(),
    }
}

fn color_capabilities_event(
    key: &ClientKey,
    capabilities: &lsp_types::ServerCapabilities,
) -> DocumentColorCapabilitiesEvent {
    DocumentColorCapabilitiesEvent {
        server: key.server.clone(),
        color_provider: color_provider_enabled(capabilities.color_provider.as_ref()),
    }
}

fn symbol_capabilities_event(
    key: &ClientKey,
    capabilities: &lsp_types::ServerCapabilities,
) -> SymbolCapabilitiesEvent {
    SymbolCapabilitiesEvent {
        server: key.server.clone(),
        document: provider_enabled(capabilities.document_symbol_provider.as_ref()),
        workspace: capabilities.workspace_symbol_provider.is_some(),
        resolve: workspace_symbol_resolve(capabilities.workspace_symbol_provider.as_ref()),
    }
}

fn request_support(capabilities: &lsp_types::ServerCapabilities) -> RequestSupport {
    RequestSupport {
        definition: provider_enabled(capabilities.definition_provider.as_ref()),
        references: provider_enabled(capabilities.references_provider.as_ref()),
        hover: capabilities
            .hover_provider
            .as_ref()
            .is_some_and(|provider| match provider {
                lsp_types::HoverProviderCapability::Simple(enabled) => *enabled,
                lsp_types::HoverProviderCapability::Options(_) => true,
            }),
        rename: provider_enabled(capabilities.rename_provider.as_ref()),
        prepare_rename: prepare_enabled(capabilities.rename_provider.as_ref()),
        completion: capabilities.completion_provider.is_some(),
        completion_resolve: capabilities
            .completion_provider
            .as_ref()
            .and_then(|provider| provider.resolve_provider)
            .unwrap_or(false),
        signature_help: capabilities.signature_help_provider.is_some(),
        code_actions: code_action_enabled(capabilities.code_action_provider.as_ref()),
        formatting: provider_enabled(capabilities.document_formatting_provider.as_ref()),
        range_formatting: provider_enabled(
            capabilities.document_range_formatting_provider.as_ref(),
        ),
        pull_diagnostics: capabilities.diagnostic_provider.is_some(),
        workspace_diagnostics: workspace_diagnostics(capabilities.diagnostic_provider.as_ref()),
        will_rename_files: file_operation_enabled(capabilities.workspace.as_ref(), |ops| {
            ops.will_rename.as_ref()
        }),
        did_rename_files: file_operation_enabled(capabilities.workspace.as_ref(), |ops| {
            ops.did_rename.as_ref()
        }),
        will_delete_files: file_operation_enabled(capabilities.workspace.as_ref(), |ops| {
            ops.will_delete.as_ref()
        }),
        did_delete_files: file_operation_enabled(capabilities.workspace.as_ref(), |ops| {
            ops.did_delete.as_ref()
        }),
        document_symbols: provider_enabled(capabilities.document_symbol_provider.as_ref()),
        workspace_symbols: capabilities.workspace_symbol_provider.is_some(),
        workspace_symbol_resolve: workspace_symbol_resolve(
            capabilities.workspace_symbol_provider.as_ref(),
        ),
        color_provider: color_provider_enabled(capabilities.color_provider.as_ref()),
    }
}

fn color_provider_enabled(provider: Option<&lsp_types::ColorProviderCapability>) -> bool {
    provider.is_some_and(|provider| match provider {
        lsp_types::ColorProviderCapability::Simple(enabled) => *enabled,
        _ => true,
    })
}

fn workspace_symbol_resolve(
    provider: Option<&lsp_types::OneOf<bool, lsp_types::WorkspaceSymbolOptions>>,
) -> bool {
    provider.is_some_and(|provider| match provider {
        lsp_types::OneOf::Left(_) => false,
        lsp_types::OneOf::Right(options) => options.resolve_provider.unwrap_or(false),
    })
}

fn file_operation_enabled(
    workspace: Option<&lsp_types::WorkspaceServerCapabilities>,
    operation: impl Fn(
        &lsp_types::WorkspaceFileOperationsServerCapabilities,
    ) -> Option<&lsp_types::FileOperationRegistrationOptions>,
) -> bool {
    workspace
        .and_then(|workspace| workspace.file_operations.as_ref())
        .and_then(operation)
        .is_some()
}

fn diagnostic_identifier(
    provider: Option<&lsp_types::DiagnosticServerCapabilities>,
) -> Option<String> {
    match provider {
        Some(lsp_types::DiagnosticServerCapabilities::Options(options)) => {
            options.identifier.clone()
        }
        Some(lsp_types::DiagnosticServerCapabilities::RegistrationOptions(options)) => {
            options.diagnostic_options.identifier.clone()
        }
        None => None,
    }
}

fn workspace_diagnostics(provider: Option<&lsp_types::DiagnosticServerCapabilities>) -> bool {
    match provider {
        Some(lsp_types::DiagnosticServerCapabilities::Options(options)) => {
            options.workspace_diagnostics
        }
        Some(lsp_types::DiagnosticServerCapabilities::RegistrationOptions(options)) => {
            options.diagnostic_options.workspace_diagnostics
        }
        None => false,
    }
}

fn code_action_enabled(provider: Option<&lsp_types::CodeActionProviderCapability>) -> bool {
    provider.is_some_and(|provider| match provider {
        lsp_types::CodeActionProviderCapability::Simple(enabled) => *enabled,
        lsp_types::CodeActionProviderCapability::Options(_) => true,
    })
}

fn prepare_enabled(provider: Option<&lsp_types::OneOf<bool, RenameOptions>>) -> bool {
    provider.is_some_and(|provider| match provider {
        lsp_types::OneOf::Left(_) => false,
        lsp_types::OneOf::Right(options) => options.prepare_provider.unwrap_or(false),
    })
}

fn provider_enabled<T>(provider: Option<&lsp_types::OneOf<bool, T>>) -> bool {
    provider.is_some_and(|provider| match provider {
        lsp_types::OneOf::Left(enabled) => *enabled,
        lsp_types::OneOf::Right(_) => true,
    })
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

fn symbol_kinds() -> Vec<lsp_types::SymbolKind> {
    vec![
        lsp_types::SymbolKind::FILE,
        lsp_types::SymbolKind::MODULE,
        lsp_types::SymbolKind::NAMESPACE,
        lsp_types::SymbolKind::PACKAGE,
        lsp_types::SymbolKind::CLASS,
        lsp_types::SymbolKind::METHOD,
        lsp_types::SymbolKind::PROPERTY,
        lsp_types::SymbolKind::FIELD,
        lsp_types::SymbolKind::CONSTRUCTOR,
        lsp_types::SymbolKind::ENUM,
        lsp_types::SymbolKind::INTERFACE,
        lsp_types::SymbolKind::FUNCTION,
        lsp_types::SymbolKind::VARIABLE,
        lsp_types::SymbolKind::CONSTANT,
        lsp_types::SymbolKind::STRING,
        lsp_types::SymbolKind::NUMBER,
        lsp_types::SymbolKind::BOOLEAN,
        lsp_types::SymbolKind::ARRAY,
        lsp_types::SymbolKind::OBJECT,
        lsp_types::SymbolKind::KEY,
        lsp_types::SymbolKind::NULL,
        lsp_types::SymbolKind::ENUM_MEMBER,
        lsp_types::SymbolKind::STRUCT,
        lsp_types::SymbolKind::EVENT,
        lsp_types::SymbolKind::OPERATOR,
        lsp_types::SymbolKind::TYPE_PARAMETER,
    ]
}

fn initialize_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            workspace_folders: Some(true),
            configuration: Some(true),
            workspace_edit: Some(WorkspaceEditClientCapabilities {
                document_changes: Some(true),
                failure_handling: Some(FailureHandlingKind::Abort),
                ..WorkspaceEditClientCapabilities::default()
            }),
            file_operations: Some(WorkspaceFileOperationsClientCapabilities {
                dynamic_registration: Some(false),
                will_rename: Some(true),
                did_rename: Some(true),
                will_delete: Some(true),
                did_delete: Some(true),
                ..WorkspaceFileOperationsClientCapabilities::default()
            }),
            diagnostic: Some(lsp_types::DiagnosticWorkspaceClientCapabilities {
                refresh_support: Some(true),
            }),
            symbol: Some(lsp_types::WorkspaceSymbolClientCapabilities {
                dynamic_registration: Some(false),
                symbol_kind: Some(lsp_types::SymbolKindCapability {
                    value_set: Some(symbol_kinds()),
                }),
                tag_support: Some(lsp_types::TagSupport {
                    value_set: vec![lsp_types::SymbolTag::DEPRECATED],
                }),
                resolve_support: Some(lsp_types::WorkspaceSymbolResolveSupportCapability {
                    properties: vec!["location.range".to_string()],
                }),
            }),
            ..WorkspaceClientCapabilities::default()
        }),
        window: Some(WindowClientCapabilities {
            work_done_progress: Some(true),
            ..WindowClientCapabilities::default()
        }),
        text_document: Some(TextDocumentClientCapabilities {
            color_provider: Some(lsp_types::DocumentColorClientCapabilities {
                dynamic_registration: Some(false),
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                related_information: Some(true),
                tag_support: Some(lsp_types::TagSupport {
                    value_set: vec![
                        lsp_types::DiagnosticTag::UNNECESSARY,
                        lsp_types::DiagnosticTag::DEPRECATED,
                    ],
                }),
                version_support: Some(true),
                code_description_support: Some(true),
                data_support: Some(false),
            }),
            definition: Some(lsp_types::GotoCapability {
                dynamic_registration: Some(false),
                link_support: Some(true),
            }),
            references: Some(lsp_types::DynamicRegistrationClientCapabilities {
                dynamic_registration: Some(false),
            }),
            hover: Some(lsp_types::HoverClientCapabilities {
                dynamic_registration: Some(false),
                content_format: Some(vec![
                    lsp_types::MarkupKind::Markdown,
                    lsp_types::MarkupKind::PlainText,
                ]),
            }),
            signature_help: Some(lsp_types::SignatureHelpClientCapabilities {
                dynamic_registration: Some(false),
                signature_information: Some(lsp_types::SignatureInformationSettings {
                    documentation_format: Some(vec![
                        lsp_types::MarkupKind::Markdown,
                        lsp_types::MarkupKind::PlainText,
                    ]),
                    parameter_information: Some(lsp_types::ParameterInformationSettings {
                        label_offset_support: Some(true),
                    }),
                    active_parameter_support: Some(true),
                }),
                context_support: Some(true),
            }),
            rename: Some(RenameClientCapabilities {
                dynamic_registration: Some(false),
                prepare_support: Some(true),
                prepare_support_default_behavior: Some(PrepareSupportDefaultBehavior::IDENTIFIER),
                honors_change_annotations: Some(false),
            }),
            completion: Some(lsp_types::CompletionClientCapabilities {
                dynamic_registration: Some(false),
                completion_item: Some(lsp_types::CompletionItemCapability {
                    snippet_support: Some(true),
                    commit_characters_support: Some(false),
                    documentation_format: Some(vec![
                        lsp_types::MarkupKind::Markdown,
                        lsp_types::MarkupKind::PlainText,
                    ]),
                    deprecated_support: Some(true),
                    preselect_support: Some(true),
                    insert_replace_support: Some(true),
                    resolve_support: Some(lsp_types::CompletionItemCapabilityResolveSupport {
                        properties: vec![
                            "documentation".to_string(),
                            "detail".to_string(),
                            "additionalTextEdits".to_string(),
                        ],
                    }),
                    ..lsp_types::CompletionItemCapability::default()
                }),
                context_support: Some(true),
                ..lsp_types::CompletionClientCapabilities::default()
            }),
            code_action: Some(lsp_types::CodeActionClientCapabilities {
                dynamic_registration: Some(false),
                code_action_literal_support: Some(lsp_types::CodeActionLiteralSupport {
                    code_action_kind: lsp_types::CodeActionKindLiteralSupport {
                        value_set: vec![
                            "quickfix".to_string(),
                            "refactor".to_string(),
                            "refactor.extract".to_string(),
                            "refactor.inline".to_string(),
                            "refactor.rewrite".to_string(),
                            "source".to_string(),
                            "source.organizeImports".to_string(),
                            "source.fixAll".to_string(),
                        ],
                    },
                }),
                is_preferred_support: Some(true),
                data_support: Some(false),
                honors_change_annotations: Some(false),
                ..lsp_types::CodeActionClientCapabilities::default()
            }),
            formatting: Some(lsp_types::DynamicRegistrationClientCapabilities {
                dynamic_registration: Some(false),
            }),
            range_formatting: Some(lsp_types::DynamicRegistrationClientCapabilities {
                dynamic_registration: Some(false),
            }),
            diagnostic: Some(lsp_types::DiagnosticClientCapabilities {
                dynamic_registration: Some(false),
                related_document_support: Some(false),
            }),
            document_symbol: Some(lsp_types::DocumentSymbolClientCapabilities {
                dynamic_registration: Some(false),
                symbol_kind: Some(lsp_types::SymbolKindCapability {
                    value_set: Some(symbol_kinds()),
                }),
                hierarchical_document_symbol_support: Some(true),
                tag_support: Some(lsp_types::TagSupport {
                    value_set: vec![lsp_types::SymbolTag::DEPRECATED],
                }),
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
        assert_eq!(diagnostics.related_information, Some(true));
        assert_eq!(diagnostics.code_description_support, Some(true));
        assert_eq!(diagnostics.data_support, Some(false));
        assert_eq!(
            diagnostics.tag_support.map(|support| support.value_set),
            Some(vec![
                lsp_types::DiagnosticTag::UNNECESSARY,
                lsp_types::DiagnosticTag::DEPRECATED,
            ])
        );
    }

    #[test]
    fn advertises_navigation_capabilities() {
        let capabilities = initialize_capabilities();
        let text_document = capabilities.text_document.expect("text document");
        assert!(text_document.definition.is_some());
        assert!(text_document.references.is_some());
        assert_eq!(
            text_document.hover.and_then(|hover| hover.content_format),
            Some(vec![
                lsp_types::MarkupKind::Markdown,
                lsp_types::MarkupKind::PlainText
            ])
        );
    }

    #[test]
    fn advertises_rename_capabilities() {
        let capabilities = initialize_capabilities();
        let rename = capabilities
            .text_document
            .and_then(|text_document| text_document.rename)
            .expect("rename capability");
        assert_eq!(rename.prepare_support, Some(true));
        assert_eq!(
            rename.prepare_support_default_behavior,
            Some(PrepareSupportDefaultBehavior::IDENTIFIER)
        );
        let workspace = capabilities
            .workspace
            .and_then(|workspace| workspace.workspace_edit)
            .expect("workspace edit capability");
        assert_eq!(workspace.document_changes, Some(true));
        assert_eq!(workspace.failure_handling, Some(FailureHandlingKind::Abort));
        assert_eq!(workspace.resource_operations, None);
    }

    #[test]
    fn reads_rename_support_from_server_capabilities() {
        let capabilities = lsp_types::ServerCapabilities {
            rename_provider: Some(lsp_types::OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
            ..lsp_types::ServerCapabilities::default()
        };
        let support = request_support(&capabilities);
        assert!(support.rename);
        assert!(support.prepare_rename);
        let simple = lsp_types::ServerCapabilities {
            rename_provider: Some(lsp_types::OneOf::Left(true)),
            ..lsp_types::ServerCapabilities::default()
        };
        let support = request_support(&simple);
        assert!(support.rename);
        assert!(!support.prepare_rename);
    }

    #[test]
    fn reads_navigation_support_from_server_capabilities() {
        let capabilities = lsp_types::ServerCapabilities {
            definition_provider: Some(lsp_types::OneOf::Left(true)),
            references_provider: Some(lsp_types::OneOf::Right(lsp_types::ReferencesOptions {
                work_done_progress_options: Default::default(),
            })),
            hover_provider: Some(lsp_types::HoverProviderCapability::Simple(false)),
            ..lsp_types::ServerCapabilities::default()
        };
        let support = request_support(&capabilities);
        assert!(support.definition);
        assert!(support.references);
        assert!(!support.hover);
    }

    #[test]
    fn reads_file_operation_support_from_server_capabilities() {
        let registration = lsp_types::FileOperationRegistrationOptions {
            filters: Vec::new(),
        };
        let capabilities = lsp_types::ServerCapabilities {
            workspace: Some(lsp_types::WorkspaceServerCapabilities {
                file_operations: Some(lsp_types::WorkspaceFileOperationsServerCapabilities {
                    will_rename: Some(registration.clone()),
                    did_delete: Some(registration),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let support = request_support(&capabilities);
        assert!(support.will_rename_files);
        assert!(!support.did_rename_files);
        assert!(!support.will_delete_files);
        assert!(support.did_delete_files);
    }

    #[test]
    fn advertises_phase_seven_capabilities() {
        let capabilities = initialize_capabilities();
        let text_document = capabilities.text_document.expect("text document");
        assert!(text_document.completion.is_some());
        assert!(text_document.code_action.is_some());
        assert!(text_document.formatting.is_some());
        assert!(text_document.range_formatting.is_some());
        assert_eq!(
            text_document
                .diagnostic
                .and_then(|diagnostic| diagnostic.related_document_support),
            Some(false)
        );
        let workspace = capabilities.workspace.expect("workspace");
        assert_eq!(
            workspace
                .diagnostic
                .and_then(|diagnostic| diagnostic.refresh_support),
            Some(true)
        );
        let operations = workspace.file_operations.expect("file operations");
        assert_eq!(operations.will_rename, Some(true));
        assert_eq!(operations.did_rename, Some(true));
        assert_eq!(operations.will_delete, Some(true));
        assert_eq!(operations.did_delete, Some(true));
    }

    #[test]
    fn advertises_completion_and_signature_capabilities() {
        let capabilities = initialize_capabilities();
        let text_document = capabilities.text_document.expect("text document");
        let completion = text_document.completion.expect("completion");
        let item = completion.completion_item.expect("completion item");
        assert_eq!(item.snippet_support, Some(true));
        assert_eq!(
            item.documentation_format,
            Some(vec![
                lsp_types::MarkupKind::Markdown,
                lsp_types::MarkupKind::PlainText,
            ])
        );
        assert_eq!(
            item.resolve_support.map(|support| support.properties),
            Some(vec![
                "documentation".to_string(),
                "detail".to_string(),
                "additionalTextEdits".to_string(),
            ])
        );
        let signature = text_document.signature_help.expect("signature help");
        assert_eq!(
            signature
                .signature_information
                .and_then(|information| information.documentation_format),
            Some(vec![
                lsp_types::MarkupKind::Markdown,
                lsp_types::MarkupKind::PlainText,
            ])
        );
    }

    #[test]
    fn reads_completion_and_signature_support_from_server_capabilities() {
        let capabilities = lsp_types::ServerCapabilities {
            completion_provider: Some(lsp_types::CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![".".to_string()]),
                ..lsp_types::CompletionOptions::default()
            }),
            signature_help_provider: Some(lsp_types::SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string()]),
                ..lsp_types::SignatureHelpOptions::default()
            }),
            ..lsp_types::ServerCapabilities::default()
        };
        let key = ClientKey {
            server: "fake".to_string(),
            root: std::path::PathBuf::from("/tmp"),
        };
        let event = completion_capabilities_event(&key, &capabilities);
        assert!(event.completion);
        assert!(event.completion_resolve);
        assert_eq!(event.completion_triggers, ["."]);
        assert!(event.signature_help);
        assert_eq!(event.signature_triggers, ["("]);
    }

    #[test]
    fn reads_phase_seven_support_from_server_capabilities() {
        let capabilities = lsp_types::ServerCapabilities {
            completion_provider: Some(lsp_types::CompletionOptions::default()),
            code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
            document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
            document_range_formatting_provider: Some(lsp_types::OneOf::Right(
                lsp_types::DocumentRangeFormattingOptions {
                    work_done_progress_options: Default::default(),
                },
            )),
            diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
                lsp_types::DiagnosticOptions::default(),
            )),
            ..lsp_types::ServerCapabilities::default()
        };
        let support = request_support(&capabilities);
        assert!(support.completion);
        assert!(support.code_actions);
        assert!(support.formatting);
        assert!(support.range_formatting);
        assert!(support.pull_diagnostics);
    }

    #[test]
    fn disabled_code_actions_are_not_supported() {
        let capabilities = lsp_types::ServerCapabilities {
            code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(false)),
            ..lsp_types::ServerCapabilities::default()
        };
        assert!(!request_support(&capabilities).code_actions);
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
