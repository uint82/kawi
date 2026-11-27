mod support;
mod typescript_support;

use std::sync::Arc;
use std::time::Instant;

use support::{large_typescript_source, parse_with};
use tree_sitter::{InputEdit, Point};
use typescript_support::{
    assert_incremental_parity, captures, grammar, native_language, parse_loaded, point_at, CORPUS,
    ERROR_SOURCE, HIGHLIGHTS, LOCALS, NATIVE_HIGHLIGHTS, PARSE_BUDGET, TYPESCRIPT_SOURCE,
};

#[test]
fn queries_match_the_native_sources() {
    assert_eq!(HIGHLIGHTS, NATIVE_HIGHLIGHTS);
    assert_eq!(LOCALS, tree_sitter_typescript::LOCALS_QUERY);
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
    let replacement = "const value = 1;\n";
    let start = replacement.find("1").expect("value");
    assert_incremental_parity(
        replacement,
        "const value = 42;\n",
        InputEdit {
            start_byte: start,
            old_end_byte: start + 1,
            new_end_byte: start + 2,
            start_position: point_at(replacement, start),
            old_end_position: point_at(replacement, start + 1),
            new_end_position: point_at(replacement, start + 2),
        },
    );

    let inserted = "const added = true;\n";
    let source = "function first(): number {\n  return 1;\n}\n";
    assert_incremental_parity(
        source,
        &format!("{inserted}{source}"),
        InputEdit {
            start_byte: 0,
            old_end_byte: 0,
            new_end_byte: inserted.len(),
            start_position: Point::new(0, 0),
            old_end_position: Point::new(0, 0),
            new_end_position: Point::new(1, 0),
        },
    );

    let source = "const added = true;\nfunction first(): number {\n  return 1;\n}\n";
    let updated = "function first(): number {\n  return 1;\n}\n";
    assert_incremental_parity(
        source,
        updated,
        InputEdit {
            start_byte: 0,
            old_end_byte: 20,
            new_end_byte: 0,
            start_position: Point::new(0, 0),
            old_end_position: Point::new(1, 0),
            new_end_position: Point::new(0, 0),
        },
    );
}

#[test]
fn threaded_parses_match_native() {
    let grammar = Arc::new(grammar());
    let expected = parse_with(native_language(), TYPESCRIPT_SOURCE)
        .root_node()
        .to_sexp();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let grammar = Arc::clone(&grammar);
            std::thread::spawn(move || {
                parse_loaded(&grammar, TYPESCRIPT_SOURCE)
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
    let source = large_typescript_source(2_000);
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
