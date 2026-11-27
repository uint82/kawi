mod markdown_support;
mod support;

use std::sync::Arc;
use std::time::Instant;

use markdown_support::{
    inline_grammar, large_inline_source, native_inline_language, INLINE_CORPUS,
    INLINE_ERROR_SOURCE, INLINE_HIGHLIGHTS, INLINE_INJECTIONS, INLINE_LATEX_SOURCE, INLINE_SOURCE,
    NATIVE_INLINE_HIGHLIGHTS, NATIVE_INLINE_INJECTIONS, PARSE_BUDGET,
};
use support::{assert_incremental_parity, captures, parse_loaded, parse_with, point_at};
use tree_sitter::InputEdit;

#[test]
fn queries_match_the_native_sources() {
    assert_eq!(INLINE_HIGHLIGHTS, NATIVE_INLINE_HIGHLIGHTS);
    assert_eq!(INLINE_INJECTIONS, NATIVE_INLINE_INJECTIONS);
}

#[test]
fn syntax_trees_match_native() {
    let grammar = inline_grammar();
    let native = native_inline_language();
    for (name, source) in INLINE_CORPUS {
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
    let grammar = inline_grammar();
    let native = native_inline_language();
    let wasm = grammar.language();
    assert_eq!(native.node_kind_count(), wasm.node_kind_count());
    for id in 0..native.node_kind_count() as u16 {
        assert_eq!(native.node_kind_for_id(id), wasm.node_kind_for_id(id));
    }
}

#[test]
fn highlight_captures_match_native() {
    let grammar = inline_grammar();
    let native = native_inline_language();
    for (name, source) in INLINE_CORPUS {
        let native_tree = parse_with(native.clone(), source);
        let wasm_tree = parse_loaded(grammar, source);
        let native_captures = captures(&native, &native_tree, INLINE_HIGHLIGHTS, source);
        let wasm_captures = captures(&grammar.language(), &wasm_tree, INLINE_HIGHLIGHTS, source);
        assert_eq!(
            native_captures, wasm_captures,
            "highlight mismatch in {name}"
        );
    }
}

#[test]
fn injection_captures_match_native() {
    let grammar = inline_grammar();
    let native = native_inline_language();
    for (name, source) in [("features", INLINE_SOURCE), ("latex", INLINE_LATEX_SOURCE)] {
        let native_tree = parse_with(native.clone(), source);
        let wasm_tree = parse_loaded(grammar, source);
        let native_captures = captures(&native, &native_tree, INLINE_INJECTIONS, source);
        let wasm_captures = captures(&grammar.language(), &wasm_tree, INLINE_INJECTIONS, source);
        assert!(!native_captures.is_empty(), "no native captures in {name}");
        assert_eq!(
            native_captures, wasm_captures,
            "injection mismatch in {name}"
        );
    }
}

#[test]
fn scanner_errors_match_native() {
    let native_tree = parse_with(native_inline_language(), INLINE_ERROR_SOURCE);
    let wasm_tree = parse_loaded(inline_grammar(), INLINE_ERROR_SOURCE);
    assert!(native_tree.root_node().has_error());
    assert!(wasm_tree.root_node().has_error());
    assert_eq!(
        native_tree.root_node().to_sexp(),
        wasm_tree.root_node().to_sexp()
    );
}

#[test]
fn incremental_edits_match_native() {
    let source = INLINE_SOURCE;
    let start = source.find("italic").expect("italic");
    let updated = source.replacen("italic", "emphasized", 1);
    assert_incremental_parity(
        inline_grammar(),
        native_inline_language(),
        source,
        &updated,
        InputEdit {
            start_byte: start,
            old_end_byte: start + "italic".len(),
            new_end_byte: start + "emphasized".len(),
            start_position: point_at(source, start),
            old_end_position: point_at(source, start + "italic".len()),
            new_end_position: point_at(source, start + "emphasized".len()),
        },
    );

    let inserted = " Added **strong**.";
    let updated = format!("{source}{inserted}");
    assert_incremental_parity(
        inline_grammar(),
        native_inline_language(),
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
}

#[test]
fn threaded_parses_match_native() {
    let grammar = Arc::new(inline_grammar());
    let expected = parse_with(native_inline_language(), INLINE_SOURCE)
        .root_node()
        .to_sexp();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let grammar = Arc::clone(&grammar);
            std::thread::spawn(move || parse_loaded(&grammar, INLINE_SOURCE).root_node().to_sexp())
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().expect("thread"), expected);
    }
}

#[test]
fn large_file_parses_within_budget() {
    let source = large_inline_source(2_000);
    let grammar = inline_grammar();
    let mut parser = grammar.parser().expect("create parser");
    let start = Instant::now();
    let tree = parser.parse(&source, None).expect("parse");
    let elapsed = start.elapsed();

    assert!(!tree.root_node().has_error());
    assert_eq!(
        tree.root_node().to_sexp(),
        parse_with(native_inline_language(), &source)
            .root_node()
            .to_sexp()
    );
    assert!(elapsed < PARSE_BUDGET, "large parse took {elapsed:?}");
}
