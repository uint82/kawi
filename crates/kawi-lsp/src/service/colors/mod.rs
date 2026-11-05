use super::*;

mod convert;
pub(super) mod presentation;
mod reply;
mod request;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentColor {
    pub start: CursorPosition,
    pub end: CursorPosition,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentColorsEvent {
    pub request: u64,
    pub path: PathBuf,
    pub colors: Vec<DocumentColor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentColorCapabilitiesEvent {
    pub server: String,
    pub color_provider: bool,
}

pub use presentation::{ColorPresentationEntry, ColorPresentationEvent};

pub(super) struct PendingColors {
    pub(super) request: u64,
    pub(super) path: PathBuf,
    pub(super) version: i32,
    pub(super) text: Rope,
    pub(super) remaining: usize,
    pub(super) responses: Vec<ColorResponse>,
    pub(super) error: Option<String>,
}

pub(super) struct ColorResponse {
    pub(super) value: Value,
    pub(super) encoding: PositionEncodingKind,
}
