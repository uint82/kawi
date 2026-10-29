use super::host::ConformanceHost;

impl ConformanceHost {
    pub(crate) fn set_stale_revision(&self, stale: bool) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stale_revision = stale;
    }

    pub(crate) fn set_oversize_response(&self, oversize: bool) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .oversize_response = oversize;
    }

    pub(crate) fn set_invalid_position(&self, invalid: bool) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .invalid_position = invalid;
    }

    pub(crate) fn withhold_host_responses(&self) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .withhold = true;
    }

    pub(crate) fn withhold_apply_edit(&self) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .withhold_method = Some("editor.apply_edit".to_string());
    }

    pub(crate) fn release_host_responses(&self) {
        let deferred = {
            let mut policy = self
                .policy
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            policy.withhold = false;
            policy.withhold_method = None;
            std::mem::take(&mut policy.deferred)
        };
        for response in deferred {
            let _ = self
                .host
                .respond(&response.plugin, response.id, response.result);
            self.mark_responded(response.index);
        }
    }

    fn mark_responded(&self, index: usize) {
        if let Some(request) = self
            .observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .host_requests
            .get_mut(index)
        {
            request.responded = true;
        }
    }
}
