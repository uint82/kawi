use kawi_plugin::Capability;

use super::super::{Action, Behavior, Expected, Group, Scenario};
use super::super::EDITOR_READ_UI;

pub(crate) const ROWS: &[Scenario] = &[
    Scenario {
        id: "oversize.invocation",
        group: Group::Oversize,
        behavior: Behavior::Echo,
        grants: &[],
        action: Action::OversizeInvocation,
        expected: Expected::Success,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "oversize.event",
        group: Group::Oversize,
        behavior: Behavior::EventBufferChanged,
        grants: EDITOR_READ_UI,
        action: Action::OversizeEvent,
        expected: Expected::Success,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "hygiene.invalid_position",
        group: Group::InputHygiene,
        behavior: Behavior::EditorWrite,
        grants: &[Capability::EditorWrite],
        action: Action::InvalidPosition,
        expected: Expected::Protocol,
        covers_methods: &["editor.apply_edit"],
        covers_events: &[],
    },
];
