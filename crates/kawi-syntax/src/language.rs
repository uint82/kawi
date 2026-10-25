use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    Toml,
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Python,
    Json,
    Bash,
}

impl LanguageId {
    pub fn from_path(path: &str) -> Option<Self> {
        let path = Path::new(path);
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.lock") {
            return Some(Self::Toml);
        }
        Self::from_alias(path.extension()?.to_str()?)
    }

    pub fn from_alias(alias: &str) -> Option<Self> {
        if alias.len() > 32 {
            return None;
        }
        match alias.to_ascii_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "toml" => Some(Self::Toml),
            "typescript" | "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "javascript" | "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "python" | "py" | "pyw" => Some(Self::Python),
            "json" => Some(Self::Json),
            "bash" | "sh" | "shell" | "zsh" => Some(Self::Bash),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Toml => "toml",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::JavaScript => "javascript",
            Self::Jsx => "jsx",
            Self::Python => "python",
            Self::Json => "json",
            Self::Bash => "bash",
        }
    }

    pub(crate) fn language(self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
        }
    }

    pub(crate) fn highlights(self) -> String {
        let javascript = tree_sitter_javascript::HIGHLIGHT_QUERY;
        let jsx = tree_sitter_javascript::JSX_HIGHLIGHT_QUERY;
        let typescript = tree_sitter_typescript::HIGHLIGHTS_QUERY;
        match self {
            Self::TypeScript => format!("{javascript}\n{typescript}"),
            Self::Tsx => format!("{javascript}\n{jsx}\n{typescript}"),
            Self::JavaScript => javascript.to_string(),
            Self::Jsx => format!("{javascript}\n{jsx}"),
            Self::Rust => tree_sitter_rust::HIGHLIGHTS_QUERY.to_string(),
            Self::Toml => tree_sitter_toml_ng::HIGHLIGHTS_QUERY.to_string(),
            Self::Python => tree_sitter_python::HIGHLIGHTS_QUERY.to_string(),
            Self::Json => tree_sitter_json::HIGHLIGHTS_QUERY.to_string(),
            Self::Bash => tree_sitter_bash::HIGHLIGHT_QUERY.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(LanguageId::from_alias("brainfuck"), None);
        assert_eq!(LanguageId::from_alias(&"x".repeat(33)), None);
    }

    #[test]
    fn builds_queries_for_every_language() {
        for language in [
            LanguageId::Rust,
            LanguageId::Toml,
            LanguageId::TypeScript,
            LanguageId::Tsx,
            LanguageId::JavaScript,
            LanguageId::Jsx,
            LanguageId::Python,
            LanguageId::Json,
            LanguageId::Bash,
        ] {
            assert!(!language.highlights().is_empty());
        }
    }
}
