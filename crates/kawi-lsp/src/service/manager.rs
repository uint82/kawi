use super::request::UiMessage;
use super::*;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

impl Manager {
    pub(super) fn new(
        languages: Languages,
        input: Receiver<ServiceInput>,
        input_tx: Sender<ServiceInput>,
        events: Sender<LspEvent>,
        wake: Sender<()>,
        initialize_timeout: Duration,
    ) -> Self {
        Self {
            languages,
            input,
            input_tx,
            events,
            wake,
            clients: HashMap::new(),
            documents: HashMap::new(),
            pending_requests: HashMap::new(),
            pending_replies: HashMap::new(),
            pending_renames: HashMap::new(),
            pending_file_operations: HashMap::new(),
            pending_text_requests: HashMap::new(),
            pending_completion_resolves: HashMap::new(),
            completion_resolve: HashMap::new(),
            pending_symbols: HashMap::new(),
            pending_symbol_resolves: HashMap::new(),
            symbol_resolve_items: HashMap::new(),
            next_symbol_item: 0,
            pending_commands: HashMap::new(),
            pull_diagnostics: pull_diagnostics::PullDiagnostics::default(),
            workspace_diagnostics: workspace_diagnostics::WorkspaceDiagnostics::default(),
            next_request: 0,
            next_completion_item: 0,
            next_client_generation: 0,
            initialize_timeout,
        }
    }

    pub(super) fn run(mut self) {
        loop {
            match self.next_input() {
                InputWait::Input(ServiceInput::Command(Command::Shutdown)) => {
                    self.shutdown_all();
                    return;
                }
                InputWait::Input(ServiceInput::Command(command)) => {
                    self.handle_command(command);
                    self.expire_deadlines();
                }
                InputWait::Input(ServiceInput::Client(event)) => {
                    self.handle_client_event(event);
                    self.expire_deadlines();
                }
                InputWait::Timeout => self.expire_deadlines(),
                InputWait::Closed => {
                    self.shutdown_all();
                    return;
                }
            }
        }
    }

    fn next_input(&self) -> InputWait {
        let Some(timeout) = self.next_deadline() else {
            return self
                .input
                .recv()
                .map_or(InputWait::Closed, InputWait::Input);
        };
        match self.input.recv_timeout(timeout) {
            Ok(input) => InputWait::Input(input),
            Err(RecvTimeoutError::Timeout) => InputWait::Timeout,
            Err(RecvTimeoutError::Disconnected) => InputWait::Closed,
        }
    }

    fn next_deadline(&self) -> Option<Duration> {
        [
            self.next_initialize_deadline(),
            self.pull_diagnostics.next_deadline(),
            self.next_file_operation_deadline(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    fn next_initialize_deadline(&self) -> Option<Duration> {
        self.clients
            .values()
            .filter_map(|client| client.initialize_deadline)
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .min()
    }

    fn expire_deadlines(&mut self) {
        self.expire_initializations();
        self.expire_pull_diagnostics();
        self.expire_file_operations();
    }

    fn expire_initializations(&mut self) {
        let expired: Vec<ClientKey> = self
            .clients
            .iter()
            .filter(|(_, client)| {
                client
                    .initialize_deadline
                    .is_some_and(|deadline| deadline <= Instant::now())
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            self.client_failed(key, "initialization timed out".to_string());
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Open { path, text } => self.open_document(path, text),
            Command::Change { path, text } => self.change_document(&path, text),
            Command::Save { path } => self.save_document(&path),
            Command::Close { path } => self.close_document(&path),
            Command::Definition {
                request,
                path,
                position,
            } => self.start_definition(request, path, position),
            Command::References {
                request,
                path,
                position,
            } => self.start_references(request, path, position),
            Command::Hover {
                request,
                path,
                position,
            } => self.start_hover(request, path, position),
            Command::PrepareRename {
                request,
                path,
                position,
            } => self.start_prepare_rename(request, path, position),
            Command::Rename { request, new_name } => self.start_rename(request, new_name),
            Command::WillRenameFiles { request, files } => {
                self.start_will_rename_files(request, files);
            }
            Command::DidRenameFiles { files } => self.did_rename_files(&files),
            Command::WillDeleteFiles { request, files } => {
                self.start_will_delete_files(request, files);
            }
            Command::DidDeleteFiles { files } => self.did_delete_files(&files),
            Command::Completion {
                request,
                path,
                position,
                trigger,
            } => self.start_completion(request, path, position, trigger),
            Command::ResolveCompletion { request, item_id } => {
                self.start_completion_resolve(request, item_id);
            }
            Command::SignatureHelp {
                request,
                path,
                position,
                trigger,
            } => self.start_signature_help(request, path, position, trigger),
            Command::CodeActions {
                request,
                path,
                range,
                diagnostics,
            } => self.start_code_actions(request, path, range, diagnostics),
            Command::Formatting {
                request,
                path,
                range,
                settings,
            } => self.start_formatting(request, path, range, settings),
            Command::DocumentSymbols {
                request,
                path,
                version,
            } => self.start_document_symbols(request, path, version),
            Command::WorkspaceSymbols { request, query } => {
                self.start_workspace_symbols(request, query);
            }
            Command::ResolveWorkspaceSymbol { request, symbol_id } => {
                self.start_workspace_symbol_resolve(request, symbol_id);
            }
            Command::Execute {
                path,
                server,
                command,
                arguments,
            } => self.execute_command(path, server, command, arguments),
            Command::WorkspaceDiagnostics => self.request_workspace_diagnostics(),
            Command::WatchedFiles { files } => self.did_change_watched_files(&files),
            Command::Cancel { request } => self.cancel_request(request),
            Command::Shutdown => {}
        }
    }

    pub(super) fn emit_status(&self, server: &str, state: ServerState, message: Option<String>) {
        self.emit(LspEvent::Status(StatusEvent {
            server: server.to_string(),
            state,
            message,
        }));
    }

    pub(super) fn emit_message(&self, key: &ClientKey, message: impl Into<UiMessage>) {
        let message = message.into();
        self.emit(LspEvent::Message {
            server: key.server.clone(),
            level: message.level,
            message: message.message,
        });
    }

    pub(super) fn emit(&self, event: LspEvent) {
        if self.events.send(event).is_ok() {
            let _ = self.wake.send(());
        }
    }
}
