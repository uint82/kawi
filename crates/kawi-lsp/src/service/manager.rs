use super::request::UiMessage;
use super::*;
use crate::client::ClientEvent;
use crate::protocol::Message;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

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
            pending_text_requests: HashMap::new(),
            pending_completion_resolves: HashMap::new(),
            completion_resolve: HashMap::new(),
            pending_commands: HashMap::new(),
            pull_diagnostics: pull_diagnostics::PullDiagnostics::default(),
            workspace_diagnostics: workspace_diagnostics::WorkspaceDiagnostics::default(),
            next_request: 0,
            next_completion_item: 0,
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
        let initialize = self.next_initialize_deadline();
        let diagnostics = self.pull_diagnostics.next_deadline();
        match (initialize, diagnostics) {
            (Some(initialize), Some(diagnostics)) => Some(initialize.min(diagnostics)),
            (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
            (None, None) => None,
        }
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
            Command::Execute {
                path,
                server,
                command,
                arguments,
            } => self.execute_command(path, server, command, arguments),
            Command::WorkspaceDiagnostics => self.request_workspace_diagnostics(),
            Command::Cancel { request } => self.cancel_request(request),
            Command::Shutdown => {}
        }
    }

    fn handle_client_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Message { key, message } => self.handle_message(key, message),
            ClientEvent::ReaderClosed { key, error } => self.client_failed(
                key,
                error.unwrap_or_else(|| "server closed stdout".to_string()),
            ),
            ClientEvent::WriterClosed { key, error } => self.client_failed(key, error),
            ClientEvent::Stderr { key, line } => self.emit(LspEvent::Log {
                server: key.server,
                message: line,
            }),
        }
    }

    fn handle_message(&mut self, key: ClientKey, message: Message) {
        match message {
            Message::Request { id, method, params } => {
                self.handle_request(key, id, method, params);
            }
            Message::Notification { method, params } => {
                self.handle_notification(key, method, params);
            }
            Message::Response { id, result } => {
                self.handle_response(key, id, result);
            }
        }
    }

    pub(super) fn client_failed(&mut self, key: ClientKey, message: String) {
        let mut client = match self.clients.remove(&key) {
            Some(client) => client,
            None => return,
        };
        let exit = client.client.try_wait().ok().flatten();
        let status = if message.is_empty() {
            client.status
        } else if exit.is_some_and(|status| status.success()) {
            ServerState::Stopped
        } else {
            ServerState::Failed
        };
        client.client.terminate();
        self.fail_requests_for_client(&key, &message);
        self.pull_diagnostics.fail_client(&key);
        self.workspace_diagnostics.fail_client(&key);
        self.pending_commands.retain(|_, client| client != &key);
        for document in self.documents.values_mut() {
            document.clients.retain(|candidate| candidate != &key);
        }
        self.emit_status(
            &key.server,
            status,
            (!message.is_empty()).then_some(message),
        );
    }

    fn shutdown_all(&mut self) {
        let clients: Vec<(ClientKey, ClientState)> = self.clients.drain().collect();
        for (key, mut client) in clients {
            self.shutdown_client(&key, &mut client);
            client.client.terminate();
            self.emit_status(&key.server, ServerState::Stopped, None);
        }
    }

    fn shutdown_client(&mut self, key: &ClientKey, client: &mut ClientState) {
        let id = self.request_id();
        if !client
            .client
            .send(Message::request(id.clone(), "shutdown", None))
        {
            return;
        }
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match self.input.recv_timeout(remaining) {
                Ok(ServiceInput::Client(ClientEvent::Message {
                    key: event_key,
                    message:
                        Message::Response {
                            id: response_id, ..
                        },
                })) if event_key == *key && response_id == id => break,
                Ok(ServiceInput::Client(ClientEvent::ReaderClosed { key: event_key, .. }))
                    if event_key == *key =>
                {
                    return
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        client.client.send(Message::notification("exit", None));
        let _ = client.client.wait_timeout(SHUTDOWN_TIMEOUT);
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
