use kawi_tui::frame::Rect;

use super::size::PanelLimits;
use crate::options::{
    COMPLETION_DOCS_MAX_WIDTH_DEFAULT, HOVER_MAX_WIDTH_DEFAULT, OVERLAY_MARGIN_DEFAULT,
    OVERLAY_MAX_HEIGHT_PERCENT_DEFAULT, OVERLAY_MIN_WIDTH_DEFAULT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverlayOptions {
    pub hover_max_width: usize,
    pub completion_docs_max_width: usize,
    pub min_width: usize,
    pub max_height_percent: usize,
    pub margin: usize,
}

impl Default for OverlayOptions {
    fn default() -> Self {
        Self {
            hover_max_width: HOVER_MAX_WIDTH_DEFAULT,
            completion_docs_max_width: COMPLETION_DOCS_MAX_WIDTH_DEFAULT,
            min_width: OVERLAY_MIN_WIDTH_DEFAULT,
            max_height_percent: OVERLAY_MAX_HEIGHT_PERCENT_DEFAULT,
            margin: OVERLAY_MARGIN_DEFAULT,
        }
    }
}

impl OverlayOptions {
    pub(super) fn limits(
        self,
        area: Rect,
        max_width: usize,
        min_width: usize,
        available_height: usize,
    ) -> PanelLimits {
        let max_width = max_width.min(area.width.saturating_sub(self.margin)).max(1);
        let min_width = min_width.min(max_width).max(1);
        let height_cap = (area.height.saturating_mul(self.max_height_percent) / 100).max(1);
        PanelLimits {
            min_width,
            max_width,
            max_height: available_height.min(height_cap).max(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: usize, height: usize) -> Rect {
        Rect::new(0, 0, width, height)
    }

    #[test]
    fn limits_follow_the_window_and_margin() {
        let options = OverlayOptions::default();
        let limits = options.limits(area(120, 40), 64, 20, 30);
        assert_eq!(limits.max_width, 64);
        assert_eq!(limits.min_width, 20);
        assert_eq!(limits.max_height, 20);
    }

    #[test]
    fn narrow_windows_clamp_both_widths() {
        let options = OverlayOptions::default();
        let limits = options.limits(area(30, 40), 64, 20, 30);
        assert_eq!(limits.max_width, 28);
        assert_eq!(limits.min_width, 20);
    }

    #[test]
    fn margin_can_reduce_the_window_below_the_minimum() {
        let options = OverlayOptions::default();
        let limits = options.limits(area(10, 40), 64, 20, 30);
        assert_eq!(limits.max_width, 8);
        assert_eq!(limits.min_width, 8);
    }

    #[test]
    fn height_uses_the_smaller_of_half_the_window_and_available_space() {
        let options = OverlayOptions::default();
        let limits = options.limits(area(120, 40), 64, 20, 10);
        assert_eq!(limits.max_height, 10);
        let limits = options.limits(area(120, 40), 64, 20, 30);
        assert_eq!(limits.max_height, 20);
    }
}
