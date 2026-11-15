use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionsSetParams {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionsResetParams {
    pub name: String,
}

pub fn validate_option_value(value: &Value) -> Result<(), String> {
    match value {
        Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(number) if number.as_i64().is_some() => Ok(()),
        _ => Err("option values must be boolean, integer, or text".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn accepts_scalar_option_values() {
        for value in [json!(true), json!(1), json!("text")] {
            assert!(validate_option_value(&value).is_ok(), "{value}");
        }
    }

    #[test]
    fn rejects_non_scalar_option_values() {
        for value in [json!(null), json!(1.5), json!([true]), json!({ "a": 1 })] {
            assert!(validate_option_value(&value).is_err(), "{value}");
        }
    }
}
