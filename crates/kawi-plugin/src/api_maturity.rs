const LEGACY_MINOR: u64 = 0;
const BASE_WASM_MINOR: u64 = 15;
// Experimental APIs landed in protocol minor 19. Pinning the number keeps the report honest
// when PROTOCOL_MINOR moves on; otherwise every experimental method would quietly claim it
// was born in the newest release.
const EXPERIMENTAL_API_MINOR: u64 = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStability {
    Stable,
    Experimental,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaturityError {
    DeprecatedWithoutVersion,
    DeprecatedWithoutReplacement,
    DeprecatedBeforeIntroduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiMaturity {
    pub since: u64,
    pub stability: ApiStability,
    pub deprecated_since: Option<u64>,
    pub replacement: Option<&'static str>,
}

impl ApiMaturity {
    pub const fn stable(since: u64) -> Self {
        Self {
            since,
            stability: ApiStability::Stable,
            deprecated_since: None,
            replacement: None,
        }
    }

    pub const fn experimental(since: u64) -> Self {
        Self {
            since,
            stability: ApiStability::Experimental,
            deprecated_since: None,
            replacement: None,
        }
    }

    pub const fn deprecated(since: u64, deprecated_since: u64, replacement: &'static str) -> Self {
        Self {
            since,
            stability: ApiStability::Deprecated,
            deprecated_since: Some(deprecated_since),
            replacement: Some(replacement),
        }
    }

    pub const fn is_stable(self) -> bool {
        matches!(self.stability, ApiStability::Stable)
    }

    pub const fn is_experimental(self) -> bool {
        matches!(self.stability, ApiStability::Experimental)
    }

    pub const fn is_deprecated(self) -> bool {
        matches!(self.stability, ApiStability::Deprecated)
    }

    pub const fn validate(self) -> Result<(), MaturityError> {
        if !self.is_deprecated() {
            return Ok(());
        }
        let Some(deprecated_since) = self.deprecated_since else {
            return Err(MaturityError::DeprecatedWithoutVersion);
        };
        if self.replacement.is_none() {
            return Err(MaturityError::DeprecatedWithoutReplacement);
        }
        if deprecated_since < self.since {
            return Err(MaturityError::DeprecatedBeforeIntroduction);
        }
        Ok(())
    }
}

macro_rules! registry {
    ($entries:ident, $names:ident; $(($name:literal, $maturity:expr)),+ $(,)?) => {
        pub(crate) const $entries: &'static [(&'static str, ApiMaturity)] = &[
            $(($name, $maturity),)+
        ];
        pub(crate) const $names: &'static [&'static str] = &[$($name,)+];
    };
}

registry!(
    HOST_METHOD_ENTRIES, HOST_METHOD_NAMES;
    // Binary search needs these sorted. A test keeps honest anyone who edits them.
    ("autocmd.clear", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("autocmd.list", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("autocmd.register", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("autocmd.remove", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("buffer.apply_edit", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("buffer.close", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("buffer.create", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("buffer.get", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("buffer.list", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("buffer.open", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("command.list", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("command.register", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("command.unregister", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("config.get", ApiMaturity::stable(LEGACY_MINOR)),
    ("config.list", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.apply_edit", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.emit_user_event", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("editor.execute", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.get_state", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.list_commands", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.set_cursor", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("editor.set_decorations", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.set_diagnostics", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.set_mode", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("editor.set_selection", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("keymap.list", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("keymap.remove", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("keymap.set", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("options.get", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("options.list", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("options.reset", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("options.set", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("process.close_stdin", ApiMaturity::stable(LEGACY_MINOR)),
    ("process.kill", ApiMaturity::stable(LEGACY_MINOR)),
    ("process.open", ApiMaturity::stable(LEGACY_MINOR)),
    ("process.poll", ApiMaturity::stable(LEGACY_MINOR)),
    ("process.signal", ApiMaturity::stable(LEGACY_MINOR)),
    ("process.status", ApiMaturity::stable(LEGACY_MINOR)),
    ("process.write", ApiMaturity::stable(LEGACY_MINOR)),
    ("task.cancel", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("task.list", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("task.poll", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("task.start", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("task.status", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("timer.after", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("timer.cancel", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("timer.every", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("timer.list", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("ui.confirm", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.input", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.message", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.overlay_close", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("ui.overlay_open", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("ui.overlay_update", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("ui.overlay_wait", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("ui.panel_close", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.panel_focus", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.panel_open", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.panel_update", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.picker", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("ui.progress_finish", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.progress_start", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.progress_update", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.select", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.set_statusline", ApiMaturity::stable(LEGACY_MINOR)),
    ("variable.delete", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("variable.get", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("variable.list", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("variable.set", ApiMaturity::experimental(EXPERIMENTAL_API_MINOR)),
    ("window.close", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window.focus", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window.get", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window.list", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window.open", ApiMaturity::stable(BASE_WASM_MINOR)),
);

registry!(
    EVENT_ENTRIES, EVENT_NAMES;
    ("buffer_changed", ApiMaturity::stable(LEGACY_MINOR)),
    ("buffer_closed", ApiMaturity::stable(LEGACY_MINOR)),
    ("buffer_enter", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("buffer_leave", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("buffer_opened", ApiMaturity::stable(LEGACY_MINOR)),
    ("cursor_moved", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("diagnostics_changed", ApiMaturity::stable(LEGACY_MINOR)),
    ("file_opened", ApiMaturity::stable(LEGACY_MINOR)),
    ("file_saved", ApiMaturity::stable(LEGACY_MINOR)),
    ("focus_gained", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("focus_lost", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("language_changed", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("mode_changed", ApiMaturity::stable(LEGACY_MINOR)),
    ("selection_changed", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("user", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window_closed", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window_created", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window_enter", ApiMaturity::stable(BASE_WASM_MINOR)),
    ("window_leave", ApiMaturity::stable(BASE_WASM_MINOR)),
);

fn lookup(name: &str, entries: &'static [(&'static str, ApiMaturity)]) -> Option<ApiMaturity> {
    let index = entries.binary_search_by_key(&name, |(key, _)| *key).ok()?;
    Some(entries[index].1)
}

pub fn host_method_maturity(method: &str) -> Option<ApiMaturity> {
    lookup(method, HOST_METHOD_ENTRIES)
}

pub fn event_maturity(event: &str) -> Option<ApiMaturity> {
    lookup(event, EVENT_ENTRIES)
}

pub fn host_method_since(method: &str) -> u64 {
    host_method_maturity(method).map_or(LEGACY_MINOR, |maturity| maturity.since)
}

pub fn event_since(event: &str) -> u64 {
    event_maturity(event).map_or(LEGACY_MINOR, |maturity| maturity.since)
}

#[cfg(test)]
mod tests;
