use super::{
    RpcError, StatuslineSegment, MAX_PLUGIN_STATUSLINE_SEGMENTS, MAX_STATUSLINE_TEXT_BYTES,
};

pub fn validate_statusline(segments: &[StatuslineSegment]) -> Result<(), RpcError> {
    if segments.len() > MAX_PLUGIN_STATUSLINE_SEGMENTS {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_PLUGIN_STATUSLINE_SEGMENTS} statusline segments are accepted"
        )));
    }
    for segment in segments {
        if segment.text.len() > MAX_STATUSLINE_TEXT_BYTES {
            return Err(RpcError::invalid_params(format!(
                "statusline text exceeds {MAX_STATUSLINE_TEXT_BYTES} bytes"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::DecorationStyle;

    fn segment(text: String) -> StatuslineSegment {
        StatuslineSegment {
            text,
            style: DecorationStyle::Info,
            priority: 0,
        }
    }

    #[test]
    fn rejects_too_many_segments() {
        let segments = vec![segment(String::new()); MAX_PLUGIN_STATUSLINE_SEGMENTS + 1];
        let error = validate_statusline(&segments).expect_err("segments");
        assert_eq!(error.code, crate::protocol::INVALID_PARAMS);
    }

    #[test]
    fn rejects_oversized_text() {
        let segments = vec![segment("x".repeat(MAX_STATUSLINE_TEXT_BYTES + 1))];
        assert!(validate_statusline(&segments).is_err());
    }

    #[test]
    fn accepts_boundary_sizes() {
        let segments = vec![segment("x".repeat(MAX_STATUSLINE_TEXT_BYTES))];
        assert!(validate_statusline(&segments).is_ok());
    }
}
