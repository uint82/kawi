use super::substitute::substitute_line;
use super::Editor;
use crate::command::parse_preview;
use crate::matcher::Query;
use crate::mode::Mode;
use crate::search::MatchSpan;

pub(super) struct CommandPreview {
    pub(super) lines: Vec<(usize, String)>,
    pub(super) highlights: Vec<(usize, usize, usize)>,
}

impl Editor {
    pub(super) fn refresh_command_preview(&mut self) {
        self.command_preview = self.build_command_preview();
    }

    pub(super) fn clear_command_preview(&mut self) {
        self.command_preview = None;
    }

    pub fn command_preview_active(&self) -> bool {
        self.command_preview.is_some()
    }

    pub fn preview_spans(&self, top: usize, height: usize) -> Vec<MatchSpan> {
        let Some(preview) = self.command_preview.as_ref() else {
            return Vec::new();
        };
        preview
            .highlights
            .iter()
            .filter(|(row, _, _)| *row >= top && *row < top.saturating_add(height))
            .map(|(row, start, end)| MatchSpan {
                row: *row,
                start: *start,
                end: *end,
                current: false,
            })
            .collect()
    }

    fn build_command_preview(&self) -> Option<CommandPreview> {
        if self.mode != Mode::Command {
            return None;
        }
        let input = parse_preview(self.command_text())?;
        let settings = self.substitute_settings(&input.flags);
        if settings.count_only {
            return None;
        }
        let (first, last) = self
            .resolve_substitute_span(input.range.as_ref(), input.count, "")
            .ok()?;
        let repeat = input.pattern.is_none();
        let pattern = self
            .resolve_substitute_pattern(input.pattern.as_deref(), repeat, settings)
            .ok()?;
        let query = self.substitute_query(&pattern, settings).ok()?;
        match input.replacement {
            None => self.pattern_preview(first, last, &query),
            Some(replacement) => {
                self.replacement_preview(first, last, &query, &replacement, settings)
            }
        }
    }

    fn pattern_preview(&self, first: usize, last: usize, query: &Query) -> Option<CommandPreview> {
        let mut highlights = Vec::new();
        for row in first..=last {
            let Some(line) = self.buffer.line(row) else {
                break;
            };
            let Some((start, end)) = query.spans_in_line(&line).into_iter().next() else {
                continue;
            };
            highlights.push((row, start, end));
        }
        if highlights.is_empty() {
            return None;
        }
        Some(CommandPreview {
            lines: Vec::new(),
            highlights,
        })
    }

