use serde_json::Value;

use kawi_plugin::{
    Capability, EditorCommandInfo, EditorExecuteParams, EditorListCommandsResult, RpcError,
    INTERNAL_ERROR, INVALID_PARAMS,
};

use super::super::App;
use super::host_methods::invalid_params;
use crate::command::{CommandHandler, CommandId, CommandSource, CommandSpec};

impl App {
    pub(crate) fn plugin_execute_command(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: EditorExecuteParams = serde_json::from_value(params).map_err(invalid_params)?;
        let id = self.composable_command_id(&params.command)?;
        let name = self.canonical_command_name(id)?;
        if !self.plugins.allows_editor_command(plugin, &name) {
            return Err(self
                .plugins
                .capability_error(plugin, Capability::EditorExecute));
        }
        self.dispatch_command_id(id, None).map_err(command_error)?;
        Ok(Value::Null)
    }

    pub(crate) fn plugin_list_commands(
        &self,
        plugin: &str,
        _params: Value,
    ) -> Result<Value, RpcError> {
        let allowed = self.plugins.editor_commands(plugin);
        let mut commands = self
            .commands
            .specs()
            .filter(|spec| is_composable(spec))
            .filter(|spec| allowed.iter().any(|allowed| allowed == &spec.name))
            .map(command_info)
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| left.id.cmp(&right.id));
        serialize_commands(commands)
    }

    fn composable_command_id(&self, name: &str) -> Result<CommandId, RpcError> {
        let id = self
            .commands
            .find(name)
            .ok_or_else(|| unknown_command(name))?;
        let spec = self
            .commands
            .spec(id)
            .ok_or_else(|| unknown_command(name))?;
        if !is_composable(spec) {
            return Err(RpcError::invalid_params(format!(
                "command is not composable: {}",
                spec.name
            )));
        }
        Ok(id)
    }

    fn canonical_command_name(&self, id: CommandId) -> Result<String, RpcError> {
        self.commands
            .spec(id)
            .map(|spec| spec.name.clone())
            .ok_or_else(|| RpcError::invalid_params("unknown editor command"))
    }
}

fn is_composable(spec: &CommandSpec) -> bool {
    matches!(&spec.source, CommandSource::Builtin)
        && matches!(
            spec.handler,
            CommandHandler::Action(_) | CommandHandler::Noop
        )
}

fn command_info(spec: &CommandSpec) -> EditorCommandInfo {
    EditorCommandInfo {
        id: spec.name.clone(),
        description: spec.description.clone(),
        aliases: spec.aliases.clone(),
    }
}

fn serialize_commands(commands: Vec<EditorCommandInfo>) -> Result<Value, RpcError> {
    serde_json::to_value(EditorListCommandsResult { commands })
        .map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))
}

fn unknown_command(name: &str) -> RpcError {
    RpcError::new(INVALID_PARAMS, format!("unknown editor command: {name}"))
}

fn command_error(error: anyhow::Error) -> RpcError {
    RpcError::new(INTERNAL_ERROR, error.to_string())
}
