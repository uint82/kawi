use super::line_width;

const VOID_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const RAW_TEXT_TAGS: [&str; 4] = ["script", "style", "textarea", "title"];

const OPTIONAL_CLOSE_TAGS: [&str; 14] = [
    "colgroup", "dd", "dt", "li", "optgroup", "option", "p", "rp", "rt", "tbody", "td", "tfoot",
    "th", "thead",
];

#[derive(Default)]
pub(super) struct TagScan {
    stack: Vec<Tag>,
    state: TagState,
}

impl TagScan {
    pub(super) fn update(&mut self, ch: char, line: usize) {
        let state = std::mem::take(&mut self.state);
        self.state = match state {
            TagState::None => self.open_char(ch, line),
            TagState::Start { line } => start_char(ch, line),
            TagState::RawClose { line } => raw_close_char(ch, line),
            TagState::Name {
                closing,
                name,
                line,
            } => self.name_char(ch, closing, name, line),
            TagState::Attributes {
                closing,
                name,
                self_closing,
                line,
            } => self.attributes_char(ch, closing, name, self_closing, line),
            TagState::Declaration => declaration_char(ch),
        };
    }

    pub(super) fn width(
        &self,
        lines: &[String],
        tabstop: usize,
        shiftwidth: usize,
    ) -> Option<usize> {
        let line = self.source_line()?;
        Some(line_width(lines.get(line)?, tabstop) + shiftwidth)
    }

    pub(super) fn closer_width(
        &self,
        lines: &[String],
        row: usize,
        col: usize,
        tabstop: usize,
    ) -> Option<usize> {
        let prefix = prefix_trimmed(lines.get(row)?, col);
        match &self.state {
            TagState::Attributes {
                closing: false,
                line,
                ..
            } if prefix.is_empty() => Some(line_width(lines.get(*line)?, tabstop)),
            TagState::Name {
                closing: true,
                name,
                ..
            }
            | TagState::Attributes {
                closing: true,
                name,
                ..
            } if closing_prefix_matches(&prefix, name) => {
                let line = self.matching_line(name)?;
                Some(line_width(lines.get(line)?, tabstop))
            }
            _ => None,
        }
    }

    fn source_line(&self) -> Option<usize> {
        match &self.state {
            TagState::Start { line }
            | TagState::Name { line, .. }
            | TagState::Attributes { line, .. } => Some(*line),
            _ => self.stack.last().map(|tag| tag.line),
        }
    }

    fn matching_line(&self, name: &str) -> Option<usize> {
        self.stack
            .iter()
            .rposition(|tag| tag.name == name)
            .map(|index| self.stack[index].line)
    }

    fn open_char(&mut self, ch: char, line: usize) -> TagState {
        if ch != '<' {
            return TagState::None;
        }
        if self.in_raw_text() {
            return TagState::RawClose { line };
        }
        TagState::Start { line }
    }

    fn name_char(&mut self, ch: char, closing: bool, mut name: String, line: usize) -> TagState {
        if is_tag_name_char(ch) {
            name.push(ch.to_ascii_lowercase());
            return TagState::Name {
                closing,
                name,
                line,
            };
        }
        if ch == '>' {
            self.finish(closing, &name, line, false);
            return TagState::None;
        }
        TagState::Attributes {
            closing,
            name,
            self_closing: ch == '/',
            line,
        }
    }

    fn attributes_char(
        &mut self,
        ch: char,
        closing: bool,
        name: String,
        self_closing: bool,
        line: usize,
    ) -> TagState {
        if ch == '>' {
            self.finish(closing, &name, line, self_closing);
            return TagState::None;
        }
        TagState::Attributes {
            closing,
            name,
            self_closing: if ch.is_whitespace() {
                self_closing
            } else {
                ch == '/'
            },
            line,
        }
    }

    fn finish(&mut self, closing: bool, name: &str, line: usize, self_closing: bool) {
        if name.is_empty() {
            return;
        }
        if closing {
            self.close(name);
        } else {
            self.open(name, line, self_closing);
        }
    }

    fn open(&mut self, name: &str, line: usize, self_closing: bool) {
        if self_closing || is_void_tag(name) {
            return;
        }
        self.clear_optional(name);
        self.stack.push(Tag {
            name: name.to_string(),
            line,
        });
    }

    fn close(&mut self, name: &str) {
        if let Some(index) = self.stack.iter().rposition(|tag| tag.name == name) {
            self.stack.truncate(index);
        }
    }

    fn clear_optional(&mut self, name: &str) {
        if !is_optional_close_tag(name) {
            return;
        }
        if self.stack.last().is_some_and(|tag| tag.name == name) {
            self.stack.pop();
        }
    }

    fn in_raw_text(&self) -> bool {
        self.stack
            .last()
            .is_some_and(|tag| is_raw_text_tag(&tag.name))
    }
}

fn prefix_trimmed(line: &str, col: usize) -> String {
    line.chars()
        .take(col)
        .collect::<String>()
        .trim()
        .to_string()
}

fn closing_prefix_matches(prefix: &str, name: &str) -> bool {
    prefix
        .strip_prefix("</")
        .is_some_and(|rest| rest.trim_end().eq_ignore_ascii_case(name))
}

fn start_char(ch: char, line: usize) -> TagState {
    match ch {
        '/' => TagState::Name {
            closing: true,
            name: String::new(),
            line,
        },
        '!' | '?' => TagState::Declaration,
        ch if is_tag_name_char(ch) => TagState::Name {
            closing: false,
            name: ch.to_ascii_lowercase().to_string(),
            line,
        },
        _ => TagState::None,
    }
}

fn raw_close_char(ch: char, line: usize) -> TagState {
    if ch == '/' {
        return TagState::Name {
            closing: true,
            name: String::new(),
            line,
        };
    }
    TagState::None
}

fn declaration_char(ch: char) -> TagState {
    if ch == '>' {
        return TagState::None;
    }
    TagState::Declaration
}

fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.')
}

fn is_void_tag(name: &str) -> bool {
    VOID_TAGS.contains(&name)
}

fn is_raw_text_tag(name: &str) -> bool {
    RAW_TEXT_TAGS.contains(&name)
}

fn is_optional_close_tag(name: &str) -> bool {
    OPTIONAL_CLOSE_TAGS.contains(&name)
}

#[derive(Default)]
enum TagState {
    #[default]
    None,
    Start {
        line: usize,
    },
    RawClose {
        line: usize,
    },
    Name {
        closing: bool,
        name: String,
        line: usize,
    },
    Attributes {
        closing: bool,
        name: String,
        self_closing: bool,
        line: usize,
    },
    Declaration,
}

struct Tag {
    name: String,
    line: usize,
}
