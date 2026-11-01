use crate::protocol::{
    UiConfirmParams, UiInputParams, UiProgressFinishParams, UiProgressOpenResult,
    UiProgressStartParams, UiProgressUpdateParams, UiProgressUpdateResult, UiPromptOutcome,
    UiSelectParams,
};

use super::bindings::kawi::plugin::types::{Error, PromptOutcome};
use super::bindings::kawi::plugin::{ui_progress, ui_prompt};
use super::convert::{error_from_rpc, prompt_outcome_to_wit};
use super::state::{decode_json, serialize_params, WasmState};

impl ui_prompt::Host for WasmState {
    fn input(
        &mut self,
        title: String,
        placeholder: Option<String>,
        value: Option<String>,
        max_length: Option<u32>,
    ) -> Result<PromptOutcome, Error> {
        let params = serialize_params(UiInputParams {
            title,
            placeholder,
            value,
            max_length,
        })?;
        let value = self
            .bridge
            .request_interactive("ui.input", params)
            .map_err(error_from_rpc)?;
        prompt_outcome(value)
    }

    fn select(
        &mut self,
        title: String,
        options: Vec<String>,
        selected: Option<u32>,
    ) -> Result<PromptOutcome, Error> {
        let params = serialize_params(UiSelectParams {
            title,
            options,
            selected,
        })?;
        let value = self
            .bridge
            .request_interactive("ui.select", params)
            .map_err(error_from_rpc)?;
        prompt_outcome(value)
    }

    fn confirm(&mut self, title: String, message: Option<String>) -> Result<PromptOutcome, Error> {
        let params = serialize_params(UiConfirmParams { title, message })?;
        let value = self
            .bridge
            .request_interactive("ui.confirm", params)
            .map_err(error_from_rpc)?;
        prompt_outcome(value)
    }
}

impl ui_progress::Host for WasmState {
    fn start(
        &mut self,
        title: String,
        message: Option<String>,
        fraction: Option<f32>,
    ) -> Result<u64, Error> {
        let params = serialize_params(UiProgressStartParams {
            title,
            message,
            fraction,
        })?;
        let value = self
            .bridge
            .request("ui.progress_start", params)
            .map_err(error_from_rpc)?;
        let result: UiProgressOpenResult = decode_json(value)?;
        Ok(result.id)
    }

    fn update(
        &mut self,
        id: u64,
        message: Option<String>,
        fraction: Option<f32>,
    ) -> Result<bool, Error> {
        let params = serialize_params(UiProgressUpdateParams {
            id,
            message,
            fraction,
        })?;
        let value = self
            .bridge
            .request("ui.progress_update", params)
            .map_err(error_from_rpc)?;
        let result: UiProgressUpdateResult = decode_json(value)?;
        Ok(result.canceled)
    }

    fn finish(&mut self, id: u64) -> Result<(), Error> {
        let params = serialize_params(UiProgressFinishParams { id })?;
        self.bridge
            .request("ui.progress_finish", params)
            .map(|_| ())
            .map_err(error_from_rpc)
    }
}

fn prompt_outcome(value: serde_json::Value) -> Result<PromptOutcome, Error> {
    let outcome: UiPromptOutcome = decode_json(value)?;
    Ok(prompt_outcome_to_wit(outcome))
}
