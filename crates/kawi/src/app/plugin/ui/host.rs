use kawi_plugin::{
    OverlayParams, PickerParams, RpcError, UiConfirmParams, UiInputParams, UiSelectParams,
    INTERNAL_ERROR, INVALID_REQUEST,
};
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
        if method == "ui.overlay_wait" && self.plugin_ui.has_prompt() {
            return Some(Err(RpcError::ui_busy("a plugin prompt is already active")));
        }
        if method != "ui.overlay_wait" && self.plugin_overlays.has_overlay() {
            return Some(Err(RpcError::ui_busy("a plugin overlay is already active")));
        }
        if method == "ui.overlay_wait" {
            let result = serde_json::from_value::<OverlayParams>(params)
                .map_err(invalid_params)
                .and_then(|params| {
                    self.plugin_overlays
                        .wait(plugin, request.clone(), params.overlay)
                });
            return match result {
                Ok(Some(event)) => Some(
                    serde_json::to_value(event)
                        .map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string())),
                ),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            };
        }
        if method == "ui.picker" {
            let result = serde_json::from_value::<PickerParams>(params)
                .map_err(invalid_params)
                .and_then(|params| self.plugin_ui.open_picker(plugin, request.clone(), params));
            return result.err().map(Err);
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
