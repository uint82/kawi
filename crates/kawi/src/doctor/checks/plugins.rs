use super::super::report::{Check, Level};
use crate::state::StatePaths;
use kawi_plugin::{load_catalog, load_managed_catalog, PluginLock};

pub(crate) fn check(paths: &StatePaths) -> Check {
    let mut details = path_details(paths);
    let mut level = Level::Ok;
    let lock = match PluginLock::load(&paths.plugin_lock) {
        Ok(lock) => Some(lock),
        Err(error) => {
            details.push(error);
            level = Level::Warn;
            None
        }
    };
    if let Some(lock) = &lock {
        level = level.max(lock_details(lock, paths, &mut details));
        level = level.max(installed_warnings(lock, paths, &mut details));
    }
    level = level.max(catalog_warnings(&paths.plugins, &mut details));
    level = level.max(development_details(paths, &mut details));
    let check = Check::new("plugins", "Plugins", level, details);
    if level == Level::Warn {
        check.with_hint("Fix the plugin installation problems listed above.")
    } else {
        check
    }
}

fn path_details(paths: &StatePaths) -> Vec<String> {
    vec![
        format!("installed: {}", paths.installed_plugins.display()),
        format!("development: {}", paths.plugins.display()),
        format!("lockfile: {}", paths.plugin_lock.display()),
    ]
}

fn lock_details(lock: &PluginLock, paths: &StatePaths, details: &mut Vec<String>) -> Level {
    let mut level = Level::Ok;
    for (name, plugin) in &lock.plugins {
        if !paths.installed_plugins.join(name).is_dir() {
            details.push(format!("{name}: installed directory is missing"));
            level = Level::Warn;
            continue;
        }
        details.push(format!(
            "{name}: {} at {} ({})",
            plugin.source,
            crate::plugin_manager::short_commit(&plugin.commit),
            if plugin.enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }
    level
}

fn installed_warnings(lock: &PluginLock, paths: &StatePaths, details: &mut Vec<String>) -> Level {
    let catalog = load_managed_catalog(&paths.installed_plugins, lock);
    warnings(catalog, details)
}

fn catalog_warnings(directory: &std::path::Path, details: &mut Vec<String>) -> Level {
    warnings(load_catalog(directory), details)
}

fn warnings(catalog: kawi_plugin::PluginCatalog, details: &mut Vec<String>) -> Level {
    let mut level = Level::Ok;
    for warning in catalog.warnings {
        details.push(warning.to_string());
        level = Level::Warn;
    }
    level
}

fn development_details(paths: &StatePaths, details: &mut Vec<String>) -> Level {
    let dev = load_catalog(&paths.plugins);
    if dev.plugins.is_empty() {
        return Level::Ok;
    }
    details.push(format!(
        "{} development plugin(s) loaded from {}",
        dev.plugins.len(),
        paths.plugins.display()
    ));
    Level::Info
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn missing_lockfile_is_ok() {
        let dir = TestDir::new("plugins_missing_lock");
        let check = check(&paths(&dir.path));
        assert_eq!(check.level, Level::Ok);
    }

    #[test]
    fn malformed_lockfile_warns() {
        let dir = TestDir::new("plugins_bad_lock");
        let paths = paths(&dir.path);
        fs::write(&paths.plugin_lock, "not toml").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
    }

    #[test]
    fn missing_installed_directory_warns() {
        let dir = TestDir::new("plugins_missing_dir");
        let paths = paths(&dir.path);
        fs::write(
            &paths.plugin_lock,
            "version = 1\n\n[plugins.echo]\nsource = \"owner/repo\"\ncommit = \"abc\"\nsha256 = \"sha256:def\"\napi = 1\nenabled = true\n",
        )
        .unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
    }
}
