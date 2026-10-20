use super::Editor;
use crate::command::{
    SubstituteAddress, SubstituteCommand, SubstituteFlag, SubstituteFlags, SubstituteForm,
    SubstituteRange,
};
use crate::matcher::Query;
use crate::search::strip_word_atoms;

const SUBSTITUTE_REPORT_THRESHOLD: usize = 2;

#[derive(Debug, Clone, Copy)]
pub(super) struct SubstituteSettings {
    pub all: bool,
    pub ask: bool,
    pub ignore_case: Option<bool>,
    pub error: bool,
    pub count_only: bool,
    pub use_last_search: bool,
    pub print: bool,
    pub number: bool,
    pub list: bool,
}

impl Default for SubstituteSettings {
    fn default() -> Self {
        Self {
            all: false,
            ask: false,
            ignore_case: None,
            error: true,
            count_only: false,
            use_last_search: false,
            print: false,
            number: false,
            list: false,
        }
    }
}

struct SubstituteRun<'a> {
    query: Query,
    pattern: &'a str,
    replacement: &'a str,
    previous_replacement: &'a str,
    settings: SubstituteSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaseMode {
    None,
    Upper,
    Lower,
}

impl Editor {
    pub fn substitute_command(
        &mut self,
        command: &SubstituteCommand,
    ) -> Result<Option<String>, String> {
        let (first, last) = self.resolve_substitute_span(command)?;
        let settings = self.substitute_settings(&command.flags);
        if settings.ask {
            return Err("The c flag is not supported yet".to_string());
        }
        let pattern = self.substitute_pattern(command, settings)?;
        let (literal, whole_word) = strip_word_atoms(&pattern);
        let literal = unescape_substitute_pattern(&literal);
        if literal.is_empty() {
            return Err(format!("E486: Pattern not found: {pattern}"));
        }
        let replacement = match &command.form {
            SubstituteForm::New { replacement, .. } => replacement.clone(),
            SubstituteForm::Repeat => self.substitute_replacement.clone(),
        };
        if replacement.starts_with("\\=") {
            return Err("The expression replacement is not supported yet".to_string());
        }
        let previous_replacement = self.substitute_replacement.clone();
        self.search_last = pattern.clone();
        self.substitute_pattern = pattern.clone();
        self.substitute_replacement = replacement.clone();
        self.substitute_settings = settings;
        let sensitive = match settings.ignore_case {
            Some(ignore_case) => !ignore_case,
            None => self.search_sensitive(&pattern),
        };
        let run = SubstituteRun {
            query: Query::new(&literal, sensitive, whole_word),
            pattern: &pattern,
            replacement: &replacement,
            previous_replacement: &previous_replacement,
            settings,
        };
        self.run_substitute(first, last, &run)
    }

    fn substitute_settings(&self, flags: &SubstituteFlags) -> SubstituteSettings {
        let mut settings = if flags.reuse {
            self.substitute_settings
        } else {
            SubstituteSettings::default()
        };
        for flag in &flags.ops {
            match flag {
                SubstituteFlag::All => settings.all = !settings.all,
                SubstituteFlag::Ask => settings.ask = !settings.ask,
                SubstituteFlag::CountOnly => settings.count_only = true,
                SubstituteFlag::Error => settings.error = !settings.error,
                SubstituteFlag::LastSearch => settings.use_last_search = true,
                SubstituteFlag::Print => settings.print = true,
                SubstituteFlag::Number => {
                    settings.print = true;
                    settings.number = true;
                }
                SubstituteFlag::List => {
                    settings.print = true;
                    settings.list = true;
                }
                SubstituteFlag::IgnoreCase => settings.ignore_case = Some(true),
                SubstituteFlag::MatchCase => settings.ignore_case = Some(false),
            }
        }
        if settings.count_only {
            settings.ask = false;
        }
        settings
    }

