use ropey::Rope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RopeChange {
    pub start: usize,
    pub old_end: usize,
    pub new_end: usize,
}

pub fn diff_ropes(old: &Rope, new: &Rope) -> Option<RopeChange> {
    let prefix = common_prefix(old, new);
    let suffix = common_suffix(old, new, prefix);
    let old_end = old.len_chars().saturating_sub(suffix);
    let new_end = new.len_chars().saturating_sub(suffix);
    (prefix != old_end || prefix != new_end).then_some(RopeChange {
        start: prefix,
        old_end,
        new_end,
    })
}

fn common_prefix(old: &Rope, new: &Rope) -> usize {
    old.chars()
        .zip(new.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_suffix(old: &Rope, new: &Rope, prefix: usize) -> usize {
    let limit = old
        .len_chars()
        .saturating_sub(prefix)
        .min(new.len_chars().saturating_sub(prefix));
    old.chars_at(old.len_chars())
        .reversed()
        .zip(new.chars_at(new.len_chars()).reversed())
        .take(limit)
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(text: &str) -> Rope {
        Rope::from_str(text)
    }

    fn change(old: &str, new: &str) -> Option<RopeChange> {
        diff_ropes(&rope(old), &rope(new))
    }

    #[test]
    fn identical_ropes_have_no_change() {
        assert_eq!(change("same", "same"), None);
        assert_eq!(change("", ""), None);
    }

    #[test]
    fn detects_insertions() {
        assert_eq!(
            change("abcd", "abXcd"),
            Some(RopeChange {
                start: 2,
                old_end: 2,
                new_end: 3,
            })
        );
        assert_eq!(
            change("", "text"),
            Some(RopeChange {
                start: 0,
                old_end: 0,
                new_end: 4,
            })
        );
    }

    #[test]
    fn detects_deletions() {
        assert_eq!(
            change("abXcd", "abcd"),
            Some(RopeChange {
                start: 2,
                old_end: 3,
                new_end: 2,
            })
        );
        assert_eq!(
            change("text", ""),
            Some(RopeChange {
                start: 0,
                old_end: 4,
                new_end: 0,
            })
        );
    }

    #[test]
    fn detects_replacements() {
        assert_eq!(
            change("abcdef", "abXYef"),
            Some(RopeChange {
                start: 2,
                old_end: 4,
                new_end: 4,
            })
        );
    }

    #[test]
    fn supports_multiline_and_unicode_edits() {
        assert_eq!(
            change("a\nb\nc", "a\nbx\nc"),
            Some(RopeChange {
                start: 3,
                old_end: 3,
                new_end: 4,
            })
        );
        assert_eq!(
            change("a𝄞b", "a𝄢b"),
            Some(RopeChange {
                start: 1,
                old_end: 2,
                new_end: 2,
            })
        );
    }

    #[test]
    fn covers_the_unchanged_middle_between_edits() {
        assert_eq!(
            change("abcdefg", "abXdefYg"),
            Some(RopeChange {
                start: 2,
                old_end: 6,
                new_end: 7,
            })
        );
    }

    #[test]
    fn handles_edits_at_both_ends() {
        assert_eq!(
            change("abc", "XbcY"),
            Some(RopeChange {
                start: 0,
                old_end: 3,
                new_end: 4,
            })
        );
    }

    #[test]
    fn applies_a_change_back_to_the_new_rope() {
        let old = rope("alpha\nbeta\ngamma\n");
        let new = rope("alpha\nBETA\ngamma\n");
        let diff = diff_ropes(&old, &new).expect("diff");
        let mut chars: Vec<char> = old.chars().collect();
        let inserted: Vec<char> = new
            .chars_at(diff.start)
            .take(diff.new_end.saturating_sub(diff.start))
            .collect();
        chars.splice(diff.start..diff.old_end, inserted);
        assert_eq!(chars.into_iter().collect::<String>(), new.to_string());
    }
}
