use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MAX_QUERY_INHERITANCE: usize = 4;

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

static USER_QUERY_DIR: OnceLock<PathBuf> = OnceLock::new();
static HIGHLIGHTS: OnceLock<[String; 9]> = OnceLock::new();

pub fn set_user_query_dir(path: PathBuf) {
    let _ = USER_QUERY_DIR.set(path);
}

impl LanguageId {
    pub const ALL: [Self; 9] = [
        Self::Rust,
        Self::Toml,
        Self::TypeScript,
        Self::Tsx,
        Self::JavaScript,
        Self::Jsx,
        Self::Python,
        Self::Json,
        Self::Bash,
    ];

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

    pub(crate) fn bundled_highlights(self) -> String {
        let javascript = tree_sitter_javascript::HIGHLIGHT_QUERY;
        let jsx = tree_sitter_javascript::JSX_HIGHLIGHT_QUERY;
        let typescript = tree_sitter_typescript::HIGHLIGHTS_QUERY;
        let base = match self {
            Self::TypeScript => format!("{javascript}\n{typescript}"),
            Self::Tsx => format!("{javascript}\n{jsx}\n{typescript}"),
            Self::JavaScript => javascript.to_string(),
            Self::Jsx => format!("{javascript}\n{jsx}"),
            Self::Rust => tree_sitter_rust::HIGHLIGHTS_QUERY.to_string(),
            Self::Toml => tree_sitter_toml_ng::HIGHLIGHTS_QUERY.to_string(),
            Self::Python => tree_sitter_python::HIGHLIGHTS_QUERY.to_string(),
            Self::Json => tree_sitter_json::HIGHLIGHTS_QUERY.to_string(),
            Self::Bash => tree_sitter_bash::HIGHLIGHT_QUERY.to_string(),
        };
        let additions = match self {
            Self::Rust => include_str!("../queries/rust.scm"),
            Self::Python => include_str!("../queries/python.scm"),
            Self::JavaScript | Self::Jsx => include_str!("../queries/javascript.scm"),
            _ => "",
        };
        format!("{base}\n{additions}")
    }

    pub(crate) fn highlights(self) -> &'static str {
        let highlights = HIGHLIGHTS.get_or_init(|| {
            LanguageId::ALL.map(|language| {
                language.highlights_from(
                    USER_QUERY_DIR.get().map(PathBuf::as_path),
                    MAX_QUERY_INHERITANCE,
                )
            })
        });
        &highlights[self.index()]
    }

    fn index(self) -> usize {
        match self {
            Self::Rust => 0,
            Self::Toml => 1,
            Self::TypeScript => 2,
            Self::Tsx => 3,
            Self::JavaScript => 4,
            Self::Jsx => 5,
            Self::Python => 6,
            Self::Json => 7,
            Self::Bash => 8,
        }
    }

    fn highlights_from(self, dir: Option<&Path>, depth: usize) -> String {
        let base = self.bundled_highlights();
        let Some(dir) = dir else {
            return base;
        };
        let path = dir.join(format!("{}.scm", self.name()));
        let Ok(text) = fs::read_to_string(path) else {
            return base;
        };
        merge_user_query(self, &base, &text, Some(dir), depth)
    }
}

fn merge_user_query(
    language: LanguageId,
    base: &str,
    text: &str,
    dir: Option<&Path>,
    depth: usize,
) -> String {
    let directives = query_directives(text);
    if directives.inherits.is_empty() {
        if directives.extends {
            return format!("{base}\n{text}");
        }
        return text.to_string();
    }
    if depth == 0 {
        return base.to_string();
    }
    let inherited = directives
        .inherits
        .iter()
        .filter_map(|name| LanguageId::from_alias(name))
        .filter(|inherited| *inherited != language)
        .map(|inherited| inherited.highlights_from(dir, depth.saturating_sub(1)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{inherited}\n{text}")
}

fn query_directives(text: &str) -> QueryDirectives {
    let mut directives = QueryDirectives::default();
    for line in text.lines() {
        let Some(comment) = line.trim_start().strip_prefix(';') else {
            continue;
        };
        let comment = comment.trim();
        if comment == "extends" {
            directives.extends = true;
        } else if let Some(names) = comment.strip_prefix("inherits:") {
            directives.inherits = names
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
        }
    }
    directives
}

#[derive(Default)]
struct QueryDirectives {
    extends: bool,
    inherits: Vec<String>,
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
        let query = tree_sitter::Query::new(&LanguageId::Python.language(), &highlights)
            .expect("user query");
        assert!(query.capture_names().contains(&"string"));
        assert!(query.capture_names().contains(&"variable.member"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn user_queries_replace_bundled_queries() {
        let dir = temp_query_dir("replace");
        fs::write(dir.join("python.scm"), "(identifier) @variable.member\n").unwrap();
        let highlights = LanguageId::Python.highlights_from(Some(&dir), MAX_QUERY_INHERITANCE);
        let query = tree_sitter::Query::new(&LanguageId::Python.language(), &highlights)
            .expect("user query");
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
}
