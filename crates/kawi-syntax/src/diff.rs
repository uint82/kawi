use ropey::Rope;

use crate::span::SyntaxEdit;

pub(crate) fn diff_edit(old: &Rope, new: &Rope) -> Option<SyntaxEdit> {
    let prefix = common_prefix(old, new);
    let suffix = common_suffix(old, new, prefix);
    let old_end = old.len_chars().saturating_sub(suffix);
    let new_end = new.len_chars().saturating_sub(suffix);
    (prefix != old_end || prefix != new_end).then(|| SyntaxEdit {
        start: prefix,
        end: old_end,
        text: new.slice(prefix..new_end).to_string(),
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

    fn edit(old: &str, new: &str) -> Option<SyntaxEdit> {
        diff_edit(&rope(old), &rope(new))
    }

    #[test]
    fn identical_ropes_have_no_edit() {
        assert_eq!(edit("same", "same"), None);
        assert_eq!(edit("", ""), None);
    }

    #[test]
    fn detects_insertions() {
        assert_eq!(
            edit("abcd", "abXcd"),
            Some(SyntaxEdit {
                start: 2,
                end: 2,
                text: "X".to_string(),
            })
        );
        assert_eq!(
            edit("", "text"),
            Some(SyntaxEdit {
                start: 0,
                end: 0,
                text: "text".to_string(),
            })
        );
    }

    #[test]
    fn detects_deletions() {
        assert_eq!(
            edit("abXcd", "abcd"),
            Some(SyntaxEdit {
                start: 2,
                end: 3,
                text: String::new(),
            })
        );
    }

    #[test]
    fn detects_replacements() {
        assert_eq!(
            edit("abcdef", "abXYef"),
            Some(SyntaxEdit {
                start: 2,
                end: 4,
                text: "XY".to_string(),
            })
        );
    }

    #[test]
    fn supports_multiline_and_unicode_edits() {
        assert_eq!(
            edit("a\nb\nc", "a\nbx\nc"),
            Some(SyntaxEdit {
                start: 3,
                end: 3,
                text: "x".to_string(),
            })
        );
        assert_eq!(
            edit("a𝄞b", "a𝄢b"),
            Some(SyntaxEdit {
                start: 1,
                end: 2,
                text: "𝄢".to_string(),
            })
        );
    }

    #[test]
    fn covers_the_unchanged_middle_between_edits() {
        assert_eq!(
            edit("abcdefg", "abXdefYg"),
            Some(SyntaxEdit {
                start: 2,
                end: 6,
                text: "XdefY".to_string(),
            })
        );
    }

    #[test]
    fn applies_the_edit_back_to_the_new_rope() {
        let old = rope("alpha\nbeta\ngamma\n");
        let new = rope("alpha\nBETA\ngamma\n");
        let edit = diff_edit(&old, &new).expect("edit");
        let mut text = old;
        text.remove(edit.start..edit.end);
        text.insert(edit.start, &edit.text);
        assert_eq!(text, new);
    }
}
