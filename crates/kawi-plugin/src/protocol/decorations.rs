use super::{Decoration, RpcError, MAX_DECORATION_TEXT_BYTES, MAX_PLUGIN_DECORATIONS};

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
    use crate::protocol::{DecorationStyle, Position};

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
