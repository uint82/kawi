use kawi_grammar_spike::Grammar;
use tree_sitter::Language;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

pub const HIGHLIGHTS_QUERY: &str = concat!(
    include_str!("../../../../crates/kawi-syntax/queries/ecmascript.scm"),
    "\n",
    include_str!("../../../../crates/kawi-syntax/queries/ecmascript-tokens.scm"),
    "\n",
    include_str!("../../../../crates/kawi-syntax/queries/typescript.scm"),
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub capture: String,
}

pub fn highlight_spans(
    language: Language,
    source: &str,
    grammar: Option<&Grammar>,
) -> Vec<HighlightSpan> {
    let mut config = HighlightConfiguration::new(language, "typescript", HIGHLIGHTS_QUERY, "", "")
        .expect("TypeScript highlight query");
    let captures: Vec<String> = config
        .query
        .capture_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    config.configure(&captures);

    let mut highlighter = Highlighter::new();
    if let Some(grammar) = grammar {
        grammar
            .configure_parser(highlighter.parser())
            .expect("Wasm parser for the highlighter");
    }

    let events = highlighter
        .highlight(&config, source.as_bytes(), None, None, |_| None)
        .expect("highlight");
    collect_spans(events, &captures)
}

fn collect_spans(
    events: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>,
    captures: &[String],
) -> Vec<HighlightSpan> {
    let mut stack = Vec::new();
    let mut spans = Vec::new();
    for event in events {
        match event.expect("highlight event") {
            HighlightEvent::HighlightStart(highlight) => stack.push(highlight.0),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                if let Some(&index) = stack.last() {
                    spans.push(HighlightSpan {
                        start,
                        end,
                        capture: captures[index].clone(),
                    });
                }
            }
        }
    }
    spans
}
