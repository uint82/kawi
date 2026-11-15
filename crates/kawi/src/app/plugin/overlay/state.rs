use std::collections::VecDeque;

use kawi_plugin::{
    sanitize, OverlayAnchor, OverlayCloseReason, OverlayEvent, OverlayLine, OverlayOpenParams,
    OverlayUpdateParams, RpcError, MAX_OVERLAY_EVENTS, OVERLAY_NOT_FOUND, UI_BUSY,
};
use serde_json::Value;

#[derive(Default)]
pub(crate) struct PluginOverlays {
    next_id: u64,
    active: Option<ActiveOverlay>,
    events: VecDeque<QueuedEvent>,
    waiter: Option<OverlayWaiter>,
}

struct ActiveOverlay {
    id: u64,
    plugin: String,
    title: String,
    status: String,
    anchor: OverlayAnchor,
    width: Option<u32>,
    height: Option<u32>,
    focus: bool,
    selected: Option<usize>,
    scroll: usize,
    lines: Vec<OverlayLine>,
}

impl ActiveOverlay {
    fn apply(&mut self, params: OverlayUpdateParams) {
        if let Some(title) = params.title {
            self.title = sanitize(&title);
        }
        if let Some(status) = params.status {
            self.status = sanitize(&status);
        }
        if let Some(anchor) = params.anchor {
            self.anchor = anchor;
        }
        if let Some(width) = params.width {
            self.width = Some(width);
        }
        if let Some(height) = params.height {
            self.height = Some(height);
        }
        if let Some(focus) = params.focus {
            self.focus = focus;
        }
        if let Some(selected) = params.selected {
            self.selected = Some(selected as usize);
        }
        if let Some(scroll) = params.scroll {
            self.scroll = scroll as usize;
        }
        if let Some(lines) = params.lines {
            self.lines = sanitize_lines(lines);
        }
    }
}

pub(crate) struct OverlayWaiter {
    pub(crate) plugin: String,
    pub(crate) request: Value,
}

struct QueuedEvent {
    plugin: String,
    overlay: u64,
    event: OverlayEvent,
}

pub(crate) struct OverlayView<'a> {
    pub(crate) title: &'a str,
    pub(crate) status: &'a str,
    pub(crate) anchor: OverlayAnchor,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) focus: bool,
    pub(crate) selected: Option<usize>,
    pub(crate) scroll: usize,
    pub(crate) lines: &'a [OverlayLine],
}

impl PluginOverlays {
    pub(crate) fn open(
        &mut self,
        plugin: &str,
        params: OverlayOpenParams,
    ) -> Result<u64, RpcError> {
        if self.active.is_some() {
            return Err(RpcError::new(UI_BUSY, "a plugin overlay is already active"));
        }
        let id = self.allocate_id();
        self.active = Some(ActiveOverlay {
            id,
            plugin: plugin.to_string(),
            title: sanitize(&params.title),
            status: params.status.as_deref().map(sanitize).unwrap_or_default(),
            anchor: params.anchor,
            width: params.width,
            height: params.height,
            focus: params.focus,
            selected: params.selected.map(|value| value as usize),
            scroll: params.scroll.unwrap_or(0) as usize,
            lines: sanitize_lines(params.lines),
        });
        Ok(id)
    }

    pub(crate) fn update(
        &mut self,
        plugin: &str,
        params: OverlayUpdateParams,
    ) -> Result<(), RpcError> {
        self.entry_mut(plugin, params.overlay)?.apply(params);
        Ok(())
    }

    pub(crate) fn close(
        &mut self,
        plugin: &str,
        id: u64,
    ) -> Result<Option<(OverlayWaiter, OverlayEvent)>, RpcError> {
        self.entry(plugin, id)?;
        Ok(self.close_active(OverlayCloseReason::Plugin))
    }

