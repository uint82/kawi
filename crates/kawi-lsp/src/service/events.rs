use super::*;
use crate::client::ClientEvent;
use crate::protocol::Message;

impl Manager {
    pub(super) fn handle_client_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Message {
                key,
                generation,
                message,
            } => {
                if self.is_current_client(&key, generation) {
                    self.handle_message(key, message);
                }
            }
            ClientEvent::ReaderClosed {
                key,
                generation,
                error,
            } => {
                if self.is_current_client(&key, generation) {
                    self.client_failed(
                        key,
                        error.unwrap_or_else(|| "server closed stdout".to_string()),
                    );
                }
            }
            ClientEvent::WriterClosed {
                key,
                generation,
                error,
            } => {
                if self.is_current_client(&key, generation) {
                    self.client_failed(key, error);
                }
            }
            ClientEvent::Stderr {
                key,
                generation,
                line,
            } => {
                if self.is_current_client(&key, generation) {
                    self.emit(LspEvent::Log {
                        server: key.server,
                        message: line,
                    });
                }
            }
        }
    }

    fn is_current_client(&self, key: &ClientKey, generation: u64) -> bool {
        self.clients
            .get(key)
            .is_some_and(|client| client.generation == generation)
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
        let Some(mut client) = self.clients.remove(&key) else {
            return;
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
        self.cleanup_client(&key, status, (!message.is_empty()).then_some(message), true);
    }

    pub(super) fn cleanup_client(
        &mut self,
        key: &ClientKey,
        status: ServerState,
        message: Option<String>,
        detach_documents: bool,
    ) {
        let request_message = message.as_deref().unwrap_or("");
        self.fail_requests_for_client(key, request_message);
        self.pull_diagnostics.fail_client(key);
        self.workspace_diagnostics.fail_client(key);
        self.pending_commands.retain(|_, client| client != key);
        if detach_documents {
            for document in self.documents.values_mut() {
                document.clients.retain(|candidate| candidate != key);
            }
        }
        self.emit_status(&key.server, status, message);
    }
}
