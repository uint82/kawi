use super::super::bindings::kawi::plugin::options_write::{self, OptionInfo};
use super::super::bindings::kawi::plugin::types::{Argument, Error, OptionKind, OptionScope};
use super::super::convert::error_from_rpc;
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{OptionsResetParams, OptionsSetParams, RpcError, PLUGIN_UNAVAILABLE};

impl options_write::Host for WasmState {
    fn set(&mut self, name: String, value: Argument) -> Result<OptionInfo, Error> {
        let params = OptionsSetParams {
            name,
            value: super::super::convert::argument_to_json(value).map_err(value_error)?,
        };
        let value = self
            .bridge
            .request("options.set", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        option_info_from_json(value)
    }

    fn reset(&mut self, name: String) -> Result<OptionInfo, Error> {
        let params = OptionsResetParams { name };
        let value = self
            .bridge
            .request("options.reset", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        option_info_from_json(value)
    }
}

fn option_info_from_json(value: serde_json::Value) -> Result<OptionInfo, Error> {
    let info: crate::protocol::OptionInfo = decode_json(value)?;
    Ok(OptionInfo {
        name: info.name,
        aliases: info.aliases,
        description: info.description,
        kind: option_kind(&info.kind)?,
        scope: option_scope(&info.scope)?,
        section: info.section,
        default: info.default,
        value: info.value,
        values: info.values,
    })
}

fn value_error(error: crate::error::PluginError) -> Error {
    error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
}

fn option_kind(kind: &str) -> Result<OptionKind, Error> {
    match kind {
        "boolean" => Ok(OptionKind::Boolean),
        "integer" => Ok(OptionKind::Integer),
        "text" => Ok(OptionKind::Text),
        "enumeration" => Ok(OptionKind::Enumeration),
        "list-values" => Ok(OptionKind::ListValues),
        other => Err(invalid_option_info(other)),
    }
}

fn option_scope(scope: &str) -> Result<OptionScope, Error> {
    match scope {
        "global" => Ok(OptionScope::Global),
        "buffer" => Ok(OptionScope::Buffer),
        "window" => Ok(OptionScope::Window),
        other => Err(invalid_option_info(other)),
    }
}

fn invalid_option_info(value: &str) -> Error {
    error_from_rpc(RpcError::new(
        PLUGIN_UNAVAILABLE,
        format!("invalid option metadata: {value}"),
    ))
}
