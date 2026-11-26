mod support;

use std::time::{Duration, Instant};

use kawi_grammar_spike::GrammarEngine;
use support::parse::native_parser;

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

#[test]
fn compares_native_and_wasm_parse_latency() {
    let grammar = support::grammar();
    let source = support::large_source(2_000);
    let mut native = native_parser();
    let mut wasm = grammar.parser().expect("parser");

    let native_start = Instant::now();
    let native_tree = native.parse(&source, None).expect("native parse");
    let native_elapsed = native_start.elapsed();

    let wasm_start = Instant::now();
    let wasm_tree = wasm.parse(&source, None).expect("Wasm parse");
    let wasm_elapsed = wasm_start.elapsed();

    eprintln!("native parse ({} bytes): {native_elapsed:?}", source.len());
    eprintln!("wasm parse ({} bytes): {wasm_elapsed:?}", source.len());
    assert_eq!(
        wasm_tree.root_node().to_sexp(),
        native_tree.root_node().to_sexp()
    );
}
