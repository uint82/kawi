use kawi_plugin::{DiagnosticSummary, Position};
use ropey::Rope;

use crate::app::problems::Problem;

pub(super) fn summaries(problems: &[&Problem], snapshot: &Rope) -> Vec<DiagnosticSummary> {
    problems
        .iter()
        .map(|problem| {
            let start = position(snapshot, problem.line, problem.column);
            let end = position(snapshot, problem.end_line, problem.end_column);
            DiagnosticSummary {
                start,
                end,
                severity: problem.severity.label().to_string(),
                message: problem.message.clone(),
                source: Some(problem.source.clone()),
                code: problem.code.clone(),
            }
        })
        .collect()
}

fn position(snapshot: &Rope, line: usize, column: usize) -> Position {
    let line = line
        .saturating_sub(1)
        .min(snapshot.len_lines().saturating_sub(1));
    let units = column.saturating_sub(1);
    let text = if snapshot.len_lines() == 0 {
        String::new()
    } else {
        snapshot.line(line).to_string()
    };
    Position {
        line,
        byte_column: utf16_to_byte(&text, units),
    }
}

fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut utf16 = 0;
    let mut bytes = 0;
    for ch in text.chars() {
        if utf16 >= units {
            break;
        }
        utf16 += ch.len_utf16();
        bytes += ch.len_utf8();
    }
    bytes.min(text.len())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::problems::ProblemSeverity;

    #[test]
    fn converts_utf16_columns_to_bytes() {
        assert_eq!(utf16_to_byte("a😀b", 0), 0);
        assert_eq!(utf16_to_byte("a😀b", 1), 1);
        assert_eq!(utf16_to_byte("a😀b", 2), 5);
        assert_eq!(utf16_to_byte("a😀b", 4), 6);
    }

    #[test]
    fn builds_diagnostic_summaries() {
        let problem = Problem {
            source: "lsp:rust".to_string(),
            path: PathBuf::from("a.rs"),
            line: 1,
            column: 3,
            end_line: 1,
            end_column: 5,
            severity: ProblemSeverity::Warning,
            message: "unused".to_string(),
            code: Some("W1".to_string()),
            code_description: None,
            provider: None,
            tags: Vec::new(),
            related: Vec::new(),
        };
        let snapshot = Rope::from_str("a😀b\n");
        let summaries = summaries(&[&problem], &snapshot);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].start.line, 0);
        assert_eq!(summaries[0].start.byte_column, 5);
        assert_eq!(summaries[0].end.byte_column, 6);
        assert_eq!(summaries[0].severity, "warning");
        assert_eq!(summaries[0].code.as_deref(), Some("W1"));
    }
}
