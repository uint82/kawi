mod markdown_support;
mod support;

use std::sync::Arc;
use std::time::Instant;

use markdown_support::{
    grammar, large_markdown_source, native_language, CORPUS, ERROR_SOURCE, HIGHLIGHTS, INJECTIONS,
    MARKDOWN_SOURCE, NATIVE_HIGHLIGHTS, NATIVE_INJECTIONS, PARSE_BUDGET, TOML_METADATA_SOURCE,
};
use support::{assert_incremental_parity, captures, parse_loaded, parse_with, point_at};
use tree_sitter::InputEdit;

#[test]
fn queries_match_the_native_sources() {
    assert_eq!(HIGHLIGHTS, NATIVE_HIGHLIGHTS);
    assert_eq!(INJECTIONS, NATIVE_INJECTIONS);
}

#[test]
fn syntax_trees_match_native() {
    let grammar = grammar();
    let native = native_language();
    for (name, source) in CORPUS {
        let native_tree = parse_with(native.clone(), source);
        let wasm_tree = parse_loaded(grammar, source);
        assert_eq!(
            wasm_tree.root_node().to_sexp(),
            native_tree.root_node().to_sexp(),
            "tree mismatch in {name}"
        );
        assert_eq!(
            wasm_tree.root_node().end_byte(),
            native_tree.root_node().end_byte(),
            "end mismatch in {name}"
        );
        assert_eq!(
            wasm_tree.root_node().has_error(),
            native_tree.root_node().has_error(),
            "error flag mismatch in {name}"
        );
    }
}

#[test]
fn node_kinds_match_native() {
    let grammar = grammar();
    let native = native_language();
    let wasm = grammar.language();
    assert_eq!(native.node_kind_count(), wasm.node_kind_count());
    for id in 0..native.node_kind_count() as u16 {
        assert_eq!(native.node_kind_for_id(id), wasm.node_kind_for_id(id));
    }
}

#[test]
fn highlight_captures_match_native() {
    let grammar = grammar();
    let native = native_language();
    for (name, source) in CORPUS {
        let native_tree = parse_with(native.clone(), source);
        let wasm_tree = parse_loaded(grammar, source);
        let native_captures = captures(&native, &native_tree, HIGHLIGHTS, source);
        let wasm_captures = captures(&grammar.language(), &wasm_tree, HIGHLIGHTS, source);
        assert_eq!(
            native_captures, wasm_captures,
            "highlight mismatch in {name}"
        );
    }
}

#[test]
fn injection_captures_match_native() {
    let grammar = grammar();
    let native = native_language();
    for (name, source) in [
        ("features", MARKDOWN_SOURCE),
        ("toml_metadata", TOML_METADATA_SOURCE),
    ] {
        let native_tree = parse_with(native.clone(), source);
        let wasm_tree = parse_loaded(grammar, source);
        let native_captures = captures(&native, &native_tree, INJECTIONS, source);
        let wasm_captures = captures(&grammar.language(), &wasm_tree, INJECTIONS, source);
        assert!(!native_captures.is_empty(), "no native captures in {name}");
        assert_eq!(
            native_captures, wasm_captures,
            "injection mismatch in {name}"
        );
    }
}

#[test]
fn scanner_errors_match_native() {
    let native_tree = parse_with(native_language(), ERROR_SOURCE);
    let wasm_tree = parse_loaded(grammar(), ERROR_SOURCE);
    assert!(native_tree.root_node().has_error());
    assert!(wasm_tree.root_node().has_error());
    assert_eq!(
        native_tree.root_node().to_sexp(),
        wasm_tree.root_node().to_sexp()
    );
}

#[test]
fn incremental_edits_match_native() {
    let source = MARKDOWN_SOURCE;
    let start = source.find("Title").expect("heading");
    let updated = source.replacen("Title", "Heading", 1);
    assert_incremental_parity(
        grammar(),
        native_language(),
        source,
        &updated,
        InputEdit {
            start_byte: start,
            old_end_byte: start + "Title".len(),
            new_end_byte: start + "Heading".len(),
            start_position: point_at(source, start),
            old_end_position: point_at(source, start + "Title".len()),
            new_end_position: point_at(source, start + "Heading".len()),
        },
    );

    let inserted = "\n## Added\n";
    let updated = format!("{source}{inserted}");
    assert_incremental_parity(
        grammar(),
        native_language(),
        source,
        &updated,
        InputEdit {
            start_byte: source.len(),
            old_end_byte: source.len(),
            new_end_byte: source.len() + inserted.len(),
            start_position: point_at(source, source.len()),
            old_end_position: point_at(source, source.len()),
            new_end_position: point_at(&updated, updated.len()),
        },
    );

    let removed = "Setext\n======\n\n";
    let start = source.find(removed).expect("setext heading");
    let updated = source.replacen(removed, "", 1);
    assert_incremental_parity(
        grammar(),
        native_language(),
        source,
        &updated,
        InputEdit {
            start_byte: start,
            old_end_byte: start + removed.len(),
            new_end_byte: start,
            start_position: point_at(source, start),
            old_end_position: point_at(source, start + removed.len()),
            new_end_position: point_at(source, start),
        },
    );
}

#[test]
fn threaded_parses_match_native() {
    let grammar = Arc::new(grammar());
    let expected = parse_with(native_language(), MARKDOWN_SOURCE)
        .root_node()
        .to_sexp();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let grammar = Arc::clone(&grammar);
            std::thread::spawn(move || {
                parse_loaded(&grammar, MARKDOWN_SOURCE)
                    .root_node()
                    .to_sexp()
            })
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().expect("thread"), expected);
    }
}

#[test]
fn large_file_parses_within_budget() {
    let source = large_markdown_source(2_000);
    let grammar = grammar();
    let mut parser = grammar.parser().expect("create parser");
    let start = Instant::now();
    let tree = parser.parse(&source, None).expect("parse");
    let elapsed = start.elapsed();

    assert!(!tree.root_node().has_error());
    assert_eq!(
        tree.root_node().to_sexp(),
        parse_with(native_language(), &source).root_node().to_sexp()
    );
    assert!(elapsed < PARSE_BUDGET, "large parse took {elapsed:?}");
}
