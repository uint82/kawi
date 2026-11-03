use crate::state::StatePaths;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub(super) struct TestDir {
    pub(super) path: PathBuf,
}

impl TestDir {
    pub(super) fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "kawi_doctor_checks_{}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(super) fn paths(root: &Path) -> StatePaths {
    StatePaths {
        config: root.join("config.toml"),
        config_log: root.join("state/config.log"),
        keymaps: root.join("keymaps.toml"),
        plugins: root.join("plugins"),
        installed_plugins: root.join("installed-plugins"),
        plugin_lock: root.join("plugins.lock"),
        plugin_settings: root.join("plugins.toml"),
        languages: root.join("languages.toml"),
        debug: root.join("debug.toml"),
        tasks: root.join("tasks.toml"),
        themes: root.join("themes"),
        queries: root.join("queries"),
        history: root.join("state/history"),
        sessions: root.join("state/sessions"),
        views: root.join("state/views"),
        recovery: root.join("state/recovery"),
        undo: root.join("state/undo"),
    }
}

pub(super) fn executable(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
