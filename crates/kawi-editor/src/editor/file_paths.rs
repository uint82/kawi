use std::path::{Path, PathBuf};

use super::buffers::BufferError;
use super::Editor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathAdoption {
    pub number: usize,
    pub path: Option<String>,
    pub source_hash: Option<[u8; 32]>,
    pub mark_clean: bool,
}

impl Editor {
    pub fn adopt_buffer_path(&mut self, adoption: PathAdoption) -> bool {
        if adoption.number == self.buffers.current() {
            self.buffer
                .set_path(adoption.path.as_deref(), adoption.source_hash);
            if adoption.mark_clean {
                self.buffer.mark_clean();
            }
            return true;
        }
        let Some(entry) = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == adoption.number)
        else {
            return false;
        };
        entry
            .state
            .buffer
            .set_path(adoption.path.as_deref(), adoption.source_hash);
        if adoption.mark_clean {
            entry.state.buffer.mark_clean();
        }
        true
    }

    pub fn detach_buffer_path(&mut self, number: usize) -> bool {
        self.adopt_buffer_path(PathAdoption {
            number,
            path: None,
            source_hash: None,
            mark_clean: false,
        })
    }

    pub fn rewrite_buffer_paths(&mut self, old: &Path, new: &Path) -> Vec<PathAdoption> {
        let cwd = super::buffers::current_dir();
        let adoptions: Vec<PathAdoption> = self
            .all_buffer_numbers()
            .into_iter()
            .filter_map(|number| {
                let path = self.buffer_path(number)?;
                let absolute = super::super::paths::absolute(path, &cwd);
                let rewritten = rewrite_prefix(&absolute, old, new)?;
                Some(PathAdoption {
                    number,
                    path: Some(rewritten.to_string_lossy().into_owned()),
                    source_hash: self.buffer_source_hash(number),
                    mark_clean: false,
                })
            })
            .collect();
        for adoption in &adoptions {
            self.adopt_buffer_path(adoption.clone());
        }
        adoptions
    }

    pub fn close_buffer(&mut self, number: usize, force: bool) -> Result<(), BufferError> {
        if self.buffer_is_dirty(number).unwrap_or(true) && !force {
            return Err(BufferError::NoWrite(number));
        }
        if number == self.buffers.current() {
            let replacement = self
                .buffers
                .listed_numbers()
                .into_iter()
                .find(|candidate| *candidate != number);
            match replacement {
                Some(next) => self.switch_buffer(next),
                None => self.open_empty_buffer(),
            }
        }
        self.buffers.remove(number);
        Ok(())
    }
}

fn rewrite_prefix(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    if path == old {
        return Some(new.to_path_buf());
    }
    let rest = path.strip_prefix(old).ok()?;
    Some(new.join(rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_file_paths_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn editor(dir: &Path, name: &str) -> Editor {
        Editor::new(Buffer::from_file(&dir.join(name).to_string_lossy()).unwrap())
    }

    fn type_text(editor: &mut Editor, text: &str) {
        editor.apply(Action::EnterInsert);
        for character in text.chars() {
            editor.apply(Action::InsertChar(character));
        }
        editor.apply(Action::ExitInsert);
    }

    #[test]
    fn adoption_preserves_dirty_state() {
        let dir = temp_dir("adopt_dirty");
        fs::write(dir.join("a.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.join("b.txt"), "text\n").unwrap();
        let mut editor = editor(&dir, "a.rs");
        type_text(&mut editor, "x");
        let number = editor.buffer_number();
        let path = dir.join("b.txt");
        assert!(editor.adopt_buffer_path(PathAdoption {
            number,
            path: Some(path.to_string_lossy().into_owned()),
            source_hash: Some([7; 32]),
            mark_clean: false,
        }));
        assert!(editor.is_dirty());
        assert_eq!(editor.filename(), path.to_string_lossy());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rewrite_updates_current_and_parked_paths() {
        let dir = temp_dir("rewrite");
        fs::create_dir_all(dir.join("old/sub")).unwrap();
        fs::write(dir.join("old/a.txt"), "a\n").unwrap();
        fs::write(dir.join("old/sub/b.txt"), "b\n").unwrap();
        let mut editor = editor(&dir, "old/a.txt");
        editor
            .edit_file(Some(&dir.join("old/sub/b.txt").to_string_lossy()), false)
            .unwrap();
        let adoptions = editor.rewrite_buffer_paths(&dir.join("old"), &dir.join("new"));
        assert_eq!(adoptions.len(), 2);
        assert_eq!(
            editor.buffer_path(1),
            Some(dir.join("new/a.txt").to_string_lossy().as_ref())
        );
        assert_eq!(
            editor.buffer_path(2),
            Some(dir.join("new/sub/b.txt").to_string_lossy().as_ref())
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn close_buffer_removes_a_parked_buffer() {
        let dir = temp_dir("close");
        fs::write(dir.join("a.txt"), "a\n").unwrap();
        fs::write(dir.join("b.txt"), "b\n").unwrap();
        let mut editor = editor(&dir, "a.txt");
        editor
            .edit_file(Some(&dir.join("b.txt").to_string_lossy()), false)
            .unwrap();
        editor.close_buffer(1, true).unwrap();
        assert_eq!(editor.buffer_count(), 1);
        assert_eq!(editor.buffer_path(1), None);
        fs::remove_dir_all(dir).unwrap();
    }
}
