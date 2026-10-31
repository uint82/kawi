use super::super::report::{Check, Level};
use crate::command::CommandRegistry;
use crate::keymap::{config::expand_sequences, has_reserved_prefix, KeyChord};
use crate::state::StatePaths;
use kawi_plugin::load_catalog;

const RESERVED_PREFIX_WARNING: &str = "count and register prefixes are reserved";

pub(crate) fn check(paths: &StatePaths) -> Check {
    let installed = load_catalog(&paths.installed_plugins);
    let dev = load_catalog(&paths.plugins);
    let leaders = crate::keymap::config::load(&paths.keymaps, &CommandRegistry::new()).leader;
    let mut details = vec![
        format!("installed: {}", paths.installed_plugins.display()),
        format!("development: {}", paths.plugins.display()),
    ];
    let mut level = Level::Ok;
    for plugin in installed.plugins.iter().chain(dev.plugins.iter()) {
        for command in &plugin.manifest.commands {
            for sequences in command.keybindings.values() {
                for sequence in sequences {
                    if is_reserved(sequence, &leaders) {
                        details.push(format!(
                            "{}: {RESERVED_PREFIX_WARNING}: {sequence}",
                            plugin.path.display()
                        ));
                        level = Level::Warn;
                    }
                }
            }
        }
    }
    let check = Check::new("plugin-keybindings", "Plugin keybindings", level, details);
    if level == Level::Warn {
        check.with_hint("Fix the rejected plugin keybindings listed above.")
    } else {
        check
    }
}

fn is_reserved(sequence: &str, leaders: &[KeyChord]) -> bool {
    match expand_sequences(sequence, leaders) {
        Ok(expanded) => expanded.iter().any(has_reserved_prefix),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    fn write_plugin(paths: &StatePaths, keybindings: &str) {
        let plugin = paths.plugins.join("sample");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(
            plugin.join("plugin.toml"),
            format!(
                "name = \"sample\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\n\n[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\n\n[commands.keybindings]\nnormal = {keybindings}\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn accepts_plugin_bindings_without_reserved_prefixes() {
        let dir = TestDir::new("plugin_keybindings_valid");
        let paths = paths(&dir.path);
        write_plugin(&paths, "[\"gx\"]");
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
    }

    #[test]
    fn reports_count_and_register_prefixes() {
        let dir = TestDir::new("plugin_keybindings_reserved");
        let paths = paths(&dir.path);
        write_plugin(&paths, "[\"1x\", \"\\\"a\"]");
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("count and register prefixes are reserved: 1x")));
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("count and register prefixes are reserved: \"a")));
    }
}
