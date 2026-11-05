use super::{languages, servers};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[test]
fn built_in_language_names_are_unique() {
    let mut names = BTreeSet::new();
    for language in languages() {
        assert!(
            names.insert(language.name.clone()),
            "duplicate language {}",
            language.name
        );
    }
}

#[test]
fn built_in_extensions_are_unique() {
    let mut seen = BTreeMap::new();
    for language in languages() {
        for extension in &language.extensions {
            let key = extension.to_ascii_lowercase();
            if let Some(existing) = seen.insert(key, language.name.clone()) {
                panic!(
                    "extension {extension} is used by {existing} and {}",
                    language.name
                );
            }
        }
    }
}

#[test]
fn built_in_language_servers_are_configured() {
    let servers = servers();
    for language in languages() {
        for server in &language.servers {
            assert!(
                servers.contains_key(server),
                "language {} references unknown server {server}",
                language.name
            );
        }
    }
}

#[test]
fn built_in_servers_are_used() {
    let used: BTreeSet<String> = languages()
        .into_iter()
        .flat_map(|language| language.servers)
        .collect();
    for name in servers().keys() {
        assert!(used.contains(name), "server {name} is not used");
    }
}

#[test]
fn built_in_languages_map_to_standard_servers() {
    let languages = languages();
    let servers_for = |name: &str| {
        languages
            .iter()
            .find(|language| language.name == name)
            .expect("language")
            .servers
            .clone()
    };
    assert_eq!(servers_for("rust"), vec!["rust-analyzer"]);
    assert_eq!(servers_for("python"), vec!["pyright", "ruff"]);
    assert_eq!(servers_for("c"), vec!["clangd"]);
    assert_eq!(servers_for("cpp"), vec!["clangd"]);
    assert_eq!(servers_for("go"), vec!["gopls"]);
    assert_eq!(servers_for("gomod"), vec!["gopls"]);
    assert_eq!(
        servers_for("typescript"),
        vec!["typescript-language-server"]
    );
    assert_eq!(servers_for("html"), vec!["vscode-html-language-server"]);
    assert_eq!(servers_for("yaml"), vec!["yaml-language-server"]);
    assert_eq!(servers_for("toml"), vec!["taplo"]);
    assert_eq!(servers_for("ruby"), vec!["ruby-lsp"]);
    assert_eq!(servers_for("lua"), vec!["lua-language-server"]);
    assert_eq!(servers_for("bash"), vec!["bash-language-server"]);
    assert_eq!(servers_for("java"), vec!["jdtls"]);
    assert_eq!(servers_for("kotlin"), vec!["kotlin-language-server"]);
    assert_eq!(servers_for("csharp"), vec!["csharp-ls"]);
    assert_eq!(servers_for("haskell"), vec!["haskell-language-server"]);
    assert_eq!(servers_for("ocaml"), vec!["ocamllsp"]);
    assert_eq!(servers_for("gleam"), vec!["gleam"]);
    assert_eq!(servers_for("sql"), vec!["sqls"]);
    assert_eq!(servers_for("protobuf"), vec!["buf"]);
    assert_eq!(servers_for("terraform"), vec!["terraform-ls"]);
    assert_eq!(servers_for("nix"), vec!["nixd"]);
    assert_eq!(servers_for("dockerfile"), vec!["docker-langserver"]);
    assert_eq!(servers_for("markdown"), vec!["marksman"]);
    assert_eq!(servers_for("latex"), vec!["texlab"]);
    assert_eq!(servers_for("typst"), vec!["tinymist"]);
}

#[test]
fn jsonc_filenames_precede_json_extensions() {
    let language = languages()
        .into_iter()
        .find(|language| language.matches(Path::new("tsconfig.json")))
        .expect("language for tsconfig.json");
    assert_eq!(language.name, "jsonc");
}

#[test]
fn built_in_languages_define_indentation() {
    let languages = languages();
    for language in &languages {
        assert!(
            language.indent.shiftwidth.is_some(),
            "language {} has no shiftwidth",
            language.name
        );
        assert!(
            language.indent.expandtab.is_some(),
            "language {} has no expandtab",
            language.name
        );
        assert!(
            language.indent.tabstop.is_some(),
            "language {} has no tabstop",
            language.name
        );
        assert_eq!(
            language.indent_override,
            crate::IndentConfig::default(),
            "built-in language {} has a user override",
            language.name
        );
    }
}

#[test]
fn built_in_indentation_matches_language_conventions() {
    let indent = |name: &str| {
        languages()
            .into_iter()
            .find(|language| language.name == name)
            .expect("language")
            .indent
    };
    assert_eq!(indent("python"), crate::IndentConfig::spaces(4));
    assert_eq!(indent("rust"), crate::IndentConfig::spaces(4));
    assert_eq!(indent("typescript"), crate::IndentConfig::spaces(2));
    assert_eq!(indent("javascript"), crate::IndentConfig::spaces(2));
    assert_eq!(indent("yaml"), crate::IndentConfig::spaces(2));
    assert_eq!(indent("go"), crate::IndentConfig::tabs(8));
    assert_eq!(indent("markdown"), crate::IndentConfig::spaces(4));
    assert_eq!(indent("fortran"), crate::IndentConfig::spaces(3));
}

#[test]
fn built_in_tag_indentation_matches_language_defaults() {
    let languages = languages();
    let indent_tags = |name: &str| {
        languages
            .iter()
            .find(|language| language.name == name)
            .expect("language")
            .indent_tags
    };
    assert!(indent_tags("html"));
    for name in ["rust", "python", "json", "xml", "markdown"] {
        assert!(!indent_tags(name), "{name} should not indent tags");
    }
}

#[test]
fn built_in_color_preview_matches_language_defaults() {
    let languages = languages();
    let preview = |name: &str| {
        languages
            .iter()
            .find(|language| language.name == name)
            .expect("language")
            .color_preview
    };
    for name in ["css", "scss", "less", "html", "vue", "svelte", "astro"] {
        assert!(preview(name), "{name} should preview colors");
    }
    for name in ["rust", "javascript", "json", "xml", "markdown"] {
        assert!(!preview(name), "{name} should not preview colors");
    }
}
