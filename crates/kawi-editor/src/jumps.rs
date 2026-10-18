use kawi_buffer::LineAdjust;

const JUMP_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Jump {
    pub buffer: usize,
    pub row: usize,
    pub col: usize,
}

impl Jump {
    fn same_line(self, other: Self) -> bool {
        self.buffer == other.buffer && self.row == other.row
    }
}

#[derive(Debug, Default)]
pub(crate) struct Jumps {
    entries: Vec<Jump>,
    index: usize,
}

impl Jumps {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn record(&mut self, jump: Jump) {
        if self.entries.len() >= JUMP_LIMIT {
            self.entries.remove(0);
        }
        self.entries.push(jump);
        self.index = self.entries.len();
    }

    pub(crate) fn older(&mut self, count: usize, cursor: Jump) -> Option<Jump> {
        self.navigate(-(count as isize), cursor)
    }

    pub(crate) fn newer(&mut self, count: usize, cursor: Jump) -> Option<Jump> {
        self.navigate(count as isize, cursor)
    }

    pub(crate) fn adjust(&mut self, buffer: usize, adjust: LineAdjust) {
        for entry in self
            .entries
            .iter_mut()
            .filter(|entry| entry.buffer == buffer)
        {
            let lnum = entry.row + 1;
            let lnum = if lnum >= adjust.line1 && lnum <= adjust.line2 {
                match adjust.amount {
                    None => adjust.line1,
                    Some(amount) => lnum.saturating_add_signed(amount),
                }
            } else if adjust.amount_after != 0 && lnum > adjust.line2 {
                lnum.saturating_add_signed(adjust.amount_after)
            } else {
                lnum
            };
            entry.row = lnum.saturating_sub(1);
        }
    }

    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[Jump] {
        &self.entries
    }

    #[cfg(test)]
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    fn navigate(&mut self, count: isize, cursor: Jump) -> Option<Jump> {
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

    fn clean(&mut self, cursor: Jump) {
        let mut kept: Vec<Jump> = Vec::with_capacity(self.entries.len());
        for (from, entry) in self.entries.iter().enumerate() {
            if self.index == from {
                self.index = kept.len();
            }
            let duplicated = self.entries[from + 1..]
                .iter()
                .any(|later| later.same_line(*entry));
            if !duplicated {
                kept.push(*entry);
            }
        }
        if self.index == self.entries.len() {
            self.index = kept.len();
        }
        self.entries = kept;
        if self.index == self.entries.len()
            && self
                .entries
                .last()
                .is_some_and(|entry| entry.same_line(cursor))
        {
            self.entries.pop();
            self.index = self.index.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jump(row: usize, col: usize) -> Jump {
        Jump {
            buffer: 1,
            row,
            col,
        }
    }

    #[test]
    fn record_appends_and_moves_index_to_end() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        jumps.record(jump(8, 1));
        assert_eq!(jumps.entries(), &[jump(2, 4), jump(8, 1)]);
        assert_eq!(jumps.index(), 2);
    }

    #[test]
    fn older_returns_previous_entry_and_pushes_cursor() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        let found = jumps.older(1, jump(8, 1));
        assert_eq!(found, Some(jump(2, 4)));
        assert_eq!(jumps.entries(), &[jump(2, 4), jump(8, 1)]);
        assert_eq!(jumps.index(), 0);
    }

    #[test]
    fn newer_returns_next_entry() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        jumps.older(1, jump(8, 1));
        let found = jumps.newer(1, jump(2, 4));
        assert_eq!(found, Some(jump(8, 1)));
        assert_eq!(jumps.index(), 1);
    }

