use kawi_plugin::{
    validate_overlay_open, validate_overlay_update, OverlayOpenParams, OverlayOpenResult,
    OverlayParams, OverlayUpdateParams, RpcError,
};
use serde_json::Value;

use super::super::host_methods::{invalid_params, json_result};
use crate::app::App;

impl App {
    pub(crate) fn plugin_overlay_open(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        if self.plugin_ui.has_prompt() {
            return Err(RpcError::ui_busy("a plugin prompt is already active"));
        }
        let params: OverlayOpenParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_overlay_open(&params)?;
        let overlay = self.plugin_overlays.open(plugin, params)?;
        let rect = self.plugin_overlay_rect();
        json_result(OverlayOpenResult {
            overlay,
            width: u32::try_from(rect.width).unwrap_or_default(),
            height: u32::try_from(rect.height).unwrap_or_default(),
        })
    }

    pub(crate) fn plugin_overlay_update(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: OverlayUpdateParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_overlay_update(&params)?;
        self.plugin_overlays.update(plugin, params)?;
        Ok(Value::Null)
    }

    pub(crate) fn plugin_overlay_close(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: OverlayParams = serde_json::from_value(params).map_err(invalid_params)?;
        if let Some((waiter, event)) = self.plugin_overlays.close(plugin, params.overlay)? {
            self.respond_plugin_overlay(waiter, event);
        }
        Ok(Value::Null)
    }
}
