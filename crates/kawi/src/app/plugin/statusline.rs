use std::collections::BTreeMap;

use kawi_plugin::{sanitize, StatuslineSegment};
use kawi_tui::statusbar::PluginSegment;

use super::style::frame_style;

#[derive(Default)]
pub(crate) struct PluginStatusline {
    sets: BTreeMap<String, Vec<StatuslineSegment>>,
}

impl PluginStatusline {
    pub(crate) fn set(&mut self, plugin: &str, segments: &[StatuslineSegment]) {
        let segments = segments
            .iter()
            .filter_map(|segment| {
                let text = sanitize(&segment.text);
                (!text.is_empty()).then_some(StatuslineSegment {
                    text,
                    style: segment.style,
                    priority: segment.priority,
                })
            })
            .collect();
        self.sets.insert(plugin.to_string(), segments);
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) {
        self.sets.remove(plugin);
    }

    pub(crate) fn segments(&self) -> Vec<PluginSegment> {
        let mut segments = self
            .sets
            .iter()
            .flat_map(|(plugin, segments)| {
                segments
                    .iter()
                    .map(move |segment| (plugin.as_str(), segment))
            })
            .collect::<Vec<_>>();
        segments.sort_by(|(left_plugin, left), (right_plugin, right)| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left_plugin.cmp(right_plugin))
        });
        segments
            .into_iter()
            .map(|(_, segment)| PluginSegment {
                text: segment.text.clone(),
                style: frame_style(segment.style),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
