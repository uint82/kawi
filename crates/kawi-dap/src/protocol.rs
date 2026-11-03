use anyhow::{bail, Result};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request {
        seq: i64,
        command: String,
        arguments: Option<Value>,
    },
    Response {
        seq: i64,
        request_seq: i64,
        success: bool,
        command: String,
        message: Option<String>,
        body: Option<Value>,
    },
    Event {
        seq: i64,
        event: String,
        body: Option<Value>,
    },
}

impl Message {
    pub fn request(seq: i64, command: impl Into<String>, arguments: Option<Value>) -> Self {
        Self::Request {
            seq,
            command: command.into(),
            arguments,
        }
    }

    pub fn response(
        seq: i64,
        request_seq: i64,
        success: bool,
        command: impl Into<String>,
        message: Option<String>,
        body: Option<Value>,
    ) -> Self {
        Self::Response {
            seq,
            request_seq,
            success,
            command: command.into(),
            message,
            body,
        }
    }

    pub fn event(seq: i64, event: impl Into<String>, body: Option<Value>) -> Self {
        Self::Event {
            seq,
            event: event.into(),
            body,
        }
    }

    pub fn parse(value: Value) -> Result<Self> {
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("dap message is missing type"))?;
        let seq = required_i64(&value, "seq")?;
        match kind {
            "request" => Ok(Self::Request {
                seq,
                command: required_string(&value, "command")?,
                arguments: value.get("arguments").cloned(),
            }),
            "response" => Ok(Self::Response {
                seq,
                request_seq: required_i64(&value, "request_seq")?,
                success: required_bool(&value, "success")?,
                command: required_string(&value, "command")?,
                message: value
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                body: value.get("body").cloned(),
            }),
            "event" => Ok(Self::Event {
                seq,
                event: required_string(&value, "event")?,
                body: value.get("body").cloned(),
            }),
            other => bail!("unknown dap message type {other}"),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Request {
                seq,
                command,
                arguments,
            } => {
                let mut value = json!({"seq": seq, "type": "request", "command": command});
                if let Some(arguments) = arguments {
                    value["arguments"] = arguments.clone();
                }
                value
            }
            Self::Response {
                seq,
                request_seq,
                success,
                command,
                message,
                body,
            } => {
                let mut value = json!({
                    "seq": seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": success,
                    "command": command,
                });
                if let Some(message) = message {
                    value["message"] = Value::String(message.clone());
                }
                if let Some(body) = body {
                    value["body"] = body.clone();
                }
                value
            }
            Self::Event { seq, event, body } => {
                let mut value = json!({"seq": seq, "type": "event", "event": event});
                if let Some(body) = body {
                    value["body"] = body.clone();
                }
                value
            }
        }
    }
}

fn required_i64(value: &Value, field: &str) -> Result<i64> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("dap message is missing integer {field}"))
}

fn required_bool(value: &Value, field: &str) -> Result<bool> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow::anyhow!("dap message is missing boolean {field}"))
}

fn required_string(value: &Value, field: &str) -> Result<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("dap message is missing string {field}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_requests_responses_and_events() {
        let messages = [
            Message::request(1, "initialize", Some(json!({"clientID": "kawi"}))),
            Message::response(2, 1, true, "initialize", None, Some(json!({"ok": true}))),
            Message::event(3, "initialized", None),
        ];
        for message in messages {
            assert_eq!(Message::parse(message.to_value()).unwrap(), message);
        }
    }

    #[test]
    fn rejects_unknown_and_incomplete_messages() {
        assert!(Message::parse(json!({"seq": 1, "type": "unknown"})).is_err());
        assert!(Message::parse(json!({"type": "event", "event": "x"})).is_err());
        assert!(Message::parse(json!({"seq": 1, "type": "request"})).is_err());
    }
}
