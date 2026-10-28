use super::super::report::{Check, Level};
use crate::state::StatePaths;
use kawi_plugin::{load_catalog, load_permission_state};

pub(crate) fn check(paths: &StatePaths) -> Check {
    let catalog = load_catalog(&paths.plugins);
    let state = load_permission_state(&catalog, &paths.permissions);
    let mut details = Vec::new();
    let mut level = if state.file_error.is_some() {
        Level::Warn
    } else if paths.permissions.is_file() {
        details.push(format!("permissions: {}", paths.permissions.display()));
        Level::Ok
    } else {
        details.push(format!(
            "permissions: {} (missing, only default grants apply)",
            paths.permissions.display()
        ));
        Level::Info
    };
    details.extend(state.warnings.iter().cloned());
    for plugin in &catalog.plugins {
        let name = plugin.manifest.name.as_str();
        if let Some(effective) = state.grants.get(name) {
            details.push(format!(
                "{name}: granted {}, missing {}",
                describe(effective.granted.keys().copied()),
                describe(effective.missing.iter().copied())
            ));
        }
    }
    if !state.warnings.is_empty() {
        level = level.max(Level::Warn);
    }
    let check = Check::new("permissions", "Plugin permissions", level, details);
    if level == Level::Warn {
        check.with_hint("Review the plugin permission warnings and permissions.toml.")
    } else {
        check
    }
}

fn describe(capabilities: impl Iterator<Item = kawi_plugin::Capability>) -> String {
    let names = capabilities.map(describe_capability).collect::<Vec<_>>();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

fn describe_capability(capability: kawi_plugin::Capability) -> String {
    if capability == kawi_plugin::Capability::NetConnect {
        return format!("{} (advisory)", capability.name());
    }
    if capability.is_advisory() && kawi_process::sandbox_supported() {
        return format!("{} (enforced)", capability.name());
    }
    if capability.is_advisory() {
        return format!("{} (advisory)", capability.name());
    }
    capability.name().to_string()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn reports_missing_permissions_as_informational() {
        let dir = TestDir::new("permissions_missing");
        let check = check(&paths(&dir.path));
        assert_eq!(check.level, Level::Info);
    }

    #[test]
    fn reports_malformed_permissions() {
        let dir = TestDir::new("permissions_invalid");
        let paths = paths(&dir.path);
        fs::write(&paths.permissions, "not toml").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
    }

    #[test]
    fn reports_stale_request_hashes() {
        let dir = TestDir::new("permissions_hash");
        let paths = paths(&dir.path);
        let plugin = paths.plugins.join("sample");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(
            plugin.join("plugin.toml"),
            "name = \"sample\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\n\n[capabilities]\n\"editor.read\" = true\n",
        )
        .unwrap();
        fs::write(
            &paths.permissions,
            "[plugins.sample]\nrequest_hash = \"sha256:stale\"\n\n[plugins.sample.grant]\n\"editor.read\" = true\n",
        )
        .unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("request hash changed")));
    }

    #[test]
    fn accepts_a_valid_permissions_file() {
        let dir = TestDir::new("permissions_valid");
        let paths = paths(&dir.path);
        fs::write(&paths.permissions, "[plugins]\n").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
    }
}
