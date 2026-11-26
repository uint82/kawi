mod support;

use std::time::{Duration, Instant};

use kawi_grammar_spike::GrammarEngine;

const LOAD_BUDGET: Duration = Duration::from_secs(10);
const PARSE_BUDGET: Duration = Duration::from_secs(10);

#[test]
fn loads_the_grammar_within_budget() {
    let start = Instant::now();
    let engine = GrammarEngine::new().expect("Wasm engine");
    let grammar = engine
        .load(&support::TYPESCRIPT, support::TYPESCRIPT_WASM)
        .expect("load TypeScript grammar");
    let elapsed = start.elapsed();

    eprintln!("first load: {elapsed:?}");
    assert!(grammar.is_wasm());
    assert!(elapsed < LOAD_BUDGET, "first load took {elapsed:?}");
}

#[test]
fn parses_a_large_file_within_budget() {
    let grammar = support::grammar();
    let source = support::large_source(2_000);
    let mut parser = grammar.parser().expect("parser");
    let start = Instant::now();
    let tree = parser.parse(&source, None).expect("parse");
    let elapsed = start.elapsed();

    eprintln!("large parse ({} bytes): {elapsed:?}", source.len());
    assert!(!tree.root_node().has_error());
    assert!(elapsed < PARSE_BUDGET, "large parse took {elapsed:?}");
}
