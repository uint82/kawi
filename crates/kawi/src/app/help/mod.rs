use std::collections::BTreeMap;

use super::window::WindowId;
use super::App;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use view::{HelpLink, HelpView};

const HELP_PREFIX: &str = "help:";
pub(super) const OPTIONS_TOPIC: &str = "options";
pub(super) const OPTION_TOPIC: &str = "option";
pub(super) const COMMANDS_TOPIC: &str = "commands";
const OPTION_TOPIC_PREFIX: &str = "option ";

#[cfg(test)]
mod tests;
mod view;

#[derive(Debug)]
pub(super) struct HelpSession {
    pub(super) buffer: usize,
    pub(super) window: Option<WindowId>,
    topic: String,
    links: BTreeMap<usize, Vec<HelpLink>>,
    back: Vec<String>,
}

impl App {
    pub(super) fn handle_help(&mut self, topic: Option<&str>) -> Result<bool> {
        let requested = topic.unwrap_or("index");
        let Some(view) = self.help_view(requested) else {
            self.show_message(format!("E149: Sorry, no help for {requested}"));
            return Ok(false);
        };
        self.show_help_view(view, true)?;
        Ok(false)
    }

    pub(super) fn handle_help_key(&mut self, key: KeyEvent) -> Result<bool> {
        if !self.help_is_active() || self.editor.mode() != kawi_editor::Mode::Normal {
            return Ok(false);
        }
        if key.code == KeyCode::Enter {
            return self.follow_help_link();
        }
        if key.code == KeyCode::Char('t') && key.modifiers == KeyModifiers::CONTROL {
            return self.help_back();
        }
        Ok(false)
    }

    fn help_is_active(&self) -> bool {
        self.help
            .as_ref()
            .is_some_and(|help| help.buffer == self.editor.buffer_number())
    }

    fn follow_help_link(&mut self) -> Result<bool> {
        let row = self.editor.row();
        let col = self.editor.col();
        let target = self.help.as_ref().and_then(|help| {
            help.links
                .get(&row)
                .and_then(|links| {
                    links
                        .iter()
                        .find(|link| col >= link.start && col < link.end)
                })
                .map(|link| link.target.clone())
        });
        let Some(target) = target else {
            return Ok(true);
        };
        let topic = target.strip_prefix(HELP_PREFIX).unwrap_or(&target);
        let Some(view) = self.help_view(topic) else {
            self.show_message(format!("E149: Sorry, no help for {topic}"));
            return Ok(true);
        };
        self.show_help_view(view, true)?;
        Ok(true)
    }

    fn help_back(&mut self) -> Result<bool> {
        let Some(topic) = self.help.as_mut().and_then(|help| help.back.pop()) else {
            return Ok(true);
        };
        let Some(view) = self.help_view(&topic) else {
            self.show_message(format!("E149: Sorry, no help for {topic}"));
            return Ok(true);
        };
        self.show_help_view(view, false)?;
        Ok(true)
    }

    fn show_help_view(&mut self, view: HelpView, push_history: bool) -> Result<()> {
        let text = format!("{}\n", view.lines.join("\n"));
        let name = format!("help:{}", view.topic);
        let previous = self.help.as_ref().map(|help| HelpSlot {
            buffer: help.buffer,
            window: help.window,
            topic: help.topic.clone(),
        });
        let buffer_valid = previous
            .as_ref()
            .is_some_and(|slot| self.editor.buffer_is_internal(slot.buffer) == Some(true));
        if buffer_valid {
            let slot = previous.expect("valid help buffer");
            let window_valid = slot
                .window
                .and_then(|window| self.windows.view(window))
                .is_some_and(|window| window.buffer == slot.buffer);
            if window_valid {
                self.focus_window(slot.window.expect("valid help window"));
            } else if self.editor.buffer_number() != slot.buffer {
                self.open_buffer_window(slot.buffer)?;
            }
            if self
                .editor
                .replace_internal_buffer(slot.buffer, &name, &text)
            {
                self.update_help_session(view, push_history, slot.topic.clone());
                self.finish_help_navigation();
                return Ok(());
            }
        }
        let buffer = self.open_internal_window(&name, &text)?;
        self.help = Some(HelpSession {
            buffer,
            window: Some(self.windows.active()),
            topic: view.topic.clone(),
            links: view.links,
            back: Vec::new(),
        });
        self.finish_help_navigation();
        Ok(())
    }

    fn update_help_session(&mut self, view: HelpView, push_history: bool, previous: String) {
        let Some(help) = self.help.as_mut() else {
            return;
        };
        if push_history && previous != view.topic {
            help.back.push(previous);
        }
        help.topic = view.topic;
        help.links = view.links;
        help.window = Some(self.windows.active());
    }

    fn finish_help_navigation(&mut self) {
        self.editor.apply(kawi_editor::Action::JumpTo(0, 0));
        self.editor.reveal_cursor();
    }
}

struct HelpSlot {
    buffer: usize,
    window: Option<WindowId>,
    topic: String,
}
