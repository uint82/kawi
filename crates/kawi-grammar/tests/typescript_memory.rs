mod support;

use std::process::Command;
use std::sync::OnceLock;

use kawi_grammar::{GrammarLoader, LoadedGrammar, Pack};
use support::{fixture_typescript_pack, large_typescript_source};

const MEMORY_BUDGET_KIB: u64 = 512 * 1024;

fn grammar() -> &'static LoadedGrammar {
    static GRAMMAR: OnceLock<LoadedGrammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| {
        let pack = Pack::open(fixture_typescript_pack()).expect("open TypeScript pack");
        GrammarLoader::new()
            .expect("create loader")
            .load(&pack)
            .expect("load TypeScript pack")
    })
}

fn resident_kib() -> u64 {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .expect("read process memory");
    assert!(output.status.success(), "ps failed");
    String::from_utf8(output.stdout)
        .expect("utf-8 ps output")
        .trim()
        .parse()
        .expect("resident set size")
}

#[test]
fn repeated_parses_stay_within_the_memory_budget() {
    let before = resident_kib();
    let source = large_typescript_source(500);
    for _ in 0..4 {
        let mut parser = grammar().parser().expect("create parser");
        let tree = parser.parse(&source, None).expect("parse");
        assert!(!tree.root_node().has_error());
    }
    let after = resident_kib();
    let growth = after.saturating_sub(before);
    assert!(
        growth < MEMORY_BUDGET_KIB,
        "wasm parsers grew resident memory by {growth} KiB"
    );
}