    fn replacement_preview(
        &self,
        first: usize,
        last: usize,
        query: &Query,
        replacement: &str,
        settings: super::substitute::SubstituteSettings,
    ) -> Option<CommandPreview> {
        let mut lines = Vec::new();
        let mut highlights = Vec::new();
        for row in first..=last {
            let Some(line) = self.buffer.line(row) else {
                break;
            };
            let spans = query.spans_in_line(&line);
            let chosen: Vec<(usize, usize)> = if settings.all {
                spans
            } else {
                spans.into_iter().take(1).collect()
            };
            if chosen.is_empty() {
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            let result =
                substitute_line(&chars, &chosen, replacement, &self.substitute_replacement);
            if result.text.contains('\n') || result.text.contains('\0') {
                return None;
            }
            if result.text != line {
                lines.push((row, result.text));
            }
            for (start, end) in result.inserted {
                highlights.push((row, start, end));
            }
        }
        if lines.is_empty() && highlights.is_empty() {
            return None;
        }
        Some(CommandPreview { lines, highlights })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn command_editor(text: &str, command: &str) -> Editor {
        let mut editor = editor_with(text);
        editor.apply(Action::EnterCommand);
        for ch in command.chars() {
            editor.apply(Action::PushCommandChar(ch));
        }
        editor
    }

    fn visible(editor: &Editor) -> Vec<String> {
        editor.visible_lines(0, editor.buffer.line_count())
    }

    fn spans(editor: &Editor) -> Vec<(usize, usize, usize)> {
        editor
            .preview_spans(0, editor.buffer.line_count())
            .into_iter()
            .map(|span| (span.row, span.start, span.end))
            .collect()
    }

    #[test]
    fn pattern_phase_highlights_first_match_per_line() {
        let editor = command_editor("apple pie apple\napple cake\n", "%s/apple");
        assert!(editor.command_preview_active());
        assert_eq!(visible(&editor), ["apple pie apple", "apple cake"]);
        assert_eq!(spans(&editor), vec![(0, 0, 5), (1, 0, 5)]);
    }

    #[test]
    fn pattern_phase_partial_pattern() {
        let editor = command_editor("apple pie apple\n", "s/app");
        assert_eq!(spans(&editor), vec![(0, 0, 3)]);
    }

    #[test]
    fn replacement_phase_previews_text_and_insertion() {
        let mut editor = editor_with("apple pie apple\n");
        editor.apply(Action::EnterCommand);
        for ch in "s/apple/orange/".chars() {
            editor.apply(Action::PushCommandChar(ch));
        }
        assert_eq!(visible(&editor), ["orange pie apple"]);
        assert_eq!(spans(&editor), vec![(0, 0, 6)]);
        assert_eq!(editor.buffer.line(0).as_deref(), Some("apple pie apple"));
    }

    #[test]
    fn replacement_phase_updates_char_by_char() {
        for (command, expected) in [
            ("s/apple/o", "o pie apple"),
            ("s/apple/or", "or pie apple"),
            ("s/apple/orange", "orange pie apple"),
        ] {
            let editor = command_editor("apple pie apple\n", command);
            assert_eq!(visible(&editor), [expected], "{command}");
        }
    }

    #[test]
    fn empty_replacement_previews_deletion_without_highlight() {
        let editor = command_editor("apple pie apple\n", "s/apple/");
        assert_eq!(visible(&editor), [" pie apple"]);
        assert!(spans(&editor).is_empty());
        assert!(editor.command_preview_active());
    }

    #[test]
    fn global_flag_previews_every_match() {
        let editor = command_editor("apple pie apple\n", "s/apple/X/g");
        assert_eq!(visible(&editor), ["X pie X"]);
        assert_eq!(spans(&editor), vec![(0, 0, 1), (0, 6, 7)]);
    }

    #[test]
    fn ignore_case_flag_updates_preview() {
        let editor = command_editor("APPLE pie\n", "s/apple/X/");
        assert!(!editor.command_preview_active());
        let editor = command_editor("APPLE pie\n", "s/apple/X/i");
        assert_eq!(visible(&editor), ["X pie"]);
    }

    #[test]
    fn count_only_flag_disables_preview() {
        let editor = command_editor("apple\n", "s/apple/X/n");
        assert!(!editor.command_preview_active());
    }

    #[test]
    fn counted_range_selects_preview_lines() {
        let editor = command_editor("apple\napple\napple\napple\n", "%s/apple/X/ 2");
        assert_eq!(visible(&editor), ["apple", "apple", "apple", "X"]);
        assert_eq!(spans(&editor), vec![(3, 0, 1)]);
    }

    #[test]
    fn visual_range_previews_selected_lines() {
        let mut editor = editor_with("apple\napple\napple\n");
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::EnterCommand);
        assert_eq!(editor.command_text(), "'<,'>");
        for ch in "s/apple/X/".chars() {
            editor.apply(Action::PushCommandChar(ch));
        }
        assert_eq!(visible(&editor), ["X", "X", "apple"]);
    }

    #[test]
    fn visual_command_enters_at_selection_start() {
        let mut editor = editor_with("apple\napple\napple\n");
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::EnterCommand);
        assert_eq!((editor.row(), editor.col()), (0, 0));
        editor.apply(Action::CancelCommand);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn cancel_command_restores_lines() {
        let mut editor = command_editor("apple pie apple\n", "s/apple/orange/");
        assert_eq!(visible(&editor), ["orange pie apple"]);
        editor.apply(Action::CancelCommand);
        assert!(!editor.command_preview_active());
        assert_eq!(visible(&editor), ["apple pie apple"]);
    }

    #[test]
    fn invalid_command_has_no_preview() {
        for command in ["s/apple/orange/z", "s//X/", "set number", "s/apple"] {
            let editor = command_editor("apple pie apple\n", command);
            if command == "s/apple" {
                assert!(editor.command_preview_active());
            } else {
                assert!(!editor.command_preview_active(), "{command}");
            }
        }
    }

    #[test]
    fn newline_replacement_has_no_preview() {
        let editor = command_editor("apple pie\n", "s/apple/a\\rb/");
        assert!(!editor.command_preview_active());
    }

    #[test]
    fn repeat_command_previews_stored_pattern() {
        let mut editor = editor_with("apple pie apple\n");
        let command = match crate::command::parse("s/apple/X/") {
            crate::command::Command::Substitute(command) => command,
            _ => panic!("expected substitute"),
        };
        editor.substitute_command(&command).unwrap();
        editor.apply(Action::EnterCommand);
        editor.apply(Action::PushCommandChar('s'));
        assert!(editor.command_preview_active());
        assert_eq!(visible(&editor), ["X pie apple"]);
        assert_eq!(spans(&editor), vec![(0, 6, 11)]);
        editor.apply(Action::CancelCommand);
    }
}
