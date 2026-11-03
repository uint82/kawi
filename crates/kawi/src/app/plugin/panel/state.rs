use kawi_plugin::{
    sanitize, PanelLine, PanelOpenParams, PanelParams, PanelUpdateParams, RpcError, PANEL_NOT_FOUND,
};
use kawi_tui::frame::PluginPanelLine;

use super::super::style::frame_style;

#[derive(Default)]
pub(crate) struct PluginPanels {
    next_id: u64,
    active: Option<ActivePanel>,
}

struct ActivePanel {
    id: u64,
    plugin: String,
    title: String,
    status: String,
    lines: Vec<PluginPanelLine>,
    requested_height: usize,
    offset: usize,
    focused: bool,
}

impl ActivePanel {
    fn matches(&self, plugin: &str, id: u64) -> bool {
        self.id == id && self.plugin == plugin
    }
}

impl PluginPanels {
    pub(crate) fn open(&mut self, plugin: &str, params: PanelOpenParams) -> Result<u64, RpcError> {
        if self.active.is_some() {
            return Err(RpcError::ui_busy("a plugin panel is already active"));
        }
        let id = self.allocate_id();
        self.active = Some(ActivePanel {
            id,
            plugin: plugin.to_string(),
            title: sanitize(&params.title),
            status: params.status.as_deref().map(sanitize).unwrap_or_default(),
            lines: convert_lines(params.lines),
            requested_height: clamp_height(params.height),
            offset: 0,
            focused: params.focus,
        });
        Ok(id)
    }

    pub(crate) fn update(
        &mut self,
        plugin: &str,
        params: PanelUpdateParams,
    ) -> Result<(), RpcError> {
        let panel = self.entry_mut(plugin, params.panel)?;
        if let Some(title) = params.title {
            panel.title = sanitize(&title);
        }
        if let Some(status) = params.status {
            panel.status = sanitize(&status);
        }
        if let Some(height) = params.height {
            panel.requested_height = clamp_height(Some(height));
        }
        if let Some(lines) = params.lines {
            panel.lines = convert_lines(lines);
        }
        Ok(())
    }

    pub(crate) fn focus(&mut self, plugin: &str, params: PanelParams) -> Result<(), RpcError> {
        self.entry_mut(plugin, params.panel)?.focused = true;
        Ok(())
    }

    pub(crate) fn close(&mut self, plugin: &str, params: PanelParams) -> Result<(), RpcError> {
        self.entry(plugin, params.panel)?;
        self.active = None;
        Ok(())
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) -> bool {
        if self
            .active
            .as_ref()
            .is_some_and(|panel| panel.plugin == plugin)
        {
            self.active = None;
            return true;
        }
        false
    }

    pub(crate) fn focus_active(&mut self) -> bool {
        match self.active.as_mut() {
            Some(panel) => {
                panel.focused = true;
                true
            }
            None => false,
        }
    }

    pub(crate) fn close_active(&mut self) -> bool {
        self.active.take().is_some()
    }

