use super::Editor;
use crate::actions::{Action, ShiftMotion};
use crate::editing::{BlockComment, CommentTokens};

struct RowChange {
    row: usize,
    start: usize,
    removed: usize,
    inserted: String,
    suffix_start: usize,
    suffix_removed: usize,
    suffix_inserted: String,
}

impl RowChange {
    fn insertion(row: usize, start: usize, text: String) -> Self {
        Self {
            row,
            start,
            removed: 0,
            inserted: text,
            suffix_start: usize::MAX,
            suffix_removed: 0,
            suffix_inserted: String::new(),
        }
    }

    fn removal(row: usize, start: usize, removed: usize) -> Self {
        Self {
            row,
            start,
            removed,
            inserted: String::new(),
            suffix_start: usize::MAX,
            suffix_removed: 0,
            suffix_inserted: String::new(),
        }
    }

    fn block_wrap(row: usize, start: usize, suffix: usize, prefix: String, close: String) -> Self {
        Self {
            row,
            start,
            removed: 0,
            inserted: prefix,
            suffix_start: suffix,
            suffix_removed: 0,
            suffix_inserted: close,
        }
    }

    fn block_unwrap(
        row: usize,
        start: usize,
        prefix_removed: usize,
        suffix_start: usize,
        suffix_removed: usize,
    ) -> Self {
        Self {
            row,
            start,
            removed: prefix_removed,
            inserted: String::new(),
            suffix_start,
            suffix_removed,
            suffix_inserted: String::new(),
        }
    }
}

impl Editor {
    pub(super) fn comment_lines(&mut self) {
        let (first, last) = if self.mode.is_visual() {
            self.note_visual_repeat(Action::CommentLines);
            let rows = self.touched_rows();
            self.exit_visual();
            rows
        } else {
            let count = self.take_count();
            let row = self.cursor.row();
            let Some(last) = self.counted_last_row(count) else {
                return;
            };
            (row, last)
        };
        self.comment_rows(first, last);
    }

    pub(super) fn comment_at(&mut self, motion: ShiftMotion) {
        let Some((first, last)) = self.line_motion_rows(motion) else {
            return;
        };
        self.comment_rows(first, last);
    }

    pub(super) fn comment_rows(&mut self, first: usize, last: usize) {
        let last = last.min(self.buffer.line_count().saturating_sub(1));
        let first = first.min(last);
        // TODO: resolve embedded language tokens through tree-sitter injections.
        let tokens = self.language_editing.comments.clone();
        let changes = self.comment_plan(first, last, &tokens);
        if changes.is_empty() {
            return;
        }
        let cursor_row = self.cursor.row();
        let cursor_col = self.cursor.col();
        let mut cursor = None;
        let mut changes = changes.iter().peekable();
        let mut lines = Vec::with_capacity(last - first + 1);
        for row in first..=last {
            let line = self.buffer.line(row).unwrap_or_default();
            let change = changes.next_if(|change| change.row == row);
            if let Some(change) = change {
                if row == cursor_row {
                    cursor = Some(adjusted_column(cursor_col, change));
                }
                lines.push(apply_change(&line, change));
            } else {
                lines.push(line);
            }
        }
        self.record_boundary();
        self.buffer.replace_lines(first, last, &lines.join("\n"));
        if let Some(col) = cursor {
            self.cursor.set_position(cursor_row, col);
        }
    }

    fn comment_plan(&self, first: usize, last: usize, tokens: &CommentTokens) -> Vec<RowChange> {
        let Some(style) = CommentStyle::from_tokens(tokens) else {
            return Vec::new();
        };
        let rows: Vec<usize> = (first..=last)
            .filter(|row| !self.buffer.line(*row).unwrap_or_default().trim().is_empty())
            .collect();
        if rows.is_empty() {
            return Vec::new();
        }
        let uncomment = rows.iter().all(|row| {
            let line = self.buffer.line(*row).unwrap_or_default();
            style.is_commented(&line)
        });
        if uncomment {
            rows.into_iter()
                .filter_map(|row| {
                    let line = self.buffer.line(row).unwrap_or_default();
                    style.uncomment_change(row, &line)
                })
                .collect()
        } else {
            let indent = self.common_indent(&rows);
            rows.into_iter()
                .filter_map(|row| {
                    let line = self.buffer.line(row).unwrap_or_default();
                    style.comment_change(row, &line, indent)
                })
                .collect()
        }
    }

