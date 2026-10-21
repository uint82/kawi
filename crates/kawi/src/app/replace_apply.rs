use std::path::{Path, PathBuf};

use kawi_picker::{content_hash, ReplacePreview};

use super::App;

impl App {
    pub(super) fn apply_replace_preview(
        &mut self,
        preview: &ReplacePreview,
        root: &Path,
    ) -> Result<(), String> {
        let paths = preview_paths(preview, root);
        self.check_replace_targets(preview, &paths)?;
        self.write_replace_files(preview, &paths)?;
        self.editor.reload_files(&paths);
        Ok(())
    }

    fn check_replace_targets(
        &self,
        preview: &ReplacePreview,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        for (file, path) in preview.files.iter().zip(paths) {
            let bytes = std::fs::read(path)
                .map_err(|error| format!("Cannot read {}: {error}", file.path))?;
            if content_hash(&bytes) != file.original_hash {
                return Err(format!("File changed on disk: {}", file.path));
            }
            if self.editor.file_is_modified(path) {
                return Err(format!("Save or revert modified buffer: {}", file.path));
            }
        }
        Ok(())
    }

    fn write_replace_files(
        &self,
        preview: &ReplacePreview,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        for (file, path) in preview.files.iter().zip(paths) {
            kawi_buffer::write::write(path, false, |out| out.write_all(file.content.as_bytes()))
                .map_err(|error| format!("Cannot write {}: {error}", file.path))?;
        }
        Ok(())
    }
}

fn preview_paths(preview: &ReplacePreview, root: &Path) -> Vec<PathBuf> {
    preview
        .files
        .iter()
        .map(|file| root.join(&file.path))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use kawi_buffer::Buffer;
    use kawi_editor::Action;
    use kawi_picker::replace_preview;

    use super::super::App;
    use super::*;
    use crate::keymap::Flavor;

    fn project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_replace_apply_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("project");
        root
    }

    fn app_for(root: &Path, name: &str) -> App {
        App::new(
            Buffer::from_file(root.join(name).to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        )
    }

    #[test]
    fn applies_preview_and_reloads_the_open_buffer() {
        let root = project("apply");
        fs::write(root.join("a.txt"), "foo\n").expect("file");
        let preview = replace_preview(&root, "foo", "bar", 10).expect("preview");
        let mut app = app_for(&root, "a.txt");
        app.apply_replace_preview(&preview, &root).expect("apply");
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).expect("disk"),
            "bar\n"
        );
        assert_eq!(app.editor.visible_lines(0, 1), vec!["bar"]);
        assert!(!app.editor.is_dirty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn refuses_to_apply_over_a_modified_buffer() {
        let root = project("dirty");
        fs::write(root.join("a.txt"), "foo\n").expect("file");
        let preview = replace_preview(&root, "foo", "bar", 10).expect("preview");
        let mut app = app_for(&root, "a.txt");
        app.editor.apply(Action::EnterInsert);
        app.editor.apply(Action::InsertChar('x'));
        app.editor.apply(Action::ExitInsert);
        let error = app
            .apply_replace_preview(&preview, &root)
            .expect_err("dirty");
        assert!(error.contains("modified buffer"));
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).expect("disk"),
            "foo\n"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn refuses_to_apply_a_stale_preview() {
        let root = project("stale");
        fs::write(root.join("a.txt"), "foo\n").expect("file");
        let preview = replace_preview(&root, "foo", "bar", 10).expect("preview");
        fs::write(root.join("a.txt"), "changed\n").expect("changed");
        let mut app = app_for(&root, "a.txt");
        let error = app
            .apply_replace_preview(&preview, &root)
            .expect_err("stale");
        assert!(error.contains("changed on disk"));
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).expect("disk"),
            "changed\n"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
