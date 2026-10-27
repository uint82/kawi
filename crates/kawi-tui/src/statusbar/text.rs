use std::iter::Peekable;
use std::str::Chars;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const ELLIPSIS: &str = "\u{2026}";

const CSI_FINAL_START: char = '\u{40}';
const CSI_FINAL_END: char = '\u{7e}';
const BEL: char = '\u{7}';
const ESC: char = '\u{1b}';
const ST: char = '\\';

pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            ESC => skip_escape(&mut chars),
            '\u{9b}' => skip_csi(&mut chars),
            '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => skip_string(&mut chars),
            character if character.is_control() => out.push(' '),
            character => out.push(character),
        }
    }
    out
}

fn skip_escape(chars: &mut Peekable<Chars<'_>>) {
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            skip_csi(chars);
        }
        Some(']') => {
            chars.next();
            skip_string(chars);
        }
        Some('P' | '^' | '_') => {
            chars.next();
            skip_string(chars);
        }
        Some(character) if ('\u{20}'..='\u{2f}').contains(&character) => skip_escape_final(chars),
        Some(_) => {
            chars.next();
        }
        None => {}
    }
}

fn skip_escape_final(chars: &mut Peekable<Chars<'_>>) {
    for character in chars.by_ref() {
        if ('\u{30}'..='\u{7e}').contains(&character) {
            return;
        }
    }
}

fn skip_csi(chars: &mut Peekable<Chars<'_>>) {
    for character in chars.by_ref() {
        if (CSI_FINAL_START..=CSI_FINAL_END).contains(&character) {
            return;
        }
    }
}

fn skip_string(chars: &mut Peekable<Chars<'_>>) {
    while let Some(character) = chars.next() {
        if character == BEL {
            return;
        }
        if character == '\u{9c}' {
            return;
        }
        if character == ESC && chars.peek() == Some(&ST) {
            chars.next();
            return;
        }
    }
}

pub fn truncate_left(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_string();
    }
    let ellipsis_width = display_width(ELLIPSIS);
    if width < ellipsis_width {
        return String::new();
    }
    let budget = width - ellipsis_width;
    let mut tail = Vec::new();
    let mut used: usize = 0;
    for grapheme in text.graphemes(true).rev() {
        let grapheme_width = display_width(grapheme);
        if used.saturating_add(grapheme_width) > budget {
            break;
        }
        used = used.saturating_add(grapheme_width);
        tail.push(grapheme);
    }
    let mut out = String::with_capacity(width);
    out.push_str(ELLIPSIS);
    for grapheme in tail.iter().rev() {
        out.push_str(grapheme);
    }
    out
}

pub fn truncate_right(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(text.len().min(width));
    let mut used: usize = 0;
    for grapheme in text.graphemes(true) {
        let grapheme_width = display_width(grapheme);
        if used.saturating_add(grapheme_width) > width {
            for _ in used..width {
                out.push(' ');
            }
            return out;
        }
        used = used.saturating_add(grapheme_width);
        out.push_str(grapheme);
    }
    out
}
