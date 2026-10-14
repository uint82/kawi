const MAX_HISTORY: usize = 200;

use ropey::Rope;

struct Snapshot {
    id: u64,
    text: Rope,
    row: usize,
    col: usize,
}

pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    group_open: bool,
    current_id: u64,
    next_id: u64,
    clean_id: u64,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            group_open: false,
            current_id: 0,
            next_id: 1,
            clean_id: 0,
        }
    }

    pub fn begin_change(&mut self) {
        self.group_open = true;
    }

    pub fn end_change(&mut self) {
        self.group_open = false;
    }

    pub fn checkpoint(&mut self, text: &Rope, row: usize, col: usize) {
        self.push_snapshot(text, row, col);
        self.group_open = false;
    }

    pub fn maybe_checkpoint(&mut self, text: &Rope, row: usize, col: usize) {
        if self.group_open {
            self.push_snapshot(text, row, col);
            self.group_open = false;
        }
    }

    pub fn undo(&mut self, text: &mut Rope, row: usize, col: usize) -> Option<(usize, usize)> {
        self.undo
            .pop()
            .map(|snapshot| self.restore_snapshot(text, row, col, snapshot, true))
    }

    pub fn redo(&mut self, text: &mut Rope, row: usize, col: usize) -> Option<(usize, usize)> {
        self.redo
            .pop()
            .map(|snapshot| self.restore_snapshot(text, row, col, snapshot, false))
    }

    pub fn mark_clean(&mut self) {
        self.clean_id = self.current_id;
    }

    pub fn is_dirty(&self) -> bool {
        self.current_id != self.clean_id
    }

    fn restore_snapshot(
        &mut self,
        text: &mut Rope,
        row: usize,
        col: usize,
        snapshot: Snapshot,
        is_undo: bool,
    ) -> (usize, usize) {
        self.push_other_side(text, row, col, is_undo);
        self.current_id = snapshot.id;
        *text = snapshot.text;
        (snapshot.row, snapshot.col)
    }

    fn push_other_side(&mut self, text: &Rope, row: usize, col: usize, is_undo: bool) {
        let snapshot = Snapshot {
            id: self.current_id,
            text: text.clone(),
            row,
            col,
        };
        if is_undo {
            self.redo.push(snapshot);
        } else {
            self.undo.push(snapshot);
        }
    }

    fn push_snapshot(&mut self, text: &Rope, row: usize, col: usize) {
        let snapshot = Snapshot {
            id: self.current_id,
            text: text.clone(),
            row,
            col,
        };
        self.undo.push(snapshot);
        self.current_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.redo.clear();
        self.trim_to_cap();
    }

    fn trim_to_cap(&mut self) {
        if self.undo.len() > MAX_HISTORY {
            let excess = self.undo.len() - MAX_HISTORY;
            self.undo.drain(..excess);
        }
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    fn rope_of(text: &str) -> Rope {
        Rope::from_str(text)
    }

    fn text_of(rope: &Rope) -> String {
        rope.to_string()
    }

    #[test]
    fn fresh_is_clean() {
        let history = History::new();
        assert!(!history.is_dirty());
    }

    #[test]
    fn checkpoint_marks_dirty() {
        let mut history = History::new();
        history.checkpoint(&rope_of("ab"), 0, 0);
        assert!(history.is_dirty());
    }

    #[test]
    fn undo_restores_snapshot_and_cursor() {
        let mut history = History::new();
        let before = rope_of("ab");
        history.checkpoint(&before, 0, 1);
        let mut now = rope_of("abcd");
        assert_eq!(history.undo(&mut now, 0, 3), Some((0, 1)));
        assert_eq!(text_of(&now), text_of(&before));
    }

    #[test]
    fn redo_restores_forward_and_cursor() {
        let mut history = History::new();
        history.checkpoint(&rope_of("ab"), 0, 0);
        let mut now = rope_of("abcd");
        history.undo(&mut now, 0, 3);
        assert_eq!(history.redo(&mut now, 0, 0), Some((0, 3)));
        assert_eq!(text_of(&now), "abcd");
    }

    #[test]
    fn new_change_clears_redo() {
        let mut history = History::new();
        history.checkpoint(&rope_of("ab"), 0, 0);
        let mut now = rope_of("abcd");
        history.undo(&mut now, 0, 0);
        history.checkpoint(&now, 0, 0);
        assert_eq!(history.redo(&mut now, 0, 0), None);
    }

    #[test]
    fn undo_to_saved_clears_dirty() {
        let mut history = History::new();
        history.checkpoint(&rope_of("ab"), 0, 0);
        let mut now = rope_of("abc");
        history.mark_clean();
        history.checkpoint(&now, 0, 1);
        now = rope_of("abcd");
        assert!(history.undo(&mut now, 0, 2).is_some());
        assert_eq!(text_of(&now), "abc");
        assert!(!history.is_dirty());
    }

    #[test]
    fn group_records_once() {
        let mut history = History::new();
        let before = rope_of("ab");
        history.begin_change();
        history.maybe_checkpoint(&before, 0, 0);
        history.maybe_checkpoint(&rope_of("abc"), 0, 1);
        let mut now = rope_of("abcd");
        assert!(history.undo(&mut now, 0, 2).is_some());
        assert_eq!(text_of(&now), text_of(&before));
        assert_eq!(history.undo(&mut now, 0, 0), None);
    }

    #[test]
    fn empty_group_records_nothing() {
        let mut history = History::new();
        history.begin_change();
        history.end_change();
        let mut now = rope_of("ab");
        assert_eq!(history.undo(&mut now, 0, 0), None);
    }
}
