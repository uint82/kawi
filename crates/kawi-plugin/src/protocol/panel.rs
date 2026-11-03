use serde::{Deserialize, Serialize};

use super::{DecorationStyle, RpcError, INVALID_PARAMS};

pub const MAX_PLUGIN_PANEL_LINES: usize = 1_000;
pub const MAX_PANEL_LINE_BYTES: usize = 4_096;
pub const MAX_PLUGIN_PANEL_BYTES: usize = 1024 * 1024;
pub const MAX_PANEL_TITLE_BYTES: usize = 256;
pub const MAX_PANEL_STATUS_BYTES: usize = 256;
pub const MIN_PLUGIN_PANEL_HEIGHT: u32 = 4;
pub const MAX_PLUGIN_PANEL_HEIGHT: u32 = 18;
pub const DEFAULT_PLUGIN_PANEL_HEIGHT: u32 = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelLine {
    pub text: String,
    #[serde(default)]
    pub style: Option<DecorationStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelOpenParams {
    pub title: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub focus: bool,
    pub lines: Vec<PanelLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelOpenResult {
    pub panel: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelUpdateParams {
    pub panel: u64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub lines: Option<Vec<PanelLine>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelParams {
    pub panel: u64,
}

pub fn validate_panel_open(params: &PanelOpenParams) -> Result<(), RpcError> {
    validate_panel_title(&params.title)?;
    if let Some(status) = params.status.as_deref() {
        validate_panel_status(status)?;
    }
    validate_panel_lines(&params.lines)
}

pub fn validate_panel_update(params: &PanelUpdateParams) -> Result<(), RpcError> {
    if let Some(title) = params.title.as_deref() {
        validate_panel_title(title)?;
    }
    if let Some(status) = params.status.as_deref() {
        validate_panel_status(status)?;
    }
    if let Some(lines) = params.lines.as_deref() {
        validate_panel_lines(lines)?;
    }
    Ok(())
}

fn validate_panel_title(title: &str) -> Result<(), RpcError> {
    validate_text("panel title", title, MAX_PANEL_TITLE_BYTES)
}

fn validate_panel_status(status: &str) -> Result<(), RpcError> {
    validate_text("panel status", status, MAX_PANEL_STATUS_BYTES)
}

fn validate_panel_lines(lines: &[PanelLine]) -> Result<(), RpcError> {
    if lines.len() > MAX_PLUGIN_PANEL_LINES {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("at most {MAX_PLUGIN_PANEL_LINES} panel lines are accepted"),
        ));
    }
    let mut total = 0usize;
    for line in lines {
        if line.text.len() > MAX_PANEL_LINE_BYTES {
            return Err(RpcError::new(
                INVALID_PARAMS,
                format!("panel line exceeds {MAX_PANEL_LINE_BYTES} bytes"),
            ));
        }
        total = total.saturating_add(line.text.len());
    }
    if total > MAX_PLUGIN_PANEL_BYTES {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("panel content exceeds {MAX_PLUGIN_PANEL_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn validate_text(label: &str, text: &str, limit: usize) -> Result<(), RpcError> {
    if text.len() > limit {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("{label} exceeds {limit} bytes"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line(text: &str) -> PanelLine {
        PanelLine {
            text: text.to_string(),
            style: None,
        }
    }

    #[test]
    fn panel_open_defaults_are_stable() {
        let params: PanelOpenParams = serde_json::from_value(json!({
            "title": "Index",
            "lines": [],
        }))
        .unwrap();
        assert_eq!(params.status, None);
        assert_eq!(params.height, None);
        assert!(!params.focus);
    }

    #[test]
    fn panel_update_defaults_to_keeping_content() {
        let params: PanelUpdateParams =
            serde_json::from_value(json!({"panel": 1, "status": "done"})).unwrap();
        assert_eq!(params.lines, None);
        assert!(validate_panel_update(&params).is_ok());
    }

    #[test]
    fn rejects_oversized_title_and_status() {
        let open = PanelOpenParams {
            title: "x".repeat(MAX_PANEL_TITLE_BYTES + 1),
            status: None,
            height: None,
            focus: false,
            lines: Vec::new(),
        };
        assert!(validate_panel_open(&open).is_err());
        let update = PanelUpdateParams {
            panel: 1,
            title: None,
            status: Some("x".repeat(MAX_PANEL_STATUS_BYTES + 1)),
            height: None,
            lines: None,
        };
        assert!(validate_panel_update(&update).is_err());
    }

    #[test]
    fn panel_lines_carry_optional_style() {
        let value = serde_json::to_value(PanelLine {
            text: "done".to_string(),
            style: Some(DecorationStyle::Success),
        })
        .unwrap();
        assert_eq!(value, json!({"text": "done", "style": "success"}));
    }

    #[test]
    fn rejects_too_many_lines() {
        let lines = vec![line(""); MAX_PLUGIN_PANEL_LINES + 1];
        assert!(validate_panel_lines(&lines).is_err());
    }

    #[test]
    fn rejects_oversized_line_and_total() {
        assert!(validate_panel_lines(&[line(&"x".repeat(MAX_PANEL_LINE_BYTES + 1))]).is_err());
        let lines = vec![
            line(&"x".repeat(MAX_PANEL_LINE_BYTES));
            MAX_PLUGIN_PANEL_BYTES / MAX_PANEL_LINE_BYTES + 1
        ];
        assert!(validate_panel_lines(&lines).is_err());
    }

    #[test]
    fn accepts_boundary_sizes() {
        let lines = vec![
            line(&"x".repeat(MAX_PANEL_LINE_BYTES));
            MAX_PLUGIN_PANEL_BYTES / MAX_PANEL_LINE_BYTES
        ];
        assert!(validate_panel_lines(&lines).is_ok());
    }
}
