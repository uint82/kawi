use regex::Regex;
use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const ESLINT_BUFFER_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "info",
            Self::Hint => "hint",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "error" | "fatal error" | "failure" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "note" | "information" | "info" => Some(Self::Information),
            "help" | "hint" => Some(Self::Hint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum ParserSpec {
    None,
    Cargo,
    Gcc,
    Go,
    Tsc,
    Eslint,
    Regex(RegexParser),
}

#[derive(Debug, Clone)]
pub struct RegexParser {
    regex: Regex,
    fallback: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    InvalidPattern(String),
    MissingGroup(&'static str),
    InvalidSeverity(String),
}

impl fmt::Display for ParserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPattern(error) => write!(formatter, "invalid pattern: {error}"),
            Self::MissingGroup(group) => write!(formatter, "pattern is missing the {group} group"),
            Self::InvalidSeverity(severity) => write!(
                formatter,
                "severity must be error, warning, info, or hint, got {severity}"
            ),
        }
    }
}

impl std::error::Error for ParserError {}

impl RegexParser {
    pub fn new(pattern: &str, severity: &str) -> Result<Self, ParserError> {
        let regex =
            Regex::new(pattern).map_err(|error| ParserError::InvalidPattern(error.to_string()))?;
        for required in ["file", "line", "message"] {
            if !regex.capture_names().flatten().any(|name| name == required) {
                return Err(ParserError::MissingGroup(required));
            }
        }
        let fallback = Severity::from_name(severity)
            .ok_or_else(|| ParserError::InvalidSeverity(severity.into()))?;
        Ok(Self { regex, fallback })
    }
}

pub struct Parser {
    spec: ParserSpec,
    cargo_pending: Option<(Severity, String)>,
    eslint_stdout: String,
    eslint_overflow: bool,
}

impl Parser {
    pub fn new(spec: ParserSpec) -> Self {
        Self {
            spec,
            cargo_pending: None,
            eslint_stdout: String::new(),
            eslint_overflow: false,
        }
    }

    pub fn parse_line(&mut self, stream: OutputStream, line: &str) -> Vec<Finding> {
        match self.spec.clone() {
            ParserSpec::None => Vec::new(),
            ParserSpec::Cargo => self.parse_cargo(line),
            ParserSpec::Gcc => capture_finding(gcc_regex(), line, Severity::Error)
                .into_iter()
                .collect(),
            ParserSpec::Go => capture_finding(go_regex(), line, Severity::Error)
                .into_iter()
                .collect(),
            ParserSpec::Tsc => capture_finding(tsc_regex(), line, Severity::Error)
                .into_iter()
                .collect(),
            ParserSpec::Eslint => self.capture_eslint(stream, line),
            ParserSpec::Regex(parser) => capture_finding(&parser.regex, line, parser.fallback)
                .into_iter()
                .collect(),
        }
    }

    pub fn finish(&mut self) -> Vec<Finding> {
        if !matches!(self.spec, ParserSpec::Eslint) || self.eslint_overflow {
            return Vec::new();
        }
        parse_eslint(&self.eslint_stdout)
    }

    fn parse_cargo(&mut self, line: &str) -> Vec<Finding> {
        if let Some(finding) = parse_cargo_json(line) {
            return vec![finding];
        }
        if let Some(captures) = cargo_header_regex().captures(line) {
            self.cargo_pending = cargo_header(&captures);
            return Vec::new();
        }
        if let Some(captures) = cargo_location_regex().captures(line) {
            return self
                .cargo_pending
                .take()
                .and_then(|(severity, message)| finding_with_message(&captures, severity, message))
                .into_iter()
                .collect();
        }
        if line.trim().is_empty() {
            self.cargo_pending = None;
        }
        Vec::new()
    }

    fn capture_eslint(&mut self, stream: OutputStream, line: &str) -> Vec<Finding> {
        if stream != OutputStream::Stdout || self.eslint_overflow {
            return Vec::new();
        }
        let needed = line.len().saturating_add(1);
        if self.eslint_stdout.len().saturating_add(needed) > ESLINT_BUFFER_LIMIT {
            self.eslint_overflow = true;
            self.eslint_stdout.clear();
            return Vec::new();
        }
        self.eslint_stdout.push_str(line);
        self.eslint_stdout.push('\n');
        Vec::new()
    }
}

pub fn infer_parser(command: &str) -> ParserSpec {
    let Some(name) = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return ParserSpec::None;
    };
    let name = name.to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    match name {
        "cargo" => ParserSpec::Cargo,
        "gcc" | "cc" | "clang" | "clang++" | "g++" | "c++" => ParserSpec::Gcc,
        "go" => ParserSpec::Go,
        "tsc" => ParserSpec::Tsc,
        "eslint" => ParserSpec::Eslint,
        _ => ParserSpec::None,
    }
}

