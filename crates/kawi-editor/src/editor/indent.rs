pub(crate) mod engine;
#[cfg(test)]
mod html_tests;
mod language;
mod tags;
#[cfg(test)]
mod tests;

pub use language::IndentRules;

pub(super) fn line_width(line: &str, tabstop: usize) -> usize {
    kawi_buffer::indent_width_of(line, tabstop.max(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentShift {
    In(usize),
    Out(usize),
    Clear,
}

pub fn shift_span_text(
    text: &str,
    shift: IndentShift,
    expandtab: bool,
    tabstop: usize,
) -> Option<String> {
    let mut changed = false;
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match shift_line(line, shift, expandtab, tabstop) {
            Some(next) => {
                changed = true;
                out.push_str(&next);
            }
            None => out.push_str(line),
        }
    }
    if changed {
        Some(out)
    } else {
        None
    }
}

fn shift_line(line: &str, shift: IndentShift, expandtab: bool, tabstop: usize) -> Option<String> {
    if line.is_empty() {
        return None;
    }
    let width = kawi_buffer::indent_width_of(line, tabstop);
    let indented = match shift {
        IndentShift::In(amount) => width + amount,
        IndentShift::Out(amount) => width.saturating_sub(amount),
        IndentShift::Clear => 0,
    };
    if indented == width {
        return None;
    }
    Some(reindent_line(line, indented, expandtab, tabstop))
}

pub(super) fn reindent_line(line: &str, width: usize, expandtab: bool, tabstop: usize) -> String {
    let rest = line.trim_start_matches([' ', '\t']);
    canonical_indent(width, expandtab, tabstop) + rest
}

pub(super) fn rounded_indent_width(width: usize, left: bool, unit: usize) -> usize {
    if !left {
        return (width / unit + 1) * unit;
    }
    let steps = width / unit;
    if width.is_multiple_of(unit) {
        steps.saturating_sub(1) * unit
    } else {
        steps * unit
    }
}

fn canonical_indent(width: usize, expandtab: bool, tabstop: usize) -> String {
    if expandtab {
        return " ".repeat(width);
    }
    let tabstop = tabstop.max(1);
    "\t".repeat(width / tabstop) + &" ".repeat(width % tabstop)
}

pub const DEFAULT_SHIFTWIDTH: usize = 8;
pub const DEFAULT_EXPANDTAB: bool = false;
pub const DEFAULT_TABSTOP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentConfig {
    pub shiftwidth: usize,
    pub expandtab: bool,
    pub tabstop: usize,
}

impl IndentConfig {
    pub(super) fn shift_unit(&self) -> usize {
        if self.shiftwidth == 0 {
            self.tabstop
        } else {
            self.shiftwidth
        }
    }
}
