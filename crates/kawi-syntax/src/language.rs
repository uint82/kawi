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

    pub fn name(self) -> &'static str {
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
        let script = include_str!("../queries/script-common.scm");
        let javascript_additions = include_str!("../queries/javascript.scm");
        let typescript_additions = include_str!("../queries/typescript.scm");
        let jsx_additions = include_str!("../queries/jsx.scm");
        let additions = match self {
            Self::Rust => include_str!("../queries/rust.scm").to_string(),
            Self::Python => include_str!("../queries/python.scm").to_string(),
            Self::JavaScript => format!("{script}\n{javascript_additions}"),
            Self::Jsx => format!("{script}\n{javascript_additions}\n{jsx_additions}"),
            Self::TypeScript => format!("{script}\n{typescript_additions}"),
            Self::Tsx => format!("{script}\n{typescript_additions}\n{jsx_additions}"),
            Self::Bash => include_str!("../queries/bash.scm").to_string(),
            Self::Toml => include_str!("../queries/toml.scm").to_string(),
            Self::Json => include_str!("../queries/json.scm").to_string(),
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
mod tests;
