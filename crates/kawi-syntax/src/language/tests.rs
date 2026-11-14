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
    assert_eq!(LanguageId::from_path("a.java"), Some(LanguageId::Java));
    assert_eq!(LanguageId::from_path("a.cs"), Some(LanguageId::CSharp));
    assert_eq!(LanguageId::from_path("a.rb"), Some(LanguageId::Ruby));
    assert_eq!(LanguageId::from_path("a.adb"), Some(LanguageId::Ada));
    assert_eq!(LanguageId::from_path("a.ads"), Some(LanguageId::Ada));
    assert_eq!(LanguageId::from_path("a.php"), Some(LanguageId::Php));
    assert_eq!(LanguageId::from_path("a.lua"), Some(LanguageId::Lua));
    assert_eq!(LanguageId::from_path("a.yaml"), Some(LanguageId::Yaml));
    assert_eq!(LanguageId::from_path("a.yml"), Some(LanguageId::Yaml));
    assert_eq!(LanguageId::from_path("a.html"), Some(LanguageId::Html));
    assert_eq!(LanguageId::from_path("a.css"), Some(LanguageId::Css));
    assert_eq!(LanguageId::from_path("a.md"), Some(LanguageId::Markdown));
    assert_eq!(
        LanguageId::from_path("a.markdown"),
        Some(LanguageId::Markdown)
    );
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
    assert_eq!(LanguageId::from_alias("java"), Some(LanguageId::Java));
    assert_eq!(LanguageId::from_alias("cs"), Some(LanguageId::CSharp));
    assert_eq!(LanguageId::from_alias("csharp"), Some(LanguageId::CSharp));
    assert_eq!(LanguageId::from_alias("rb"), Some(LanguageId::Ruby));
    assert_eq!(LanguageId::from_alias("ada"), Some(LanguageId::Ada));
    assert_eq!(LanguageId::from_alias("adb"), Some(LanguageId::Ada));
    assert_eq!(LanguageId::from_alias("ads"), Some(LanguageId::Ada));
    assert_eq!(LanguageId::from_alias("php"), Some(LanguageId::Php));
    assert_eq!(LanguageId::from_alias("lua"), Some(LanguageId::Lua));
    assert_eq!(LanguageId::from_alias("yaml"), Some(LanguageId::Yaml));
    assert_eq!(LanguageId::from_alias("yml"), Some(LanguageId::Yaml));
    assert_eq!(LanguageId::from_alias("html"), Some(LanguageId::Html));
    assert_eq!(LanguageId::from_alias("css"), Some(LanguageId::Css));
    assert_eq!(LanguageId::from_alias("md"), Some(LanguageId::Markdown));
    assert_eq!(
        LanguageId::from_alias("markdown"),
        Some(LanguageId::Markdown)
    );
    assert_eq!(
        LanguageId::from_alias("markdown_inline"),
        Some(LanguageId::MarkdownInline)
    );
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

#[test]
fn query_cache_reloads_after_a_file_change() {
    let dir = temp_query_dir("cache_reload");
    let path = dir.join("python.scm");
    fs::write(&path, "(identifier) @variable.member\n").unwrap();
    let mut cache = QueryCache::new();
    cache.set_dir(dir.clone());
    assert!(cache
        .highlights(LanguageId::Python)
        .contains("variable.member"));

    fs::write(&path, "(identifier) @function\n").unwrap();
    cache.reload();
    let highlights = cache.highlights(LanguageId::Python);
    assert!(highlights.contains("@function"));
    assert!(!highlights.contains("variable.member"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn query_cache_switches_directories() {
    let first = temp_query_dir("cache_first");
    let second = temp_query_dir("cache_second");
    fs::write(first.join("python.scm"), "(identifier) @variable.member\n").unwrap();
    fs::write(second.join("python.scm"), "(identifier) @function\n").unwrap();
    let mut cache = QueryCache::new();
    cache.set_dir(first.clone());
    assert!(cache
        .highlights(LanguageId::Python)
        .contains("variable.member"));

    cache.set_dir(second.clone());
    let highlights = cache.highlights(LanguageId::Python);
    assert!(highlights.contains("@function"));
    assert!(!highlights.contains("variable.member"));
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn invalid_user_queries_are_reported() {
    let dir = temp_query_dir("invalid_warning");
    let path = dir.join("python.scm");
    fs::write(&path, "(identifier\n").unwrap();
    let warnings = query_warnings(&dir);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].starts_with(&path.display().to_string()));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn valid_user_queries_produce_no_warnings() {
    let dir = temp_query_dir("valid_warning");
    fs::write(dir.join("python.scm"), "(identifier) @variable.member\n").unwrap();
    assert!(query_warnings(&dir).is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn invalid_user_queries_fall_back_to_bundled_queries() {
    let dir = temp_query_dir("invalid_fallback");
    fs::write(dir.join("python.scm"), "(identifier\n").unwrap();
    let mut cache = QueryCache::new();
    cache.set_dir(dir.clone());
    assert_eq!(
        &*cache.highlights(LanguageId::Python),
        LanguageId::Python.bundled_highlights()
    );
    let _ = fs::remove_dir_all(dir);
}
