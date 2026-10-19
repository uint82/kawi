use kawi_buffer::LineAdjust;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Position {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileMark {
    pub path: Option<String>,
    pub buffer: usize,
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Default)]
pub(crate) struct Marks {
    local: HashMap<usize, HashMap<char, Position>>,
    files: HashMap<char, FileMark>,
}

impl Marks {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_local(&mut self, buffer: usize, name: char, position: Position) {
        self.local.entry(buffer).or_default().insert(name, position);
    }

    pub(crate) fn set_file(&mut self, name: char, mark: FileMark) {
        self.files.insert(name, mark);
    }

    pub(crate) fn local(&self, buffer: usize, name: char) -> Option<Position> {
        self.local.get(&buffer)?.get(&name).copied()
    }

    pub(crate) fn file(&self, name: char) -> Option<&FileMark> {
        self.files.get(&name)
    }

    pub(crate) fn adjust(&mut self, buffer: usize, path: Option<&str>, adjust: LineAdjust) {
        if let Some(marks) = self.local.get_mut(&buffer) {
            marks.retain(|_, position| {
                let Some(row) = adjusted_line(position.row, adjust, false) else {
                    return false;
                };
                position.row = row;
                true
            });
        }
        for mark in self.files.values_mut() {
            if !mark.same_file(buffer, path) {
                continue;
            }
            if let Some(row) = adjusted_line(mark.row, adjust, true) {
                mark.row = row;
            }
        }
    }

    pub(crate) fn local_marks(&self, buffer: usize) -> Vec<(char, Position)> {
        let mut found: Vec<(char, Position)> = self
            .local
            .get(&buffer)
            .map(|marks| {
                marks
                    .iter()
                    .map(|(name, position)| (*name, *position))
                    .collect()
            })
            .unwrap_or_default();
        found.sort_by_key(|(name, _)| *name);
        found
    }

    pub(crate) fn file_marks(&self) -> Vec<(char, &FileMark)> {
        let mut found: Vec<(char, &FileMark)> = self
            .files
            .iter()
            .map(|(name, mark)| (*name, mark))
            .collect();
        found.sort_by_key(|(name, _)| *name);
        found
    }
}

impl FileMark {
    fn same_file(&self, buffer: usize, path: Option<&str>) -> bool {
        match self.path.as_deref() {
            Some(mark_path) => path == Some(mark_path),
            None => self.buffer == buffer,
        }
    }
}

fn adjusted_line(row: usize, adjust: LineAdjust, keep_deleted: bool) -> Option<usize> {
    let line = row.saturating_add(1);
    let moved = if line >= adjust.line1 && line <= adjust.line2 {
        match adjust.amount {
            Some(amount) => line.saturating_add_signed(amount),
            None if keep_deleted => adjust.line1,
            None => return None,
        }
    } else if adjust.amount_after != 0 && line > adjust.line2 {
        line.saturating_add_signed(adjust.amount_after)
    } else {
        line
    };
    Some(moved.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(row: usize, col: usize) -> Position {
        Position { row, col }
    }

    #[test]
    fn local_marks_are_per_buffer() {
        let mut marks = Marks::new();
        marks.set_local(1, 'a', position(3, 4));
        assert_eq!(marks.local(1, 'a'), Some(position(3, 4)));
        assert_eq!(marks.local(2, 'a'), None);
    }

    #[test]
    fn set_local_replaces_position() {
        let mut marks = Marks::new();
        marks.set_local(1, 'a', position(3, 4));
        marks.set_local(1, 'a', position(1, 0));
        assert_eq!(marks.local(1, 'a'), Some(position(1, 0)));
    }

    #[test]
    fn file_marks_match_by_path() {
        let mut marks = Marks::new();
        marks.set_file(
            'A',
            FileMark {
                path: Some("/tmp/one.txt".to_string()),
                buffer: 1,
                row: 2,
                col: 1,
            },
        );
        assert_eq!(marks.file('A').unwrap().row, 2);
    }

    #[test]
    fn insertion_above_shifts_local_marks() {
        let mut marks = Marks::new();
        marks.set_local(1, 'a', position(2, 4));
        marks.adjust(
            1,
            None,
            LineAdjust {
                line1: 1,
                line2: usize::MAX,
                amount: Some(1),
                amount_after: 0,
            },
        );
        assert_eq!(marks.local(1, 'a'), Some(position(3, 4)));
    }

    #[test]
    fn deletion_after_shifts_marks() {
        let mut marks = Marks::new();
        marks.set_local(1, 'a', position(3, 1));
        marks.adjust(
            1,
            None,
            LineAdjust {
                line1: 2,
                line2: 2,
                amount: None,
                amount_after: -1,
            },
        );
        assert_eq!(marks.local(1, 'a'), Some(position(2, 1)));
    }

    #[test]
    fn deleted_line_removes_local_mark() {
        let mut marks = Marks::new();
        marks.set_local(1, 'a', position(1, 1));
        marks.adjust(
            1,
            None,
            LineAdjust {
                line1: 2,
                line2: 2,
                amount: None,
                amount_after: -1,
            },
        );
        assert_eq!(marks.local(1, 'a'), None);
    }

    #[test]
    fn deleted_line_keeps_file_mark() {
        let mut marks = Marks::new();
        marks.set_file(
            'A',
            FileMark {
                path: Some("/tmp/one.txt".to_string()),
                buffer: 1,
                row: 2,
                col: 1,
            },
        );
        marks.adjust(
            1,
            Some("/tmp/one.txt"),
            LineAdjust {
                line1: 2,
                line2: 3,
                amount: None,
                amount_after: -2,
            },
        );
        assert_eq!(marks.file('A').unwrap().row, 1);
    }

    #[test]
    fn adjust_ignores_other_buffers() {
        let mut marks = Marks::new();
        marks.set_local(1, 'a', position(2, 1));
        marks.adjust(
            2,
            None,
            LineAdjust {
                line1: 1,
                line2: 1,
                amount: None,
                amount_after: -5,
            },
        );
        assert_eq!(marks.local(1, 'a'), Some(position(2, 1)));
    }

    #[test]
    fn file_mark_follows_path_across_buffers() {
        let mut marks = Marks::new();
        marks.set_file(
            'A',
            FileMark {
                path: Some("/tmp/one.txt".to_string()),
                buffer: 4,
                row: 1,
                col: 0,
            },
        );
        marks.adjust(
            9,
            Some("/tmp/one.txt"),
            LineAdjust {
                line1: 1,
                line2: usize::MAX,
                amount: Some(2),
                amount_after: 0,
            },
        );
        assert_eq!(marks.file('A').unwrap().row, 3);
    }

    #[test]
    fn listings_are_sorted() {
        let mut marks = Marks::new();
        marks.set_local(1, 'b', position(1, 0));
        marks.set_local(1, 'a', position(2, 0));
        marks.set_file(
            'Z',
            FileMark {
                path: Some("/tmp/one.txt".to_string()),
                buffer: 1,
                row: 1,
                col: 0,
            },
        );
        marks.set_file(
            'A',
            FileMark {
                path: Some("/tmp/two.txt".to_string()),
                buffer: 1,
                row: 1,
                col: 0,
            },
        );
        let local: Vec<char> = marks
            .local_marks(1)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(local, vec!['a', 'b']);
        let files: Vec<char> = marks
            .file_marks()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(files, vec!['A', 'Z']);
    }
}
