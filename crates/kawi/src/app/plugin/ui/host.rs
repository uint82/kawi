use kawi_plugin::{RpcError, UiConfirmParams, UiInputParams, UiSelectParams, INVALID_REQUEST};
use serde_json::Value;

use super::super::host_methods::invalid_params;
use crate::app::App;

impl App {
    pub(crate) fn handle_plugin_prompt_request(
        &mut self,
        plugin: &str,
        request: &Value,
        method: &str,
        params: Value,
    ) -> Option<Result<Value, RpcError>> {
        if !request.is_u64() {
            return Some(Err(RpcError::new(
                INVALID_REQUEST,
                "interactive UI requires an integer request id",
            )));
        }
        let result = match method {
            "ui.input" => serde_json::from_value::<UiInputParams>(params)
                .map_err(invalid_params)
                .and_then(|params| self.plugin_ui.open_input(plugin, request.clone(), params)),
            "ui.select" => serde_json::from_value::<UiSelectParams>(params)
                .map_err(invalid_params)
                .and_then(|params| self.plugin_ui.open_select(plugin, request.clone(), params)),
            "ui.confirm" => serde_json::from_value::<UiConfirmParams>(params)
                .map_err(invalid_params)
                .and_then(|params| self.plugin_ui.open_confirm(plugin, request.clone(), params)),
            _ => return None,
        };
        result.err().map(Err)
    }
}
