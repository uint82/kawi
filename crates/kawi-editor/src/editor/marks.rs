use super::{buffers, EditError, Editor};
use crate::marks::{FileMark, Position};
use crate::paths;

enum MarkTarget {
    Local(Position),
    File(FileMark),
}

enum MarkSpan {
    Lines {
        first: usize,
        last: usize,
    },
    Chars {
        start: (usize, usize),
        end: (usize, usize),
    },
}

impl Editor {
    pub(super) fn set_mark(&mut self, name: char) {
        let position = Position {
            row: self.cursor.row(),
            col: self.cursor.col(),
        };
        if name.is_ascii_lowercase() {
            let buffer = self.buffers.current();
            self.marks.set_local(buffer, name, position);
            return;
        }
        let mark = FileMark {
            path: self.absolute_path(),
            buffer: self.buffers.current(),
            row: position.row,
            col: position.col,
        };
        self.marks.set_file(name, mark);
    }

    pub fn jump_to_mark(&mut self, name: char, line_wise: bool) -> Result<(), EditError> {
        let _ = self.take_count();
        let target = self.mark_target(name)?;
        self.set_pcmark();
        match target {
            MarkTarget::Local(position) => self.place_mark(position, line_wise),
            MarkTarget::File(mark) => {
                self.open_file_mark(&mark)?;
                let position = Position {
                    row: mark.row,
                    col: mark.col,
                };
                self.place_mark(position, line_wise);
            }
        }
        Ok(())
    }

    pub fn jump_to_previous(&mut self, line_wise: bool) -> Result<(), EditError> {
        let _ = self.take_count();
        self.jump_older()?;
        if line_wise {
            let row = self.cursor.row();
            let col = self.first_non_blank_of(row);
            self.cursor.set_position(row, col);
            self.remember_wanted_column();
            self.reveal_cursor();
        }
        Ok(())
    }

    pub fn delete_to_mark(&mut self, name: char, line_wise: bool) -> Result<(), EditError> {
        let _ = self.take_count();
        match self.mark_span(name, line_wise)? {
            MarkSpan::Lines { first, last } => self.remove_lines(first, last),
            MarkSpan::Chars { start, end } => self.remove_span(start, end),
        }
        self.reveal_cursor();
        Ok(())
    }

    pub fn change_to_mark(&mut self, name: char, line_wise: bool) -> Result<(), EditError> {
        let _ = self.take_count();
        match self.mark_span(name, line_wise)? {
            MarkSpan::Lines { first, last } => self.change_lines(first, last),
            MarkSpan::Chars { start, end } => {
                self.remove_span(start, end);
                self.switch_to_insert();
            }
        }
        self.reveal_cursor();
        Ok(())
    }

    pub fn yank_to_mark(&mut self, name: char, line_wise: bool) -> Result<(), EditError> {
        let _ = self.take_count();
        match self.mark_span(name, line_wise)? {
            MarkSpan::Lines { first, last } => self.yank_line_range(first, last),
            MarkSpan::Chars { start, end } => self.yank_span(start.0, start.1, end.0, end.1),
        }
        self.reveal_cursor();
        Ok(())
    }

    pub fn marks_listing(&self, width: usize) -> String {
        let local = self.marks.local_marks(self.buffers.current());
        let files = self.marks.file_marks();
        if local.is_empty() && files.is_empty() {
            return "No marks set".to_string();
        }
        let mut listing = String::from("\nmark line  col file/text");
        for (name, position) in local {
            let text = self.mark_text(position.row);
            listing.push('\n');
            listing.push_str(&self.mark_row(name, position.row, position.col, &text, width));
        }
        for (name, mark) in files {
            let text = self.file_mark_text(mark);
            listing.push('\n');
            listing.push_str(&self.mark_row(name, mark.row, mark.col, &text, width));
        }
        listing
    }

    fn mark_target(&self, name: char) -> Result<MarkTarget, EditError> {
        if name.is_ascii_lowercase() {
            return self
                .marks
                .local(self.buffers.current(), name)
                .map(MarkTarget::Local)
                .ok_or(EditError::MarkNotSet);
        }
        self.marks
            .file(name)
            .cloned()
            .map(MarkTarget::File)
            .ok_or(EditError::MarkNotSet)
    }