    pub(crate) fn unfocus(&mut self) -> bool {
        match self.active.as_mut() {
            Some(panel) if panel.focused => {
                panel.focused = false;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn is_focused(&self) -> bool {
        self.active.as_ref().is_some_and(|panel| panel.focused)
    }

    pub(crate) fn requested_height(&self) -> Option<usize> {
        self.active.as_ref().map(|panel| panel.requested_height)
    }

    pub(crate) fn clamped_offset(&self, viewport: usize) -> usize {
        let Some(panel) = self.active.as_ref() else {
            return 0;
        };
        panel.offset.min(max_offset(panel.lines.len(), viewport))
    }

    pub(crate) fn clamp_offset(&mut self, viewport: usize) {
        if let Some(panel) = self.active.as_mut() {
            panel.offset = panel.offset.min(max_offset(panel.lines.len(), viewport));
        }
    }

    pub(crate) fn scroll(&mut self, delta: isize, viewport: usize) {
        let Some(panel) = self.active.as_mut() else {
            return;
        };
        let offset = panel.offset.min(max_offset(panel.lines.len(), viewport));
        panel.offset = if delta.is_negative() {
            offset.saturating_sub(delta.unsigned_abs())
        } else {
            offset.saturating_add(delta as usize)
        }
        .min(max_offset(panel.lines.len(), viewport));
    }

    pub(crate) fn scroll_page(&mut self, pages: isize, viewport: usize) {
        let page = viewport.max(1);
        self.scroll(pages.saturating_mul(page as isize), viewport);
    }

    pub(crate) fn scroll_start(&mut self) {
        if let Some(panel) = self.active.as_mut() {
            panel.offset = 0;
        }
    }

    pub(crate) fn scroll_end(&mut self, viewport: usize) {
        if let Some(panel) = self.active.as_mut() {
            panel.offset = max_offset(panel.lines.len(), viewport);
        }
    }

    pub(crate) fn active(&self) -> Option<PanelView<'_>> {
        let panel = self.active.as_ref()?;
        Some(PanelView {
            title: &panel.title,
            status: &panel.status,
            lines: &panel.lines,
            focused: panel.focused,
        })
    }

    fn entry(&self, plugin: &str, id: u64) -> Result<&ActivePanel, RpcError> {
        self.active
            .as_ref()
            .filter(|panel| panel.matches(plugin, id))
            .ok_or_else(|| RpcError::new(PANEL_NOT_FOUND, "unknown panel"))
    }

    fn entry_mut(&mut self, plugin: &str, id: u64) -> Result<&mut ActivePanel, RpcError> {
        self.active
            .as_mut()
            .filter(|panel| panel.matches(plugin, id))
            .ok_or_else(|| RpcError::new(PANEL_NOT_FOUND, "unknown panel"))
    }

    fn allocate_id(&mut self) -> u64 {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.next_id
    }
}

pub(crate) struct PanelView<'a> {
    pub(crate) title: &'a str,
    pub(crate) status: &'a str,
    pub(crate) lines: &'a [PluginPanelLine],
    pub(crate) focused: bool,
}

fn convert_lines(lines: Vec<PanelLine>) -> Vec<PluginPanelLine> {
    lines
        .into_iter()
        .map(|line| PluginPanelLine {
            text: sanitize(&line.text),
            style: line.style.map(frame_style),
        })
        .collect()
}

fn clamp_height(height: Option<u32>) -> usize {
    height
        .unwrap_or(kawi_plugin::DEFAULT_PLUGIN_PANEL_HEIGHT)
        .clamp(
            kawi_plugin::MIN_PLUGIN_PANEL_HEIGHT,
            kawi_plugin::MAX_PLUGIN_PANEL_HEIGHT,
        ) as usize
}

fn max_offset(lines: usize, viewport: usize) -> usize {
    lines.saturating_sub(viewport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn open_params(focus: bool) -> PanelOpenParams {
        serde_json::from_value(json!({
            "title": "Index",
            "status": "running",
            "height": 8,
            "focus": focus,
            "lines": [{"text": "one"}, {"text": "two", "style": "success"}],
        }))
        .unwrap()
    }

    #[test]
    fn opens_one_panel_per_session() {
        let mut panels = PluginPanels::default();
        let id = panels.open("first", open_params(true)).unwrap();
        assert_eq!(id, 1);
        assert!(panels.is_focused());
        let error = panels.open("second", open_params(false)).unwrap_err();
        assert_eq!(error.code, kawi_plugin::UI_BUSY);
    }

    #[test]
    fn updates_only_the_owner() {
        let mut panels = PluginPanels::default();
        let id = panels.open("first", open_params(false)).unwrap();
        let params: PanelUpdateParams = serde_json::from_value(json!({
            "panel": id,
            "title": "Done",
            "lines": [{"text": "ok"}],
        }))
        .unwrap();
        panels.update("first", params.clone()).unwrap();
        let error = panels.update("second", params).unwrap_err();
        assert_eq!(error.code, kawi_plugin::PANEL_NOT_FOUND);
    }

    #[test]
    fn scrolling_stays_inside_the_content() {
        let mut panels = PluginPanels::default();
        panels.open("first", open_params(false)).unwrap();
        panels.scroll_end(1);
        assert_eq!(panels.clamped_offset(1), 1);
        panels.scroll(10, 1);
        assert_eq!(panels.clamped_offset(1), 1);
        panels.scroll(-10, 1);
        assert_eq!(panels.clamped_offset(1), 0);
    }

    #[test]
    fn removes_panels_when_the_plugin_stops() {
        let mut panels = PluginPanels::default();
        panels.open("first", open_params(false)).unwrap();
        assert!(!panels.remove_plugin("second"));
        assert!(panels.remove_plugin("first"));
        assert!(panels.active().is_none());
    }
}
