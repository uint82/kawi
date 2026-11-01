mod coalesce;
mod commands;
mod composition;
mod config;
mod decorations;
mod diagnostics;
mod events;
mod host_methods;
mod install;
mod invoke;
mod load;
mod notify;
mod queue;
mod reload;
mod runtime;
mod settings;
mod settings_ui;
mod statusline;
mod style;
mod text;
mod ui;
mod visual;

pub(super) use self::commands::SUBCOMMANDS;
pub(super) use self::decorations::PluginDecorations;
pub(crate) use self::load::PluginLoadContext;
pub(super) use self::runtime::{PluginCommandTarget, PluginRuntime};
pub(super) use self::settings_ui::PluginSettingsUi;
pub(super) use self::statusline::PluginStatusline;
pub(super) use self::style::PluginStyles;
pub(super) use self::ui::PluginUi;

#[cfg(test)]
mod tests;
