// TODO(jumps): adjust entries when lines are inserted or deleted like mark_adjust.
const JUMP_LIMIT: usize = 100;

#[derive(Debug, Default)]
pub(crate) struct Jumps {
    entries: Vec<(usize, usize)>,
    index: usize,
}

impl Jumps {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn record(&mut self, position: (usize, usize)) {
        if self.entries.len() >= JUMP_LIMIT {
            self.entries.remove(0);
        }
        self.entries.push(position);
        self.index = self.entries.len();
    }

    pub(crate) fn older(&mut self, count: usize, cursor: (usize, usize)) -> Option<(usize, usize)> {
        self.navigate(-(count as isize), cursor)
    }

    pub(crate) fn newer(&mut self, count: usize, cursor: (usize, usize)) -> Option<(usize, usize)> {
        self.navigate(count as isize, cursor)
    }

    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[(usize, usize)] {
        &self.entries
    }

    #[cfg(test)]
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    fn navigate(&mut self, count: isize, cursor: (usize, usize)) -> Option<(usize, usize)> {
        self.clean(cursor);
        if self.entries.is_empty() {
            return None;
        }
        let target = self.index as isize + count;
        if target < 0 || target as usize >= self.entries.len() {
            return None;
        }
        if self.index == self.entries.len() {
            self.record(cursor);
            self.index -= 1;
            if self.index as isize + count < 0 {
                return None;
            }
        }
        self.index = (self.index as isize + count) as usize;
        self.entries.get(self.index).copied()
    }

    fn clean(&mut self, cursor: (usize, usize)) {
        let mut kept: Vec<(usize, usize)> = Vec::with_capacity(self.entries.len());
        for (from, entry) in self.entries.iter().enumerate() {
            if self.index == from {
                self.index = kept.len();
            }
            let duplicated = self.entries[from + 1..]
                .iter()
                .any(|later| later.0 == entry.0);
            if !duplicated {
                kept.push(*entry);
            }
        }
        if self.index == self.entries.len() {
            self.index = kept.len();
        }
        self.entries = kept;
        if self.index == self.entries.len()
            && self.entries.last().is_some_and(|entry| entry.0 == cursor.0)
        {
            self.entries.pop();
            self.index = self.index.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_appends_and_moves_index_to_end() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        jumps.record((8, 1));
        assert_eq!(jumps.entries(), &[(2, 4), (8, 1)]);
        assert_eq!(jumps.index(), 2);
    }

    #[test]
    fn older_returns_previous_entry_and_pushes_cursor() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        let found = jumps.older(1, (8, 1));
        assert_eq!(found, Some((2, 4)));
        assert_eq!(jumps.entries(), &[(2, 4), (8, 1)]);
        assert_eq!(jumps.index(), 0);
    }

    #[test]
    fn newer_returns_next_entry() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        jumps.older(1, (8, 1));
        let found = jumps.newer(1, (2, 4));
        assert_eq!(found, Some((8, 1)));
        assert_eq!(jumps.index(), 1);
    }

    #[test]
    fn older_at_start_returns_none() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        assert_eq!(jumps.older(1, (8, 1)), Some((2, 4)));
        assert_eq!(jumps.older(1, (2, 4)), None);
        assert_eq!(jumps.entries(), &[(2, 4), (8, 1)]);
        assert_eq!(jumps.index(), 0);
    }

    #[test]
    fn newer_at_end_does_not_push_cursor() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        assert_eq!(jumps.newer(1, (8, 1)), None);
        assert_eq!(jumps.entries(), &[(2, 4)]);
        assert_eq!(jumps.index(), 1);
    }

    #[test]
    fn count_out_of_bounds_returns_none() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        jumps.record((8, 1));
        assert_eq!(jumps.older(3, (9, 0)), None);
        assert_eq!(jumps.entries(), &[(2, 4), (8, 1)]);
    }

    #[test]
    fn count_skips_entries() {
        let mut jumps = Jumps::new();
        jumps.record((1, 0));
        jumps.record((2, 0));
        jumps.record((3, 0));
        assert_eq!(jumps.older(2, (4, 0)), Some((2, 0)));
        assert_eq!(jumps.index(), 1);
    }

    #[test]
    fn duplicate_rows_keep_last_entry() {
        let mut jumps = Jumps::new();
        jumps.record((1, 0));
        jumps.record((2, 0));
        jumps.record((1, 0));
        jumps.record((3, 0));
        assert_eq!(jumps.older(1, (9, 0)), Some((3, 0)));
        assert_eq!(jumps.entries(), &[(2, 0), (1, 0), (3, 0), (9, 0)]);
        assert_eq!(jumps.index(), 2);
    }

    #[test]
    fn cursor_on_last_entry_row_drops_it() {
        let mut jumps = Jumps::new();
        jumps.record((2, 4));
        assert_eq!(jumps.older(1, (2, 4)), None);
        assert_eq!(jumps.entries(), &[]);
        assert_eq!(jumps.index(), 0);
    }

    #[test]
    fn limit_drops_oldest_entry() {
        let mut jumps = Jumps::new();
        for row in 0..JUMP_LIMIT + 5 {
            jumps.record((row, 0));
        }
        assert_eq!(jumps.entries().len(), JUMP_LIMIT);
        assert_eq!(jumps.entries()[0], (5, 0));
    }
}
