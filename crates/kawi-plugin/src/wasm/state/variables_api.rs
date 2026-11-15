use super::super::bindings::kawi::plugin::editor_variables::{self, Variable};
use super::super::bindings::kawi::plugin::types::{
    Argument, Error, VariableScope as WitVariableScope, VariableTarget as WitVariableTarget,
};
use super::super::convert::{argument_from_json, argument_to_json, error_from_rpc};
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    RpcError, VariableParams, VariableScope, VariableSetParams, VariableTarget,
    VariableTargetParams, PLUGIN_UNAVAILABLE,
};

impl editor_variables::Host for WasmState {
    fn get(&mut self, target: WitVariableTarget, key: String) -> Result<Option<Argument>, Error> {
        let params = VariableParams {
            target: target_from_wit(target),
            key,
        };
        let value = self
            .bridge
            .request("variable.get", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        if value.is_null() {
            return Ok(None);
        }
        argument_from_json(value).map(Some).map_err(error_from_rpc)
    }

    fn set(
        &mut self,
        target: WitVariableTarget,
        key: String,
        value: Argument,
    ) -> Result<(), Error> {
        let params = VariableSetParams {
            target: target_from_wit(target),
            key,
            value: argument_to_json(value).map_err(value_error)?,
        };
        self.bridge
            .request("variable.set", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        Ok(())
    }

    fn delete(&mut self, target: WitVariableTarget, key: String) -> Result<bool, Error> {
        let params = VariableParams {
            target: target_from_wit(target),
            key,
        };
        let value = self
            .bridge
            .request("variable.delete", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        decode_json(value)
    }

    fn list_variables(&mut self, target: WitVariableTarget) -> Result<Vec<Variable>, Error> {
        let params = VariableTargetParams {
            target: target_from_wit(target),
        };
        let value = self
            .bridge
            .request("variable.list", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let variables: Vec<crate::protocol::Variable> = decode_json(value)?;
        variables.into_iter().map(variable_from_protocol).collect()
    }
}

fn target_from_wit(target: WitVariableTarget) -> VariableTarget {
    VariableTarget {
        scope: match target.scope {
            WitVariableScope::Global => VariableScope::Global,
            WitVariableScope::Plugin => VariableScope::Plugin,
            WitVariableScope::Buffer => VariableScope::Buffer,
            WitVariableScope::Window => VariableScope::Window,
        },
        buffer: target.buffer,
        window: target.window,
    }
}

fn variable_from_protocol(variable: crate::protocol::Variable) -> Result<Variable, Error> {
    Ok(Variable {
        key: variable.key,
        value: argument_from_json(variable.value).map_err(error_from_rpc)?,
    })
}

fn value_error(error: crate::error::PluginError) -> Error {
    error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
}
