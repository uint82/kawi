use std::path::Path;

use super::App;

impl App {
    pub(crate) fn sync_language_indent(&mut self) {
        let indent = if self.editor.has_file() {
            self.lsp
                .language_indent(Some(Path::new(self.editor.filename())))
        } else {
            Default::default()
        };
        self.editor.set_language_indent(indent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use kawi_editor::BufferOptionDefaults;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_language_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app(path: &str) -> App {
        let mut buffer = Buffer::from_string("fn main() {}\n");
        buffer.path = Some(path.to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.editor.set_buffer_defaults(BufferOptionDefaults {
            shiftwidth: Some(6),
            expandtab: Some(false),
            autoindent: true,
        });
        app
    }

    fn load_languages(app: &mut App, dir: &Path, text: &str) {
        let path = dir.join("languages.toml");
        fs::write(&path, text).unwrap();
        app.lsp.load(&path, app.events.wake());
        app.sync_language_indent();
    }

    #[test]
    fn language_indent_precedes_editor_defaults() {
        let dir = temp_dir("override");
        let mut app = app("main.rs");
        load_languages(
            &mut app,
            &dir,
            "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
        );
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(app.editor.expandtab());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn language_indent_travels_with_each_buffer() {
        let dir = temp_dir("buffers");
        let mut app = app("main.rs");
        let first = app.editor.buffer_number();
        load_languages(
            &mut app,
            &dir,
            "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
        );
        app.editor.open_empty_buffer();
        app.sync_language_indent();
        assert_eq!(app.editor.shiftwidth(), 6);
        assert!(!app.editor.expandtab());
        assert!(app.editor.buffer_select(first));
        app.sync_language_indent();
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(app.editor.expandtab());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn runtime_indent_survives_language_sync() {
        let dir = temp_dir("runtime");
        let mut app = app("main.rs");
        load_languages(
            &mut app,
            &dir,
            "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
        );
        app.run_command(kawi_editor::parse("set sw=5")).unwrap();
        app.run_command(kawi_editor::parse("set noexpandtab"))
            .unwrap();
        app.sync_language_indent();
        assert_eq!(app.editor.shiftwidth(), 5);
        assert!(!app.editor.expandtab());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn buffers_without_a_language_use_editor_defaults() {
        let dir = temp_dir("fallback");
        let mut app = app("notes.txt");
        load_languages(
            &mut app,
            &dir,
            "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
        );
        assert_eq!(app.editor.shiftwidth(), 6);
        assert!(!app.editor.expandtab());
        let _ = fs::remove_dir_all(dir);
    }
}
