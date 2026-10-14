const MAX_HISTORY: usize = 200;

struct Snapshot {
    id: u64,
    lines: Vec<String>,
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

    pub fn checkpoint(&mut self, lines: &[String]) {
        self.push_snapshot(lines);
        self.group_open = false;
    }

    pub fn maybe_checkpoint(&mut self, lines: &[String]) {
        if self.group_open {
            self.push_snapshot(lines);
            self.group_open = false;
        }
    }

    pub fn undo(&mut self, lines: &mut Vec<String>) -> bool {
        match self.undo.pop() {
            Some(snapshot) => self.restore_snapshot(lines, snapshot, true),
            None => false,
        }
    }

    pub fn redo(&mut self, lines: &mut Vec<String>) -> bool {
        match self.redo.pop() {
            Some(snapshot) => self.restore_snapshot(lines, snapshot, false),
            None => false,
        }
    }

    pub fn mark_clean(&mut self) {
        self.clean_id = self.current_id;
    }

    pub fn is_dirty(&self) -> bool {
        self.current_id != self.clean_id
    }

    fn restore_snapshot(
        &mut self,
        lines: &mut Vec<String>,
        snapshot: Snapshot,
        is_undo: bool,
    ) -> bool {
        self.push_other_side(lines, is_undo);
        self.current_id = snapshot.id;
        *lines = snapshot.lines;
        true
    }

    fn push_other_side(&mut self, lines: &[String], is_undo: bool) {
        let snapshot = Snapshot {
            id: self.current_id,
            lines: lines.to_vec(),
        };
        if is_undo {
            self.redo.push(snapshot);
        } else {
            self.undo.push(snapshot);
        }
    }

    fn push_snapshot(&mut self, lines: &[String]) {
        let snapshot = Snapshot {
            id: self.current_id,
            lines: lines.to_vec(),
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

    fn lines_of(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn fresh_is_clean() {
        let history = History::new();
        assert!(!history.is_dirty());
    }

    #[test]
    fn checkpoint_marks_dirty() {
        let mut history = History::new();
        history.checkpoint(&lines_of("ab"));
        assert!(history.is_dirty());
    }

    #[test]
    fn undo_restores_snapshot() {
        let mut history = History::new();
        let before = lines_of("ab");
        history.checkpoint(&before);
        let mut now = lines_of("abcd");
        assert!(history.undo(&mut now));
        assert_eq!(now, before);
    }

    #[test]
    fn redo_restores_forward() {
        let mut history = History::new();
        history.checkpoint(&lines_of("ab"));
        let mut now = lines_of("abcd");
        history.undo(&mut now);
        assert!(history.redo(&mut now));
        assert_eq!(now, lines_of("abcd"));
    }

    #[test]
    fn new_change_clears_redo() {
        let mut history = History::new();
        history.checkpoint(&lines_of("ab"));
        let mut now = lines_of("abcd");
        history.undo(&mut now);
        history.checkpoint(&now);
        assert!(!history.redo(&mut now));
    }

    #[test]
    fn undo_to_saved_clears_dirty() {
        let mut history = History::new();
        history.checkpoint(&lines_of("ab"));
        let mut now = lines_of("abc");
        history.mark_clean();
        history.checkpoint(&now);
        now = lines_of("abcd");
        assert!(history.undo(&mut now));
        assert_eq!(now, lines_of("abc"));
        assert!(!history.is_dirty());
    }

    #[test]
    fn group_records_once() {
        let mut history = History::new();
        let before = lines_of("ab");
        history.begin_change();
        history.maybe_checkpoint(&before);
        history.maybe_checkpoint(&lines_of("abc"));
        let mut now = lines_of("abcd");
        assert!(history.undo(&mut now));
        assert_eq!(now, before);
        assert!(!history.undo(&mut now));
    }

    #[test]
    fn empty_group_records_nothing() {
        let mut history = History::new();
        history.begin_change();
        history.end_change();
        let mut now = lines_of("ab");
        assert!(!history.undo(&mut now));
    }
}
