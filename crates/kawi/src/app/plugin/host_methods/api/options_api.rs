use kawi_plugin::{OptionInfo, OptionsGetParams, OptionsListParams, RpcError};
use serde_json::Value;

use crate::app::App;
use crate::options::{self, OptionSpec};

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
