use super::*;
use crate::protocol::{Message, ResponseError};
use crate::workspace;
use lsp_types::ExecuteCommandParams;
use serde_json::Value;
use std::path::PathBuf;

impl Manager {
    pub(super) fn execute_command(
        &mut self,
        path: PathBuf,
        server: String,
        command: String,
        arguments: Option<Vec<Value>>,
    ) {
        let path = workspace::absolute(&path);
        let Some(document) = self.documents.get(&path) else {
            self.emit_command_error(&server, "document is not open");
            return;
        };
        let Some(key) = document.clients.iter().find(|key| {
            key.server == server
                && self.clients.get(*key).is_some_and(|client| {
                    client.status == ServerState::Ready && client.opened.contains(&path)
                })
        }) else {
            self.emit_command_error(&server, "server is not ready");
            return;
        };
        let key = key.clone();
        let params = ExecuteCommandParams {
            command,
            arguments: arguments.unwrap_or_default(),
            work_done_progress_params: Default::default(),
        };
        let Some(params) = to_params(params) else {
            self.emit_command_error(&server, "command params could not be serialized");
            return;
        };
        let id = self.request_id();
        let sent = self.clients.get(&key).is_some_and(|client| {
            client.client.send(Message::request(
                id.clone(),
                "workspace/executeCommand",
                Some(params),
            ))
        });
        if sent {
            self.pending_commands.insert(id, key);
        } else {
            self.emit_command_error(&server, "server is not accepting messages");
        }
    }

    pub(super) fn handle_command_response(
        &mut self,
        key: ClientKey,
        result: std::result::Result<Value, ResponseError>,
    ) {
        if let Err(error) = result {
            self.emit_command_error(&key.server, &one_line_message(&error.message));
        }
    }

    fn emit_command_error(&self, server: &str, message: &str) {
        self.emit(LspEvent::Message {
            server: server.to_string(),
            level: MessageLevel::Error,
            message: format!("command failed: {message}"),
        });
    }
}