pub fn parser_from_name(name: &str) -> Option<ParserSpec> {
    match name {
        "none" => Some(ParserSpec::None),
        "cargo" => Some(ParserSpec::Cargo),
        "gcc" => Some(ParserSpec::Gcc),
        "go" => Some(ParserSpec::Go),
        "tsc" => Some(ParserSpec::Tsc),
        "eslint" => Some(ParserSpec::Eslint),
        _ => None,
    }
}

fn cargo_header(captures: &regex::Captures<'_>) -> Option<(Severity, String)> {
    let severity = Severity::from_name(captures.name("severity")?.as_str())?;
    let message = one_line(captures.name("message")?.as_str());
    Some((severity, message))
}

fn capture_finding(regex: &Regex, line: &str, fallback: Severity) -> Option<Finding> {
    let captures = regex.captures(line)?;
    let message = captures.name("message")?.as_str().to_string();
    finding_with_message(&captures, fallback, message)
}

fn finding_with_message(
    captures: &regex::Captures<'_>,
    fallback: Severity,
    message: String,
) -> Option<Finding> {
    let path = captures.name("file")?.as_str();
    let line = captures.name("line")?.as_str().parse::<usize>().ok()?;
    let column = captures
        .name("col")
        .and_then(|value| value.as_str().parse::<usize>().ok())
        .unwrap_or(1);
    let severity = captures
        .name("severity")
        .and_then(|value| Severity::from_name(value.as_str()))
        .unwrap_or(fallback);
    Some(Finding {
        path: PathBuf::from(path),
        line: line.max(1),
        column: column.max(1),
        severity,
        message: one_line(&message),
    })
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Deserialize)]
struct CargoLine {
    reason: Option<String>,
    message: Option<CargoMessage>,
}

#[derive(Deserialize)]
struct CargoMessage {
    level: String,
    message: String,
    #[serde(default)]
    spans: Vec<CargoSpan>,
}

#[derive(Deserialize)]
struct CargoSpan {
    file_name: String,
    line_start: usize,
    column_start: usize,
    #[serde(default)]
    is_primary: bool,
}

fn parse_cargo_json(line: &str) -> Option<Finding> {
    if !line.trim_start().starts_with('{') {
        return None;
    }
    let line: CargoLine = serde_json::from_str(line).ok()?;
    if line.reason.as_deref() != Some("compiler-message") {
        return None;
    }
    let message = line.message?;
    let span = message
        .spans
        .iter()
        .find(|span| span.is_primary)
        .or_else(|| message.spans.first())?;
    Some(Finding {
        path: PathBuf::from(&span.file_name),
        line: span.line_start.max(1),
        column: span.column_start.max(1),
        severity: Severity::from_name(&message.level).unwrap_or(Severity::Information),
        message: one_line(&message.message),
    })
}

#[derive(Deserialize)]
struct EslintReport {
    #[serde(rename = "filePath")]
    file_path: String,
    #[serde(default)]
    messages: Vec<EslintMessage>,
}

#[derive(Deserialize)]
struct EslintMessage {
    line: Option<usize>,
    column: Option<usize>,
    severity: Option<u8>,
    message: String,
    #[serde(rename = "ruleId")]
    rule_id: Option<String>,
}

fn parse_eslint(text: &str) -> Vec<Finding> {
    let Ok(reports) = serde_json::from_str::<Vec<EslintReport>>(text) else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for report in reports {
        for message in report.messages {
            findings.push(Finding {
                path: PathBuf::from(&report.file_path),
                line: message.line.unwrap_or(1).max(1),
                column: message.column.unwrap_or(1).max(1),
                severity: match message.severity {
                    Some(2) => Severity::Error,
                    Some(1) => Severity::Warning,
                    _ => Severity::Information,
                },
                message: eslint_message(&message),
            });
        }
    }
    findings
}

fn eslint_message(message: &EslintMessage) -> String {
    let text = one_line(&message.message);
    match message.rule_id.as_deref() {
        Some(rule) if !rule.is_empty() => format!("{text} ({rule})"),
        _ => text,
    }
}

fn cargo_header_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^(?P<severity>error|warning)(?:\[[^\]]+\])?:\s+(?P<message>.+)$")
            .expect("cargo header pattern")
    })
}

fn cargo_location_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^\s*-->\s+(?P<file>.+?):(?P<line>\d+):(?P<col>\d+)\s*$")
            .expect("cargo location pattern")
    })
}

fn gcc_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"^(?P<file>.+?):(?P<line>\d+):(?P<col>\d+):\s+(?P<severity>fatal error|error|warning|note):\s+(?P<message>.+)$",
        )
        .expect("gcc pattern")
    })
}

fn go_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^(?P<file>.+?):(?P<line>\d+):(?P<col>\d+):\s+(?P<message>.+)$")
            .expect("go pattern")
    })
}