    fn open_file_mark(&mut self, mark: &FileMark) -> Result<(), EditError> {
        if mark.buffer == self.buffers.current() {
            return Ok(());
        }
        let Some(path) = mark.path.clone() else {
            if !self.buffers.contains(mark.buffer) {
                return Err(EditError::MarkNotSet);
            }
            self.switch_buffer(mark.buffer);
            return Ok(());
        };
        if self.absolute_path().as_deref() == Some(path.as_str()) {
            return Ok(());
        }
        self.edit_file(Some(&path), false)
    }

    fn place_mark(&mut self, position: Position, line_wise: bool) {
        let row = position.row.min(self.line_count().saturating_sub(1));
        let col = if line_wise {
            self.first_non_blank_of(row)
        } else {
            position.col
        };
        let col = col.min(self.max_col_for_row(row));
        self.cursor.set_position(row, col);
        self.remember_wanted_column();
        self.reveal_cursor();
    }

    fn mark_span(&self, name: char, line_wise: bool) -> Result<MarkSpan, EditError> {
        let mark = self.operator_mark_position(name)?;
        let row = mark.row.min(self.line_count().saturating_sub(1));
        let cursor_row = self.cursor.row();
        if line_wise {
            return Ok(MarkSpan::Lines {
                first: cursor_row.min(row),
                last: cursor_row.max(row),
            });
        }
        let cursor = (cursor_row, self.cursor.col());
        let target = (row, mark.col.min(self.max_col_for_row(row)));
        let (start, end) = if cursor <= target {
            (cursor, target)
        } else {
            (target, cursor)
        };
        Ok(MarkSpan::Chars { start, end })
    }

    fn operator_mark_position(&self, name: char) -> Result<Position, EditError> {
        match self.mark_target(name)? {
            MarkTarget::Local(position) => Ok(position),
            MarkTarget::File(mark) if mark.buffer == self.buffers.current() => Ok(Position {
                row: mark.row,
                col: mark.col,
            }),
            MarkTarget::File(_) => Err(EditError::MarkNotSet),
        }
    }

    fn mark_row(&self, name: char, row: usize, col: usize, text: &str, width: usize) -> String {
        format!(
            " {} {:>6} {:>4} {}",
            name,
            row.saturating_add(1),
            col,
            truncate_text(text, width)
        )
    }

    fn mark_text(&self, row: usize) -> String {
        self.buffer
            .line(row)
            .map(|line| line.trim_start().to_string())
            .unwrap_or_else(|| "-invalid-".to_string())
    }

    fn file_mark_text(&self, mark: &FileMark) -> String {
        if mark.buffer == self.buffers.current() && mark.path == self.absolute_path() {
            return self.mark_text(mark.row);
        }
        mark.path.clone().unwrap_or_else(|| "[No Name]".to_string())
    }

    pub(super) fn absolute_path(&self) -> Option<String> {
        let path = self.buffer.path.as_deref()?;
        let absolute = paths::absolute(path, &buffers::current_dir());
        Some(absolute.to_string_lossy().into_owned())
    }
}

