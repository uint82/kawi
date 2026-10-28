use anyhow::Result;
use serde_json::Value;

use super::super::App;

impl App {
    pub(crate) fn run_plugin_command(
        &mut self,
        target: u32,
        args: Option<toml::Value>,
    ) -> Result<bool> {
        let Some((plugin, command)) = self
            .plugins
            .target(target)
            .map(|(plugin, command)| (plugin.to_string(), command.to_string()))
        else {
            self.show_message("Unknown plugin command".to_string());
            return Ok(false);
        };
        let args = match args {
            Some(args) => match serde_json::to_value(args) {
                Ok(args) => args,
                Err(error) => {
                    self.show_message(format!("{plugin}.{command}: {error}"));
                    return Ok(false);
                }
            },
            None => Value::Null,
        };
        let context = self.plugin_context();
        if let Err(error) = self.plugins.invoke(target, args, context) {
            self.show_message(format!("{plugin}.{command}: {error}"));
        }
        Ok(false)
    }
}