fn tsc_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"^(?P<file>.+)\((?P<line>\d+),(?P<col>\d+)\):\s+(?P<severity>error|warning)\s+TS\d+:\s+(?P<message>.+)$",
        )
        .expect("tsc pattern")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(spec: ParserSpec, line: &str) -> Vec<Finding> {
        Parser::new(spec).parse_line(OutputStream::Stdout, line)
    }

    #[test]
    fn custom_regex_reads_named_groups() {
        let parser = RegexParser::new(
            r"^(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+) (?P<severity>error|warning): (?P<message>.+)$",
            "error",
        )
        .unwrap();
        let finding = parse(
            ParserSpec::Regex(parser),
            "src/a.rs:12:3 warning: unused value",
        )
        .pop()
        .unwrap();
        assert_eq!(finding.path, PathBuf::from("src/a.rs"));
        assert_eq!(finding.line, 12);
        assert_eq!(finding.column, 3);
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.message, "unused value");
    }

    #[test]
    fn custom_regex_uses_fallback_severity_and_column() {
        let parser =
            RegexParser::new(r"^(?P<file>.+):(?P<line>\d+): (?P<message>.+)$", "hint").unwrap();
        let finding = parse(ParserSpec::Regex(parser), "a.go:9: value")
            .pop()
            .unwrap();
        assert_eq!(finding.column, 1);
        assert_eq!(finding.severity, Severity::Hint);
    }

    #[test]
    fn custom_regex_requires_named_groups() {
        let error = RegexParser::new(r"^(?P<file>.+)$", "error").unwrap_err();
        assert_eq!(error, ParserError::MissingGroup("line"));
    }

    #[test]
    fn gcc_parses_location_severity_and_message() {
        let finding = parse(
            ParserSpec::Gcc,
            "src/main.c:7:9: warning: unused variable 'value'",
        )
        .pop()
        .unwrap();
        assert_eq!(finding.path, PathBuf::from("src/main.c"));
        assert_eq!(finding.line, 7);
        assert_eq!(finding.column, 9);
        assert_eq!(finding.severity, Severity::Warning);
    }

    #[test]
    fn go_parses_location_and_message() {
        let finding = parse(ParserSpec::Go, "./main.go:3:1: undefined: value")
            .pop()
            .unwrap();
        assert_eq!(finding.line, 3);
        assert_eq!(finding.severity, Severity::Error);
    }

    #[test]
    fn tsc_parses_typescript_diagnostics() {
        let finding = parse(
            ParserSpec::Tsc,
            "src/app.ts(4,11): error TS2322: Type 'string' is not assignable",
        )
        .pop()
        .unwrap();
        assert_eq!(finding.path, PathBuf::from("src/app.ts"));
        assert_eq!(finding.line, 4);
        assert_eq!(finding.column, 11);
        assert_eq!(finding.severity, Severity::Error);
    }

    #[test]
    fn cargo_json_parses_primary_spans() {
        let line = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable","spans":[{"file_name":"src/main.rs","line_start":2,"column_start":5,"is_primary":true}]}}"#;
        let finding = parse(ParserSpec::Cargo, line).pop().unwrap();
        assert_eq!(finding.path, PathBuf::from("src/main.rs"));
        assert_eq!(finding.line, 2);
        assert_eq!(finding.column, 5);
        assert_eq!(finding.severity, Severity::Warning);
    }

    #[test]
    fn cargo_human_output_combines_header_and_location() {
        let mut parser = Parser::new(ParserSpec::Cargo);
        assert!(parser
            .parse_line(OutputStream::Stderr, "error[E0425]: cannot find value `x`")
            .is_empty());
        let finding = parser
            .parse_line(OutputStream::Stderr, " --> src/main.rs:8:13")
            .pop()
            .unwrap();
        assert_eq!(finding.path, PathBuf::from("src/main.rs"));
        assert_eq!(finding.line, 8);
        assert_eq!(finding.column, 13);
        assert_eq!(finding.severity, Severity::Error);
    }

    #[test]
    fn eslint_json_is_parsed_at_finish() {
        let mut parser = Parser::new(ParserSpec::Eslint);
        parser.parse_line(
            OutputStream::Stdout,
            r#"[{"filePath":"/tmp/a.js","messages":["#,
        );
        parser.parse_line(
            OutputStream::Stdout,
            r#"{"line":2,"column":4,"severity":2,"message":"bad value","ruleId":"eqeqeq"}]}]"#,
        );
        let findings = parser.finish();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].path, PathBuf::from("/tmp/a.js"));
        assert_eq!(findings[0].message, "bad value (eqeqeq)");
    }

    #[test]
    fn parser_names_and_commands_are_inferred() {
        assert!(matches!(infer_parser("/usr/bin/cargo"), ParserSpec::Cargo));
        assert!(matches!(infer_parser("clang++"), ParserSpec::Gcc));
        assert!(matches!(parser_from_name("tsc"), Some(ParserSpec::Tsc)));
        assert!(parser_from_name("unknown").is_none());
    }
}
