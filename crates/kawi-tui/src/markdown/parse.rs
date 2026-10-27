use kawi_completion::{Documentation, MarkupKind};

use super::{
    sanitize, MarkdownBlock, MarkdownCodeBlock, MarkdownDocument, MarkdownLine, MarkdownSpan,
    MarkdownStyle,
};

struct Fence {
    marker: char,
    length: usize,
}

struct OpenCode {
    fence: Fence,
    block: MarkdownCodeBlock,
}

pub fn parse(document: &Documentation) -> MarkdownDocument {
    if document.kind == MarkupKind::PlainText {
        return MarkdownDocument {
            blocks: vec![MarkdownBlock::Text(plain_lines(&document.text))],
        };
    }
    let mut blocks = Vec::new();
    let mut text = Vec::new();
    let mut code: Option<OpenCode> = None;
    for line in document.text.lines() {
        if let Some(open) = code.as_mut() {
            if closes_fence(line, &open.fence) {
                blocks.push(MarkdownBlock::Code(code.take().expect("open code").block));
                continue;
            }
            open.block.lines.push(code_line(line));
            continue;
        }
        if let Some((fence, language)) = opening_fence(line) {
            flush_text(&mut blocks, &mut text);
            code = Some(OpenCode {
                fence,
                block: MarkdownCodeBlock {
                    language,
                    lines: Vec::new(),
                },
            });
            continue;
        }
        parse_text_line(line, &mut blocks, &mut text);
    }
    if let Some(open) = code {
        blocks.push(MarkdownBlock::Code(open.block));
    }
    flush_text(&mut blocks, &mut text);
    MarkdownDocument { blocks }
}

fn plain_lines(text: &str) -> Vec<MarkdownLine> {
    text.lines()
        .map(|line| MarkdownLine {
            spans: vec![normal_span(sanitize(line))],
            code: false,
        })
        .collect()
}

fn parse_text_line(line: &str, blocks: &mut Vec<MarkdownBlock>, text: &mut Vec<MarkdownLine>) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        text.push(MarkdownLine::blank());
    } else if let Some(heading) = heading(trimmed) {
        text.push(styled_line(sanitize(heading), MarkdownStyle::Heading));
    } else if let Some(quote) = trimmed.strip_prefix("> ") {
        text.push(styled_line(
            format!("> {}", sanitize(quote)),
            MarkdownStyle::Quote,
        ));
    } else if is_rule(trimmed) {
        flush_text(blocks, text);
        blocks.push(MarkdownBlock::Rule);
    } else {
        let line = list_line(trimmed, line);
        text.push(MarkdownLine {
            spans: inline_spans(&line),
            code: false,
        });
    }
}

fn list_line<'a>(trimmed: &'a str, line: &'a str) -> String {
    trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
        .map_or_else(|| sanitize(line), |text| format!("- {}", sanitize(text)))
}

fn styled_line(text: String, style: MarkdownStyle) -> MarkdownLine {
    MarkdownLine {
        spans: vec![MarkdownSpan {
            text,
            style,
            link: None,
        }],
        code: false,
    }
}

fn normal_span(text: String) -> MarkdownSpan {
    MarkdownSpan {
        text,
        style: MarkdownStyle::Normal,
        link: None,
    }
}

fn code_line(line: &str) -> MarkdownLine {
    MarkdownLine::code(vec![MarkdownSpan {
        text: sanitize(line),
        style: MarkdownStyle::Code,
        link: None,
    }])
}

fn flush_text(blocks: &mut Vec<MarkdownBlock>, text: &mut Vec<MarkdownLine>) {
    if !text.is_empty() {
        blocks.push(MarkdownBlock::Text(std::mem::take(text)));
    }
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

fn opening_fence(line: &str) -> Option<(Fence, Option<String>)> {
    let rest = fence_rest(line)?;
    let marker = rest.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = rest
        .chars()
        .take_while(|character| *character == marker)
        .count();
    if length < 3 {
        return None;
    }
    let info = &rest[marker.len_utf8() * length..];
    Some((
        Fence { marker, length },
        first_info_token(info).map(str::to_string),
    ))
}

fn closes_fence(line: &str, fence: &Fence) -> bool {
    let Some(rest) = fence_rest(line) else {
        return false;
    };
    let length = rest
        .chars()
        .take_while(|character| *character == fence.marker)
        .count();
    length >= fence.length && rest[length * fence.marker.len_utf8()..].trim().is_empty()
}

fn fence_rest(line: &str) -> Option<&str> {
    let spaces = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    (spaces <= 3).then(|| &line[spaces..])
}

fn first_info_token(info: &str) -> Option<&str> {
    info.split(|character: char| character.is_whitespace() || character == ',')
        .find(|token| !token.is_empty())
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
                if let Some(end) = chars[index + 1..].iter().position(|ch| *ch == '`') {
                    let end = index + 1 + end;
                    push_span(&mut spans, &mut current, style);
                    let text: String = chars[index + 1..end].iter().collect();
                    spans.push(MarkdownSpan {
                        text: sanitize(&text),
                        style: MarkdownStyle::Code,
                        link: None,
                    });
                    index = end + 1;
                    continue;
                }
                current.push('`');
                index += 1;
            }
            '*' | '_' if emphasis_delimiter(&chars, index) => {
                let strong = chars.get(index + 1) == Some(&chars[index]);
                push_span(&mut spans, &mut current, style);
                style = toggle(
                    style,
                    if strong {
                        MarkdownStyle::Strong
                    } else {
                        MarkdownStyle::Emphasis
                    },
                );
                index += if strong { 2 } else { 1 };
            }
            '[' => {
                if let Some(close) = chars[index + 1..].iter().position(|ch| *ch == ']') {
                    let close = index + 1 + close;
                    if chars.get(close + 1) == Some(&'(') {
                        if let Some(end) = chars[close + 2..].iter().position(|ch| *ch == ')') {
                            push_span(&mut spans, &mut current, style);
                            let text: String = chars[index + 1..close].iter().collect();
                            let target: String = chars[close + 2..close + 2 + end].iter().collect();
                            spans.push(MarkdownSpan {
                                text: sanitize(&text),
                                style,
                                link: Some(sanitize(&target)),
                            });
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

fn emphasis_delimiter(chars: &[char], index: usize) -> bool {
    if chars[index] != '_' {
        return true;
    }
    let previous = index.checked_sub(1).and_then(|value| chars.get(value));
    let next = chars.get(index + 1);
    !previous.is_some_and(|value| value.is_alphanumeric())
        || !next.is_some_and(|value| value.is_alphanumeric())
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
            link: None,
        });
    }
}
