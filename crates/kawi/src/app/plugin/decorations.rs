use std::collections::BTreeMap;

use kawi_editor::Editor;
use kawi_plugin::{
    decoration_ref, Decoration as ProtocolDecoration, DecorationRef, DecorationScope, RpcError,
};
use kawi_tui::frame::{DecorationSpan, PluginSign, Sign, VirtualText};
use ropey::Rope;

use super::style::PluginStyles;

mod items;

use items::{Highlight, PluginVirtualText, SignMark};

#[derive(Default)]
pub(crate) struct PluginDecorations {
    sets: BTreeMap<(String, usize), DecorationSet>,
}

#[derive(Default)]
struct DecorationSet {
    highlights: Vec<Highlight>,
    virtual_texts: Vec<PluginVirtualText>,
    signs: Vec<SignMark>,
}

impl PluginDecorations {
    pub(crate) fn set(
        &mut self,
        plugin: &str,
        buffer: usize,
        scope: DecorationScope,
        decorations: &[ProtocolDecoration],
        snapshot: &Rope,
        styles: &PluginStyles,
    ) -> Result<(), RpcError> {
        let incoming = DecorationSet::new(decorations, snapshot, plugin, styles)?;
        let key = (plugin.to_string(), buffer);
        let set = match scope {
            DecorationScope::All => incoming,
            DecorationScope::Visual => {
                let mut set = self.sets.remove(&key).unwrap_or_default();
                set.highlights = incoming.highlights;
                set.virtual_texts = incoming.virtual_texts;
                set
            }
            DecorationScope::Signs => {
                let mut set = self.sets.remove(&key).unwrap_or_default();
                set.signs = incoming.signs;
                set
            }
        };
        self.sets.insert(key, set);
        Ok(())
    }

    pub(crate) fn retheme(&mut self, styles: &PluginStyles) {
        for ((plugin, _), set) in &mut self.sets {
            set.retheme(plugin, styles);
        }
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) {
        self.sets.retain(|(name, _), _| name != plugin);
    }

    pub(crate) fn retain_live(&mut self, editor: &Editor) {
        self.sets
            .retain(|(_, buffer), _| editor.buffer_revision_at(*buffer).is_some());
    }

    pub(crate) fn has_signs(&self, buffer: usize) -> bool {
        self.sets
            .iter()
            .any(|((_, number), set)| *number == buffer && !set.signs.is_empty())
    }

    pub(crate) fn signs(&self, buffer: usize, rows: &[usize]) -> Vec<Option<Sign>> {
        let mut signs = vec![None; rows.len()];
        for ((_, number), set) in &self.sets {
            if *number != buffer {
                continue;
            }
            for mark in &set.signs {
                let Some(index) = rows.iter().position(|row| *row == mark.line) else {
                    continue;
                };
                if signs[index].is_none() {
                    signs[index] = Some(Sign::Plugin(PluginSign {
                        text: mark.text.clone(),
                        style: mark.style,
                    }));
                }
            }
        }
        signs
    }

    pub(crate) fn spans(&self, buffer: usize, rows: &[usize]) -> Vec<DecorationSpan> {
        let mut spans = Vec::new();
        for ((_, number), set) in &self.sets {
            if *number != buffer {
                continue;
            }
            for highlight in &set.highlights {
                highlight.push_spans(rows, &mut spans);
            }
        }
        spans
    }

    pub(crate) fn virtual_texts(&self, buffer: usize, rows: &[usize]) -> Vec<Vec<VirtualText>> {
        let mut texts = std::iter::repeat_with(Vec::new)
            .take(rows.len())
            .collect::<Vec<Vec<VirtualText>>>();
        for ((_, number), set) in &self.sets {
            if *number != buffer {
                continue;
            }
            for text in &set.virtual_texts {
                let Some(index) = rows.iter().position(|row| *row == text.line) else {
                    continue;
                };
                texts[index].push(VirtualText::plugin(text.text.clone(), text.style));
            }
        }
        texts
    }
}

impl DecorationSet {
    fn new(
        decorations: &[ProtocolDecoration],
        snapshot: &Rope,
        plugin: &str,
        styles: &PluginStyles,
    ) -> Result<Self, RpcError> {
        let mut set = Self::default();
        for decoration in decorations {
            let appearance = decoration_ref(decoration)?;
            let style = resolve_style(plugin, &appearance, styles)?;
            set.push(decoration, appearance, style, snapshot)?;
        }
        Ok(set)
    }

    fn push(
        &mut self,
        decoration: &ProtocolDecoration,
        appearance: DecorationRef,
        style: kawi_tui::frame::PluginStyle,
        snapshot: &Rope,
    ) -> Result<(), RpcError> {
        match decoration {
            ProtocolDecoration::Highlight { start, end, .. } => {
                if let Some(highlight) = Highlight::new(start, end, appearance, style, snapshot)? {
                    self.highlights.push(highlight);
                }
            }
            ProtocolDecoration::VirtualText { line, text, .. } => {
                if let Some(text) =
                    PluginVirtualText::new(*line, text, appearance, style, snapshot)?
                {
                    self.virtual_texts.push(text);
                }
            }
            ProtocolDecoration::Sign { line, text, .. } => {
                if let Some(sign) = SignMark::new(*line, text, appearance, style, snapshot)? {
                    self.signs.push(sign);
                }
            }
        }
        Ok(())
    }

    fn retheme(&mut self, plugin: &str, styles: &PluginStyles) {
        for highlight in &mut self.highlights {
            highlight.retheme(plugin, styles);
        }
        for text in &mut self.virtual_texts {
            text.retheme(plugin, styles);
        }
        for sign in &mut self.signs {
            sign.retheme(plugin, styles);
        }
    }
}

fn resolve_style(
    plugin: &str,
    appearance: &DecorationRef,
    styles: &PluginStyles,
) -> Result<kawi_tui::frame::PluginStyle, RpcError> {
    styles
        .resolve(plugin, appearance)
        .ok_or_else(|| RpcError::invalid_params("unknown decoration group"))
}

#[cfg(test)]
mod tests;
