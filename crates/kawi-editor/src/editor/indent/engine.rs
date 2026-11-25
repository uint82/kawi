use kawi_syntax::LanguageId;

use crate::editing::LanguageEditing;

use super::language::IndentRules;
use super::line_width;
use super::scan::{matching_closer, matching_opener, scan, Bracket, Scan};

pub(crate) const INDENT_WINDOW_LINES: usize = 200;

pub(crate) use super::scan::is_closing_bracket;

pub(crate) struct IndentRequest<'a> {
    pub lines: &'a [String],
    pub row: usize,
    pub col: usize,
    pub current_width: usize,
    pub shiftwidth: usize,
    pub tabstop: usize,
    pub rules: &'a IndentRules,
    pub editing: &'a LanguageEditing,
    pub language: Option<LanguageId>,
}

pub(crate) fn next_width(request: &IndentRequest<'_>) -> usize {
    let scan = scan(request);
    if !scan.allows_indent() {
        return request.current_width;
    }
    if matches_block(scan.token(), request.rules)
        && (scan.stack.is_empty() || request.rules.indent_keywords_in_brackets)
    {
        return request.current_width + request.shiftwidth;
    }
    let bracket = request
        .rules
        .indent_brackets
        .then(|| bracket_width(&scan, request))
        .flatten();
    let tag = request
        .rules
        .indent_tags
        .then(|| {
            scan.tags
                .width(request.lines, request.tabstop, request.shiftwidth)
        })
        .flatten();
    bracket.or(tag).unwrap_or(request.current_width)
}

fn bracket_width(scan: &Scan, request: &IndentRequest<'_>) -> Option<usize> {
    if scan.last_bracket.is_some_and(is_closing_bracket) {
        return Some(request.current_width);
    }
    let opener = if scan
        .last_bracket
        .is_some_and(|ch| matching_closer(ch).is_some())
    {
        scan.stack.last()
    } else {
        base_opener(scan)
    };
    let width = opener.map(|opener| {
        line_width(&request.lines[opener.line], request.tabstop) + request.shiftwidth
    });
    if request.rules.indent_keywords_in_brackets {
        width.map(|width| width.max(request.current_width))
    } else {
        width
    }
}

fn base_opener(scan: &Scan) -> Option<&Bracket> {
    let brace = scan.stack.iter().rposition(|bracket| bracket.open == '{');
    let start = brace.map_or(0, |index| index + 1);
    scan.stack[start..]
        .iter()
        .find(|bracket| is_continuation_bracket(bracket.open))
        .or_else(|| brace.map(|index| &scan.stack[index]))
}

fn is_continuation_bracket(ch: char) -> bool {
    matches!(ch, '(' | '[')
}

pub(crate) fn closer_width(request: &IndentRequest<'_>, closer: char) -> Option<usize> {
    if !request.rules.indent_brackets {
        return None;
    }
    let scan = scan(request);
    if !scan.code {
        return None;
    }
    let opener = matching_opener(closer)?;
    let bracket = scan.stack.last()?;
    if bracket.open != opener {
        return None;
    }
    Some(line_width(&request.lines[bracket.line], request.tabstop))
}

pub(crate) fn tag_closer_width(request: &IndentRequest<'_>) -> Option<usize> {
    if !request.rules.indent_tags {
        return None;
    }
    let scan = scan(request);
    if !scan.code {
        return None;
    }
    scan.tags
        .closer_width(request.lines, request.row, request.col, request.tabstop)
}

fn matches_block(token: &str, rules: &IndentRules) -> bool {
    !token.is_empty()
        && (rules
            .indent_after_suffixes
            .iter()
            .any(|suffix| token.ends_with(suffix))
            || rules
                .indent_after_keywords
                .iter()
                .any(|keyword| rules.token_matches_keyword(token, keyword)))
}
