use kawi_plugin::{RpcError, UiProgressStartParams, UiProgressUpdateParams};

use super::validation::{
    optional_text, required_text, validate_fraction, MAX_PROGRESS_MESSAGE_BYTES, MAX_TITLE_BYTES,
};

pub(super) struct ProgressSession {
    id: u64,
    plugin: String,
    title: String,
    message: Option<String>,
    fraction: Option<f32>,
    canceled: bool,
}

impl ProgressSession {
    pub(super) fn start(
        id: u64,
        plugin: &str,
        params: UiProgressStartParams,
    ) -> Result<Self, RpcError> {
        let title = required_text(&params.title, "title", MAX_TITLE_BYTES)?;
        let message = optional_text(
            params.message.as_deref(),
            "message",
            MAX_PROGRESS_MESSAGE_BYTES,
        )?;
        validate_fraction(params.fraction)?;
        Ok(Self {
            id,
            plugin: plugin.to_string(),
            title,
            message,
            fraction: params.fraction,
            canceled: false,
        })
    }

    pub(super) fn update(&mut self, params: UiProgressUpdateParams) -> Result<bool, RpcError> {
        self.ensure_id(params.id)?;
        validate_fraction(params.fraction)?;
        if let Some(message) = params.message.as_deref() {
            self.message = optional_text(Some(message), "message", MAX_PROGRESS_MESSAGE_BYTES)?;
        }
        if params.fraction.is_some() {
            self.fraction = params.fraction;
        }
        Ok(self.canceled)
    }

    pub(super) fn finish(&mut self, id: u64) -> Result<(), RpcError> {
        self.ensure_id(id)
    }

    pub(super) fn cancel(&mut self) -> bool {
        if self.canceled {
            return false;
        }
        self.canceled = true;
        true
    }

    pub(super) fn text(&self) -> Option<String> {
        if self.canceled {
            return None;
        }
        let mut text = self.title.clone();
        if let Some(message) = self
            .message
            .as_deref()
            .filter(|message| !message.is_empty())
        {
            text.push_str(": ");
            text.push_str(message);
        }
        if let Some(fraction) = self.fraction {
            text.push_str(&format!(" {}%", (fraction * 100.0).round() as u32));
        }
        text.push_str(" [plugin.cancel]");
        Some(text)
    }

    pub(super) fn is_canceled(&self) -> bool {
        self.canceled
    }

    pub(super) fn plugin(&self) -> &str {
        &self.plugin
    }

    pub(super) fn id(&self) -> u64 {
        self.id
    }

    fn ensure_id(&self, id: u64) -> Result<(), RpcError> {
        if self.id == id {
            Ok(())
        } else {
            Err(RpcError::invalid_params("unknown plugin progress id"))
        }
    }
}
