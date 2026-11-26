#![allow(dead_code)]

pub mod highlight;
pub mod parse;

use std::sync::OnceLock;

use kawi_grammar_spike::{Grammar, GrammarEngine, GrammarSpec};

pub const TYPESCRIPT_SHA256: &str =
    "13677574a6375e1b12e98579850c55a4db5952ad8bacd673110007ac7cfd7e69";
pub const TYPESCRIPT_ABI: usize = 14;
pub const TYPESCRIPT_WASM: &[u8] = include_bytes!("../../fixtures/typescript.wasm");

pub const TYPESCRIPT: GrammarSpec = GrammarSpec {
    name: "typescript",
    sha256: TYPESCRIPT_SHA256,
};

pub const CORPUS: &[(&str, &str)] = &[
    (
        "features.ts",
        include_str!("../../fixtures/corpus/features.ts"),
    ),
    ("unicode.ts", include_str!("../../fixtures/corpus/unicode.ts")),
    ("errors.ts", include_str!("../../fixtures/corpus/errors.ts")),
    ("empty.ts", ""),
    (
        "crlf.ts",
        "const value: number = 1;\r\nfunction add(a: number, b: number): number {\r\n  return a + b;\r\n}\r\n",
    ),
];

pub fn grammar() -> &'static Grammar {
    static GRAMMAR: OnceLock<Grammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| {
        let engine = GrammarEngine::new().expect("Wasm engine");
        engine
            .load(&TYPESCRIPT, TYPESCRIPT_WASM)
            .expect("TypeScript grammar")
    })
}

pub fn native_language() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

pub fn large_source(functions: usize) -> String {
    use std::fmt::Write;

    let mut source = String::with_capacity(functions * 96);
    for index in 0..functions {
        writeln!(
            source,
            "export function fn{index}(value: number): number {{"
        )
        .expect("writing to a String cannot fail");
        writeln!(source, "  const next = value + {index};")
            .expect("writing to a String cannot fail");
        source.push_str("  return next * 2;\n}\n");
    }
    source
}
