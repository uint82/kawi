use kawi_plugin::{
    validate_panel_open, validate_panel_update, PanelOpenParams, PanelOpenResult, PanelParams,
    PanelUpdateParams, RpcError,
};
use serde_json::Value;

use super::super::host_methods::{invalid_params, json_result};
use crate::app::App;

impl App {
    pub(crate) fn plugin_panel_open(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: PanelOpenParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_panel_open(&params)?;
        let focus = params.focus;
        let panel = self.plugin_panel.open(plugin, params)?;
        self.sync_plugin_panel_size();
        if focus && !self.focus_plugin_panel() {
            self.plugin_panel.unfocus();
        }
        let rect = self.plugin_panel_rect();
        json_result(PanelOpenResult {
            panel,
            width: dimension(rect.map(|rect| rect.width)),
            height: dimension(rect.map(|rect| rect.height)),
        })
    }

    pub(crate) fn plugin_panel_update(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: PanelUpdateParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_panel_update(&params)?;
        self.plugin_panel.update(plugin, params)?;
        self.sync_plugin_panel_size();
        Ok(Value::Null)
    }

    pub(crate) fn plugin_panel_focus(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: PanelParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.plugin_panel.focus(plugin, params)?;
        if !self.focus_plugin_panel() {
            self.plugin_panel.unfocus();
        }
        Ok(Value::Null)
    }

    pub(crate) fn plugin_panel_close(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: PanelParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.plugin_panel.close(plugin, params)?;
        self.sync_plugin_panel_size();
        Ok(Value::Null)
    }
}

fn dimension(value: Option<usize>) -> u32 {
    value
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_default()
}
