mod support;

use std::time::{Duration, Instant};

use support::highlight::highlight_spans;

const HIGHLIGHT_BUDGET: Duration = Duration::from_secs(10);

#[test]
fn highlight_spans_match_across_the_corpus() {
    let grammar = support::grammar();
    let native = support::native_language();

    for (name, source) in support::CORPUS {
        let native_spans = highlight_spans(native.clone(), source, None);
        let wasm_spans = highlight_spans(grammar.language().clone(), source, Some(grammar));
        assert_eq!(wasm_spans, native_spans, "highlight mismatch in {name}");
    }
}

#[test]
fn highlights_the_features_corpus() {
    let grammar = support::grammar();
    let source = include_str!("../fixtures/corpus/features.ts");
    let spans = highlight_spans(grammar.language().clone(), source, Some(grammar));

    assert!(!spans.is_empty());
}

#[test]
fn highlights_within_budget() {
    let grammar = support::grammar();
    let source = include_str!("../fixtures/corpus/features.ts");
    let start = Instant::now();
    let spans = highlight_spans(grammar.language().clone(), source, Some(grammar));
    let elapsed = start.elapsed();

    eprintln!(
        "highlighted {} bytes into {} spans in {elapsed:?}",
        source.len(),
        spans.len()
    );
    assert!(elapsed < HIGHLIGHT_BUDGET, "highlight took {elapsed:?}");
}

#[test]
fn compares_native_and_wasm_highlight_latency() {
    let grammar = support::grammar();
    let source = support::large_source(200);

    let native_start = Instant::now();
    let native_spans = highlight_spans(support::native_language(), &source, None);
    let native_elapsed = native_start.elapsed();

    let wasm_start = Instant::now();
    let wasm_spans = highlight_spans(grammar.language().clone(), &source, Some(grammar));
    let wasm_elapsed = wasm_start.elapsed();

    eprintln!(
        "native highlight ({} bytes): {native_elapsed:?}",
        source.len()
    );
    eprintln!("wasm highlight ({} bytes): {wasm_elapsed:?}", source.len());
    assert_eq!(wasm_spans, native_spans);
}
