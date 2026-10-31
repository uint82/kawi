mod config;
mod git;
mod keymaps;
mod languages;
mod paths;
mod plugin_keybindings;
mod plugins;
mod probe;
mod session;
mod state;
mod tasks;
#[cfg(test)]
mod test_support;

use super::report::Report;
use crate::state::StatePaths;
use std::path::Path;

pub(crate) fn collect(target: &Path, workspace: &Path, paths: Option<&StatePaths>) -> Report {
    let paths_check = paths::check(paths);
    let Some(paths) = paths else {
        return Report::new(target, workspace, vec![paths_check]);
    };
    let checks = vec![
        paths_check,
        config::check(paths),
        keymaps::check(paths),
        plugin_keybindings::check(paths),
        plugins::check(paths),
        languages::check(paths),
        tasks::check(paths, workspace),
        git::check(workspace),
        session::check(paths, super::working_directory(target)),
        state::check(paths),
    ];
    Report::new(target, workspace, checks)
}