    pub(crate) fn wait(
        &mut self,
        plugin: &str,
        request: Value,
        id: u64,
    ) -> Result<Option<OverlayEvent>, RpcError> {
        if let Some(event) = self.pop_event(plugin, id) {
            return Ok(Some(event));
        }
        let Some(active) = self.active.as_ref() else {
            return Err(overlay_not_found());
        };
        if active.id != id || active.plugin != plugin {
            return Err(overlay_not_found());
        }
        if self.waiter.is_some() {
            return Err(RpcError::new(
                UI_BUSY,
                "a plugin overlay wait is already active",
            ));
        }
        self.waiter = Some(OverlayWaiter {
            plugin: plugin.to_string(),
            request,
        });
        Ok(None)
    }

    pub(crate) fn emit(&mut self, event: OverlayEvent) -> Option<(OverlayWaiter, OverlayEvent)> {
        let active = self.active.as_ref()?;
        let overlay = active.id;
        let plugin = active.plugin.clone();
        if self
            .waiter
            .as_ref()
            .is_some_and(|waiter| waiter.plugin == plugin)
        {
            return self.waiter.take().map(|waiter| (waiter, event));
        }
        self.push_event(plugin, overlay, event);
        None
    }

    pub(crate) fn close_active(
        &mut self,
        reason: OverlayCloseReason,
    ) -> Option<(OverlayWaiter, OverlayEvent)> {
        let active = self.active.take()?;
        self.events.retain(|event| event.overlay != active.id);
        let event = OverlayEvent::Closed(reason);
        if let Some(waiter) = self.waiter.take() {
            if waiter.plugin == active.plugin {
                return Some((waiter, event));
            }
            self.waiter = Some(waiter);
        }
        self.push_event(active.plugin.clone(), active.id, event);
        None
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) -> Option<OverlayWaiter> {
        let active = self.active.as_ref()?;
        if active.plugin != plugin {
            return None;
        }
        let id = active.id;
        self.active = None;
        self.events.retain(|event| event.overlay != id);
        self.waiter.take().filter(|waiter| waiter.plugin == plugin)
    }

    pub(crate) fn is_focused(&self) -> bool {
        self.active.as_ref().is_some_and(|overlay| overlay.focus)
    }

    pub(crate) fn has_overlay(&self) -> bool {
        self.active.is_some()
    }

    pub(crate) fn view(&self) -> Option<OverlayView<'_>> {
        let overlay = self.active.as_ref()?;
        Some(OverlayView {
            title: &overlay.title,
            status: &overlay.status,
            anchor: overlay.anchor,
            width: overlay.width,
            height: overlay.height,
            focus: overlay.focus,
            selected: overlay.selected,
            scroll: overlay.scroll,
            lines: &overlay.lines,
        })
    }

    fn push_event(&mut self, plugin: String, overlay: u64, event: OverlayEvent) {
        if self.events.len() >= MAX_OVERLAY_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(QueuedEvent {
            plugin,
            overlay,
            event,
        });
    }

    fn pop_event(&mut self, plugin: &str, id: u64) -> Option<OverlayEvent> {
        let index = self
            .events
            .iter()
            .position(|event| event.overlay == id && event.plugin == plugin)?;
        self.events.remove(index).map(|event| event.event)
    }

    fn entry(&self, plugin: &str, id: u64) -> Result<&ActiveOverlay, RpcError> {
        self.active
            .as_ref()
            .filter(|overlay| overlay.id == id && overlay.plugin == plugin)
            .ok_or_else(overlay_not_found)
    }

    fn entry_mut(&mut self, plugin: &str, id: u64) -> Result<&mut ActiveOverlay, RpcError> {
        self.active
            .as_mut()
            .filter(|overlay| overlay.id == id && overlay.plugin == plugin)
            .ok_or_else(overlay_not_found)
    }

    fn allocate_id(&mut self) -> u64 {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.next_id
    }
}

fn overlay_not_found() -> RpcError {
    RpcError::new(OVERLAY_NOT_FOUND, "unknown overlay")
}

fn sanitize_lines(lines: Vec<OverlayLine>) -> Vec<OverlayLine> {
    lines
        .into_iter()
        .map(|line| OverlayLine {
            spans: line
                .spans
                .into_iter()
                .map(|span| kawi_plugin::OverlaySpan {
                    text: sanitize(&span.text),
                    style: span.style,
                })
                .collect(),
        })
        .collect()
}
