use kawi_plugin::{RpcError, WindowCloseParams, WindowInfo, WindowOpenParams, WindowParams};
use serde_json::Value;

use super::super::super::super::App;
use super::super::{invalid_params, json_result};

impl App {
    pub(super) fn plugin_window_list(&self) -> Result<Value, RpcError> {
        let infos = self
            .window_ids()
            .into_iter()
            .filter_map(|id| self.window_info(id))
            .collect::<Vec<_>>();
        json_result(infos)
    }

    pub(super) fn plugin_window_get(&self, params: Value) -> Result<Value, RpcError> {
        let params: WindowParams = serde_json::from_value(params).map_err(invalid_params)?;
        json_result(
            self.window_info(params.window)
                .ok_or_else(|| RpcError::window_not_found(params.window))?,
        )
    }

    pub(super) fn plugin_window_focus(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: WindowParams = serde_json::from_value(params).map_err(invalid_params)?;
        if self.window_info(params.window).is_none() {
            return Err(RpcError::window_not_found(params.window));
        }
        self.focus_window(params.window);
        json_result(
            self.window_info(params.window)
                .ok_or_else(|| RpcError::window_not_found(params.window))?,
        )
    }

    pub(super) fn plugin_window_open(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: WindowOpenParams = serde_json::from_value(params).map_err(invalid_params)?;
        let number = usize::try_from(params.buffer)
            .map_err(|_| RpcError::invalid_params("buffer id out of range"))?;
        if self.editor.buffer_is_dirty(number).is_none() {
            return Err(RpcError::buffer_not_found(params.buffer));
        }
        let vertical = match params.direction.as_str() {
            "horizontal" => false,
            "vertical" => true,
            other => {
                return Err(RpcError::invalid_params(format!(
                    "invalid split direction: {other}"
                )))
            }
        };
        if !self.open_buffer_window_direction(number, vertical) {
            return Err(RpcError::invalid_params("cannot open buffer in a split"));
        }
        let window = self.windows.active();
        json_result(
            self.window_info(window)
                .ok_or_else(|| RpcError::window_not_found(window))?,
        )
    }

    pub(super) fn plugin_window_close(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: WindowCloseParams = serde_json::from_value(params).map_err(invalid_params)?;
        if self.window_info(params.window).is_none() {
            return Err(RpcError::window_not_found(params.window));
        }
        if self.windows.len() <= 1 {
            return Err(RpcError::last_window());
        }
        let before = self.windows.len();
        self.close_window(params.window, params.force)
            .map_err(RpcError::invalid_params)?;
        if self.windows.len() == before {
            return Err(RpcError::invalid_params("cannot close window"));
        }
        let window = self.windows.active();
        json_result(
            self.window_info(window)
                .ok_or_else(|| RpcError::window_not_found(window))?,
        )
    }

    fn window_info(&self, id: u64) -> Option<WindowInfo> {
        let (buffer, rect, active) = self.window_info_at(id)?;
        Some(WindowInfo {
            window: id,
            buffer: buffer as u64,
            active,
            width: rect.width as u32,
            height: rect.height as u32,
        })
    }
}
