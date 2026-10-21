use similar::{Algorithm, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkKind {
    Add,
    Change,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignKind {
    Add,
    Change,
    Delete,
    TopDelete,
    ChangeDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sign {
    pub row: usize,
    pub kind: SignKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub kind: HunkKind,
    pub removed_start: usize,
    pub removed_count: usize,
    pub added_start: usize,
    pub added_count: usize,
    pub removed_lines: Vec<String>,
    pub added_lines: Vec<String>,
}

impl Hunk {
    pub fn start_line(&self) -> usize {
        self.added_start.max(1)
    }

    pub fn end_line(&self) -> usize {
        if self.added_count == 0 {
            self.added_start.max(1)
        } else {
            self.added_start
                .saturating_add(self.added_count)
                .saturating_sub(1)
        }
    }

    pub fn old_header_start(&self) -> usize {
        self.removed_start
    }

    pub fn new_header_start(&self) -> usize {
        self.added_start
    }

    pub fn apply(&self, lines: &[String], invert: bool) -> Vec<String> {
        let start = if invert {
            self.added_start
        } else {
            self.removed_start
        };
        let count = if invert {
            self.added_count
        } else {
            self.removed_count
        };
        let added = if invert {
            &self.removed_lines
        } else {
            &self.added_lines
        };
        let index = if count == 0 {
            start.min(lines.len())
        } else {
            start.saturating_sub(1).min(lines.len())
        };
        let end = index.saturating_add(count).min(lines.len());
        let mut result = Vec::with_capacity(lines.len().saturating_add(added.len()));
        result.extend_from_slice(&lines[..index]);
        result.extend(added.iter().cloned());
        result.extend_from_slice(&lines[end..]);
        result
    }

    pub fn reversed(&self) -> Self {
        Self {
            kind: hunk_kind(self.added_count, self.removed_count),
            removed_start: self.added_start,
            removed_count: self.added_count,
            added_start: self.removed_start,
            added_count: self.removed_count,
            removed_lines: self.added_lines.clone(),
            added_lines: self.removed_lines.clone(),
        }
    }

    pub fn signs(&self) -> Vec<Sign> {
        match self.kind {
            HunkKind::Add => self.added_rows(),
            HunkKind::Delete => vec![Sign {
                row: self.added_start.saturating_sub(1),
                kind: if self.added_start == 0 {
                    SignKind::TopDelete
                } else {
                    SignKind::Delete
                },
            }],
            HunkKind::Change => self.change_signs(),
        }
    }

    fn added_rows(&self) -> Vec<Sign> {
        (self.start_line()..=self.end_line())
            .map(|line| Sign {
                row: line.saturating_sub(1),
                kind: SignKind::Add,
            })
            .collect()
    }

    fn change_signs(&self) -> Vec<Sign> {
        let changed = self
            .added_start
            .saturating_add(self.added_count.min(self.removed_count).saturating_sub(1));
        let mut signs = Vec::new();
        for line in self.added_start..=changed {
            let kind = if self.removed_count > self.added_count && line == changed {
                SignKind::ChangeDelete
            } else {
                SignKind::Change
            };
            signs.push(Sign {
                row: line.saturating_sub(1),
                kind,
            });
        }
        if self.added_count > self.removed_count {
            for line in changed.saturating_add(1)..=self.end_line() {
                signs.push(Sign {
                    row: line.saturating_sub(1),
                    kind: SignKind::Add,
                });
            }
        }
        signs
    }
}

pub fn diff_hunks(base: &[String], buffer: &[String]) -> Vec<Hunk> {
    let old: Vec<&str> = base.iter().map(String::as_str).collect();
    let new: Vec<&str> = buffer.iter().map(String::as_str).collect();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_slices(&old, &new);
    let mut hunks = Vec::new();
    let mut pending: Option<HunkBuilder> = None;
    for op in diff.ops() {
        if op.tag() == similar::DiffTag::Equal {
            if let Some(builder) = pending.take() {
                hunks.push(builder.finish(base, buffer));
            }
            continue;
        }
        let old_range = op.old_range();
        let new_range = op.new_range();
        let builder = pending.get_or_insert(HunkBuilder {
            old_start: old_range.start,
            new_start: new_range.start,
            old_count: 0,
            new_count: 0,
        });
        builder.old_count += old_range.len();
        builder.new_count += new_range.len();
    }
    if let Some(builder) = pending {
        hunks.push(builder.finish(base, buffer));
    }
    hunks
}

pub fn apply_hunks(lines: &[String], hunks: &[Hunk]) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    let mut cursor = 0;
    for hunk in hunks {
        let index = if hunk.removed_count == 0 {
            hunk.removed_start
        } else {
            hunk.removed_start.saturating_sub(1)
        };
        let index = index.max(cursor).min(lines.len());
        result.extend_from_slice(&lines[cursor..index]);
        result.extend(hunk.added_lines.iter().cloned());
        cursor = index.saturating_add(hunk.removed_count).min(lines.len());
    }
    result.extend_from_slice(&lines[cursor..]);
    result
}

pub fn filter_common(staged: &[Hunk], unstaged: &[Hunk]) -> Vec<Hunk> {
    let mut filtered = Vec::new();
    let mut staged_index = 0;
    let mut unstaged_index = 0;
    while let Some(staged_hunk) = staged.get(staged_index) {
        let Some(unstaged_hunk) = unstaged.get(unstaged_index) else {
            filtered.extend_from_slice(&staged[staged_index..]);
            break;
        };
        if staged_hunk.added_start > unstaged_hunk.added_start {
            unstaged_index += 1;
        } else if staged_hunk.added_start < unstaged_hunk.added_start {
            filtered.push(staged_hunk.clone());
            staged_index += 1;
        } else {
            if !same_added(staged_hunk, unstaged_hunk) {
                filtered.push(staged_hunk.clone());
            }
            staged_index += 1;
            unstaged_index += 1;
        }
    }
    filtered
}

fn same_added(left: &Hunk, right: &Hunk) -> bool {
    left.added_start == right.added_start
        && left.added_count == right.added_count
        && left.added_lines == right.added_lines
}

pub fn hunk_at(hunks: &[Hunk], line: usize) -> Option<usize> {
    hunks.iter().position(|hunk| hunk_contains(hunk, line))
}

pub fn next_hunk(hunks: &[Hunk], line: usize) -> Option<usize> {
    if hunks.is_empty() {
        return None;
    }
    if hunks[0].start_line() > line {
        return Some(0);
    }
    for index in (0..hunks.len()).rev() {
        if hunks[index].start_line() <= line {
            if index + 1 < hunks.len() && hunks[index + 1].start_line() > line {
                return Some(index + 1);
            }
            return Some(0);
        }
    }
    Some(0)
}

pub fn prev_hunk(hunks: &[Hunk], line: usize) -> Option<usize> {
    if hunks.is_empty() {
        return None;
    }
    if hunks[hunks.len() - 1].end_line() < line {
        return Some(hunks.len() - 1);
    }
    for index in 0..hunks.len() {
        if line <= hunks[index].end_line().max(1) {
            if index > 0 && hunks[index - 1].end_line().max(1) < line {
                return Some(index - 1);
            }
            return Some(hunks.len() - 1);
        }
    }
    Some(hunks.len() - 1)
}

fn hunk_contains(hunk: &Hunk, line: usize) -> bool {
    if hunk.kind == HunkKind::Delete && hunk.added_start == 0 {
        return line == 1;
    }
    hunk.start_line() <= line && line <= hunk.end_line()
}

fn hunk_kind(removed: usize, added: usize) -> HunkKind {
    if added == 0 {
        HunkKind::Delete
    } else if removed == 0 {
        HunkKind::Add
    } else {
        HunkKind::Change
    }
}

struct HunkBuilder {
    old_start: usize,
    new_start: usize,
    old_count: usize,
    new_count: usize,
}

impl HunkBuilder {
    fn finish(self, base: &[String], buffer: &[String]) -> Hunk {
        let old_end = self.old_start.saturating_add(self.old_count);
        let new_end = self.new_start.saturating_add(self.new_count);
        Hunk {
            kind: hunk_kind(self.old_count, self.new_count),
            removed_start: if self.old_count == 0 {
                self.old_start
            } else {
                self.old_start + 1
            },
            removed_count: self.old_count,
            added_start: if self.new_count == 0 {
                self.new_start
            } else {
                self.new_start + 1
            },
            added_count: self.new_count,
            removed_lines: base[self.old_start..old_end].to_vec(),
            added_lines: buffer[self.new_start..new_end].to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    fn hunk_signs(text: &[Sign]) -> Vec<(usize, SignKind)> {
        text.iter().map(|sign| (sign.row, sign.kind)).collect()
    }

    #[test]
    fn identical_text_has_no_hunks() {
        let text = lines("a\nb\nc");
        assert!(diff_hunks(&text, &text).is_empty());
    }

    #[test]
    fn change_hunk_matches_gitsigns_coordinates() {
        let base = lines("one\ntwo\nthree");
        let buffer = lines("one\nTWO\nthree");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].kind, HunkKind::Change);
        assert_eq!(hunks[0].removed_start, 2);
        assert_eq!(hunks[0].removed_count, 1);
        assert_eq!(hunks[0].added_start, 2);
        assert_eq!(hunks[0].added_count, 1);
        assert_eq!(hunks[0].added_lines, vec!["TWO"]);
    }

    #[test]
    fn insertion_at_end_uses_previous_line_as_removed_start() {
        let base = lines("one\ntwo");
        let buffer = lines("one\ntwo\nthree");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunks[0].kind, HunkKind::Add);
        assert_eq!(hunks[0].removed_start, 2);
        assert_eq!(hunks[0].added_start, 3);
        assert_eq!(hunks[0].start_line(), 3);
    }

    #[test]
    fn deletion_in_middle_gets_a_sign_before_the_deleted_lines() {
        let base = lines("a\nb\nc\nd\ne");
        let buffer = lines("a\nd\ne");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunks[0].kind, HunkKind::Delete);
        assert_eq!(hunks[0].removed_start, 2);
        assert_eq!(hunks[0].removed_count, 2);
        assert_eq!(hunks[0].added_start, 1);
        assert_eq!(hunk_signs(&hunks[0].signs()), vec![(0, SignKind::Delete)]);
    }

    #[test]
    fn deletion_at_top_gets_a_topdelete_sign() {
        let base = lines("a\nb\nc");
        let buffer = lines("b\nc");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunks[0].kind, HunkKind::Delete);
        assert_eq!(hunks[0].added_start, 0);
        assert_eq!(
            hunk_signs(&hunks[0].signs()),
            vec![(0, SignKind::TopDelete)]
        );
    }

    #[test]
    fn deletion_at_eof_gets_a_sign_on_the_last_line() {
        let base = lines("a\nb\nc");
        let buffer = lines("a\nb");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunks[0].added_start, 2);
        assert_eq!(hunk_signs(&hunks[0].signs()), vec![(1, SignKind::Delete)]);
    }

    #[test]
    fn change_with_more_removed_lines_marks_the_last_changed_line() {
        let base = lines("a\nb\nc\nd\ne");
        let buffer = lines("a\nX\ne");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(
            hunk_signs(&hunks[0].signs()),
            vec![(1, SignKind::ChangeDelete)]
        );
    }

    #[test]
    fn change_with_more_added_lines_marks_extra_lines_as_added() {
        let base = lines("a\nb\ne");
        let buffer = lines("a\nX\nY\nZ\ne");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(
            hunk_signs(&hunks[0].signs()),
            vec![
                (1, SignKind::Change),
                (2, SignKind::Add),
                (3, SignKind::Add)
            ]
        );
    }

    #[test]
    fn apply_and_reverse_restore_the_original_lines() {
        let base = lines("a\nb\nc\nd");
        let buffer = lines("a\nX\nY\nd");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunks[0].apply(&base, false), buffer);
        assert_eq!(hunks[0].apply(&buffer, true), base);
    }

    #[test]
    fn apply_hunks_applies_every_hunk() {
        let base = lines("a\nb\nc\nd\ne");
        let buffer = lines("a\nB\nc\nD\ne\nf");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(apply_hunks(&base, &hunks), buffer);
    }

    #[test]
    fn filter_common_removes_the_same_unstaged_change() {
        let base = lines("a\nb\nc");
        let buffer = lines("a\nB\nc");
        let staged = diff_hunks(&base, &buffer);
        assert!(filter_common(&staged, &staged).is_empty());
    }

    #[test]
    fn filter_common_keeps_staged_changes() {
        let head = lines("a\nb\nc");
        let index = lines("a\nB\nc");
        let buffer = lines("a\nB\nC");
        let staged = diff_hunks(&head, &buffer);
        let unstaged = diff_hunks(&index, &buffer);
        let filtered = filter_common(&staged, &unstaged);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].added_start, 2);
    }

    #[test]
    fn next_hunk_advances_and_wraps() {
        let base = lines("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let buffer = lines("a\nB\nc\nd\ne\nf\ng\nh\nI\nj\nk");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(next_hunk(&hunks, 1), Some(0));
        assert_eq!(next_hunk(&hunks, 2), Some(1));
        assert_eq!(next_hunk(&hunks, 9), Some(2));
        assert_eq!(next_hunk(&hunks, 11), Some(0));
    }

    #[test]
    fn prev_hunk_moves_before_the_current_hunk_and_wraps() {
        let base = lines("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let buffer = lines("a\nB\nc\nd\ne\nf\ng\nh\nI\nj\nk");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(prev_hunk(&hunks, 2), Some(2));
        assert_eq!(prev_hunk(&hunks, 9), Some(0));
        assert_eq!(prev_hunk(&hunks, 11), Some(1));
        assert_eq!(prev_hunk(&hunks, 1), Some(2));
    }

    #[test]
    fn hunk_at_finds_deletion_and_addition_rows() {
        let base = lines("a\nb\nc");
        let buffer = lines("a\nc\nd");
        let hunks = diff_hunks(&base, &buffer);
        assert_eq!(hunk_at(&hunks, 1), Some(0));
        assert_eq!(hunk_at(&hunks, 2), None);
        assert_eq!(hunk_at(&hunks, 3), Some(1));
    }
}
