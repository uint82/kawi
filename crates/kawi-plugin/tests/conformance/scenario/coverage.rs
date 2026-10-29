use kawi_plugin::{EVENT_NAMES, HOST_METHODS};

use super::scenarios;

pub(crate) struct Uncovered {
    pub(crate) name: &'static str,
    pub(crate) reason: &'static str,
}

pub(crate) const UNCOVERED_METHODS: &[Uncovered] = &[];
pub(crate) const UNCOVERED_EVENTS: &[Uncovered] = &[];

pub(crate) fn covered_methods() -> Vec<&'static str> {
    scenarios()
        .iter()
        .flat_map(|scenario| scenario.covers_methods.iter().copied())
        .collect()
}

pub(crate) fn covered_events() -> Vec<&'static str> {
    scenarios()
        .iter()
        .flat_map(|scenario| scenario.covers_events.iter().copied())
        .collect()
}

pub(crate) fn all_methods() -> &'static [&'static str] {
    &HOST_METHODS
}

pub(crate) fn all_events() -> &'static [&'static str] {
    &EVENT_NAMES
}
