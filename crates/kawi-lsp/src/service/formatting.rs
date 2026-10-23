use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use ropey::Rope;

impl Manager {
    pub(super) fn formatting_event(
        &self,
        pending: &PendingTextRequest,
        responses: &[TextResponse],
        text: &Rope,
    ) -> Result<FormattingEvent, String> {
        let Some(response) = responses.first() else {
            return Err("No formatting changes".to_string());
        };
        if response.value.is_null() {
            return Err("No formatting changes".to_string());
        }
        let edits = serde_json::from_value::<Vec<lsp_types::TextEdit>>(response.value.clone())
            .map_err(|error| format!("Invalid formatting response: {error}"))?;
        if edits.is_empty() {
            return Err("No formatting changes".to_string());
        }
        Ok(FormattingEvent {
            request: pending.request,
            edit: self.text_edits_workspace(
                &pending.path,
                text.clone(),
                &edits,
                &response.encoding,
            )?,
        })
    }
}