    #[test]
    fn older_at_start_returns_none() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        assert_eq!(jumps.older(1, jump(8, 1)), Some(jump(2, 4)));
        assert_eq!(jumps.older(1, jump(2, 4)), None);
        assert_eq!(jumps.entries(), &[jump(2, 4), jump(8, 1)]);
        assert_eq!(jumps.index(), 0);
    }

    #[test]
    fn newer_at_end_does_not_push_cursor() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        assert_eq!(jumps.newer(1, jump(8, 1)), None);
        assert_eq!(jumps.entries(), &[jump(2, 4)]);
        assert_eq!(jumps.index(), 1);
    }

    #[test]
    fn count_out_of_bounds_returns_none() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        jumps.record(jump(8, 1));
        assert_eq!(jumps.older(3, jump(9, 0)), None);
        assert_eq!(jumps.entries(), &[jump(2, 4), jump(8, 1)]);
    }

    #[test]
    fn count_skips_entries() {
        let mut jumps = Jumps::new();
        jumps.record(jump(1, 0));
        jumps.record(jump(2, 0));
        jumps.record(jump(3, 0));
        assert_eq!(jumps.older(2, jump(4, 0)), Some(jump(2, 0)));
        assert_eq!(jumps.index(), 1);
    }

    #[test]
    fn duplicate_rows_keep_last_entry() {
        let mut jumps = Jumps::new();
        jumps.record(jump(1, 0));
        jumps.record(jump(2, 0));
        jumps.record(jump(1, 0));
        jumps.record(jump(3, 0));
        assert_eq!(jumps.older(1, jump(9, 0)), Some(jump(3, 0)));
        assert_eq!(
            jumps.entries(),
            &[jump(2, 0), jump(1, 0), jump(3, 0), jump(9, 0)]
        );
        assert_eq!(jumps.index(), 2);
    }

    #[test]
    fn same_row_in_another_buffer_is_not_duplicate() {
        let mut jumps = Jumps::new();
        jumps.record(Jump {
            buffer: 2,
            row: 0,
            col: 0,
        });
        jumps.record(jump(0, 4));
        assert_eq!(jumps.older(1, jump(9, 0)), Some(jump(0, 4)));
        assert_eq!(jumps.entries().len(), 3);
    }

    #[test]
    fn cursor_on_last_entry_row_drops_it() {
        let mut jumps = Jumps::new();
        jumps.record(jump(2, 4));
        assert_eq!(jumps.older(1, jump(2, 4)), None);
        assert_eq!(jumps.entries(), &[]);
        assert_eq!(jumps.index(), 0);
    }

    fn adjust(
        line1: usize,
        line2: usize,
        amount: Option<isize>,
        amount_after: isize,
    ) -> LineAdjust {
        LineAdjust {
            line1,
            line2,
            amount,
            amount_after,
        }
    }

    #[test]
    fn adjust_clamps_deleted_range() {
        let mut jumps = Jumps::new();
        for row in [0, 4, 5, 6, 19] {
            jumps.record(jump(row, 3));
        }
        jumps.adjust(1, adjust(5, 6, None, -2));
        assert_eq!(
            jumps.entries(),
            &[jump(0, 3), jump(4, 3), jump(4, 3), jump(4, 3), jump(17, 3)]
        );
    }

    #[test]
    fn adjust_ignores_other_buffers() {
        let mut jumps = Jumps::new();
        jumps.record(Jump {
            buffer: 2,
            row: 5,
            col: 0,
        });
        jumps.record(jump(5, 0));
        jumps.adjust(1, adjust(5, 6, None, -2));
        assert_eq!(
            jumps.entries(),
            &[
                Jump {
                    buffer: 2,
                    row: 5,
                    col: 0
                },
                jump(4, 0)
            ]
        );
    }

    #[test]
    fn adjust_shifts_after_open_ended_insertion() {
        let mut jumps = Jumps::new();
        for row in [4, 5, 19] {
            jumps.record(jump(row, 1));
        }
        jumps.adjust(1, adjust(6, usize::MAX, Some(2), 0));
        assert_eq!(jumps.entries(), &[jump(4, 1), jump(7, 1), jump(21, 1)]);
    }

    #[test]
    fn adjust_empty_range_shifts_lines_after() {
        let mut jumps = Jumps::new();
        jumps.record(jump(0, 0));
        jumps.record(jump(3, 0));
        jumps.adjust(1, adjust(2, 1, None, 1));
        assert_eq!(jumps.entries(), &[jump(0, 0), jump(4, 0)]);
    }

    #[test]
    fn limit_drops_oldest_entry() {
        let mut jumps = Jumps::new();
        for row in 0..JUMP_LIMIT + 5 {
            jumps.record(jump(row, 0));
        }
        assert_eq!(jumps.entries().len(), JUMP_LIMIT);
        assert_eq!(jumps.entries()[0], jump(5, 0));
    }
}
