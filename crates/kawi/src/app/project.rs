use std::path::{Path, PathBuf};

use super::App;

impl App {
    pub(super) fn project_root(&self) -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_default();
        let cwd_root = kawi_picker::project_root(&cwd);
        if !self.editor.has_file() {
            return cwd_root;
        }
        let file_root = kawi_picker::project_root(Path::new(self.editor.filename()));
        if file_root.join(".git").exists() {
            file_root
        } else {
            cwd_root
        }
    }
}
