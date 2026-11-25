use super::engine::IndentRequest;
use super::language::IndentRules;
use super::line_width;
use super::scan::{scan_before, LineInfo, Scan};

pub(crate) fn keyword_width(request: &IndentRequest<'_>, keyword: &str) -> Option<usize> {
    let rules = request.rules;
    if request.row == 0 {
        return None;
    }
    let scan = scan_before(request)?;
    if !scan.code {
        return None;
    }
    if rules.is_block_closer(keyword) {
        return scan.blocks.last().map(|block| block.indent);
    }
    if rules.is_branch_keyword(keyword) {
        let current = line_width(&request.lines[request.row], request.tabstop);
        return branch_width(&scan, rules, current, request.shiftwidth);
    }
    if rules.is_align_keyword(keyword) {
        let current = line_width(&request.lines[request.row], request.tabstop);
        return align_width(&scan, rules, current);
    }
    None
}

fn align_width(scan: &Scan, rules: &IndentRules, current: usize) -> Option<usize> {
    let previous = scan.lines.iter().rev().find(|line| line.has_code)?;
    if starts_opener(rules, previous) {
        return None;
    }
    let depth = scan.stack.len();
    scan.lines
        .iter()
        .rev()
        .find(|line| {
            line.has_code
                && line.depth == depth
                && line.indent < current
                && (starts_opener(rules, line) || rules.is_align_keyword(&line.first_word))
        })
        .map(|line| line.indent)
}

fn branch_width(
    scan: &Scan,
    rules: &IndentRules,
    current: usize,
    shiftwidth: usize,
) -> Option<usize> {
    let depth = scan.stack.len();
    scan.lines
        .iter()
        .rev()
        .find(|line| {
            line.has_code
                && line.depth == depth
                && line.indent < current
                && (rules.is_branch_keyword(&line.first_word) || starts_opener(rules, line))
        })
        .and_then(|line| branch_target(rules, line, shiftwidth))
}

fn branch_target(rules: &IndentRules, line: &LineInfo, shiftwidth: usize) -> Option<usize> {
    if rules.is_branch_opener(&line.first_word)
        || (line.first_word != line.last_word && rules.is_branch_opener(&line.last_word))
    {
        return Some(line.indent + shiftwidth);
    }
    if starts_opener(rules, line) || rules.is_branch_keyword(&line.first_word) {
        return Some(line.indent);
    }
    None
}

fn starts_opener(rules: &IndentRules, line: &LineInfo) -> bool {
    rules.is_block_opener(&line.first_word)
        || (line.first_word != line.last_word && rules.is_block_opener(&line.last_word))
}
