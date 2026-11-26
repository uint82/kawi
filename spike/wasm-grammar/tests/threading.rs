mod support;

use kawi_grammar_spike::Grammar;

#[test]
fn grammar_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Grammar>();
}

#[test]
fn two_parsers_use_separate_stores() {
    let grammar = support::grammar();
    let mut first = grammar.parser().expect("first parser");
    let mut second = grammar.parser().expect("second parser");

    let first_tree = first
        .parse("const value = 1;\n", None)
        .expect("first parse");
    let second_tree = second
        .parse("function run(): void {}\n", None)
        .expect("second parse");

    assert!(!first_tree.root_node().has_error());
    assert!(!second_tree.root_node().has_error());
}

#[test]
fn parsers_share_one_language_across_threads() {
    let grammar = support::grammar();

    std::thread::scope(|scope| {
        for index in 0..4 {
            scope.spawn(move || {
                let source = support::large_source(25 + index);
                let mut parser = grammar.parser().expect("thread parser");
                let tree = parser.parse(&source, None).expect("thread parse");

                assert!(!tree.root_node().has_error());
            });
        }
    });
}