    fn resolve_substitute_span(
        &self,
        command: &SubstituteCommand,
    ) -> Result<(usize, usize), String> {
        let last_line = self.buffer.line_count().saturating_sub(1);
        let cursor_row = self.cursor.row();
        let (first, last) = match &command.range {
            None => (cursor_row, cursor_row),
            Some(SubstituteRange::WholeFile) => (0, last_line),
            Some(SubstituteRange::Visual) => self
                .substitute_visual_rows()
                .ok_or_else(|| "E20: Mark not set".to_string())?,
            Some(SubstituteRange::Single(address)) => {
                let row = self.substitute_address(address, cursor_row, last_line)?;
                (row, row)
            }
            Some(SubstituteRange::Pair {
                first,
                second,
                semicolon,
            }) => {
                let first_row = self.substitute_address(first, cursor_row, last_line)?;
                let base = if *semicolon { first_row } else { cursor_row };
                let second_row = self.substitute_address(second, base, last_line)?;
                (first_row, second_row)
            }
        };
        if first > last {
            return Err(format!(
                "E493: Backwards range given: {},{}{}",
                first + 1,
                last + 1,
                command.raw
            ));
        }
        let Some(count) = command.count else {
            return Ok((first, last));
        };
        let first = last;
        let last = last.saturating_add(count.saturating_sub(1)).min(last_line);
        Ok((first, last))
    }

    fn substitute_address(
        &self,
        address: &SubstituteAddress,
        base: usize,
        last_line: usize,
    ) -> Result<usize, String> {
        let row = match address {
            SubstituteAddress::Current => self.cursor.row(),
            SubstituteAddress::Last => last_line,
            SubstituteAddress::Line(line) => {
                if *line == 0 {
                    0
                } else {
                    line.saturating_sub(1).min(last_line)
                }
            }
            SubstituteAddress::Relative(offset) => (base as isize)
                .saturating_add(*offset)
                .clamp(0, last_line as isize)
                as usize,
            SubstituteAddress::WithOffset {
                base: inner,
                offset,
            } => {
                let row = self.substitute_address(inner, base, last_line)?;
                (row as isize)
                    .saturating_add(*offset)
                    .clamp(0, last_line as isize) as usize
            }
            SubstituteAddress::Mark('<') => self
                .substitute_visual_rows()
                .map(|(first, _)| first)
                .ok_or_else(|| "E20: Mark not set".to_string())?,
            SubstituteAddress::Mark('>') => self
                .substitute_visual_rows()
                .map(|(_, last)| last)
                .ok_or_else(|| "E20: Mark not set".to_string())?,
            SubstituteAddress::Mark(name) => self
                .marks
                .local(self.buffers.current(), *name)
                .map(|position| position.row)
                .ok_or_else(|| "E20: Mark not set".to_string())?,
        };
        Ok(row)
    }

    fn substitute_visual_rows(&self) -> Option<(usize, usize)> {
        let saved = self.previous_visual?;
        let selection = saved.selection();
        let first = selection.start().0;
        let last = selection.end().0;
        Some((first.min(last), first.max(last)))
    }

    fn substitute_pattern(
        &self,
        command: &SubstituteCommand,
        settings: SubstituteSettings,
    ) -> Result<String, String> {
        match &command.form {
            SubstituteForm::New { pattern, .. } => {
                if pattern.is_empty() {
                    if self.search_last.is_empty() {
                        Err("E35: No previous regular expression".to_string())
                    } else {
                        Ok(self.search_last.clone())
                    }
                } else {
                    Ok(pattern.clone())
                }
            }
            SubstituteForm::Repeat => {
                let pattern = if settings.use_last_search {
                    &self.search_last
                } else {
                    &self.substitute_pattern
                };
                if pattern.is_empty() {
                    Err("E33: No previous substitute regular expression".to_string())
                } else {
                    Ok(pattern.clone())
                }
            }
        }
    }

