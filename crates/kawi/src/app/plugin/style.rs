use kawi_plugin::DecorationStyle as PluginStyle;
use kawi_tui::frame::DecorationStyle;

pub(super) fn frame_style(style: PluginStyle) -> DecorationStyle {
    match style {
        PluginStyle::Accent => DecorationStyle::Accent,
        PluginStyle::Info => DecorationStyle::Info,
        PluginStyle::Success => DecorationStyle::Success,
        PluginStyle::Warning => DecorationStyle::Warning,
        PluginStyle::Error => DecorationStyle::Error,
        PluginStyle::Muted => DecorationStyle::Muted,
    }
}
