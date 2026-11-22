// `since` is the protocol minor that introduced the API, not the minor this registry was
// written. docs/help/plugin-protocol.md holds the history these constants encode.
pub(crate) const LEGACY_MINOR: u64 = 0;
pub(crate) const EVENTS_MINOR: u64 = 1;
pub(crate) const DIAGNOSTICS_MINOR: u64 = 2;
pub(crate) const COMMAND_COMPOSITION_MINOR: u64 = 3;
pub(crate) const SETTINGS_MINOR: u64 = 4;
pub(crate) const INTERACTIVE_UI_MINOR: u64 = 5;
pub(crate) const DECORATIONS_MINOR: u64 = 7;
pub(crate) const PROCESS_JOBS_MINOR: u64 = 10;
pub(crate) const PROCESS_POLL_MINOR: u64 = 11;
pub(crate) const PANELS_MINOR: u64 = 12;
pub(crate) const WASM_EDITOR_MINOR: u64 = 13;
pub(crate) const WASM_EXTENSIONS_MINOR: u64 = 14;
pub(crate) const WASM_AUTOCMDS_MINOR: u64 = 15;
pub(crate) const WASM_AUTOMATION_MINOR: u64 = 16;
pub(crate) const WASM_OVERLAYS_MINOR: u64 = 17;
pub(crate) const WASM_TASKS_MINOR: u64 = 18;
pub(crate) const WASM_SCRATCH_BUFFERS_MINOR: u64 = 19;

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
    ActiveWithDeprecationData,
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
            return self.validate_active();
        }
        self.validate_deprecated()
    }

    const fn validate_active(self) -> Result<(), MaturityError> {
        if self.deprecated_since.is_some() || self.replacement.is_some() {
            return Err(MaturityError::ActiveWithDeprecationData);
        }
        Ok(())
    }

    const fn validate_deprecated(self) -> Result<(), MaturityError> {
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

// One declaration feeds both the lookup table and the public name list.
// Drift between them is how a method ends up advertised but unreachable.
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
    ("autocmd.clear", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("autocmd.list", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("autocmd.register", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("autocmd.remove", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("buffer.apply_edit", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("buffer.close", ApiMaturity::experimental(WASM_SCRATCH_BUFFERS_MINOR)),
    ("buffer.create", ApiMaturity::experimental(WASM_SCRATCH_BUFFERS_MINOR)),
    ("buffer.get", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("buffer.list", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("buffer.open", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("command.list", ApiMaturity::stable(WASM_EXTENSIONS_MINOR)),
    ("command.register", ApiMaturity::stable(WASM_EXTENSIONS_MINOR)),
    ("command.unregister", ApiMaturity::stable(WASM_EXTENSIONS_MINOR)),
    ("config.get", ApiMaturity::stable(SETTINGS_MINOR)),
    ("config.list", ApiMaturity::stable(SETTINGS_MINOR)),
    ("editor.apply_edit", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.emit_user_event", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("editor.execute", ApiMaturity::stable(COMMAND_COMPOSITION_MINOR)),
    ("editor.get_state", ApiMaturity::stable(LEGACY_MINOR)),
    ("editor.list_commands", ApiMaturity::stable(COMMAND_COMPOSITION_MINOR)),
    ("editor.set_cursor", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("editor.set_decorations", ApiMaturity::stable(DECORATIONS_MINOR)),
    ("editor.set_diagnostics", ApiMaturity::stable(DIAGNOSTICS_MINOR)),
    ("editor.set_mode", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("editor.set_selection", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("keymap.list", ApiMaturity::stable(WASM_EXTENSIONS_MINOR)),
    ("keymap.remove", ApiMaturity::stable(WASM_EXTENSIONS_MINOR)),
    ("keymap.set", ApiMaturity::stable(WASM_EXTENSIONS_MINOR)),
    ("options.get", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("options.list", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("options.reset", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("options.set", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("process.close_stdin", ApiMaturity::stable(PROCESS_JOBS_MINOR)),
    ("process.kill", ApiMaturity::stable(PROCESS_JOBS_MINOR)),
    ("process.open", ApiMaturity::stable(PROCESS_JOBS_MINOR)),
    ("process.poll", ApiMaturity::stable(PROCESS_POLL_MINOR)),
    ("process.signal", ApiMaturity::stable(PROCESS_JOBS_MINOR)),
    ("process.status", ApiMaturity::stable(PROCESS_JOBS_MINOR)),
    ("process.write", ApiMaturity::stable(PROCESS_JOBS_MINOR)),
    ("task.cancel", ApiMaturity::experimental(WASM_TASKS_MINOR)),
    ("task.list", ApiMaturity::experimental(WASM_TASKS_MINOR)),
    ("task.poll", ApiMaturity::experimental(WASM_TASKS_MINOR)),
    ("task.start", ApiMaturity::experimental(WASM_TASKS_MINOR)),
    ("task.status", ApiMaturity::experimental(WASM_TASKS_MINOR)),
    ("timer.after", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("timer.cancel", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("timer.every", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("timer.list", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("ui.confirm", ApiMaturity::stable(INTERACTIVE_UI_MINOR)),
    ("ui.input", ApiMaturity::stable(INTERACTIVE_UI_MINOR)),
    ("ui.message", ApiMaturity::stable(LEGACY_MINOR)),
    ("ui.overlay_close", ApiMaturity::experimental(WASM_OVERLAYS_MINOR)),
    ("ui.overlay_open", ApiMaturity::experimental(WASM_OVERLAYS_MINOR)),
    ("ui.overlay_update", ApiMaturity::experimental(WASM_OVERLAYS_MINOR)),
    ("ui.overlay_wait", ApiMaturity::experimental(WASM_OVERLAYS_MINOR)),
    ("ui.panel_close", ApiMaturity::stable(PANELS_MINOR)),
    ("ui.panel_focus", ApiMaturity::stable(PANELS_MINOR)),
    ("ui.panel_open", ApiMaturity::stable(PANELS_MINOR)),
    ("ui.panel_update", ApiMaturity::stable(PANELS_MINOR)),
    ("ui.picker", ApiMaturity::experimental(WASM_OVERLAYS_MINOR)),
    ("ui.progress_finish", ApiMaturity::stable(INTERACTIVE_UI_MINOR)),
    ("ui.progress_start", ApiMaturity::stable(INTERACTIVE_UI_MINOR)),
    ("ui.progress_update", ApiMaturity::stable(INTERACTIVE_UI_MINOR)),
    ("ui.select", ApiMaturity::stable(INTERACTIVE_UI_MINOR)),
    ("ui.set_statusline", ApiMaturity::stable(DECORATIONS_MINOR)),
    ("variable.delete", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("variable.get", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("variable.list", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("variable.set", ApiMaturity::experimental(WASM_AUTOMATION_MINOR)),
    ("window.close", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("window.focus", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("window.get", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("window.list", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("window.open", ApiMaturity::stable(WASM_EDITOR_MINOR)),
);

registry!(
    EVENT_ENTRIES, EVENT_NAMES;
    ("buffer_changed", ApiMaturity::stable(EVENTS_MINOR)),
    ("buffer_closed", ApiMaturity::stable(EVENTS_MINOR)),
    ("buffer_enter", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("buffer_leave", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("buffer_opened", ApiMaturity::stable(EVENTS_MINOR)),
    ("cursor_moved", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("diagnostics_changed", ApiMaturity::stable(EVENTS_MINOR)),
    ("file_opened", ApiMaturity::stable(EVENTS_MINOR)),
    ("file_saved", ApiMaturity::stable(EVENTS_MINOR)),
    ("focus_gained", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("focus_lost", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("language_changed", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("mode_changed", ApiMaturity::stable(EVENTS_MINOR)),
    ("selection_changed", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("user", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("window_closed", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("window_created", ApiMaturity::stable(WASM_AUTOCMDS_MINOR)),
    ("window_enter", ApiMaturity::stable(WASM_EDITOR_MINOR)),
    ("window_leave", ApiMaturity::stable(WASM_EDITOR_MINOR)),
);

fn lookup(name: &str, entries: &'static [(&'static str, ApiMaturity)]) -> Option<ApiMaturity> {
    let index = entries.binary_search_by_key(&name, |(key, _)| *key).ok()?;
    Some(entries[index].1)
}

fn introduction_minor(name: &str, entries: &'static [(&'static str, ApiMaturity)]) -> Option<u64> {
    lookup(name, entries).map(|maturity| maturity.since)
}

pub fn host_method_maturity(method: &str) -> Option<ApiMaturity> {
    lookup(method, HOST_METHOD_ENTRIES)
}

pub fn event_maturity(event: &str) -> Option<ApiMaturity> {
    lookup(event, EVENT_ENTRIES)
}

pub fn host_method_since(method: &str) -> Option<u64> {
    introduction_minor(method, HOST_METHOD_ENTRIES)
}

pub fn event_since(event: &str) -> Option<u64> {
    introduction_minor(event, EVENT_ENTRIES)
}

#[cfg(test)]
mod tests;
