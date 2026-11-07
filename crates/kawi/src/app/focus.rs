use kawi_tui::frame::Rect;

use super::window::{direction_score, score_precedes, FocusDirection, WindowId};
use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    Editor(WindowId),
    Explorer,
    Terminal,
    Debug,
    PluginPanel,
}

impl App {
    pub(crate) fn focused_target(&self) -> FocusTarget {
        if self.terminal_focus && self.terminal.is_open() {
            return FocusTarget::Terminal;
        }
        if self.plugin_panel.is_focused() && self.plugin_panel_rect().is_some() {
            return FocusTarget::PluginPanel;
        }
        if self.debug_focus && self.debug_panel_rect().is_some() {
            return FocusTarget::Debug;
        }
        if self.explorer.is_focused() && self.explorer_rect().is_some() {
            return FocusTarget::Explorer;
        }
        FocusTarget::Editor(self.windows.active())
    }

    pub(crate) fn is_editor_focused(&self) -> bool {
        matches!(self.focused_target(), FocusTarget::Editor(_))
    }

    pub(crate) fn clear_focus(&mut self) {
        self.terminal_focus = false;
        self.explorer.focus_editor();
        self.debug_focus = false;
        self.plugin_panel.unfocus();
    }

    pub(crate) fn focus_target(&mut self, target: FocusTarget) -> bool {
        if !self.can_focus_target(target) {
            return false;
        }
        match target {
            FocusTarget::Editor(id) => self.focus_editor_window(id),
            FocusTarget::Explorer => {
                self.focus_explorer();
                self.explorer.is_focused()
            }
            FocusTarget::Terminal => self.focus_terminal_target(),
            FocusTarget::Debug => self.focus_debug_panel(),
            FocusTarget::PluginPanel => self.focus_plugin_panel(),
        }
    }

    pub(crate) fn focus_editor(&mut self) -> bool {
        self.focus_target(FocusTarget::Editor(self.windows.active()))
    }

    pub(crate) fn focus_direction(&mut self, direction: FocusDirection) -> bool {
        let current_target = self.focused_target();
        let candidates = self.focus_candidates();
        let Some(current) = candidates
            .iter()
            .find_map(|(target, rect)| (*target == current_target).then_some(*rect))
        else {
            return false;
        };
        let mut best: Option<(FocusTarget, (usize, usize, usize))> = None;
        for (target, rect) in candidates {
            if target == current_target {
                continue;
            }
            let Some(score) = direction_score(direction, current, rect) else {
                continue;
            };
            let replace = best.is_none_or(|(_, best_score)| score_precedes(score, best_score));
            if replace {
                best = Some((target, score));
            }
        }
        let Some((target, _)) = best else {
            return false;
        };
        self.focus_target(target)
    }

    fn focus_editor_window(&mut self, id: WindowId) -> bool {
        self.cancel_pending_input();
        self.clear_focus();
        self.focus_window(id);
        self.windows.active() == id
    }

    fn focus_terminal_target(&mut self) -> bool {
        self.focus_terminal();
        self.terminal_focus
    }

    fn can_focus_target(&self, target: FocusTarget) -> bool {
        match target {
            FocusTarget::Editor(id) => self.windows.view(id).is_some(),
            FocusTarget::Explorer => self.explorer.is_enabled() && self.explorer_rect().is_some(),
            FocusTarget::Terminal => {
                self.terminal.is_open() && self.terminal_panel_rect().is_some()
            }
            FocusTarget::Debug => self.debug_panel_rect().is_some(),
            FocusTarget::PluginPanel => {
                self.plugin_panel.active().is_some() && self.plugin_panel_rect().is_some()
            }
        }
    }

    fn focus_candidates(&self) -> Vec<(FocusTarget, Rect)> {
        let mut candidates: Vec<(FocusTarget, Rect)> = self
            .window_layout()
            .panes
            .into_iter()
            .map(|(id, rect)| (FocusTarget::Editor(id), rect))
            .collect();
        if let Some(rect) = self.explorer_rect() {
            candidates.push((FocusTarget::Explorer, rect));
        }
        if self.terminal.is_open() {
            if let Some(rect) = self.terminal_panel_rect() {
                candidates.push((FocusTarget::Terminal, rect));
            }
        }
        if let Some(rect) = self.debug_panel_rect() {
            candidates.push((FocusTarget::Debug, rect));
        }
        if self.plugin_panel.active().is_some() {
            if let Some(rect) = self.plugin_panel_rect() {
                candidates.push((FocusTarget::PluginPanel, rect));
            }
        }
        candidates
    }
}
