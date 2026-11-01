use super::{
    Decoration, DecorationRef, DecorationScope, RpcError, MAX_DECORATION_TEXT_BYTES,
    MAX_PLUGIN_DECORATIONS,
};
use crate::validate_decoration_group;

pub fn decoration_ref(decoration: &Decoration) -> Result<DecorationRef, RpcError> {
    let (group, style) = match decoration {
        Decoration::Highlight { group, style, .. }
        | Decoration::VirtualText { group, style, .. }
        | Decoration::Sign { group, style, .. } => (group.as_deref(), *style),
    };
    match (group, style) {
        (Some(group), None) => {
            validate_decoration_group(group).map_err(RpcError::invalid_params)?;
            Ok(DecorationRef::Group(group.to_string()))
        }
        (None, Some(style)) => Ok(DecorationRef::Style(style)),
        _ => Err(RpcError::invalid_params(
            "decoration must set exactly one of group or style",
        )),
    }
}

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
        decoration_ref(decoration)?;
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
            group: None,
            style: Some(DecorationStyle::Muted),
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
            group: None,
            style: Some(DecorationStyle::Info),
        }
    }

    fn grouped_highlight(group: &str) -> Decoration {
        Decoration::Highlight {
            start: Position {
                line: 0,
                byte_column: 0,
            },
            end: Position {
                line: 0,
                byte_column: 1,
            },
            group: Some(group.to_string()),
            style: None,
        }
    }

    fn sign() -> Decoration {
        Decoration::Sign {
            line: 0,
            text: "!".to_string(),
            group: None,
            style: Some(DecorationStyle::Error),
        }
    }

    #[test]
    fn accepts_named_groups_and_rejects_ambiguous_decorations() {
        assert!(validate_decorations(&[grouped_highlight("todo")]).is_ok());
        assert!(validate_decorations(&[grouped_highlight("info")]).is_err());
        assert!(validate_decorations(&[grouped_highlight("Todo")]).is_err());
        let ambiguous = Decoration::Highlight {
            start: Position {
                line: 0,
                byte_column: 0,
            },
            end: Position {
                line: 0,
                byte_column: 1,
            },
            group: Some("todo".to_string()),
            style: Some(DecorationStyle::Info),
        };
        assert!(validate_decorations(&[ambiguous]).is_err());
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
