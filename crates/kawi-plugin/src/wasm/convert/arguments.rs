use serde_json::Value;

use crate::error::PluginError;
use crate::protocol::RpcError;

use super::super::bindings::kawi::plugin::types::Argument;

pub fn argument_from_json(value: Value) -> Result<Argument, RpcError> {
    match value {
        Value::Null => Ok(Argument::Null),
        Value::Bool(value) => Ok(Argument::Boolean(value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                return Ok(Argument::Integer(value));
            }
            let value = value
                .as_f64()
                .ok_or_else(|| RpcError::invalid_params("wasm arguments must be finite numbers"))?;
            if !value.is_finite() {
                return Err(RpcError::invalid_params(
                    "wasm arguments must be finite numbers",
                ));
            }
            Ok(Argument::Float(value))
        }
        Value::String(value) => Ok(Argument::Text(value)),
        _ => Err(RpcError::invalid_params(
            "wasm arguments must be null, boolean, integer, float, or text",
        )),
    }
}

pub fn argument_to_json(value: Argument) -> Result<Value, PluginError> {
    match value {
        Argument::Null => Ok(Value::Null),
        Argument::Boolean(value) => Ok(Value::Bool(value)),
        Argument::Integer(value) => Ok(Value::from(value)),
        Argument::Float(value) if value.is_finite() => Ok(Value::from(value)),
        Argument::Float(_) => Err(PluginError::Protocol(
            "wasm plugin returned a non-finite number".to_string(),
        )),
        Argument::Text(value) => Ok(Value::String(value)),
    }
}
