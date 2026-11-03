use crate::types::Error;

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
pub const PLUGIN_ERROR: i32 = -32000;
pub const STALE_REVISION: i32 = -32010;
pub const INVALID_POSITION: i32 = -32011;
pub const CAPABILITY_DENIED: i32 = -32012;
pub const REQUEST_CANCELED: i32 = -32020;
pub const PLUGIN_UNAVAILABLE: i32 = -32030;
pub const UI_BUSY: i32 = -32040;
pub const PANEL_NOT_FOUND: i32 = -32060;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Parse,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    Internal,
    Plugin,
    StaleRevision,
    InvalidPosition,
    CapabilityDenied,
    Busy,
    PanelNotFound,
    RequestCanceled,
    Unavailable,
    Other,
}

pub fn error_kind(error: &Error) -> ErrorKind {
    match error.code {
        PARSE_ERROR => ErrorKind::Parse,
        INVALID_REQUEST => ErrorKind::InvalidRequest,
        METHOD_NOT_FOUND => ErrorKind::MethodNotFound,
        INVALID_PARAMS => ErrorKind::InvalidParams,
        INTERNAL_ERROR => ErrorKind::Internal,
        PLUGIN_ERROR => ErrorKind::Plugin,
        STALE_REVISION => ErrorKind::StaleRevision,
        INVALID_POSITION => ErrorKind::InvalidPosition,
        CAPABILITY_DENIED => ErrorKind::CapabilityDenied,
        UI_BUSY => ErrorKind::Busy,
        PANEL_NOT_FOUND => ErrorKind::PanelNotFound,
        REQUEST_CANCELED => ErrorKind::RequestCanceled,
        PLUGIN_UNAVAILABLE => ErrorKind::Unavailable,
        _ => ErrorKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(code: i32) -> Error {
        Error {
            code,
            message: String::new(),
            capability: None,
        }
    }

    #[test]
    fn maps_host_error_codes() {
        assert_eq!(error_kind(&error(STALE_REVISION)), ErrorKind::StaleRevision);
        assert_eq!(
            error_kind(&error(CAPABILITY_DENIED)),
            ErrorKind::CapabilityDenied
        );
        assert_eq!(error_kind(&error(UI_BUSY)), ErrorKind::Busy);
        assert_eq!(
            error_kind(&error(PANEL_NOT_FOUND)),
            ErrorKind::PanelNotFound
        );
        assert_eq!(error_kind(&error(42)), ErrorKind::Other);
    }
}