    fn common_indent(&self, rows: &[usize]) -> usize {
        let mut common: Option<Vec<char>> = None;
        for row in rows {
            let line = self.buffer.line(*row).unwrap_or_default();
            let indent: Vec<char> = line.chars().take_while(|ch| ch.is_whitespace()).collect();
            common = Some(match common {
                None => indent,
                Some(previous) => previous
                    .iter()
                    .zip(indent.iter())
                    .take_while(|(left, right)| left == right)
                    .map(|(ch, _)| *ch)
                    .collect(),
            });
        }
        common.map_or(0, |prefix| prefix.len())
    }
}

enum CommentStyle {
    Line(String),
    Block(BlockComment),
}

impl CommentStyle {
    fn from_tokens(tokens: &CommentTokens) -> Option<Self> {
        if let Some(line) = tokens.line.as_ref().filter(|token| !token.is_empty()) {
            return Some(Self::Line(line.clone()));
        }
        tokens
            .block
            .as_ref()
            .filter(|block| !block.open.is_empty() && !block.close.is_empty())
            .map(|block| Self::Block(block.clone()))
    }

    fn is_commented(&self, line: &str) -> bool {
        match self {
            Self::Line(token) => line_comment_span(line, token).is_some(),
            Self::Block(block) => block_comment_span(line, block).is_some(),
        }
    }

    fn comment_change(&self, row: usize, line: &str, indent: usize) -> Option<RowChange> {
        if line.chars().count() < indent {
            return None;
        }
        match self {
            Self::Line(token) => Some(RowChange::insertion(row, indent, format!("{token} "))),
            Self::Block(block) => Some(RowChange::block_wrap(
                row,
                indent,
                line.chars().count(),
                format!("{} ", block.open),
                format!(" {}", block.close),
            )),
        }
    }

    fn uncomment_change(&self, row: usize, line: &str) -> Option<RowChange> {
        match self {
            Self::Line(token) => {
                let (start, end) = line_comment_span(line, token)?;
                Some(RowChange::removal(row, start, end - start))
            }
            Self::Block(block) => {
                let (start, prefix_end, close_start, trimmed_end) =
                    block_comment_span(line, block)?;
                Some(RowChange::block_unwrap(
                    row,
                    start,
                    prefix_end - start,
                    close_start,
                    trimmed_end - close_start,
                ))
            }
        }
    }
}

fn line_comment_span(line: &str, token: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let token: Vec<char> = token.chars().collect();
    let indent = indent_len(&chars);
    if !chars.get(indent..)?.starts_with(&token) {
        return None;
    }
    let mut end = indent + token.len();
    if token.ends_with(&['/']) {
        while matches!(chars.get(end), Some('/' | '!')) {
            end += 1;
        }
    }
    if chars.get(end) == Some(&' ') {
        end += 1;
    }
    Some((indent, end))
}

fn block_comment_span(line: &str, block: &BlockComment) -> Option<(usize, usize, usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let open: Vec<char> = block.open.chars().collect();
    let close: Vec<char> = block.close.chars().collect();
    let indent = indent_len(&chars);
    if !chars.get(indent..)?.starts_with(&open) {
        return None;
    }
    let trimmed_end = chars
        .iter()
        .rposition(|ch| !ch.is_whitespace())
        .map_or(0, |index| index + 1);
    if trimmed_end < close.len() || !chars[..trimmed_end].ends_with(&close) {
        return None;
    }
    let mut prefix_end = indent + open.len();
    if open.ends_with(&['*']) {
        while matches!(chars.get(prefix_end), Some('*' | '!')) {
            prefix_end += 1;
        }
    }
    if chars.get(prefix_end) == Some(&' ') {
        prefix_end += 1;
    }
    let mut close_start = trimmed_end - close.len();
    if close_start > prefix_end && chars.get(close_start - 1) == Some(&' ') {
        close_start -= 1;
    }
    if close_start < prefix_end {
        return None;
    }
    Some((indent, prefix_end, close_start, trimmed_end))
}

