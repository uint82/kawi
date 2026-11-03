use kawi_plugin::{
    ProcessJobEvent, ProcessJobParams, ProcessOpenParams, ProcessSignalParams, ProcessWriteParams,
    RpcError, INTERNAL_ERROR,
};
use serde_json::Value;

use super::super::App;
use super::host_methods::{invalid_params, json_result};
use super::runtime::PluginRuntime;

impl PluginRuntime {
    pub(crate) fn open_job(
        &self,
        plugin: &str,
        params: ProcessOpenParams,
    ) -> Result<kawi_plugin::ProcessOpenResult, RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .open_job(plugin, params)
    }

    pub(crate) fn write_job(
        &self,
        plugin: &str,
        params: ProcessWriteParams,
    ) -> Result<(), RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .write_job(plugin, params)
    }

    pub(crate) fn close_job_stdin(
        &self,
        plugin: &str,
        params: ProcessJobParams,
    ) -> Result<(), RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .close_job_stdin(plugin, params)
    }

    pub(crate) fn signal_job(
        &self,
        plugin: &str,
        params: ProcessSignalParams,
    ) -> Result<(), RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .signal_job(plugin, params)
    }

    pub(crate) fn kill_job(&self, plugin: &str, params: ProcessJobParams) -> Result<(), RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .kill_job(plugin, params)
    }

    pub(crate) fn job_status(
        &self,
        plugin: &str,
        params: ProcessJobParams,
    ) -> Result<kawi_plugin::ProcessStatusResult, RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .job_status(plugin, params)
    }

    pub(crate) fn poll_job(
        &self,
        plugin: &str,
        params: ProcessJobParams,
    ) -> Result<Vec<ProcessJobEvent>, RpcError> {
        self.host
            .as_ref()
            .ok_or_else(|| RpcError::unavailable("plugin host is not running"))?
            .poll_job(plugin, params)
    }

    pub(crate) fn stop_jobs(&self, plugin: &str) {
        if let Some(host) = self.host.as_ref() {
            host.stop_jobs(plugin);
        }
    }

    pub(crate) fn notify_job(
        &self,
        plugin: &str,
        event: &str,
        payload: Value,
    ) -> Result<kawi_plugin::NotifyOutcome, kawi_plugin::PluginError> {
        self.host
            .as_ref()
            .ok_or_else(|| {
                kawi_plugin::PluginError::Spawn("plugin host is not running".to_string())
            })?
            .notify_job(plugin, event, payload)
    }
}

impl App {
    pub(super) fn plugin_open_job(&self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: ProcessOpenParams = serde_json::from_value(params).map_err(invalid_params)?;
        json_result(self.plugins.open_job(plugin, params)?)
    }

    pub(super) fn plugin_write_job(&self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: ProcessWriteParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.plugins.write_job(plugin, params)?;
        Ok(Value::Null)
    }

    pub(super) fn plugin_close_job_stdin(
        &self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: ProcessJobParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.plugins.close_job_stdin(plugin, params)?;
        Ok(Value::Null)
    }

    pub(super) fn plugin_signal_job(&self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: ProcessSignalParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.plugins.signal_job(plugin, params)?;
        Ok(Value::Null)
    }

    pub(super) fn plugin_kill_job(&self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: ProcessJobParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.plugins.kill_job(plugin, params)?;
        Ok(Value::Null)
    }

    pub(super) fn plugin_job_status(&self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: ProcessJobParams = serde_json::from_value(params).map_err(invalid_params)?;
        let status = self.plugins.job_status(plugin, params)?;
        serde_json::to_value(status)
            .map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))
    }

    pub(super) fn plugin_poll_job(&self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: ProcessJobParams = serde_json::from_value(params).map_err(invalid_params)?;
        json_result(self.plugins.poll_job(plugin, params)?)
    }
}
