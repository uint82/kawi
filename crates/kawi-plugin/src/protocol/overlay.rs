use serde::{Deserialize, Serialize};

use super::{DecorationStyle, RpcError, INVALID_PARAMS};

pub const MAX_OVERLAY_LINES: usize = 1_000;
pub const MAX_OVERLAY_LINE_BYTES: usize = 4_096;
pub const MAX_OVERLAY_BYTES: usize = 1024 * 1024;
pub const MAX_OVERLAY_TITLE_BYTES: usize = 256;
pub const MAX_OVERLAY_STATUS_BYTES: usize = 256;
pub const MAX_OVERLAY_WIDTH: u32 = 160;
pub const MAX_OVERLAY_HEIGHT: u32 = 40;
pub const MIN_OVERLAY_WIDTH: u32 = 8;
pub const MIN_OVERLAY_HEIGHT: u32 = 3;
pub const DEFAULT_OVERLAY_WIDTH: u32 = 60;
pub const DEFAULT_OVERLAY_HEIGHT: u32 = 12;
pub const MAX_OVERLAY_EVENTS: usize = 256;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayAnchor {
    #[default]
    Cursor,
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlaySpan {
    pub text: String,
    #[serde(default)]
    pub style: Option<DecorationStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayLine {
    pub spans: Vec<OverlaySpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayOpenParams {
    pub title: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub anchor: OverlayAnchor,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub focus: bool,
    #[serde(default)]
    pub selected: Option<u32>,
    #[serde(default)]
    pub scroll: Option<u32>,
    pub lines: Vec<OverlayLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayOpenResult {
    pub overlay: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayUpdateParams {
    pub overlay: u64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub anchor: Option<OverlayAnchor>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub focus: Option<bool>,
    #[serde(default)]
    pub selected: Option<u32>,
    #[serde(default)]
    pub scroll: Option<u32>,
    #[serde(default)]
    pub lines: Option<Vec<OverlayLine>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayParams {
    pub overlay: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayCloseReason {
    Plugin,
    Escape,
    Command,
    FocusLost,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KeyModifier {
    Shift,
    Control,
    Alt,
    Super,
    Hyper,
    Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum Key {
    Character(char),
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Insert,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Function(u8),
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayKeyEvent {
    pub key: Key,
    pub modifiers: Vec<KeyModifier>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum OverlayEvent {
    Key(OverlayKeyEvent),
    Paste(String),
    Closed(OverlayCloseReason),
}

pub fn validate_overlay_open(params: &OverlayOpenParams) -> Result<(), RpcError> {
    validate_title(&params.title)?;
    if let Some(status) = params.status.as_deref() {
        validate_status(status)?;
    }
    validate_dimensions(params.width, params.height)?;
    validate_lines(&params.lines)
}

pub fn validate_overlay_update(params: &OverlayUpdateParams) -> Result<(), RpcError> {
    if let Some(title) = params.title.as_deref() {
        validate_title(title)?;
    }
    if let Some(status) = params.status.as_deref() {
        validate_status(status)?;
    }
    validate_dimensions(params.width, params.height)?;
    if let Some(lines) = params.lines.as_deref() {
        validate_lines(lines)?;
    }
    Ok(())
}

fn validate_title(title: &str) -> Result<(), RpcError> {
    validate_text("overlay title", title, MAX_OVERLAY_TITLE_BYTES)
}

fn validate_status(status: &str) -> Result<(), RpcError> {
    validate_text("overlay status", status, MAX_OVERLAY_STATUS_BYTES)
}

fn validate_dimensions(width: Option<u32>, height: Option<u32>) -> Result<(), RpcError> {
    if let Some(width) = width {
        if !(MIN_OVERLAY_WIDTH..=MAX_OVERLAY_WIDTH).contains(&width) {
            return Err(RpcError::new(
                INVALID_PARAMS,
                format!(
                    "overlay width must be between {MIN_OVERLAY_WIDTH} and {MAX_OVERLAY_WIDTH}"
                ),
            ));
        }
    }
    if let Some(height) = height {
        if !(MIN_OVERLAY_HEIGHT..=MAX_OVERLAY_HEIGHT).contains(&height) {
            return Err(RpcError::new(
                INVALID_PARAMS,
                format!(
                    "overlay height must be between {MIN_OVERLAY_HEIGHT} and {MAX_OVERLAY_HEIGHT}"
                ),
            ));
        }
    }
    Ok(())
}

fn validate_lines(lines: &[OverlayLine]) -> Result<(), RpcError> {
    if lines.len() > MAX_OVERLAY_LINES {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("at most {MAX_OVERLAY_LINES} overlay lines are accepted"),
        ));
    }
    let mut total = 0usize;
    for line in lines {
        let line_bytes = line
            .spans
            .iter()
            .map(|span| span.text.len())
            .fold(0usize, usize::saturating_add);
        if line_bytes > MAX_OVERLAY_LINE_BYTES {
            return Err(RpcError::new(
                INVALID_PARAMS,
                format!("overlay line exceeds {MAX_OVERLAY_LINE_BYTES} bytes"),
            ));
        }
        total = total.saturating_add(line_bytes);
    }
    validate_total(total)
}

fn validate_total(total: usize) -> Result<(), RpcError> {
    if total > MAX_OVERLAY_BYTES {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("overlay content exceeds {MAX_OVERLAY_BYTES} bytes"),
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
mod tests;
