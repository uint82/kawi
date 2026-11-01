use std::collections::BTreeMap;

use kawi_plugin::{
    DecorationFill, DecorationRef, DecorationStyle as ProtocolStyle, PluginCatalog,
    PluginDecorationGroup,
};
use kawi_tui::frame::{DecorationStyle, PluginFill, PluginStyle};
use kawi_tui::theme::Theme;

use super::super::App;
use super::runtime::PluginRuntime;

#[derive(Default)]
pub(crate) struct PluginStyles {
    groups: BTreeMap<String, BTreeMap<String, PluginStyle>>,
    builtins: BTreeMap<ProtocolStyle, PluginStyle>,
}

impl PluginStyles {
    pub(crate) fn new(catalog: &PluginCatalog, theme: &Theme) -> Self {
        let builtins = ProtocolStyle::ALL
            .into_iter()
            .map(|style| (style, builtin_style(theme, style)))
            .collect();
        let mut groups = BTreeMap::new();
        for plugin in &catalog.plugins {
            let styles = plugin
                .manifest
                .decorations
                .iter()
                .map(|group| {
                    (
                        group.id.clone(),
                        group_style(theme, &plugin.manifest.name, group),
                    )
                })
                .collect();
            groups.insert(plugin.manifest.name.clone(), styles);
        }
        Self { groups, builtins }
    }

    pub(crate) fn resolve(&self, plugin: &str, appearance: &DecorationRef) -> Option<PluginStyle> {
        match appearance {
            DecorationRef::Group(group) => self.groups.get(plugin)?.get(group).copied(),
            DecorationRef::Style(style) => self.builtins.get(style).copied(),
        }
    }
}

impl App {
    pub(crate) fn rebuild_plugin_styles(&mut self) {
        self.plugin_styles = PluginStyles::new(self.plugins.catalog(), &self.theme);
        self.plugin_decorations.retheme(&self.plugin_styles);
    }
}

impl PluginRuntime {
    pub(crate) fn catalog(&self) -> &PluginCatalog {
        &self.catalog
    }
}

pub(super) fn frame_style(style: ProtocolStyle) -> DecorationStyle {
    match style {
        ProtocolStyle::Accent => DecorationStyle::Accent,
        ProtocolStyle::Info => DecorationStyle::Info,
        ProtocolStyle::Success => DecorationStyle::Success,
        ProtocolStyle::Warning => DecorationStyle::Warning,
        ProtocolStyle::Error => DecorationStyle::Error,
        ProtocolStyle::Muted => DecorationStyle::Muted,
    }
}

fn frame_fill(fill: DecorationFill) -> PluginFill {
    match fill {
        DecorationFill::Both => PluginFill::Both,
        DecorationFill::Foreground => PluginFill::Foreground,
        DecorationFill::Background => PluginFill::Background,
    }
}

fn builtin_style(theme: &Theme, style: ProtocolStyle) -> PluginStyle {
    let colors = theme.decoration_colors(frame_style(style));
    PluginStyle {
        foreground: colors.foreground,
        background: colors.background,
        accent: colors.background,
        bold: false,
        italic: false,
        underline: false,
    }
}

fn group_style(theme: &Theme, plugin: &str, group: &PluginDecorationGroup) -> PluginStyle {
    let colors = theme.decoration_colors(frame_style(group.color));
    let overrides = theme.plugin_decorations.get(plugin, &group.id);
    let fill = overrides
        .and_then(|value| value.fill)
        .unwrap_or_else(|| frame_fill(group.fill));
    let accent = match fill {
        PluginFill::Foreground => overrides
            .and_then(|value| value.foreground)
            .unwrap_or(colors.background),
        _ => overrides
            .and_then(|value| value.background)
            .unwrap_or(colors.background),
    };
    PluginStyle {
        foreground: overrides
            .and_then(|value| value.foreground)
            .unwrap_or(match fill {
                PluginFill::Both => colors.foreground,
                PluginFill::Foreground => accent,
                PluginFill::Background => theme.base.foreground,
            }),
        background: overrides
            .and_then(|value| value.background)
            .unwrap_or(match fill {
                PluginFill::Foreground => theme.base.background,
                PluginFill::Both | PluginFill::Background => accent,
            }),
        accent,
        bold: overrides.and_then(|value| value.bold).unwrap_or(group.bold),
        italic: overrides
            .and_then(|value| value.italic)
            .unwrap_or(group.italic),
        underline: overrides
            .and_then(|value| value.underline)
            .unwrap_or(group.underline),
    }
}
