mod support;

use std::sync::Arc;

use kawi_grammar::{GrammarLoader, LoadedGrammar, Pack};
use support::{fixture_pack, ADL_SOURCE};
use tree_sitter::{
    InputEdit, Language, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

fn native_language() -> Language {
    tree_sitter_adl::LANGUAGE.into()
}

fn loaded() -> LoadedGrammar {
    let pack = Pack::open(fixture_pack()).expect("open fixture");
    GrammarLoader::new()
        .expect("create loader")
        .load(&pack)
        .expect("load fixture")
}

fn parse_loaded(grammar: &LoadedGrammar, source: &str) -> Tree {
    let mut parser = grammar.parser().expect("create parser");
    parser.parse(source, None).expect("parse")
}

fn captures(
    language: &Language,
    tree: &Tree,
    query_source: &str,
    source: &str,
) -> Vec<(String, usize, usize)> {
    let query = Query::new(language, query_source).expect("compile query");
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut result = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures() {
            result.push((
                names[capture.index as usize].to_string(),
                capture.node.start_byte(),
                capture.node.end_byte(),
            ));
        }
    }
    result
}

fn point_at(source: &str, offset: usize) -> Point {
    let mut point = Point::new(0, 0);
    for byte in source.bytes().take(offset) {
        if byte == b'\n' {
            point.row += 1;
            point.column = 0;
        } else {
            point.column += 1;
        }
    }
    point
}

#[test]
fn syntax_trees_match_native() {
    let grammar = loaded();
    let native = support::parse_with(native_language(), ADL_SOURCE);
    let wasm = parse_loaded(&grammar, ADL_SOURCE);
    assert!(!native.root_node().has_error());
    assert!(!wasm.root_node().has_error());
    assert_eq!(native.root_node().to_sexp(), wasm.root_node().to_sexp());
}

#[test]
fn node_kinds_match_native() {
    let grammar = loaded();
    let native = native_language();
    let wasm = grammar.language();
    assert_eq!(native.node_kind_count(), wasm.node_kind_count());
    for id in 0..native.node_kind_count() as u16 {
        assert_eq!(native.node_kind_for_id(id), wasm.node_kind_for_id(id));
    }
}

#[test]
fn highlight_captures_match_native() {
    let grammar = loaded();
    let native = native_language();
    let native_tree = support::parse_with(native.clone(), ADL_SOURCE);
    let wasm_tree = parse_loaded(&grammar, ADL_SOURCE);

    let native_captures = captures(&native, &native_tree, grammar.highlights(), ADL_SOURCE);
    let wasm_captures = captures(
        &grammar.language(),
        &wasm_tree,
        grammar.highlights(),
        ADL_SOURCE,
    );

    assert!(!native_captures.is_empty());
    assert_eq!(native_captures, wasm_captures);
}

#[test]
fn incremental_parses_match_native() {
    let grammar = loaded();
    let mut native_parser = Parser::new();
    native_parser
        .set_language(&native_language())
        .expect("set native language");
    let mut wasm_parser = grammar.parser().expect("create wasm parser");

    let mut native_tree = native_parser.parse(ADL_SOURCE, None).expect("native parse");
    let mut wasm_tree = wasm_parser.parse(ADL_SOURCE, None).expect("wasm parse");

    let start = ADL_SOURCE.find("Person").expect("fixture contains Person");
    let edit = InputEdit {
        start_byte: start,
        old_end_byte: start + "Person".len(),
        new_end_byte: start + "People".len(),
        start_position: point_at(ADL_SOURCE, start),
        old_end_position: point_at(ADL_SOURCE, start + "Person".len()),
        new_end_position: point_at(ADL_SOURCE, start + "People".len()),
    };
    let mut edited = ADL_SOURCE.to_string();
    edited.replace_range(start..start + "Person".len(), "People");

    native_tree.edit(&edit);
    wasm_tree.edit(&edit);
    let native_after = native_parser
        .parse(&edited, Some(&native_tree))
        .expect("native reparse");
    let wasm_after = wasm_parser
        .parse(&edited, Some(&wasm_tree))
        .expect("wasm reparse");

    assert_eq!(
        native_after.root_node().to_sexp(),
        wasm_after.root_node().to_sexp()
    );
}

#[test]
fn threaded_parses_match_native() {
    let grammar = Arc::new(loaded());
    let expected = support::parse_with(native_language(), ADL_SOURCE)
        .root_node()
        .to_sexp();

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let grammar = Arc::clone(&grammar);
            std::thread::spawn(move || parse_loaded(&grammar, ADL_SOURCE).root_node().to_sexp())
        })
        .collect();

    for handle in handles {
        assert_eq!(handle.join().expect("thread"), expected);
    }
}
