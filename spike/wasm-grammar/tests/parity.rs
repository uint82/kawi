mod support;

use support::parse::{assert_incremental_parity, assert_tree_parity};
use tree_sitter::{InputEdit, Point};

#[test]
fn trees_match_across_the_corpus() {
    for (name, source) in support::CORPUS {
        assert_tree_parity(support::grammar(), source);
        eprintln!("matched {name}");
    }
}

#[test]
fn large_generated_file_matches() {
    let source = support::large_source(200);
    assert_tree_parity(support::grammar(), &source);
}

#[test]
fn incremental_replacement_matches() {
    let source = "const value = 1;\n";
    let updated = "const value = 42;\n";
    let edit = InputEdit {
        start_byte: 14,
        old_end_byte: 15,
        new_end_byte: 16,
        start_position: Point::new(0, 14),
        old_end_position: Point::new(0, 15),
        new_end_position: Point::new(0, 16),
    };

    assert_incremental_parity(support::grammar(), source, updated, &edit);
}

#[test]
fn incremental_insertion_matches() {
    let source = "function first(): number {\n  return 1;\n}\n";
    let inserted = "const added = true;\n";
    let updated = format!("{inserted}{source}");
    let edit = InputEdit {
        start_byte: 0,
        old_end_byte: 0,
        new_end_byte: inserted.len(),
        start_position: Point::new(0, 0),
        old_end_position: Point::new(0, 0),
        new_end_position: Point::new(1, 0),
    };

    assert_incremental_parity(support::grammar(), source, &updated, &edit);
}

#[test]
fn incremental_deletion_matches() {
    let source = "const added = true;\nfunction first(): number {\n  return 1;\n}\n";
    let updated = "function first(): number {\n  return 1;\n}\n";
    let edit = InputEdit {
        start_byte: 0,
        old_end_byte: 20,
        new_end_byte: 0,
        start_position: Point::new(0, 0),
        old_end_position: Point::new(1, 0),
        new_end_position: Point::new(0, 0),
    };

    assert_incremental_parity(support::grammar(), source, updated, &edit);
}