fn indent_len(chars: &[char]) -> usize {
    chars.iter().take_while(|ch| ch.is_whitespace()).count()
}

fn apply_change(line: &str, change: &RowChange) -> String {
    let mut chars: Vec<char> = line.chars().collect();
    if change.suffix_start != usize::MAX {
        let suffix_start = change.suffix_start.min(chars.len());
        let suffix_end = suffix_start
            .saturating_add(change.suffix_removed)
            .min(chars.len());
        chars.splice(suffix_start..suffix_end, change.suffix_inserted.chars());
    }
    let start = change.start.min(chars.len());
    let end = start.saturating_add(change.removed).min(chars.len());
    chars.splice(start..end, change.inserted.chars());
    chars.into_iter().collect()
}

fn adjusted_column(col: usize, change: &RowChange) -> usize {
    let mut col = col;
    if change.suffix_removed > 0 && col >= change.suffix_start {
        col = if col >= change.suffix_start.saturating_add(change.suffix_removed) {
            col.saturating_sub(change.suffix_removed)
        } else {
            change.suffix_start
        };
    }
    let inserted = change.inserted.chars().count();
    if col < change.start {
        return col;
    }
    if col < change.start.saturating_add(change.removed) {
        return change.start + inserted;
    }
    col.saturating_add(inserted).saturating_sub(change.removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{BlockComment, CommentTokens, LanguageEditing};
    use kawi_buffer::Buffer;

    fn editor_with(text: &str, tokens: CommentTokens) -> Editor {
        let mut editor = Editor::new(Buffer::from_string(text));
        editor.set_language_editing(LanguageEditing {
            comments: tokens,
            pairs: Vec::new(),
        });
        editor
    }

    fn line_comments() -> CommentTokens {
        CommentTokens {
            line: Some("//".to_string()),
            block: Some(BlockComment::new("/*", "*/")),
        }
    }

    fn block_comments() -> CommentTokens {
        CommentTokens {
            line: None,
            block: Some(BlockComment::new("<!--", "-->")),
        }
    }

    #[test]
    fn line_toggle_uses_common_indent() {
        let mut editor = editor_with("  one\n    two\n", line_comments());
        editor.comment_rows(0, 1);
        assert_eq!(
            editor.text_lines(),
            vec!["  // one".to_string(), "  //   two".to_string()]
        );
        editor.comment_rows(0, 1);
        assert_eq!(
            editor.text_lines(),
            vec!["  one".to_string(), "    two".to_string()]
        );
    }

    #[test]
    fn blank_lines_are_skipped() {
        let mut editor = editor_with("one\n\n  \ntwo\n", line_comments());
        editor.comment_rows(0, 3);
        assert_eq!(
            editor.text_lines(),
            vec![
                "// one".to_string(),
                String::new(),
                "  ".to_string(),
                "// two".to_string()
            ]
        );
        editor.comment_rows(0, 3);
        assert_eq!(
            editor.text_lines(),
            vec![
                "one".to_string(),
                String::new(),
                "  ".to_string(),
                "two".to_string()
            ]
        );
    }

    #[test]
    fn doc_comments_uncomment_without_stray_markers() {
        let mut editor = editor_with("/// docs\n//! module\n// plain\n", line_comments());
        editor.comment_rows(0, 2);
        assert_eq!(
            editor.text_lines(),
            vec![
                "docs".to_string(),
                "module".to_string(),
                "plain".to_string()
            ]
        );
    }

    #[test]
    fn block_doc_comments_uncomment_without_stray_markers() {
        let tokens = CommentTokens {
            line: None,
            block: Some(BlockComment::new("/*", "*/")),
        };
        let mut editor = editor_with("/** docs */\n/*! module */\n", tokens);
        editor.comment_rows(0, 1);
        assert_eq!(
            editor.text_lines(),
            vec!["docs".to_string(), "module".to_string()]
        );
    }

    #[test]
    fn mixed_comments_comment_every_line() {
        let mut editor = editor_with("/// docs\nplain\n", line_comments());
        editor.comment_rows(0, 1);
        assert_eq!(
            editor.text_lines(),
            vec!["// /// docs".to_string(), "// plain".to_string()]
        );
    }

    #[test]
    fn block_only_languages_wrap_each_line() {
        let mut editor = editor_with("  <div>\n    text\n", block_comments());
        editor.comment_rows(0, 1);
        assert_eq!(
            editor.text_lines(),
            vec![
                "  <!-- <div> -->".to_string(),
                "  <!--   text -->".to_string()
            ]
        );
        editor.comment_rows(0, 1);
        assert_eq!(
            editor.text_lines(),
            vec!["  <div>".to_string(), "    text".to_string()]
        );
    }

    #[test]
    fn block_toggle_handles_unicode_content() {
        let mut editor = editor_with("  <p>héllo wörld</p>\n", block_comments());
        editor.comment_rows(0, 0);
        assert_eq!(
            editor.text_lines(),
            vec!["  <!-- <p>héllo wörld</p> -->".to_string()]
        );
        editor.comment_rows(0, 0);
        assert_eq!(
            editor.text_lines(),
            vec!["  <p>héllo wörld</p>".to_string()]
        );
    }

    #[test]
    fn cursor_shifts_with_inserted_and_removed_prefix() {
        let mut editor = editor_with("  one\n", line_comments());
        editor.cursor.set_position(0, 4);
        editor.comment_rows(0, 0);
        assert_eq!((editor.row(), editor.col()), (0, 7));
        editor.comment_rows(0, 0);
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }

    #[test]
    fn toggle_is_one_undo_step() {
        let mut editor = editor_with("one\ntwo\n", line_comments());
        editor.comment_rows(0, 1);
        editor.apply(Action::Undo);
        assert_eq!(
            editor.text_lines(),
            vec!["one".to_string(), "two".to_string()]
        );
    }

    #[test]
    fn missing_tokens_are_a_noop() {
        let mut editor = editor_with("one\n", CommentTokens::default());
        editor.comment_rows(0, 0);
        assert_eq!(editor.text_lines(), vec!["one".to_string()]);
    }

    #[test]
    fn counted_comment_lines_toggle_multiple_rows() {
        let mut editor = editor_with("one\ntwo\nthree\nfour\n", line_comments());
        editor.apply(Action::PushCount(3));
        editor.apply(Action::CommentLines);
        assert_eq!(
            editor.text_lines(),
            vec![
                "// one".to_string(),
                "// two".to_string(),
                "// three".to_string(),
                "four".to_string()
            ]
        );
    }

    #[test]
    fn paragraph_motion_comments_the_touched_lines() {
        let mut editor = editor_with("one\ntwo\n\nthree\n", line_comments());
        editor.apply(Action::CommentAt {
            motion: crate::actions::ShiftMotion::Object {
                inner: false,
                object: crate::actions::Object::Paragraph,
            },
        });
        assert_eq!(
            editor.text_lines(),
            vec![
                "// one".to_string(),
                "// two".to_string(),
                String::new(),
                "three".to_string()
            ]
        );
    }

    #[test]
    fn normal_comment_repeats_with_dot() {
        let mut editor = editor_with("one\ntwo\n", line_comments());
        editor.apply(Action::CommentLines);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(
            editor.text_lines(),
            vec!["// one".to_string(), "// two".to_string()]
        );
    }

    #[test]
    fn visual_comment_exits_visual_and_repeats() {
        let mut editor = editor_with("one\ntwo\nthree\n", line_comments());
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::CommentLines);
        assert_eq!(
            editor.text_lines(),
            vec![
                "// one".to_string(),
                "// two".to_string(),
                "three".to_string()
            ]
        );
        assert_eq!(editor.mode(), crate::mode::Mode::Normal);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(
            editor.text_lines(),
            vec![
                "// one".to_string(),
                "// two".to_string(),
                "// three".to_string()
            ]
        );
    }
}
