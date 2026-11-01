use crossterm::event::KeyEvent;
use kawi_picker::Picker;
use kawi_plugin::{
    RpcError, UiConfirmParams, UiInputParams, UiProgressFinishParams, UiProgressStartParams,
    UiProgressUpdateParams, UiPromptOutcome, UiSelectParams,
};
use kawi_tui::frame::PromptMenu;
use serde_json::Value;

mod host;
mod progress;
mod prompt;
mod validation;

use progress::ProgressSession;
use prompt::PromptSession;

#[derive(Default)]
pub(crate) struct PluginUi {
    next_id: u64,
    prompt: Option<PromptSession>,
    progress: Option<ProgressSession>,
}

pub(crate) struct PromptCompletion {
    pub(crate) plugin: String,
    pub(crate) request: Value,
    pub(crate) outcome: UiPromptOutcome,
}

impl PluginUi {
    pub(crate) fn open_input(
        &mut self,
        plugin: &str,
        request: Value,
        params: UiInputParams,
    ) -> Result<(), RpcError> {
        self.ensure_prompt_available()?;
        self.prompt = Some(PromptSession::input(plugin, request, params)?);
        Ok(())
    }

    pub(crate) fn open_select(
        &mut self,
        plugin: &str,
        request: Value,
        params: UiSelectParams,
    ) -> Result<(), RpcError> {
        self.ensure_prompt_available()?;
        self.prompt = Some(PromptSession::select(plugin, request, params)?);
        Ok(())
    }

    pub(crate) fn open_confirm(
        &mut self,
        plugin: &str,
        request: Value,
        params: UiConfirmParams,
    ) -> Result<(), RpcError> {
        self.ensure_prompt_available()?;
        self.prompt = Some(PromptSession::confirm(plugin, request, params)?);
        Ok(())
    }

    pub(crate) fn start_progress(
        &mut self,
        plugin: &str,
        params: UiProgressStartParams,
    ) -> Result<u64, RpcError> {
        if self
            .progress
            .as_ref()
            .is_some_and(|progress| !progress.is_canceled())
        {
            return Err(RpcError::ui_busy("a plugin progress is already active"));
        }
        let id = self.allocate_id();
        self.progress = Some(ProgressSession::start(id, plugin, params)?);
        Ok(id)
    }

    pub(crate) fn update_progress(
        &mut self,
        params: UiProgressUpdateParams,
    ) -> Result<bool, RpcError> {
        self.active_progress_mut()?.update(params)
    }

    pub(crate) fn finish_progress(
        &mut self,
        params: UiProgressFinishParams,
    ) -> Result<(), RpcError> {
        self.active_progress_mut()?.finish(params.id)?;
        self.progress = None;
        Ok(())
    }

    pub(crate) fn cancel_progress(&mut self) -> Option<(String, u64)> {
        let progress = self.progress.as_mut()?;
        if !progress.cancel() {
            return None;
        }
        Some((progress.plugin().to_string(), progress.id()))
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<PromptCompletion> {
        let outcome = self.prompt.as_mut()?.handle_key(key)?;
        let session = self.prompt.take()?;
        Some(PromptCompletion {
            plugin: session.plugin,
            request: session.request,
            outcome,
        })
    }

    pub(crate) fn paste(&mut self, text: &str) -> bool {
        let Some(session) = self.prompt.as_mut() else {
            return false;
        };
        session.paste(text);
        true
    }

    pub(crate) fn has_prompt(&self) -> bool {
        self.prompt.is_some()
    }

    pub(crate) fn prompt(&self) -> Option<PromptMenu<'_>> {
        self.prompt.as_ref()?.menu()
    }

    pub(crate) fn picker(&self) -> Option<&Picker> {
        self.prompt.as_ref()?.picker()
    }

    pub(crate) fn progress_text(&self) -> Option<String> {
        self.progress.as_ref()?.text()
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) -> Option<(String, Value)> {
        let request = self
            .prompt
            .as_ref()
            .filter(|prompt| prompt.plugin == plugin)
            .map(|prompt| (prompt.plugin.clone(), prompt.request.clone()));
        if request.is_some() {
            self.prompt = None;
        }
        if self
            .progress
            .as_ref()
            .is_some_and(|progress| progress.plugin() == plugin)
        {
            self.progress = None;
        }
        request
    }

    fn ensure_prompt_available(&self) -> Result<(), RpcError> {
        if self.prompt.is_some() {
            return Err(RpcError::ui_busy("a plugin prompt is already active"));
        }
        Ok(())
    }

    fn active_progress_mut(&mut self) -> Result<&mut ProgressSession, RpcError> {
        self.progress
            .as_mut()
            .ok_or_else(|| RpcError::invalid_params("no plugin progress is active"))
    }

    fn allocate_id(&mut self) -> u64 {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.next_id
    }
}

#[cfg(test)]
mod tests;

impl crate::app::App {
    pub(crate) fn handle_plugin_ui_key(&mut self, key: KeyEvent) {
        let Some(completion) = self.plugin_ui.handle_key(key) else {
            return;
        };
        let value = serde_json::to_value(completion.outcome).unwrap_or(Value::Null);
        let _ = self
            .plugins
            .respond(&completion.plugin, completion.request, Ok(value));
    }

    pub(crate) fn paste_plugin_ui(&mut self, text: &str) -> bool {
        self.plugin_ui.paste(text)
    }

    pub(crate) fn cancel_plugin_progress(&mut self) -> anyhow::Result<bool> {
        if self.plugin_ui.cancel_progress().is_none() {
            self.show_message("No plugin progress to cancel".to_string());
        } else {
            self.show_message("Canceled plugin progress".to_string());
        }
        Ok(false)
    }
}