    fn run_substitute(
        &mut self,
        first: usize,
        last: usize,
        run: &SubstituteRun,
    ) -> Result<Option<String>, String> {
        let mut substitutions = 0usize;
        let mut changed_lines = 0usize;
        let settings = run.settings;
        let mut offset = 0isize;
        let mut last_row = None;
        let mut first_changed_row = None;
        let mut checkpointed = false;
        for original_row in first..=last {
            let row = (original_row as isize).saturating_add(offset);
            let Some(line) = usize::try_from(row)
                .ok()
                .and_then(|row| self.buffer.line(row))
            else {
                break;
            };
            let spans = run.query.spans_in_line(&line);
            if spans.is_empty() {
                continue;
            }
            let chosen: Vec<(usize, usize)> = if settings.all {
                spans
            } else {
                spans.into_iter().take(1).collect()
            };
            substitutions += chosen.len();
            changed_lines += 1;
            if settings.count_only {
                continue;
            }
            let row = row as usize;
            if !checkpointed {
                self.record_boundary_at(row, 0);
                first_changed_row = Some(row);
                checkpointed = true;
            }
            let chars: Vec<char> = line.chars().collect();
            let mut result = String::new();
            let mut position = 0usize;
            for (start, end) in &chosen {
                result.extend(chars[position..*start].iter());
                let matched: String = chars[*start..*end].iter().collect();
                result.push_str(&expand_replacement(
                    run.replacement,
                    &matched,
                    run.previous_replacement,
                ));
                position = *end;
            }
            result.extend(chars[position..].iter());
            let added = result.chars().filter(|ch| *ch == '\n').count();
            offset = offset.saturating_add(added as isize);
            last_row = Some(row + added);
            self.buffer.replace_lines(row, row, &result);
        }
        if substitutions == 0 {
            if settings.error {
                return Err(format!("E486: Pattern not found: {}", run.pattern));
            }
            return Ok(None);
        }
        if settings.count_only {
            return Ok(Some(substitute_count_message(substitutions, changed_lines)));
        }
        if let Some(row) = first_changed_row {
            self.note_buffer_change((row, 0), false, false);
        }
        let row = last_row.unwrap_or(self.cursor.row());
        let row = row.min(self.buffer.line_count().saturating_sub(1));
        let col = self.first_non_blank_of(row);
        self.cursor.set_position(row, col);
        self.remember_wanted_column();
        let mut message = String::new();
        if substitutions > SUBSTITUTE_REPORT_THRESHOLD {
            message.push_str(&substitute_report(substitutions, changed_lines));
        }
        if settings.print {
            let printed = self.printed_line(row, settings);
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(&printed);
        }
        if message.is_empty() {
            Ok(None)
        } else {
            Ok(Some(message))
        }
    }

    fn printed_line(&self, row: usize, settings: SubstituteSettings) -> String {
        let line = self.buffer.line(row).unwrap_or_default();
        let text = if settings.list {
            format!("{}$", line.replace('\t', "^I"))
        } else {
            line
        };
        if settings.number {
            format!("{} {text}", row + 1)
        } else {
            text
        }
    }
}

fn substitute_report(substitutions: usize, lines: usize) -> String {
    let substitution_word = if substitutions == 1 {
        "substitution"
    } else {
        "substitutions"
    };
    let line_word = if lines == 1 { "line" } else { "lines" };
    format!("{substitutions} {substitution_word} on {lines} {line_word}")
}

fn substitute_count_message(matches: usize, lines: usize) -> String {
    let match_word = if matches == 1 { "match" } else { "matches" };
    let line_word = if lines == 1 { "line" } else { "lines" };
    format!("{matches} {match_word} on {lines} {line_word}")
}

fn unescape_substitute_pattern(pattern: &str) -> String {
    let mut result = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some(next) if !next.is_alphanumeric() => {
                    result.push(next);
                    chars.next();
                }
                Some(_) => result.push('\\'),
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn expand_replacement(raw: &str, matched: &str, previous: &str) -> String {
    let mut result = String::new();
    let mut chars = raw.chars();
    let mut case = CaseMode::None;
    let mut next_case = CaseMode::None;
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                None => push_styled(&mut result, '\\', case, &mut next_case),
                Some('r') => result.push('\n'),
                Some('n') => result.push('\0'),
                Some('t') => result.push('\t'),
                Some('e') | Some('E') => case = CaseMode::None,
                Some('u') => next_case = CaseMode::Upper,
                Some('l') => next_case = CaseMode::Lower,
                Some('U') => case = CaseMode::Upper,
                Some('L') => case = CaseMode::Lower,
                Some('0') => push_styled_str(&mut result, matched, case, &mut next_case),
                Some('1'..='9') => {}
                Some('&') => push_styled(&mut result, '&', case, &mut next_case),
                Some('~') => push_styled(&mut result, '~', case, &mut next_case),
                Some('\\') => push_styled(&mut result, '\\', case, &mut next_case),
                Some('=') => push_styled(&mut result, '=', case, &mut next_case),
                Some(other) => push_styled(&mut result, other, case, &mut next_case),
            }
        } else if ch == '&' {
            push_styled_str(&mut result, matched, case, &mut next_case);
        } else if ch == '~' {
            push_styled_str(&mut result, previous, case, &mut next_case);
        } else {
            push_styled(&mut result, ch, case, &mut next_case);
        }
    }
    result
}

