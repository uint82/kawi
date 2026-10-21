use anyhow::{bail, Result};
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    pub fn number(number: i64) -> Self {
        Self::Number(number)
    }

    fn to_value(&self) -> Value {
        match self {
            Self::Number(number) => Value::from(*number),
            Self::String(text) => Value::from(text.clone()),
        }
    }

    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Number(number) => number
                .as_i64()
                .map(Self::Number)
                .map_or_else(|| bail!("request id is not an integer"), Ok),
            Value::String(text) => Ok(Self::String(text)),
            _ => bail!("request id must be a number or string"),
        }
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => write!(formatter, "{number}"),
            Self::String(text) => write!(formatter, "{text}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponseError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl ResponseError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(-32601, format!("method not found: {method}"))
    }

    fn to_value(&self) -> Value {
        let mut error = Map::new();
        error.insert("code".to_string(), Value::from(self.code));
        error.insert("message".to_string(), Value::from(self.message.clone()));
        if let Some(data) = &self.data {
            error.insert("data".to_string(), data.clone());
        }
        Value::Object(error)
    }

    fn from_value(value: Value) -> Result<Self> {
        let Value::Object(mut object) = value else {
            bail!("response error must be an object");
        };
        let code = object
            .remove("code")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| anyhow::anyhow!("response error is missing a code"))?;
        let message = object
            .remove("message")
            .and_then(|value| value.as_str().map(str::to_string))
            .ok_or_else(|| anyhow::anyhow!("response error is missing a message"))?;
        Ok(Self {
            code,
            message,
            data: object.remove("data"),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request {
        id: RequestId,
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
    Response {
        id: RequestId,
        result: Result<Value, ResponseError>,
    },
}

impl Message {
    pub fn request(id: RequestId, method: impl Into<String>, params: Option<Value>) -> Self {
        Self::Request {
            id,
            method: method.into(),
            params,
        }
    }

    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self::Notification {
            method: method.into(),
            params,
        }
    }

    pub fn response(id: RequestId, result: Value) -> Self {
        Self::Response {
            id,
            result: Ok(result),
        }
    }

    pub fn error(id: RequestId, error: ResponseError) -> Self {
        Self::Response {
            id,
            result: Err(error),
        }
    }

    pub fn parse(value: Value) -> Result<Self> {
        let Value::Object(mut object) = value else {
            bail!("json-rpc message must be an object");
        };
        if object.remove("jsonrpc").as_ref().and_then(Value::as_str) != Some("2.0") {
            bail!("json-rpc version must be 2.0");
        }
        let id = object.remove("id").map(RequestId::from_value).transpose()?;
        let method = object
            .remove("method")
            .and_then(|value| value.as_str().map(str::to_string));
        let params = object.remove("params");
        match (id, method) {
            (Some(id), Some(method)) => Ok(Self::Request { id, method, params }),
            (None, Some(method)) => Ok(Self::Notification { method, params }),
            (Some(id), None) => parse_response(id, object),
            (None, None) => bail!("json-rpc message has no id or method"),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("jsonrpc".to_string(), Value::from("2.0"));
        match self {
            Self::Request { id, method, params } => {
                object.insert("id".to_string(), id.to_value());
                object.insert("method".to_string(), Value::from(method.clone()));
                if let Some(params) = params {
                    object.insert("params".to_string(), params.clone());
                }
            }
            Self::Notification { method, params } => {
                object.insert("method".to_string(), Value::from(method.clone()));
                if let Some(params) = params {
                    object.insert("params".to_string(), params.clone());
                }
            }
            Self::Response { id, result } => {
                object.insert("id".to_string(), id.to_value());
                match result {
                    Ok(result) => {
                        object.insert("result".to_string(), result.clone());
                    }
                    Err(error) => {
                        object.insert("error".to_string(), error.to_value());
                    }
                }
            }
        }
        Value::Object(object)
    }
}

fn parse_response(id: RequestId, mut object: Map<String, Value>) -> Result<Message> {
    let result = object.remove("result");
    let error = object.remove("error");
    match (result, error) {
        (Some(result), None) => Ok(Message::Response {
            id,
            result: Ok(result),
        }),
        (None, Some(error)) => Ok(Message::Response {
            id,
            result: Err(ResponseError::from_value(error)?),
        }),
        _ => bail!("response must contain exactly one of result or error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_request() {
        let message = Message::parse(json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "initialize",
            "params": {"rootUri": null}
        }))
        .expect("request");
        assert_eq!(
            message,
            Message::Request {
                id: RequestId::Number(7),
                method: "initialize".to_string(),
                params: Some(json!({"rootUri": null})),
            }
        );
    }

    #[test]
    fn parses_a_notification() {
        let message = Message::parse(json!({
            "jsonrpc": "2.0",
            "method": "initialized"
        }))
        .expect("notification");
        assert_eq!(
            message,
            Message::Notification {
                method: "initialized".to_string(),
                params: None,
            }
        );
    }

    #[test]
    fn parses_a_successful_response() {
        let message = Message::parse(json!({
            "jsonrpc": "2.0",
            "id": "abc",
            "result": null
        }))
        .expect("response");
        assert_eq!(
            message,
            Message::Response {
                id: RequestId::String("abc".to_string()),
                result: Ok(Value::Null),
            }
        );
    }

    #[test]
    fn parses_an_error_response() {
        let message = Message::parse(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "error": {"code": -32601, "message": "missing"}
        }))
        .expect("error response");
        let Message::Response { result, .. } = message else {
            panic!("not a response");
        };
        assert_eq!(result, Err(ResponseError::new(-32601, "missing")));
    }

    #[test]
    fn rejects_invalid_messages() {
        assert!(Message::parse(json!({"id": 1, "method": "x"})).is_err());
        assert!(Message::parse(json!({"jsonrpc": "2.0", "id": 1})).is_err());
        assert!(Message::parse(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": null,
            "error": {"code": 1, "message": "x"}
        }))
        .is_err());
    }

    #[test]
    fn serializes_each_message_kind() {
        let request = Message::request(RequestId::Number(1), "test", Some(json!({"x": 1})));
        assert_eq!(request.to_value()["method"], "test");
        assert_eq!(request.to_value()["params"]["x"], 1);

        let notification = Message::notification("ready", None);
        assert!(notification.to_value().get("id").is_none());

        let response = Message::response(RequestId::Number(2), Value::Null);
        assert_eq!(response.to_value()["result"], Value::Null);
    }
}