fn truncate_text(text: &str, width: usize) -> String {
    let limit = width.saturating_sub(15);
    text.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::register::RegisterKind;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn temp_file(name: &str, text: &str) -> String {
        let path = std::env::temp_dir().join(format!("kawi_marks_{name}.txt"));
        std::fs::write(&path, text).expect("write temp file");
        path.to_string_lossy().into_owned()
    }

    fn set_mark(editor: &mut Editor, row: usize, col: usize, name: char) {
        editor.apply(Action::JumpTo(row, col));
        editor.apply(Action::SetMark(name));
    }

    #[test]
    fn line_jump_lands_on_first_non_blank() {
        let mut editor = editor_with("  alpha\n    beta gamma\nthird");
        set_mark(&mut editor, 1, 9, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::JumpToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!((editor.row(), editor.col()), (1, 4));
    }

    #[test]
    fn exact_jump_keeps_column() {
        let mut editor = editor_with("  alpha\n    beta gamma\nthird");
        set_mark(&mut editor, 1, 9, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::JumpToMark {
            name: 'a',
            line_wise: false,
        });
        assert_eq!((editor.row(), editor.col()), (1, 9));
    }

    #[test]
    fn unset_mark_reports_e20() {
        let mut editor = editor_with("abc");
        assert_eq!(editor.jump_to_mark('z', false), Err(EditError::MarkNotSet));
        assert_eq!(editor.jump_to_mark('Z', false), Err(EditError::MarkNotSet));
    }

    #[test]
    fn lower_case_marks_are_buffer_local() {
        let first = temp_file("local_first", "alpha\n");
        let second = temp_file("local_second", "beta\n");
        let mut editor = Editor::new(Buffer::from_file(&first).expect("open"));
        editor.apply(Action::SetMark('a'));
        editor.edit_file(Some(&second), false).expect("switch");
        assert_eq!(editor.jump_to_mark('a', false), Err(EditError::MarkNotSet));
        editor.edit_file(Some(&first), false).expect("switch back");
        editor.jump_to_mark('a', false).expect("local mark");
    }

    #[test]
    fn upper_case_mark_switches_to_its_file() {
        let first = temp_file("file_first", "first\n  second line\nthird\n");
        let second = temp_file("file_second", "other\nbuffer\n");
        let mut editor = Editor::new(Buffer::from_file(&first).expect("open"));
        set_mark(&mut editor, 1, 5, 'Q');
        editor.edit_file(Some(&second), false).expect("switch");
        editor.jump_to_mark('Q', true).expect("file mark");
        assert_eq!(editor.filename(), first);
        assert_eq!((editor.row(), editor.col()), (1, 2));
        editor.edit_file(Some(&second), false).expect("switch");
        editor.jump_to_mark('Q', false).expect("file mark");
        assert_eq!((editor.row(), editor.col()), (1, 5));
    }

    #[test]
    fn inserting_above_moves_mark() {
        let mut editor = editor_with("a\nb\nc\nd");
        set_mark(&mut editor, 2, 1, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::OpenLineBelow);
        editor.apply(Action::ExitInsert);
        editor.apply(Action::JumpToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!(editor.row(), 3);
    }

    #[test]
    fn relative_file_mark_moves_with_edits() {
        let mut buffer = Buffer::from_string("a\nb\nc\nd");
        buffer.path = Some("relative-mark.txt".to_string());
        let mut editor = Editor::new(buffer);
        set_mark(&mut editor, 2, 1, 'A');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::OpenLineBelow);
        editor.apply(Action::ExitInsert);
        editor.apply(Action::JumpToMark {
            name: 'A',
            line_wise: true,
        });
        assert_eq!(editor.row(), 3);
    }

    #[test]
    fn deleting_marked_line_removes_local_mark() {
        let mut editor = editor_with("a\nb\nc\nd");
        set_mark(&mut editor, 1, 0, 'a');
        editor.apply(Action::JumpTo(1, 0));
        editor.apply(Action::DeleteLine);
        assert_eq!(editor.jump_to_mark('a', true), Err(EditError::MarkNotSet));
    }

    #[test]
    fn deleting_marked_line_keeps_file_mark() {
        let mut editor = editor_with("a\nb\nc\nd");
        set_mark(&mut editor, 1, 0, 'A');
        editor.apply(Action::JumpTo(1, 0));
        editor.apply(Action::DeleteLine);
        editor.apply(Action::JumpToMark {
            name: 'A',
            line_wise: true,
        });
        assert_eq!(editor.row(), 1);
        assert_eq!(editor.text_lines()[1], "c");
    }

    #[test]
    fn same_line_edits_keep_mark_column() {
        let mut editor = editor_with("abcdefgh");
        set_mark(&mut editor, 0, 4, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::JumpToMark {
            name: 'a',
            line_wise: false,
        });
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }

    #[test]
    fn previous_mark_returns_to_last_jump_origin() {
        let mut editor = editor_with("ab\ncd\nef");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::JumpToPrevious { line_wise: true });
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn marks_listing_matches_nvim_format() {
        let mut editor = editor_with("  alpha\n    beta gamma\nthird");
        set_mark(&mut editor, 1, 9, 'a');
        set_mark(&mut editor, 0, 3, 'B');
        assert_eq!(
            editor.marks_listing(80),
            "\nmark line  col file/text\n a      2    9 beta gamma\n B      1    3 alpha"
        );
    }

    #[test]
    fn marks_listing_without_marks() {
        let editor = editor_with("abc");
        assert_eq!(editor.marks_listing(80), "No marks set");
    }

    #[test]
    fn marks_listing_shows_path_for_other_files() {
        let first = temp_file("listing_first", "first\n");
        let second = temp_file("listing_second", "second\n");
        let mut editor = Editor::new(Buffer::from_file(&first).expect("open"));
        editor.apply(Action::SetMark('A'));
        editor.edit_file(Some(&second), false).expect("switch");
        let listing = editor.marks_listing(80);
        assert!(listing.contains(&first), "{listing}");
    }

    #[test]
    fn delete_to_line_mark_removes_lines() {
        let mut editor = editor_with("one\ntwo\nthree\nfour");
        set_mark(&mut editor, 2, 2, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::DeleteToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!(editor.text_lines(), vec!["four".to_string()]);
        assert_eq!(editor.register_text(), "one\ntwo\nthree");
        assert_eq!(editor.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_to_exact_mark_excludes_mark_char() {
        let mut editor = editor_with("one\ntwo\nthree\nfour");
        set_mark(&mut editor, 2, 2, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::DeleteToMark {
            name: 'a',
            line_wise: false,
        });
        assert_eq!(
            editor.text_lines(),
            vec!["ree".to_string(), "four".to_string()]
        );
        assert_eq!(editor.register_text(), "one\ntwo\nth");
        assert_eq!(editor.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_to_backward_mark_keeps_cursor_at_start() {
        let mut editor = editor_with("one\ntwo\nthree\nfour");
        set_mark(&mut editor, 0, 1, 'a');
        editor.apply(Action::JumpTo(2, 2));
        editor.apply(Action::DeleteToMark {
            name: 'a',
            line_wise: false,
        });
        assert_eq!(
            editor.text_lines(),
            vec!["oree".to_string(), "four".to_string()]
        );
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn yank_to_line_mark_keeps_cursor() {
        let mut editor = editor_with("one\ntwo\nthree\nfour");
        set_mark(&mut editor, 2, 2, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::YankToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!(editor.text_lines().len(), 4);
        assert_eq!(editor.register_text(), "one\ntwo\nthree");
        assert_eq!(editor.register_kind(), RegisterKind::LineWise);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn change_to_line_mark_enters_insert() {
        let mut editor = editor_with("one\ntwo\nthree\nfour");
        set_mark(&mut editor, 2, 2, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::ChangeToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!(
            editor.text_lines(),
            vec!["".to_string(), "four".to_string()]
        );
        assert_eq!(editor.mode(), crate::mode::Mode::Insert);
        assert_eq!(editor.register_text(), "one\ntwo\nthree");
    }

    #[test]
    fn change_to_exact_mark_does_not_switch_files() {
        let first = temp_file("change_first", "one\ntwo\n");
        let second = temp_file("change_second", "other\n");
        let mut editor = Editor::new(Buffer::from_file(&first).expect("open"));
        editor.apply(Action::SetMark('A'));
        editor.edit_file(Some(&second), false).expect("switch");
        assert_eq!(editor.change_to_mark('A', true), Err(EditError::MarkNotSet));
        assert_eq!(editor.filename(), second);
    }

    #[test]
    fn delete_to_mark_with_count_ignores_count() {
        let mut editor = editor_with("a1\nb2\nc3\nd4\ne5");
        set_mark(&mut editor, 1, 0, 'a');
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::PushCount(2));
        editor.apply(Action::DeleteToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!(editor.text_lines(), vec!["c3", "d4", "e5"]);
    }
}
