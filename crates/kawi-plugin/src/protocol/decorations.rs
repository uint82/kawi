use super::{
    Decoration, DecorationScope, RpcError, MAX_DECORATION_TEXT_BYTES, MAX_PLUGIN_DECORATIONS,
};

pub fn validate_decoration_scope(
    decorations: &[Decoration],
    scope: DecorationScope,
) -> Result<(), RpcError> {
    if scope == DecorationScope::All {
        return Ok(());
    }
    for decoration in decorations {
        let matches = matches!(
            (scope, decoration),
            (
                DecorationScope::Visual,
                Decoration::Highlight { .. } | Decoration::VirtualText { .. },
            ) | (DecorationScope::Signs, Decoration::Sign { .. })
        );
        if !matches {
            return Err(RpcError::invalid_params(
                "decoration kind does not match scope",
            ));
        }
    }
    Ok(())
}

pub fn validate_decorations(decorations: &[Decoration]) -> Result<(), RpcError> {
    if decorations.len() > MAX_PLUGIN_DECORATIONS {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_PLUGIN_DECORATIONS} decorations are accepted"
        )));
    }
    for decoration in decorations {
        let Decoration::VirtualText { text, .. } = decoration else {
            continue;
        };
        if text.len() > MAX_DECORATION_TEXT_BYTES {
            return Err(RpcError::invalid_params(format!(
                "decoration text exceeds {MAX_DECORATION_TEXT_BYTES} bytes"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{DecorationScope, DecorationStyle, Position, SetDecorationsParams};

    fn virtual_text(text: String) -> Decoration {
        Decoration::VirtualText {
            line: 0,
            text,
            style: DecorationStyle::Muted,
        }
    }

    fn highlight() -> Decoration {
        Decoration::Highlight {
            start: Position {
                line: 0,
                byte_column: 0,
            },
            end: Position {
                line: 0,
                byte_column: 1,
            },
            style: DecorationStyle::Info,
        }
    }

    fn sign() -> Decoration {
        Decoration::Sign {
            line: 0,
            text: "!".to_string(),
            style: DecorationStyle::Error,
        }
    }

    #[test]
    fn scope_defaults_to_all() {
        let params: SetDecorationsParams = serde_json::from_value(serde_json::json!({
            "buffer": 1,
            "revision": 2,
            "decorations": []
        }))
        .expect("params");
        assert_eq!(params.scope, DecorationScope::All);
    }

    #[test]
    fn rejects_decorations_outside_scope() {
        assert!(validate_decoration_scope(&[highlight()], DecorationScope::Visual).is_ok());
        assert!(validate_decoration_scope(&[sign()], DecorationScope::Signs).is_ok());
        assert!(validate_decoration_scope(&[sign()], DecorationScope::Visual).is_err());
        assert!(validate_decoration_scope(&[highlight()], DecorationScope::Signs).is_err());
    }

    #[test]
    fn rejects_too_many_decorations() {
        let decorations = vec![highlight(); MAX_PLUGIN_DECORATIONS + 1];
        let error = validate_decorations(&decorations).expect_err("decorations");
        assert_eq!(error.code, crate::protocol::INVALID_PARAMS);
    }

    #[test]
    fn rejects_oversized_text() {
        let decoration = virtual_text("x".repeat(MAX_DECORATION_TEXT_BYTES + 1));
        assert!(validate_decorations(&[decoration]).is_err());
    }

    #[test]
    fn accepts_boundary_sizes() {
        let decoration = virtual_text("x".repeat(MAX_DECORATION_TEXT_BYTES));
        assert!(validate_decorations(&[decoration]).is_ok());
    }
}
