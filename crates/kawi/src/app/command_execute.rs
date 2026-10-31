use anyhow::Result;
use kawi_editor::{parse, Action};

use super::command_text::split_command;
use super::App;
use crate::command::{CommandHandler, CommandId};

impl App {
    pub(super) fn handle_execute(&mut self) -> Result<bool> {
        let text = self.editor.command_text().to_string();
        self.last_command = Some(text.clone());
        self.editor.apply(Action::CancelCommand);
        self.execute_command_text(&text)
    }

    pub(super) fn execute_command_text(&mut self, text: &str) -> Result<bool> {
        let (name, rest) = split_command(text);
        if let Some(id) = self.plugin_command_id(name) {
            return self.execute_plugin_command(id, rest);
        }
        self.run_command(parse(text))
    }

    fn plugin_command_id(&self, name: &str) -> Option<CommandId> {
        let id = self.commands.find(name)?;
        matches!(self.commands.spec(id)?.handler, CommandHandler::Plugin(_)).then_some(id)
    }

    fn execute_plugin_command(&mut self, id: CommandId, rest: &str) -> Result<bool> {
        if !rest.is_empty() {
            self.show_message(format!("E488: Trailing characters: {rest}"));
            return Ok(false);
        }
        self.dispatch_command_id(id, None)
    }
}
