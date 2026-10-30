use super::{
    DiagnosticSummary, RpcError, MAX_DIAGNOSTIC_CODE_BYTES, MAX_DIAGNOSTIC_MESSAGE_BYTES,
    MAX_PLUGIN_DIAGNOSTICS,
};

pub fn validate_diagnostics(diagnostics: &[DiagnosticSummary]) -> Result<(), RpcError> {
    if diagnostics.len() > MAX_PLUGIN_DIAGNOSTICS {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_PLUGIN_DIAGNOSTICS} diagnostics are accepted"
        )));
    }
    validate_sizes(diagnostics)
}

fn validate_sizes(diagnostics: &[DiagnosticSummary]) -> Result<(), RpcError> {
    for diagnostic in diagnostics {
        if diagnostic.message.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES {
            return Err(RpcError::invalid_params(format!(
                "diagnostic message exceeds {MAX_DIAGNOSTIC_MESSAGE_BYTES} bytes"
            )));
        }
        if diagnostic
            .code
            .as_ref()
            .is_some_and(|code| code.len() > MAX_DIAGNOSTIC_CODE_BYTES)
        {
            return Err(RpcError::invalid_params(format!(
                "diagnostic code exceeds {MAX_DIAGNOSTIC_CODE_BYTES} bytes"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Position, INVALID_PARAMS};

    fn diagnostic(message: String, code: Option<String>) -> DiagnosticSummary {
        DiagnosticSummary {
            start: Position {
                line: 0,
                byte_column: 0,
            },
            end: Position {
                line: 0,
                byte_column: 0,
            },
            severity: "error".to_string(),
            message,
            source: None,
            code,
        }
    }

    #[test]
    fn rejects_too_many_diagnostics() {
        let diagnostics = vec![diagnostic(String::new(), None); MAX_PLUGIN_DIAGNOSTICS + 1];
        let error = validate_diagnostics(&diagnostics).expect_err("diagnostics");
        assert_eq!(error.code, INVALID_PARAMS);
    }

    #[test]
    fn rejects_oversized_messages() {
        let diagnostics = vec![diagnostic(
            "x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES + 1),
            None,
        )];
        assert!(validate_diagnostics(&diagnostics).is_err());
    }

    #[test]
    fn rejects_oversized_codes() {
        let diagnostics = vec![diagnostic(
            String::new(),
            Some("x".repeat(MAX_DIAGNOSTIC_CODE_BYTES + 1)),
        )];
        assert!(validate_diagnostics(&diagnostics).is_err());
    }

    #[test]
    fn accepts_boundary_sizes() {
        let diagnostics = vec![diagnostic(
            "x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES),
            Some("x".repeat(MAX_DIAGNOSTIC_CODE_BYTES)),
        )];
        assert!(validate_diagnostics(&diagnostics).is_ok());
    }
}
