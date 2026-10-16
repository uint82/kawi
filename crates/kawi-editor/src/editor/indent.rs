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
    let rest = line.trim_start_matches([' ', '\t']);
    let width = kawi_buffer::indent_width_of(line, tabstop);
    let indented = match shift {
        IndentShift::In(amount) => width + amount,
        IndentShift::Out(amount) => width.saturating_sub(amount),
        IndentShift::Clear => 0,
    };
    if indented == width {
        return None;
    }
    Some(canonical_indent(indented, expandtab, tabstop) + rest)
}

fn canonical_indent(width: usize, expandtab: bool, tabstop: usize) -> String {
    if expandtab {
        return " ".repeat(width);
    }
    let tabstop = tabstop.max(1);
    "\t".repeat(width / tabstop) + &" ".repeat(width % tabstop)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentConfig {
    pub shiftwidth: usize,
    pub expandtab: bool,
    pub tabstop: usize,
}

impl IndentConfig {
    fn filetype(shiftwidth: usize, expandtab: bool, tabstop: usize) -> Self {
        Self {
            shiftwidth,
            expandtab,
            tabstop,
        }
    }

    fn fallback() -> Self {
        Self {
            shiftwidth: 8,
            expandtab: false,
            tabstop: 8,
        }
    }
}

pub fn detect(path: Option<&str>) -> IndentConfig {
    let extension = path
        .filter(|name| name.contains('.'))
        .and_then(|name| name.rsplit('.').next())
        .map(str::to_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "py" => IndentConfig::filetype(4, true, 4),
        "md" | "markdown" | "mdown" => IndentConfig::filetype(4, true, 4),
        "rs" | "zig" | "swift" => IndentConfig::filetype(4, true, 8),
        "yaml" | "yml" | "rb" | "scala" | "ex" | "exs" | "clj" | "scss" | "proto" | "graphql"
        | "gql" => IndentConfig::filetype(2, true, 8),
        "tf" | "idr" => IndentConfig::filetype(2, true, 2),
        "ml" | "mli" | "elm" | "f90" | "rst" => IndentConfig::filetype(8, true, 8),
        "go" => IndentConfig::filetype(0, false, 8),
        _ => IndentConfig::fallback(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_language_defaults() {
        assert_eq!(detect(Some("x.py")), IndentConfig::filetype(4, true, 4));
        assert_eq!(detect(Some("x.rs")), IndentConfig::filetype(4, true, 8));
        assert_eq!(detect(Some("x.zig")), IndentConfig::filetype(4, true, 8));
        assert_eq!(detect(Some("x.swift")), IndentConfig::filetype(4, true, 8));
        assert_eq!(detect(Some("x.go")), IndentConfig::filetype(0, false, 8));
        assert_eq!(detect(Some("x.yaml")), IndentConfig::filetype(2, true, 8));
        assert_eq!(detect(Some("x.yml")), IndentConfig::filetype(2, true, 8));
        assert_eq!(detect(Some("x.rb")), IndentConfig::filetype(2, true, 8));
        assert_eq!(detect(Some("x.scala")), IndentConfig::filetype(2, true, 8));
        assert_eq!(detect(Some("x.ex")), IndentConfig::filetype(2, true, 8));
        assert_eq!(detect(Some("x.tf")), IndentConfig::filetype(2, true, 2));
        assert_eq!(detect(Some("x.ml")), IndentConfig::filetype(8, true, 8));
        assert_eq!(detect(Some("x.md")), IndentConfig::filetype(4, true, 4));
        assert_eq!(
            detect(Some("x.markdown")),
            IndentConfig::filetype(4, true, 4)
        );
    }

    #[test]
    fn falls_back_without_extension() {
        assert_eq!(detect(None), IndentConfig::fallback());
        assert_eq!(detect(Some("Makefile")), IndentConfig::fallback());
        assert_eq!(detect(Some("x.js")), IndentConfig::fallback());
        assert_eq!(detect(Some("X.PY")), IndentConfig::filetype(4, true, 4));
    }
}
