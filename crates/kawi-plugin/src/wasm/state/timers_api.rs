use super::super::bindings::kawi::plugin::editor_schedule::{self, TimerInfo};
use super::super::bindings::kawi::plugin::types::Error;
use super::super::convert::error_from_rpc;
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{TimerAfterParams, TimerCancelParams, TimerEveryParams};

impl editor_schedule::Host for WasmState {
    fn after(&mut self, delay_ms: u32, payload: Option<String>) -> Result<u64, Error> {
        let params = TimerAfterParams { delay_ms, payload };
        let value = self
            .bridge
            .request("timer.after", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        decode_json(value)
    }

    fn every(&mut self, interval_ms: u32, payload: Option<String>) -> Result<u64, Error> {
        let params = TimerEveryParams {
            interval_ms,
            payload,
        };
        let value = self
            .bridge
            .request("timer.every", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        decode_json(value)
    }

    fn cancel(&mut self, timer: u64) -> Result<bool, Error> {
        let params = TimerCancelParams { timer };
        let value = self
            .bridge
            .request("timer.cancel", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        decode_json(value)
    }

    fn list_timers(&mut self) -> Result<Vec<TimerInfo>, Error> {
        let value = self
            .bridge
            .request("timer.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        let timers: Vec<crate::protocol::TimerInfo> = decode_json(value)?;
        Ok(timers.into_iter().map(timer_info).collect())
    }
}

fn timer_info(timer: crate::protocol::TimerInfo) -> TimerInfo {
    TimerInfo {
        id: timer.id,
        interval_ms: timer.interval_ms,
        payload: timer.payload,
    }
}
