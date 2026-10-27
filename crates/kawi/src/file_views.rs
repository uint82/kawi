use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::fold_view::FoldView;
use crate::persist::write_atomically;
use crate::workspace;

const FILE_VIEW_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub(crate) struct FileViewStore {
    root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileView {
    version: u32,
    folds: FoldView,
}

impl FileViewStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn load(&self, path: &Path) -> Option<FoldView> {
        let view_path = self.view_path(path)?;
        let text = fs::read_to_string(view_path).ok()?;
        let view: FileView = serde_json::from_str(&text).ok()?;
        if view.version != FILE_VIEW_VERSION {
            return None;
        }
        view.folds.validate().ok()?;
        Some(view.folds)
    }

    pub(crate) fn save(&self, path: &Path, folds: &FoldView) -> Result<()> {
        let Some(view_path) = self.view_path(path) else {
            return Ok(());
        };
        let view = FileView {
            version: FILE_VIEW_VERSION,
            folds: folds.clone(),
        };
        let text = serde_json::to_string_pretty(&view).context("serialize file view")?;
        write_atomically(&view_path, &format!("{text}\n"))
    }

    pub(crate) fn remove(&self, path: &Path) -> Result<()> {
        let Some(view_path) = self.view_path(path) else {
            return Ok(());
        };
        match fs::remove_file(view_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("remove file view"),
        }
    }

    fn view_path(&self, path: &Path) -> Option<PathBuf> {
        let path = fs::canonicalize(path).ok()?;
        Some(self.root.join(format!("{}.json", workspace::key(&path))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_view::{StoredFold, StoredFoldMethod};

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_file_views_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn view() -> FoldView {
        FoldView {
            method: StoredFoldMethod::Manual,
            content_hash: 7,
            manual: vec![StoredFold { start: 1, end: 3 }],
            closed: vec![1],
        }
    }

    #[test]
    fn views_round_trip_through_the_store() {
        let dir = temp_dir("roundtrip");
        let file = dir.join("alpha.txt");
        fs::write(&file, "alpha\n").unwrap();
        let store = FileViewStore::new(dir.join("views"));
        let folds = view();
        store.save(&file, &folds).unwrap();
        assert_eq!(store.load(&file), Some(folds));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_views_are_absent() {
        let dir = temp_dir("missing");
        let file = dir.join("alpha.txt");
        fs::write(&file, "alpha\n").unwrap();
        let store = FileViewStore::new(dir.join("views"));
        assert_eq!(store.load(&file), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_views_are_ignored() {
        let dir = temp_dir("corrupt");
        let file = dir.join("alpha.txt");
        fs::write(&file, "alpha\n").unwrap();
        let store = FileViewStore::new(dir.join("views"));
        store.save(&file, &view()).unwrap();
        let view_path = store.view_path(&file).unwrap();
        fs::write(view_path, "{").unwrap();
        assert_eq!(store.load(&file), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unsupported_versions_are_ignored() {
        let dir = temp_dir("version");
        let file = dir.join("alpha.txt");
        fs::write(&file, "alpha\n").unwrap();
        let store = FileViewStore::new(dir.join("views"));
        store.save(&file, &view()).unwrap();
        let view_path = store.view_path(&file).unwrap();
        let text = fs::read_to_string(&view_path).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["version"] = serde_json::json!(FILE_VIEW_VERSION + 1);
        fs::write(view_path, serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(store.load(&file), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn removing_a_view_deletes_the_file() {
        let dir = temp_dir("remove");
        let file = dir.join("alpha.txt");
        fs::write(&file, "alpha\n").unwrap();
        let store = FileViewStore::new(dir.join("views"));
        store.save(&file, &view()).unwrap();
        store.remove(&file).unwrap();
        assert_eq!(store.load(&file), None);
        fs::remove_dir_all(dir).unwrap();
    }
}
