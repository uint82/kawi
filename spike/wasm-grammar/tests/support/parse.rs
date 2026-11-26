use kawi_grammar_spike::Grammar;
use tree_sitter::{InputEdit, Parser, Tree};

pub fn native_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&super::native_language())
        .expect("native TypeScript grammar");
    parser
}

pub fn wasm_parser(grammar: &Grammar) -> Parser {
    grammar.parser().expect("Wasm TypeScript grammar")
}

pub fn parse(parser: &mut Parser, source: &str) -> Tree {
    parser.parse(source, None).expect("parse")
}

pub fn assert_tree_parity(grammar: &Grammar, source: &str) {
    let native_tree = parse(&mut native_parser(), source);
    let wasm_tree = parse(&mut wasm_parser(grammar), source);

    assert_eq!(
        wasm_tree.root_node().to_sexp(),
        native_tree.root_node().to_sexp()
    );
    assert_eq!(
        wasm_tree.root_node().end_byte(),
        native_tree.root_node().end_byte()
    );
    assert_eq!(
        wasm_tree.root_node().has_error(),
        native_tree.root_node().has_error()
    );
}

pub fn assert_incremental_parity(grammar: &Grammar, source: &str, updated: &str, edit: &InputEdit) {
    let mut native = native_parser();
    let mut native_tree = parse(&mut native, source);
    native_tree.edit(edit);
    let native_updated = native
        .parse(updated, Some(&native_tree))
        .expect("native reparse");

    let mut wasm = wasm_parser(grammar);
    let mut wasm_tree = parse(&mut wasm, source);
    wasm_tree.edit(edit);
    let wasm_updated = wasm.parse(updated, Some(&wasm_tree)).expect("Wasm reparse");

    assert_eq!(
        wasm_updated.root_node().to_sexp(),
        native_updated.root_node().to_sexp()
    );
    assert_eq!(
        wasm_updated.root_node().end_byte(),
        native_updated.root_node().end_byte()
    );
}
