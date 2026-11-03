use super::*;

#[test]
fn detects_known_extensions() {
    assert_eq!(LanguageId::from_path("a.rs"), Some(LanguageId::Rust));
    assert_eq!(LanguageId::from_path("Cargo.toml"), Some(LanguageId::Toml));
    assert_eq!(LanguageId::from_path("Cargo.lock"), Some(LanguageId::Toml));
    assert_eq!(LanguageId::from_path("a.ts"), Some(LanguageId::TypeScript));
    assert_eq!(LanguageId::from_path("a.mts"), Some(LanguageId::TypeScript));
    assert_eq!(LanguageId::from_path("a.tsx"), Some(LanguageId::Tsx));
    assert_eq!(LanguageId::from_path("a.js"), Some(LanguageId::JavaScript));
    assert_eq!(LanguageId::from_path("a.jsx"), Some(LanguageId::Jsx));
    assert_eq!(LanguageId::from_path("a.py"), Some(LanguageId::Python));
    assert_eq!(LanguageId::from_path("a.json"), Some(LanguageId::Json));
    assert_eq!(LanguageId::from_path("a.sh"), Some(LanguageId::Bash));
    assert_eq!(LanguageId::from_path("a.c"), Some(LanguageId::C));
    assert_eq!(LanguageId::from_path("a.cpp"), Some(LanguageId::Cpp));
    assert_eq!(LanguageId::from_path("a.h"), Some(LanguageId::Cpp));
    assert_eq!(LanguageId::from_path("a.go"), Some(LanguageId::Go));
}

#[test]
fn ignores_case_and_unknown_extensions() {
    assert_eq!(LanguageId::from_path("A.RS"), Some(LanguageId::Rust));
    assert_eq!(LanguageId::from_path("a.txt"), None);
    assert_eq!(LanguageId::from_path("noextension"), None);
}

#[test]
fn normalizes_alias_names() {
    assert_eq!(LanguageId::from_alias("Rust"), Some(LanguageId::Rust));
    assert_eq!(LanguageId::from_alias("rs"), Some(LanguageId::Rust));
    assert_eq!(LanguageId::from_alias("ts"), Some(LanguageId::TypeScript));
    assert_eq!(LanguageId::from_alias("mts"), Some(LanguageId::TypeScript));
    assert_eq!(LanguageId::from_alias("js"), Some(LanguageId::JavaScript));
    assert_eq!(LanguageId::from_alias("py"), Some(LanguageId::Python));
    assert_eq!(LanguageId::from_alias("sh"), Some(LanguageId::Bash));
    assert_eq!(LanguageId::from_alias("c"), Some(LanguageId::C));
    assert_eq!(LanguageId::from_alias("cpp"), Some(LanguageId::Cpp));
    assert_eq!(LanguageId::from_alias("c++"), Some(LanguageId::Cpp));
    assert_eq!(LanguageId::from_alias("go"), Some(LanguageId::Go));
    assert_eq!(LanguageId::from_alias("brainfuck"), None);
    assert_eq!(LanguageId::from_alias(&"x".repeat(33)), None);
}

fn temp_query_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_queries_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn user_queries_extend_bundled_queries() {
    let dir = temp_query_dir("extends");
    fs::write(
        dir.join("python.scm"),
        "; extends\n(attribute attribute: (identifier) @variable.member)\n",
    )
    .unwrap();
    let highlights = LanguageId::Python.highlights_from(Some(&dir), MAX_QUERY_INHERITANCE);
    let query =
        tree_sitter::Query::new(&LanguageId::Python.language(), &highlights).expect("user query");
    assert!(query.capture_names().contains(&"string"));
    assert!(query.capture_names().contains(&"variable.member"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn user_queries_replace_bundled_queries() {
    let dir = temp_query_dir("replace");
    fs::write(dir.join("python.scm"), "(identifier) @variable.member\n").unwrap();
    let highlights = LanguageId::Python.highlights_from(Some(&dir), MAX_QUERY_INHERITANCE);
    let query =
        tree_sitter::Query::new(&LanguageId::Python.language(), &highlights).expect("user query");
    assert!(!query.capture_names().contains(&"string"));
    assert!(query.capture_names().contains(&"variable.member"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn parses_query_directives() {
    let directives = query_directives("; inherits: javascript, tsx\n; extends\n");
    assert_eq!(directives.inherits, ["javascript", "tsx"]);
    assert!(directives.extends);
}

#[test]
fn builds_queries_for_every_language() {
    for language in LanguageId::ALL {
        let highlights = language.bundled_highlights();
        assert!(!highlights.is_empty());
        tree_sitter::Query::new(&language.language(), &highlights)
            .unwrap_or_else(|error| panic!("{language:?}: {error}"));
    }
}