fn push_styled_str(result: &mut String, text: &str, case: CaseMode, next_case: &mut CaseMode) {
    for ch in text.chars() {
        push_styled(result, ch, case, next_case);
    }
}

fn push_styled(result: &mut String, ch: char, case: CaseMode, next_case: &mut CaseMode) {
    let effective = if *next_case != CaseMode::None {
        *next_case
    } else {
        case
    };
    match effective {
        CaseMode::None => result.push(ch),
        CaseMode::Upper => result.extend(ch.to_uppercase()),
        CaseMode::Lower => result.extend(ch.to_lowercase()),
    }
    *next_case = CaseMode::None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::command::{parse, Command};
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn run(editor: &mut Editor, text: &str) -> Result<Option<String>, String> {
        let command = match parse(text) {
            Command::Substitute(command) => command,
            other => panic!("expected substitute for {text}: {other:?}"),
        };
        editor.substitute_command(&command)
    }

    fn lines(editor: &Editor) -> Vec<String> {
        (0..editor.buffer.line_count())
            .map(|row| editor.buffer.line(row).unwrap())
            .collect()
    }

    #[test]
    fn substitutes_first_match_on_current_line() {
        let mut editor = editor_with("foo bar foo\nbaz foo\n");
        editor.cursor.set_position(1, 0);
        assert_eq!(run(&mut editor, "s/foo/X/"), Ok(None));
        assert_eq!(lines(&editor), ["foo bar foo", "baz X"]);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn global_flag_substitutes_every_match() {
        let mut editor = editor_with("foo foo foo\n");
        assert_eq!(
            run(&mut editor, "s/foo/X/g"),
            Ok(Some("3 substitutions on 1 line".to_string()))
        );
        assert_eq!(lines(&editor), ["X X X"]);
    }

    #[test]
    fn range_substitution_places_cursor_on_first_non_blank() {
        let mut editor = editor_with("  foo\n  foo\n");
        assert_eq!(run(&mut editor, "%s/foo/X/"), Ok(None));
        assert_eq!(lines(&editor), ["  X", "  X"]);
        assert_eq!((editor.row(), editor.col()), (1, 2));
    }

    #[test]
    fn ignore_case_and_match_case_flags() {
        let mut editor = editor_with("Foo foo\n");
        assert_eq!(run(&mut editor, "s/foo/X/i"), Ok(None));
        assert_eq!(lines(&editor), ["X foo"]);
        let mut editor = editor_with("Foo foo\n");
        assert_eq!(run(&mut editor, "s/foo/X/I"), Ok(None));
        assert_eq!(lines(&editor), ["Foo X"]);
        let mut editor = editor_with("Foo foo\n");
        assert_eq!(run(&mut editor, "s/foo/X/gi"), Ok(None));
        assert_eq!(lines(&editor), ["X X"]);
    }

    #[test]
    fn count_only_reports_matches_without_changing() {
        let mut editor = editor_with("foo foo\nbaz foo\n");
        assert_eq!(
            run(&mut editor, "%s/foo/X/n"),
            Ok(Some("2 matches on 2 lines".to_string()))
        );
        assert_eq!(lines(&editor), ["foo foo", "baz foo"]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn count_selects_lines_from_range_end() {
        let mut editor = editor_with("L1 foo\nL2 foo\nL3 foo\nL4 foo\n");
        assert_eq!(run(&mut editor, "%s/foo/X/ 2"), Ok(None));
        assert_eq!(lines(&editor), ["L1 foo", "L2 foo", "L3 foo", "L4 X"]);
    }

    #[test]
    fn numeric_and_relative_ranges() {
        let mut editor = editor_with("a{a}\nb{a}\nc{a}\nd{a}\n");
        editor.cursor.set_position(1, 0);
        assert_eq!(run(&mut editor, "2s/{a}/X/"), Ok(None));
        assert_eq!(lines(&editor)[1], "bX");
        assert_eq!(run(&mut editor, ".+1s/{a}/X/"), Ok(None));
        assert_eq!(lines(&editor)[2], "cX");
        assert_eq!(run(&mut editor, "$s/{a}/X/"), Ok(None));
        assert_eq!(lines(&editor)[3], "dX");
        assert_eq!(run(&mut editor, "0s/a{a}/X/"), Ok(None));
        assert_eq!(lines(&editor)[0], "X");
    }

    #[test]
    fn semicolon_range_resolves_second_address_from_first() {
        let mut editor = editor_with("a{a}\nb{a}\nc{a}\n");
        editor.cursor.set_position(2, 0);
        assert_eq!(run(&mut editor, "1;+1s/{a}/X/g"), Ok(None));
        assert_eq!(lines(&editor), ["aX", "bX", "c{a}"]);
    }

    #[test]
    fn empty_pattern_uses_last_search() {
        let mut editor = editor_with("bar bar\n");
        assert_eq!(run(&mut editor, "s/bar/X/"), Ok(None));
        assert_eq!(lines(&editor), ["X bar"]);
        assert_eq!(run(&mut editor, "s//Y/"), Ok(None));
        assert_eq!(lines(&editor), ["X Y"]);
    }

    #[test]
    fn repeat_form_reuses_pattern_and_replacement() {
        let mut editor = editor_with("foo bar foo\n");
        assert_eq!(run(&mut editor, "s/foo/X/"), Ok(None));
        assert_eq!(lines(&editor), ["X bar foo"]);
        assert_eq!(run(&mut editor, "s"), Ok(None));
        assert_eq!(lines(&editor), ["X bar X"]);
    }

    #[test]
    fn amp_flag_reuses_previous_flags() {
        let mut editor = editor_with("foo foo bar bar\n");
        assert_eq!(run(&mut editor, "s/foo/X/g"), Ok(None));
        assert_eq!(run(&mut editor, "s/bar/Y/&"), Ok(None));
        assert_eq!(lines(&editor), ["X X Y Y"]);
        let mut editor = editor_with("foo foo bar bar\n");
        assert_eq!(run(&mut editor, "s/foo/X/"), Ok(None));
        assert_eq!(run(&mut editor, "s/bar/Y/&"), Ok(None));
        assert_eq!(lines(&editor), ["X foo Y bar"]);
    }

    #[test]
    fn replacement_specials() {
        let cases = [
            ("s/foo/[&]/", "[foo]"),
            ("s/foo/[\\0]/", "[foo]"),
            ("s/foo/\\U&/", "FOO"),
            ("s/foo/\\u&/", "Foo"),
            ("s/foo/\\U&\\E!/", "FOO!"),
            ("s/foo/a\\tb/", "a\tb"),
            ("s/foo/\\q/", "q"),
            ("s/foo/\\\\/", "\\"),
            ("s/foo/\\&/", "&"),
            ("s/foo/\\~/", "~"),
            ("s/foo/\\1/", ""),
        ];
        for (command, expected) in cases {
            let mut editor = editor_with("foo\n");
            assert_eq!(run(&mut editor, command), Ok(None), "{command}");
            assert_eq!(lines(&editor), [expected], "{command}");
        }
        let mut editor = editor_with("ABC\n");
        assert_eq!(run(&mut editor, "s/ABC/\\L&/"), Ok(None));
        assert_eq!(lines(&editor), ["abc"]);
        let mut editor = editor_with("ABC\n");
        assert_eq!(run(&mut editor, "s/A/\\l&/"), Ok(None));
        assert_eq!(lines(&editor), ["aBC"]);
    }

    #[test]
    fn replacement_newline_splits_lines_and_moves_cursor() {
        let mut editor = editor_with("foo bar\n");
        assert_eq!(run(&mut editor, "s/foo/a\\rb/"), Ok(None));
        assert_eq!(lines(&editor), ["a", "b bar"]);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn replacement_nul_is_inserted() {
        let mut editor = editor_with("foo\n");
        assert_eq!(run(&mut editor, "s/foo/a\\nb/"), Ok(None));
        assert_eq!(lines(&editor), ["a\0b"]);
    }

    #[test]
    fn tilde_inserts_previous_replacement() {
        let mut editor = editor_with("foo bar\n");
        assert_eq!(run(&mut editor, "s/foo/X/"), Ok(None));
        assert_eq!(run(&mut editor, "s/bar/~Y/"), Ok(None));
        assert_eq!(lines(&editor), ["X XY"]);
    }

    #[test]
    fn missing_match_reports_error_unless_e_flag() {
        let mut editor = editor_with("abc\n");
        assert_eq!(
            run(&mut editor, "s/zzz/X/"),
            Err("E486: Pattern not found: zzz".to_string())
        );
        assert_eq!(run(&mut editor, "s/zzz/X/e"), Ok(None));
    }

    #[test]
    fn missing_previous_patterns_report_errors() {
        let mut editor = editor_with("abc\n");
        assert_eq!(
            run(&mut editor, "s//X/"),
            Err("E35: No previous regular expression".to_string())
        );
        assert_eq!(
            run(&mut editor, "s"),
            Err("E33: No previous substitute regular expression".to_string())
        );
    }

    #[test]
    fn backwards_range_is_rejected() {
        let mut editor = editor_with("a\nb\nc\n");
        assert_eq!(
            run(&mut editor, "3,1s/a/X/"),
            Err("E493: Backwards range given: 3,13,1s/a/X/".to_string())
        );
        assert_eq!(lines(&editor), ["a", "b", "c"]);
    }

    #[test]
    fn substition_is_a_single_undo_block() {
        let mut editor = editor_with("foo\nfoo\nfoo\n");
        editor.cursor.set_position(1, 1);
        assert!(run(&mut editor, "%s/foo/X/g").is_ok());
        assert_eq!((editor.row(), editor.col()), (2, 0));
        editor.apply(Action::Undo);
        assert_eq!(lines(&editor), ["foo", "foo", "foo"]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn visual_command_prefills_range() {
        let mut editor = editor_with("foo\nfoo\nfoo\n");
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::EnterCommand);
        assert_eq!(editor.command_text(), "'<,'>");
        assert_eq!(run(&mut editor, "'<,'>s/foo/X/g"), Ok(None));
        assert_eq!(lines(&editor), ["X", "X", "foo"]);
    }

    #[test]
    fn printed_lines_follow_p_number_and_list_flags() {
        let mut editor = editor_with("foo\n");
        assert_eq!(run(&mut editor, "s/foo/X/p"), Ok(Some("X".to_string())));
        let mut editor = editor_with("foo\n");
        assert_eq!(run(&mut editor, "s/foo/X/#"), Ok(Some("1 X".to_string())));
        let mut editor = editor_with("foo\tx\n");
        assert_eq!(run(&mut editor, "s/foo/X/l"), Ok(Some("X^Ix$".to_string())));
    }

    #[test]
    fn substitution_creates_changelist_entry() {
        let mut editor = editor_with("foo\nfoo\nfoo\n");
        editor.apply(Action::MoveDown);
        assert!(run(&mut editor, "2s/foo/X/").is_ok());
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn search_pattern_updates_after_substitute() {
        let mut editor = editor_with("foo\n");
        assert!(run(&mut editor, "s/foo/X/").is_ok());
        assert_eq!(editor.search_last, "foo");
        assert_eq!(editor.substitute_pattern, "foo");
        assert_eq!(editor.substitute_replacement, "X");
    }
}
