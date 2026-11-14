use super::super::bindings::kawi::plugin::editor_commands::{
    self, CommandSpec, RegisteredCommand as WitRegisteredCommand,
};
use super::super::bindings::kawi::plugin::types::Error;
use super::super::convert::error_from_rpc;
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    CommandRegisterParams, CommandUnregisterParams, RegisteredCommand, RpcError, PLUGIN_UNAVAILABLE,
};

impl editor_commands::Host for WasmState {
    fn register(&mut self, spec: CommandSpec) -> Result<WitRegisteredCommand, Error> {
        let params = CommandRegisterParams {
            id: spec.id,
            title: spec.title,
            description: spec.description,
            aliases: spec.aliases,
        };
        let value = self
            .bridge
            .request("command.register", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        registered_command_from_json(value)
    }

    fn unregister(&mut self, id: String) -> Result<(), Error> {
        let params = serialize_params(CommandUnregisterParams { id })?;
        self.bridge
            .request("command.unregister", params)
            .map_err(error_from_rpc)?;
        Ok(())
    }

    fn list_registered(&mut self) -> Result<Vec<WitRegisteredCommand>, Error> {
        let value = self
            .bridge
            .request("command.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        let commands: Vec<RegisteredCommand> = decode_json(value)?;
        Ok(commands.into_iter().map(registered_command).collect())
    }
}

fn registered_command_from_json(value: serde_json::Value) -> Result<WitRegisteredCommand, Error> {
    let command: RegisteredCommand = serde_json::from_value(value)
        .map_err(|error| error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string())))?;
    Ok(registered_command(command))
}

fn registered_command(command: RegisteredCommand) -> WitRegisteredCommand {
    WitRegisteredCommand {
        id: command.id,
        name: command.name,
        title: command.title,
        description: command.description,
        aliases: command.aliases,
    }
}
