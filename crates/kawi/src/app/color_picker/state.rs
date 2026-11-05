use std::path::PathBuf;

use kawi_lsp::{ColorPresentationEntry, ColorPresentationEvent, CursorPosition, DocumentColor};

use super::super::color_preview::Rgba;
use super::hsla::Hsla;

const HUE_MAX: f32 = 360.0;
const PERCENT_MAX: f32 = 100.0;
const FINE_STEP: f32 = 1.0;
const COARSE_STEP: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
    Hue,
    Saturation,
    Lightness,
    Alpha,
}

impl Field {
    pub(super) const ALL: [Self; 4] = [Self::Hue, Self::Saturation, Self::Lightness, Self::Alpha];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Lightness => "Lightness",
            Self::Alpha => "Alpha",
        }
    }

    pub(super) fn value(self, color: Hsla) -> f32 {
        match self {
            Self::Hue => color.hue,
            Self::Saturation => color.saturation,
            Self::Lightness => color.lightness,
            Self::Alpha => color.alpha,
        }
    }

    pub(super) fn maximum(self) -> f32 {
        match self {
            Self::Hue => HUE_MAX,
            Self::Saturation | Self::Lightness | Self::Alpha => PERCENT_MAX,
        }
    }

    pub(super) fn set(self, color: &mut Hsla, value: f32) {
        let value = match self {
            Self::Hue => value.rem_euclid(HUE_MAX),
            Self::Saturation | Self::Lightness | Self::Alpha => value.clamp(0.0, PERCENT_MAX),
        };
        match self {
            Self::Hue => color.hue = value,
            Self::Saturation => color.saturation = value,
            Self::Lightness => color.lightness = value,
            Self::Alpha => color.alpha = value,
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Adjust,
    Loading,
    Present,
}

#[derive(Debug, Clone)]
pub(crate) struct ColorPicker {
    path: PathBuf,
    revision: u64,
    range: (CursorPosition, CursorPosition),
    hsla: Hsla,
    rgba: Rgba,
    field: Field,
    mode: Mode,
    presentations: Vec<ColorPresentationEntry>,
    selected: usize,
    request: Option<u64>,
    message: Option<String>,
}

impl ColorPicker {
    pub(super) fn new(document: &DocumentColor, path: PathBuf, revision: u64) -> Self {
        let color = Rgba::new(document.red, document.green, document.blue, document.alpha);
        Self {
            path,
            revision,
            range: (document.start, document.end),
            hsla: Hsla::from_rgba(color),
            rgba: color,
            field: Field::Hue,
            mode: Mode::Adjust,
            presentations: Vec::new(),
            selected: 0,
            request: None,
            message: None,
        }
    }

    pub(super) fn color(&self) -> Rgba {
        self.rgba
    }

    pub(super) fn path(&self) -> &PathBuf {
        &self.path
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) fn range(&self) -> (CursorPosition, CursorPosition) {
        self.range
    }

    pub(super) fn field(&self) -> Field {
        self.field
    }

    pub(super) fn value(&self, field: Field) -> f32 {
        field.value(self.hsla)
    }

    pub(super) fn mode(&self) -> Mode {
        self.mode
    }

    pub(super) fn presentations(&self) -> &[ColorPresentationEntry] {
        &self.presentations
    }

    pub(super) fn selected(&self) -> usize {
        self.selected
    }

    pub(super) fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub(super) fn select_field(&mut self, forward: bool) {
        let index = self.field.index();
        let next = if forward {
            (index + 1) % Field::ALL.len()
        } else {
            (index + Field::ALL.len() - 1) % Field::ALL.len()
        };
        self.field = Field::from_index(next);
    }

    pub(super) fn adjust(&mut self, coarse: bool) {
        self.adjust_by(step(coarse));
    }

    pub(super) fn decrease(&mut self, coarse: bool) {
        self.adjust_by(-step(coarse));
    }

    fn adjust_by(&mut self, delta: f32) {
        let value = self.field.value(self.hsla) + delta;
        self.field.set(&mut self.hsla, value);
        self.rgba = self.hsla.to_rgba();
    }

    pub(super) fn begin_request(&mut self, request: u64) {
        self.request = Some(request);
        self.mode = Mode::Loading;
        self.message = None;
    }

    pub(super) fn cancel_request(&mut self) -> Option<u64> {
        let request = self.request.take();
        self.mode = Mode::Adjust;
        request
    }

    pub(super) fn apply_event(&mut self, event: ColorPresentationEvent) -> bool {
        if self.request != Some(event.request) || self.path != event.path {
            return false;
        }
        self.request = None;
        if event.presentations.is_empty() {
            self.mode = Mode::Adjust;
            self.message = Some("No color formats available".to_string());
            return true;
        }
        self.presentations = event.presentations;
        self.selected = 0;
        self.mode = Mode::Present;
        self.message = None;
        true
    }

    pub(super) fn fail_request(&mut self, request: u64, message: &str) -> bool {
        if self.request != Some(request) {
            return false;
        }
        self.request = None;
        self.mode = Mode::Adjust;
        self.message = Some(message.to_string());
        true
    }

    pub(super) fn set_message(&mut self, message: String) {
        self.message = Some(message);
    }

    pub(super) fn clear_message(&mut self) {
        self.message = None;
    }

    pub(super) fn select_presentation(&mut self, forward: bool) {
        if self.presentations.is_empty() {
            return;
        }
        self.selected = if forward {
            (self.selected + 1).min(self.presentations.len() - 1)
        } else {
            self.selected.saturating_sub(1)
        };
    }

    pub(super) fn back_to_adjust(&mut self) {
        self.mode = Mode::Adjust;
        self.message = None;
    }

    pub(super) fn selected_presentation(&self) -> Option<ColorPresentationEntry> {
        self.presentations.get(self.selected).cloned()
    }
}

fn step(coarse: bool) -> f32 {
    if coarse {
        COARSE_STEP
    } else {
        FINE_STEP
    }
}

pub(super) fn contains_position(color: &DocumentColor, position: CursorPosition) -> bool {
    let start = (color.start.line, color.start.column);
    let end = (color.end.line, color.end.column);
    let current = (position.line, position.column);
    start <= current && current < end
}

#[cfg(test)]
mod tests;
