use super::format;
use super::*;
use kawi_tui::frame::DebugRowKind;
use std::path::PathBuf;

mod app;
mod rows;

use rows::*;

const PANEL_WIDTH_MIN: usize = 28;
const PANEL_WIDTH_MAX: usize = 60;
const PANEL_CONSOLE_LINES: usize = 12;
const PANEL_SCROLL_STEP: usize = 8;

#[derive(Clone)]
pub(super) enum PanelTarget {
    None,
    Thread(i64),
    Frame(i64),
    Variable(i64),
    Watch(usize),
    Breakpoint(PathBuf, u32),
    FunctionBreakpoint(String),
    ExceptionFilter(String),
    LoadMore(i64),
}

pub(super) struct PanelEntry {
    pub(super) depth: usize,
    pub(super) label: String,
    pub(super) kind: DebugRowKind,
    pub(super) target: PanelTarget,
}

impl DebugUi {
    pub(super) fn panel_width(&self, available_width: usize) -> usize {
        if !self.panel_visible {
            return 0;
        }
        let available = available_width.saturating_sub(super::super::terminal_ui::MIN_EDITOR_WIDTH);
        if available < PANEL_WIDTH_MIN {
            return 0;
        }
        (available_width / 3)
            .clamp(PANEL_WIDTH_MIN, PANEL_WIDTH_MAX)
            .min(available)
    }

    pub(super) fn panel_visible(&self) -> bool {
        self.panel_visible
    }

    pub(crate) fn panel_selected(&self) -> usize {
        self.panel_selected
    }

    pub(crate) fn panel_offset(&self) -> usize {
        self.panel_offset
    }

    pub(super) fn show_panel(&mut self) {
        self.panel_visible = true;
    }

    pub(super) fn hide_panel(&mut self) {
        self.panel_visible = false;
        self.panel_selected = 0;
        self.panel_offset = 0;
    }

    pub(crate) fn panel_status(&self) -> &'static str {
        match self.status {
            DebugStatus::Idle => "idle",
            DebugStatus::Starting => "starting",
            DebugStatus::Running => "running",
            DebugStatus::Stopped => "stopped",
            DebugStatus::Terminated => "terminated",
            DebugStatus::Failed => "failed",
        }
    }

    pub(super) fn panel_entries(&self) -> Vec<PanelEntry> {
        let mut entries = Vec::new();
        push_section(&mut entries, "Session");
        entries.push(text_entry(
            1,
            format!("Status: {}", self.panel_status()),
            false,
        ));
        if let Some(configuration) = self.configuration.as_ref() {
            entries.push(text_entry(
                1,
                format!("Configuration: {configuration}"),
                false,
            ));
        }
        if let Some(thread_id) = self.thread_id {
            entries.push(text_entry(1, format!("Thread: {thread_id}"), false));
        }
        push_threads(&mut entries, self);
        push_stack(&mut entries, self);
        push_variables(&mut entries, self);
        push_watches(&mut entries, self);
        push_breakpoints(&mut entries, self);
        push_function_breakpoints(&mut entries, self);
        push_exception_filters(&mut entries, self);
        push_console(&mut entries, self);
        entries
    }

    pub(super) fn activate_panel(&mut self) -> bool {
        let entries = self.panel_entries();
        let Some(entry) = entries.get(self.panel_selected) else {
            return false;
        };
        match entry.target.clone() {
            PanelTarget::Thread(thread_id) => self.select_thread(thread_id),
            PanelTarget::Frame(frame_id) => self.select_frame(frame_id),
            PanelTarget::Variable(reference) => self.toggle_variable(reference),
            PanelTarget::Watch(index) => {
                self.show_watch(index);
                true
            }
            PanelTarget::Breakpoint(path, line) => {
                self.jump = Some(DebugJump {
                    path,
                    line: line.saturating_sub(1) as usize,
                    column: 0,
                });
                true
            }
            PanelTarget::FunctionBreakpoint(name) => {
                self.message = Some(format!("debug: function breakpoint {name}"));
                true
            }
            PanelTarget::ExceptionFilter(filter) => self.toggle_exception_filter(filter),
            PanelTarget::LoadMore(reference) => self.load_more_variables(reference),
            PanelTarget::None => false,
        }
    }

    pub(super) fn move_panel_selection(&mut self, delta: isize, viewport: usize) {
        let entries = self.panel_entries();
        if entries.is_empty() {
            self.panel_selected = 0;
            self.panel_offset = 0;
            return;
        }
        let last = entries.len().saturating_sub(1);
        let selected = self.panel_selected.min(last) as isize;
        self.panel_selected = selected.saturating_add(delta).clamp(0, last as isize) as usize;
        self.sync_panel_offset(viewport);
    }

    pub(super) fn move_panel_page(&mut self, down: bool, viewport: usize) {
        let delta = PANEL_SCROLL_STEP.min(viewport.max(1)) as isize;
        self.move_panel_selection(if down { delta } else { -delta }, viewport);
    }

    pub(super) fn select_panel_start(&mut self, viewport: usize) {
        self.panel_selected = 0;
        self.sync_panel_offset(viewport);
    }

    pub(super) fn select_panel_end(&mut self, viewport: usize) {
        self.panel_selected = self.panel_entries().len().saturating_sub(1);
        self.sync_panel_offset(viewport);
    }

    pub(crate) fn sync_panel_selection(&mut self, viewport: usize) {
        let last = self.panel_entries().len().saturating_sub(1);
        self.panel_selected = self.panel_selected.min(last);
        self.sync_panel_offset(viewport);
    }

    fn sync_panel_offset(&mut self, viewport: usize) {
        let viewport = viewport.max(1);
        if self.panel_selected < self.panel_offset {
            self.panel_offset = self.panel_selected;
        }
        let bottom = self.panel_offset.saturating_add(viewport);
        if self.panel_selected >= bottom {
            self.panel_offset = self
                .panel_selected
                .saturating_add(1)
                .saturating_sub(viewport);
        }
    }

    fn show_watch(&mut self, index: usize) {
        let Some(watch) = self.watches.get(index) else {
            self.message = Some(format!("debug: unknown watch {}", index.saturating_add(1)));
            return;
        };
        self.message = Some(format!(
            "debug: watch {}: {}",
            index.saturating_add(1),
            format::watch_value(watch)
        ));
    }
}
