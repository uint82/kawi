use super::*;
use crate::protocol::{Message, RequestId, ResponseError};
use crate::workspace::file_uri;
use lsp_types::{
    ConfigurationParams, LogMessageParams, MessageType, NumberOrString, ProgressParams,
    ProgressParamsValue, ShowMessageParams, ShowMessageRequestParams, WorkspaceFolder,
};
use serde_json::Value;

impl Manager {
    pub(super) fn handle_request(
        &mut self,
        key: ClientKey,
        id: RequestId,
        method: String,
        params: Option<Value>,
    ) {
        let response = match method.as_str() {
            "window/workDoneProgress/create" => {
                parse_params::<lsp_types::WorkDoneProgressCreateParams>(params)
                    .map(|_| Message::response(id.clone(), Value::Null))
                    .unwrap_or_else(|error| invalid_params(id.clone(), error))
            }
            "workspace/configuration" => match parse_params::<ConfigurationParams>(params) {
                Ok(params) => Message::response(
                    id.clone(),
                    serde_json::to_value(self.configuration(&key, &params)).unwrap_or(Value::Null),
                ),
                Err(error) => invalid_params(id.clone(), error),
            },
            "workspace/workspaceFolders" => Message::response(
                id.clone(),
                serde_json::to_value(self.workspace_folders(&key)).unwrap_or(Value::Null),
            ),
            "client/registerCapability" | "client/unregisterCapability" => {
                Message::response(id.clone(), Value::Null)
            }
            "window/showMessageRequest" => {
                if let Ok(params) = parse_params::<ShowMessageRequestParams>(params) {
                    self.emit_message(&key, params);
                }
                Message::response(id.clone(), Value::Null)
            }
            _ => Message::error(id, ResponseError::method_not_found(&method)),
        };
        if let Some(client) = self.clients.get(&key) {
            client.client.send(response);
        }
    }

    fn configuration(&self, key: &ClientKey, params: &ConfigurationParams) -> Vec<Value> {
        let settings = self
            .clients
            .get(key)
            .map_or(Value::Null, |client| client.settings.clone());
        params
            .items
            .iter()
            .map(|item| configuration_item(&settings, item.section.as_deref()))
            .collect()
    }

    fn workspace_folders(&self, key: &ClientKey) -> Option<Vec<WorkspaceFolder>> {
        let uri = file_uri(&key.root).ok()?;
        let name = key.root.file_name().map_or_else(
            || key.root.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        Some(vec![WorkspaceFolder { uri, name }])
    }

    pub(super) fn handle_notification(
        &mut self,
        key: ClientKey,
        method: String,
        params: Option<Value>,
    ) {
        match method.as_str() {
            "window/showMessage" => {
                if let Ok(params) = parse_params::<ShowMessageParams>(params) {
                    self.emit_message(&key, params);
                }
            }
            "window/logMessage" => {
                if let Ok(params) = parse_params::<LogMessageParams>(params) {
                    if !params.message.is_empty() {
                        self.emit(LspEvent::Log {
                            server: key.server,
                            message: params.message,
                        });
                    }
                }
            }
            "$/progress" => {
                if let Ok(params) = parse_params::<ProgressParams>(params) {
                    self.emit_progress(&key, params);
                }
            }
            _ => {}
        }
    }

    fn emit_progress(&mut self, key: &ClientKey, params: ProgressParams) {
        let token = token_string(&params.token);
        match params.value {
            ProgressParamsValue::WorkDone(progress) => match progress {
                lsp_types::WorkDoneProgress::Begin(begin) => {
                    self.emit(LspEvent::Progress(ProgressEvent {
                        server: key.server.clone(),
                        token,
                        title: Some(begin.title),
                        message: begin.message,
                        percentage: begin.percentage,
                        done: false,
                    }))
                }
                lsp_types::WorkDoneProgress::Report(report) => {
                    self.emit(LspEvent::Progress(ProgressEvent {
                        server: key.server.clone(),
                        token,
                        title: None,
                        message: report.message,
                        percentage: report.percentage,
                        done: false,
                    }))
                }
                lsp_types::WorkDoneProgress::End(end) => {
                    self.emit(LspEvent::Progress(ProgressEvent {
                        server: key.server.clone(),
                        token,
                        title: None,
                        message: end.message,
                        percentage: None,
                        done: true,
                    }))
                }
            },
        }
    }
}

pub(super) struct UiMessage {
    pub(super) level: MessageLevel,
    pub(super) message: String,
}

impl From<ShowMessageParams> for UiMessage {
    fn from(params: ShowMessageParams) -> Self {
        Self {
            level: message_level(params.typ),
            message: params.message,
        }
    }
}

impl From<ShowMessageRequestParams> for UiMessage {
    fn from(params: ShowMessageRequestParams) -> Self {
        Self {
            level: message_level(params.typ),
            message: params.message,
        }
    }
}

fn message_level(typ: MessageType) -> MessageLevel {
    if typ == MessageType::ERROR {
        MessageLevel::Error
    } else if typ == MessageType::WARNING {
        MessageLevel::Warning
    } else if typ == MessageType::INFO {
        MessageLevel::Info
    } else {
        MessageLevel::Log
    }
}

fn configuration_item(settings: &Value, section: Option<&str>) -> Value {
    let Some(section) = section else {
        return settings.clone();
    };
    let mut value = settings;
    for part in section.split('.') {
        let Some(next) = value.get(part) else {
            return Value::Null;
        };
        value = next;
    }
    value.clone()
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Option<Value>) -> Result<T> {
    let value = params.unwrap_or(Value::Null);
    serde_json::from_value(value).context("invalid request params")
}

fn invalid_params(id: RequestId, error: anyhow::Error) -> Message {
    Message::error(
        id,
        ResponseError::new(-32602, format!("invalid params: {error}")),
    )
}

fn token_string(token: &lsp_types::ProgressToken) -> String {
    match token {
        NumberOrString::Number(number) => number.to_string(),
        NumberOrString::String(text) => text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_sections_are_resolved() {
        let settings = serde_json::json!({"python": {"analysis": {"typeCheckingMode": "strict"}}});
        assert_eq!(
            configuration_item(&settings, Some("python.analysis.typeCheckingMode")),
            serde_json::json!("strict")
        );
        assert_eq!(configuration_item(&settings, Some("missing")), Value::Null);
    }
}
