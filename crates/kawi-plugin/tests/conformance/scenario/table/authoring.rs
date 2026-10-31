use kawi_plugin::Capability;

use super::super::{Action, Behavior, Expected, Group, Scenario};

const TRIM_GRANTS: &[Capability] = &[
    Capability::EditorRead,
    Capability::EditorWrite,
    Capability::UiMessage,
];

pub(crate) const ROWS: &[Scenario] = &[
    Scenario {
        id: "authoring.trim",
        group: Group::Authoring,
        behavior: Behavior::Trim,
        grants: TRIM_GRANTS,
        action: Action::Trim,
        expected: Expected::Success,
        covers_methods: &["editor.get_state", "editor.apply_edit"],
        covers_events: &[],
    },
    Scenario {
        id: "authoring.trim_stale",
        group: Group::Authoring,
        behavior: Behavior::Trim,
        grants: TRIM_GRANTS,
        action: Action::TrimStale,
        expected: Expected::Success,
        covers_methods: &["editor.get_state", "editor.apply_edit"],
        covers_events: &[],
    },
    Scenario {
        id: "authoring.trim_stale_exhausted",
        group: Group::Authoring,
        behavior: Behavior::Trim,
        grants: TRIM_GRANTS,
        action: Action::TrimStaleExhausted,
        expected: Expected::Protocol,
        covers_methods: &["editor.get_state", "editor.apply_edit"],
        covers_events: &[],
    },
    Scenario {
        id: "authoring.trim_buffer_changed",
        group: Group::Authoring,
        behavior: Behavior::Trim,
        grants: TRIM_GRANTS,
        action: Action::TrimBufferChanged,
        expected: Expected::Protocol,
        covers_methods: &["editor.get_state", "editor.apply_edit"],
        covers_events: &[],
    },
    Scenario {
        id: "authoring.trim_denied",
        group: Group::Authoring,
        behavior: Behavior::Trim,
        grants: TRIM_GRANTS,
        action: Action::TrimDenied,
        expected: Expected::Denied("editor.write"),
        covers_methods: &["editor.get_state", "editor.apply_edit"],
        covers_events: &[],
    },
];
