use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostEnvironment {
    pub workspace: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

impl HostEnvironment {
    pub fn from_paths(workspace: Option<PathBuf>, config: Option<PathBuf>) -> Self {
        Self {
            workspace,
            config,
            home: std::env::var_os("HOME").map(PathBuf::from),
        }
    }

    pub fn resolve_path(&self, raw: &str, plugin_dir: &Path) -> Option<PathBuf> {
        expand_path(raw, plugin_dir, self)
    }
}

pub(crate) const WORKSPACE_VAR: &str = "WORKSPACE";
pub(crate) const PLUGIN_DIR_VAR: &str = "PLUGIN_DIR";
pub(crate) const CONFIG_VAR: &str = "CONFIG";
pub(crate) const HOME_VAR: &str = "HOME";

pub(crate) fn is_path_variable(name: &str) -> bool {
    matches!(name, WORKSPACE_VAR | PLUGIN_DIR_VAR | CONFIG_VAR | HOME_VAR)
}

pub(crate) fn expand_path(
    raw: &str,
    plugin_dir: &Path,
    environment: &HostEnvironment,
) -> Option<PathBuf> {
    let Some(rest) = raw.strip_prefix('$') else {
        return Some(PathBuf::from(raw));
    };
    let (name, rest) = rest.split_once('/').unwrap_or((rest, ""));
    let root = match name {
        WORKSPACE_VAR => environment.workspace.as_deref()?,
        PLUGIN_DIR_VAR => plugin_dir,
        CONFIG_VAR => environment.config.as_deref()?,
        HOME_VAR => environment.home.as_deref()?,
        _ => return None,
    };
    Some(root.join(rest.trim_start_matches('/')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_plugin_and_environment_paths() {
        let environment = HostEnvironment {
            workspace: Some(PathBuf::from("/work")),
            config: Some(PathBuf::from("/config")),
            home: Some(PathBuf::from("/home/user")),
        };
        let plugin_dir = Path::new("/plugins/sample");
        assert_eq!(
            expand_path("$WORKSPACE/src", plugin_dir, &environment),
            Some(PathBuf::from("/work/src"))
        );
        assert_eq!(
            expand_path("$PLUGIN_DIR/data", plugin_dir, &environment),
            Some(PathBuf::from("/plugins/sample/data"))
        );
        assert_eq!(
            expand_path("$CONFIG/kawi", plugin_dir, &environment),
            Some(PathBuf::from("/config/kawi"))
        );
        assert_eq!(
            expand_path("$HOME/.cache", plugin_dir, &environment),
            Some(PathBuf::from("/home/user/.cache"))
        );
    }

    #[test]
    fn leaves_plain_paths_and_reports_missing_variables() {
        let environment = HostEnvironment::default();
        let plugin_dir = Path::new("/plugins/sample");
        assert_eq!(
            expand_path("/tmp/example", plugin_dir, &environment),
            Some(PathBuf::from("/tmp/example"))
        );
        assert_eq!(
            expand_path("$WORKSPACE/src", plugin_dir, &environment),
            None
        );
        assert_eq!(
            expand_path("$WORKSPACE_SUFFIX/src", plugin_dir, &environment),
            None
        );
    }
}
