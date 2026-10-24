use kawi_completion::{Documentation, MarkupKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownStyle {
    Normal,
    Strong,
    Emphasis,
    Code,
    Heading,
    Quote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSpan {
    pub text: String,
    pub style: MarkdownStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownLine {
    pub spans: Vec<MarkdownSpan>,
}

impl MarkdownLine {
    pub fn blank() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
}

pub fn render_spans(spans: &[MarkdownSpan], width: usize) -> Vec<MarkdownLine> {
    let sanitized: Vec<MarkdownSpan> = spans
        .iter()
        .map(|span| MarkdownSpan {
            text: sanitize(&span.text),
            style: span.style,
        })
        .collect();
    let mut lines = Vec::new();
    push_wrapped(&mut lines, &sanitized, width.max(1));
    lines
}

pub fn render(document: &Documentation, width: usize) -> Vec<MarkdownLine> {
    let width = width.max(1);
    let mut lines = Vec::new();
    if document.kind == MarkupKind::PlainText {
        for line in document.text.lines() {
            push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: sanitize(line),
                    style: MarkdownStyle::Normal,
                }],
                width,
            );
        }
        return lines;
    }
    let mut in_code = false;
    for line in document.text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: sanitize(line),
                    style: MarkdownStyle::Code,
                }],
                width,
            );
            continue;
        }
        if trimmed.is_empty() {
            lines.push(MarkdownLine::blank());
        } else if let Some(text) = heading(trimmed) {
            push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: sanitize(text),
                    style: MarkdownStyle::Heading,
                }],
                width,
            );
        } else if let Some(text) = trimmed.strip_prefix("> ") {
            push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: format!("> {}", sanitize(text)),
                    style: MarkdownStyle::Quote,
                }],
                width,
            );
        } else if is_rule(trimmed) {
            push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: "-".repeat(width.min(20)),
                    style: MarkdownStyle::Quote,
                }],
                width,
            );
        } else {
            let text = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .or_else(|| trimmed.strip_prefix("+ "))
                .map_or_else(|| sanitize(line), |text| format!("- {}", sanitize(text)));
            push_wrapped(&mut lines, &inline_spans(&text), width);
        }
    }
    lines
}

fn heading(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('#')?;
    Some(rest.trim_start_matches('#').trim_start())
}

fn is_rule(line: &str) -> bool {
    line.len() >= 3
        && line
            .chars()
            .all(|character| matches!(character, '-' | '*' | '_'))
}

fn inline_spans(text: &str) -> Vec<MarkdownSpan> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut style = MarkdownStyle::Normal;
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '`' => {
                push_span(&mut spans, &mut current, style);
                style = toggle(style, MarkdownStyle::Code);
                index += 1;
            }
            '*' | '_' if chars.get(index + 1) == Some(&chars[index]) => {
                push_span(&mut spans, &mut current, style);
                style = toggle(style, MarkdownStyle::Strong);
                index += 2;
            }
            '*' | '_' => {
                push_span(&mut spans, &mut current, style);
                style = toggle(style, MarkdownStyle::Emphasis);
                index += 1;
            }
            '[' => {
                if let Some(close) = chars[index + 1..].iter().position(|ch| *ch == ']') {
                    let close = index + 1 + close;
                    if chars.get(close + 1) == Some(&'(') {
                        if let Some(end) = chars[close + 2..].iter().position(|ch| *ch == ')') {
                            for character in &chars[index + 1..close] {
                                current.push(*character);
                            }
                            index = close + 2 + end + 1;
                            continue;
                        }
                    }
                }
                current.push(chars[index]);
                index += 1;
            }
            character => {
                current.push(character);
                index += 1;
            }
        }
    }
    push_span(&mut spans, &mut current, style);
    spans
}

fn toggle(style: MarkdownStyle, next: MarkdownStyle) -> MarkdownStyle {
    if style == next {
        MarkdownStyle::Normal
    } else {
        next
    }
}

fn push_span(spans: &mut Vec<MarkdownSpan>, current: &mut String, style: MarkdownStyle) {
    if !current.is_empty() {
        spans.push(MarkdownSpan {
            text: std::mem::take(current),
            style,
        });
    }
}

fn push_wrapped(lines: &mut Vec<MarkdownLine>, spans: &[MarkdownSpan], width: usize) {
    if spans.is_empty() {
        lines.push(MarkdownLine::blank());
        return;
    }
    let mut line = Vec::new();
    let mut current = MarkdownSpan {
        text: String::new(),
        style: spans[0].style,
    };
    let mut used = 0;
    for span in spans {
        for character in span.text.chars() {
            if used == width {
                push_char(&mut line, &mut current);
                lines.push(MarkdownLine { spans: line });
                line = Vec::new();
                current = MarkdownSpan {
                    text: String::new(),
                    style: span.style,
                };
                used = 0;
            }
            if current.style != span.style {
                push_char(&mut line, &mut current);
                current.style = span.style;
            }
            current.text.push(character);
            used += 1;
        }
    }
    push_char(&mut line, &mut current);
    lines.push(MarkdownLine { spans: line });
}

fn push_char(line: &mut Vec<MarkdownSpan>, current: &mut MarkdownSpan) {
    if !current.text.is_empty() {
        line.push(MarkdownSpan {
            text: std::mem::take(&mut current.text),
            style: current.style,
        });
    }
}

fn sanitize(line: &str) -> String {
    line.chars()
        .map(|character| match character {
            '\t' => ' ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect()
}

#[cfg(test)]
mod tests;
