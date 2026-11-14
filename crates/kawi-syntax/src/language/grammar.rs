use std::borrow::Cow;

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
        LanguageId::MarkdownInline => tree_sitter_md::INLINE_LANGUAGE.into(),
        LanguageId::Ada => tree_sitter_ada::LANGUAGE.into(),
    }
}

pub(super) fn injections(language: LanguageId) -> Cow<'static, str> {
    match language {
        LanguageId::Html => Cow::Borrowed(tree_sitter_html::INJECTIONS_QUERY),
        LanguageId::Markdown => {
            Cow::Borrowed(include_str!("../../queries/markdown-injections.scm"))
        }
        LanguageId::MarkdownInline => {
            Cow::Borrowed(include_str!("../../queries/markdown-inline-injections.scm"))
        }
        LanguageId::Php => Cow::Owned(format!(
            "{}\n{}",
            tree_sitter_php::INJECTIONS_QUERY,
            include_str!("../../queries/php-injections.scm")
        )),
        _ => Cow::Borrowed(""),
    }
}

pub(super) fn locals(language: LanguageId) -> &'static str {
    match language {
        LanguageId::JavaScript | LanguageId::Jsx => tree_sitter_javascript::LOCALS_QUERY,
        LanguageId::TypeScript | LanguageId::Tsx => tree_sitter_typescript::LOCALS_QUERY,
        LanguageId::Ruby => tree_sitter_ruby::LOCALS_QUERY,
        LanguageId::Lua => tree_sitter_lua::LOCALS_QUERY,
        LanguageId::Ada => tree_sitter_ada::LOCALS_QUERY,
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
    match language {
        LanguageId::TypeScript | LanguageId::Tsx | LanguageId::JavaScript | LanguageId::Jsx => {
            format!(
                "{}\n{}",
                include_str!("../../queries/ecmascript.scm"),
                include_str!("../../queries/ecmascript-tokens.scm")
            )
        }
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
        LanguageId::Json => include_str!("../../queries/json.scm").to_string(),
        LanguageId::Bash => include_str!("../../queries/bash.scm").to_string(),
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
        LanguageId::Go => include_str!("../../queries/go.scm").to_string(),
        LanguageId::Java => format!(
            "{}\n{}\n{}",
            include_str!("../../queries/java.scm"),
            include_str!("../../queries/java-types.scm"),
            include_str!("../../queries/java-tokens.scm")
        ),
        LanguageId::CSharp => format!(
            "{}\n{}\n{}",
            include_str!("../../queries/csharp.scm"),
            include_str!("../../queries/csharp-types.scm"),
            include_str!("../../queries/csharp-tokens.scm")
        ),
        LanguageId::Ruby => format!(
            "{}\n{}",
            include_str!("../../queries/ruby-tokens.scm"),
            include_str!("../../queries/ruby.scm")
        ),
        LanguageId::Php => format!(
            "{}\n{}\n{}",
            include_str!("../../queries/php-tokens.scm"),
            include_str!("../../queries/php.scm"),
            include_str!("../../queries/php-imports.scm")
        ),
        LanguageId::Lua => format!(
            "{}\n{}",
            include_str!("../../queries/lua.scm"),
            include_str!("../../queries/lua-tokens.scm")
        ),
        LanguageId::Yaml => include_str!("../../queries/yaml.scm").to_string(),
        LanguageId::Html => include_str!("../../queries/html.scm").to_string(),
        LanguageId::Css => include_str!("../../queries/css.scm").to_string(),
        LanguageId::Markdown => include_str!("../../queries/markdown.scm").to_string(),
        LanguageId::MarkdownInline => include_str!("../../queries/markdown-inline.scm").to_string(),
        LanguageId::Ada => format!(
            "{}\n{}\n{}",
            include_str!("../../queries/ada-tokens.scm"),
            include_str!("../../queries/ada.scm"),
            include_str!("../../queries/ada-types.scm")
        ),
    }
}

fn query_additions(language: LanguageId) -> String {
    let javascript = include_str!("../../queries/javascript.scm");
    let typescript = include_str!("../../queries/typescript.scm");
    let jsx = include_str!("../../queries/jsx.scm");
    match language {
        LanguageId::Rust => String::new(),
        LanguageId::Python => String::new(),
        LanguageId::JavaScript => javascript.to_string(),
        LanguageId::Jsx => format!("{javascript}\n{jsx}"),
        LanguageId::TypeScript => typescript.to_string(),
        LanguageId::Tsx => format!("{typescript}\n{jsx}"),
        LanguageId::Bash => String::new(),
        LanguageId::Toml => include_str!("../../queries/toml.scm").to_string(),
        LanguageId::Json => String::new(),
        LanguageId::C => String::new(),
        LanguageId::Cpp => format!(
            "{}\n{}",
            include_str!("../../queries/cpp.scm"),
            include_str!("../../queries/cpp-symbols.scm")
        ),
        LanguageId::Go => include_str!("../../queries/go-regex.scm").to_string(),
        LanguageId::Java => include_str!("../../queries/java-symbols.scm").to_string(),
        LanguageId::CSharp => include_str!("../../queries/csharp-symbols.scm").to_string(),
        LanguageId::Ruby => String::new(),
        LanguageId::Php => String::new(),
        LanguageId::Lua => String::new(),
        LanguageId::Yaml => String::new(),
        LanguageId::Html => String::new(),
        LanguageId::Css => String::new(),
        LanguageId::Markdown => String::new(),
        LanguageId::MarkdownInline => String::new(),
        LanguageId::Ada => String::new(),
    }
}
