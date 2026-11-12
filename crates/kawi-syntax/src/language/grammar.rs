use super::LanguageId;

pub(super) fn language(language: LanguageId) -> tree_sitter::Language {
    match language {
        LanguageId::Rust => tree_sitter_rust::LANGUAGE.into(),
        LanguageId::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
        LanguageId::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        LanguageId::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        LanguageId::JavaScript | LanguageId::Jsx => tree_sitter_javascript::LANGUAGE.into(),
        LanguageId::Python => tree_sitter_python::LANGUAGE.into(),
        LanguageId::Json => tree_sitter_json::LANGUAGE.into(),
        LanguageId::Bash => tree_sitter_bash::LANGUAGE.into(),
        LanguageId::C => tree_sitter_c::LANGUAGE.into(),
        LanguageId::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        LanguageId::Go => tree_sitter_go::LANGUAGE.into(),
        LanguageId::Java => tree_sitter_java::LANGUAGE.into(),
        LanguageId::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        LanguageId::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        LanguageId::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        LanguageId::Lua => tree_sitter_lua::LANGUAGE.into(),
        LanguageId::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        LanguageId::Html => tree_sitter_html::LANGUAGE.into(),
        LanguageId::Css => tree_sitter_css::LANGUAGE.into(),
        LanguageId::Markdown => tree_sitter_md::LANGUAGE.into(),
    }
}

pub(super) fn injections(language: LanguageId) -> &'static str {
    match language {
        LanguageId::Html => tree_sitter_html::INJECTIONS_QUERY,
        LanguageId::Markdown => tree_sitter_md::INJECTION_QUERY_BLOCK,
        _ => "",
    }
}

pub(super) fn bundled_highlights(language: LanguageId) -> String {
    format!(
        "{}\n{}",
        base_highlights(language),
        query_additions(language)
    )
}

fn base_highlights(language: LanguageId) -> String {
    let javascript = tree_sitter_javascript::HIGHLIGHT_QUERY;
    let jsx = tree_sitter_javascript::JSX_HIGHLIGHT_QUERY;
    match language {
        LanguageId::TypeScript | LanguageId::Tsx => format!(
            "{}\n{}",
            include_str!("../../queries/ecmascript.scm"),
            include_str!("../../queries/ecmascript-tokens.scm")
        ),
        LanguageId::JavaScript => javascript.to_string(),
        LanguageId::Jsx => format!("{javascript}\n{jsx}"),
        LanguageId::Rust => format!(
            "{}\n{}\n{}",
            include_str!("../../queries/rust.scm"),
            include_str!("../../queries/rust-tokens.scm"),
            include_str!("../../queries/rust-regex.scm")
        ),
        LanguageId::Toml => tree_sitter_toml_ng::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Python => format!(
            "{}\n{}\n{}\n{}",
            include_str!("../../queries/python.scm"),
            include_str!("../../queries/python-symbols.scm"),
            include_str!("../../queries/python-types.scm"),
            include_str!("../../queries/python-regex.scm")
        ),
        LanguageId::Json => tree_sitter_json::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Bash => tree_sitter_bash::HIGHLIGHT_QUERY.to_string(),
        LanguageId::C => format!(
            "{}\n{}",
            include_str!("../../queries/c.scm"),
            include_str!("../../queries/c-symbols.scm")
        ),
        LanguageId::Cpp => format!(
            "{}\n{}\n{}",
            include_str!("../../queries/c.scm"),
            include_str!("../../queries/c-symbols.scm"),
            tree_sitter_cpp::HIGHLIGHT_QUERY
        ),
        LanguageId::Go => tree_sitter_go::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Java => tree_sitter_java::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::CSharp => tree_sitter_c_sharp::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Ruby => tree_sitter_ruby::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Php => tree_sitter_php::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Lua => tree_sitter_lua::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Yaml => tree_sitter_yaml::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Html => tree_sitter_html::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Css => tree_sitter_css::HIGHLIGHTS_QUERY.to_string(),
        LanguageId::Markdown => tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.to_string(),
    }
}

fn query_additions(language: LanguageId) -> String {
    let script = include_str!("../../queries/script-common.scm");
    let javascript = include_str!("../../queries/javascript.scm");
    let typescript = include_str!("../../queries/typescript.scm");
    let jsx = include_str!("../../queries/jsx.scm");
    match language {
        LanguageId::Rust => String::new(),
        LanguageId::Python => String::new(),
        LanguageId::JavaScript => format!("{script}\n{javascript}"),
        LanguageId::Jsx => format!("{script}\n{javascript}\n{jsx}"),
        LanguageId::TypeScript => typescript.to_string(),
        LanguageId::Tsx => format!("{typescript}\n{jsx}"),
        LanguageId::Bash => include_str!("../../queries/bash.scm").to_string(),
        LanguageId::Toml => include_str!("../../queries/toml.scm").to_string(),
        LanguageId::Json => include_str!("../../queries/json.scm").to_string(),
        LanguageId::C => String::new(),
        LanguageId::Cpp => format!(
            "{}\n{}",
            include_str!("../../queries/cpp.scm"),
            include_str!("../../queries/cpp-symbols.scm")
        ),
        LanguageId::Go => include_str!("../../queries/go.scm").to_string(),
        LanguageId::Java => include_str!("../../queries/java.scm").to_string(),
        LanguageId::CSharp => include_str!("../../queries/csharp.scm").to_string(),
        LanguageId::Ruby => include_str!("../../queries/ruby.scm").to_string(),
        LanguageId::Php => include_str!("../../queries/php.scm").to_string(),
        LanguageId::Lua => include_str!("../../queries/lua.scm").to_string(),
        LanguageId::Yaml => include_str!("../../queries/yaml.scm").to_string(),
        LanguageId::Html => include_str!("../../queries/html.scm").to_string(),
        LanguageId::Css => include_str!("../../queries/css.scm").to_string(),
        LanguageId::Markdown => include_str!("../../queries/markdown.scm").to_string(),
    }
}
