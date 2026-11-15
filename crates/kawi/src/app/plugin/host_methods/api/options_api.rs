use kawi_plugin::{
    validate_option_value, OptionInfo, OptionsGetParams, OptionsListParams, OptionsResetParams,
    OptionsSetParams, RpcError,
};
use serde_json::Value;

use crate::app::App;
use crate::options::{self, OptionInput, OptionSpec};

use super::super::{invalid_params, json_result};

impl App {
    pub(super) fn plugin_options_list(&self, params: Value) -> Result<Value, RpcError> {
        let params: OptionsListParams = serde_json::from_value(params).map_err(invalid_params)?;
        let prefix = params.prefix.as_deref().unwrap_or_default();
        let infos = options::OPTIONS
            .iter()
            .filter(|spec| spec.name.starts_with(prefix))
            .map(|spec| self.option_info(spec))
            .collect::<Vec<_>>();
        json_result(infos)
    }

    pub(super) fn plugin_options_get(&self, params: Value) -> Result<Value, RpcError> {
        let params: OptionsGetParams = serde_json::from_value(params).map_err(invalid_params)?;
        let spec = options::lookup(&params.name)
            .ok_or_else(|| RpcError::option_not_found(&params.name))?;
        json_result(self.option_info(spec))
    }

    pub(super) fn plugin_options_set(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: OptionsSetParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_option_value(&params.value).map_err(RpcError::invalid_params)?;
        let spec = options::lookup(&params.name)
            .ok_or_else(|| RpcError::option_not_found(&params.name))?;
        let input = option_input(&params.value)?;
        self.write_option(spec, input)
            .map_err(RpcError::invalid_params)?;
        json_result(self.option_info(spec))
    }

    pub(super) fn plugin_options_reset(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: OptionsResetParams = serde_json::from_value(params).map_err(invalid_params)?;
        let spec = options::lookup(&params.name)
            .ok_or_else(|| RpcError::option_not_found(&params.name))?;
        self.reset_option(spec).map_err(RpcError::invalid_params)?;
        json_result(self.option_info(spec))
    }

    fn option_info(&self, spec: &'static OptionSpec) -> OptionInfo {
        OptionInfo {
            name: spec.name.to_string(),
            aliases: spec
                .aliases
                .iter()
                .map(|alias| (*alias).to_string())
                .collect(),
            description: spec.description.to_string(),
            kind: spec.kind.name().to_string(),
            scope: spec.scope.name().to_string(),
            section: spec.section.name().to_string(),
            default: spec.default_value().display(),
            value: self.option_value(spec).display(),
            values: spec.values(&self.option_context()),
        }
    }
}

fn option_input(value: &Value) -> Result<OptionInput<'_>, RpcError> {
    match value {
        Value::Bool(value) => Ok(OptionInput::Bool(*value)),
        Value::String(value) => Ok(OptionInput::Text(value)),
        Value::Number(value) => value
            .as_i64()
            .map(OptionInput::Integer)
            .ok_or_else(|| RpcError::invalid_params("option integers must fit in 64 bits")),
        _ => Err(RpcError::invalid_params(
            "option values must be boolean, integer, or text",
        )),
    }
}
