use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

mod grammar;

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
    C,
    Cpp,
    Go,
    Java,
    CSharp,
    Ruby,
    Php,
    Lua,
    Yaml,
    Html,
    Css,
    Markdown,
    Ada,
    MarkdownInline,
}

static USER_QUERY_DIR: OnceLock<PathBuf> = OnceLock::new();
static HIGHLIGHTS: OnceLock<[String; 23]> = OnceLock::new();

pub fn set_user_query_dir(path: PathBuf) {
    let _ = USER_QUERY_DIR.set(path);
}

impl LanguageId {
    pub const ALL: [Self; 23] = [
        Self::Rust,
        Self::Toml,
        Self::TypeScript,
        Self::Tsx,
        Self::JavaScript,
        Self::Jsx,
        Self::Python,
        Self::Json,
        Self::Bash,
        Self::C,
        Self::Cpp,
        Self::Go,
        Self::Java,
        Self::CSharp,
        Self::Ruby,
        Self::Php,
        Self::Lua,
        Self::Yaml,
        Self::Html,
        Self::Css,
        Self::Markdown,
        Self::Ada,
        Self::MarkdownInline,
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
            "c" => Some(Self::C),
            "cpp" | "c++" | "cxx" | "cc" | "h" | "hpp" | "hh" | "hxx" => Some(Self::Cpp),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "cs" | "csharp" | "c-sharp" => Some(Self::CSharp),
            "rb" | "ruby" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            "lua" => Some(Self::Lua),
            "yaml" | "yml" => Some(Self::Yaml),
            "html" | "htm" => Some(Self::Html),
            "css" => Some(Self::Css),
            "md" | "markdown" => Some(Self::Markdown),
            "ada" | "adb" | "ads" => Some(Self::Ada),
            "markdown_inline" => Some(Self::MarkdownInline),
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
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Go => "go",
            Self::Java => "java",
            Self::CSharp => "csharp",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Lua => "lua",
            Self::Yaml => "yaml",
            Self::Html => "html",
            Self::Css => "css",
            Self::Markdown => "markdown",
            Self::Ada => "ada",
            Self::MarkdownInline => "markdown_inline",
        }
    }

    pub(crate) fn language(self) -> tree_sitter::Language {
        grammar::language(self)
    }

    pub(crate) fn bundled_highlights(self) -> String {
        grammar::bundled_highlights(self)
    }

    pub(crate) fn injections(self) -> Cow<'static, str> {
        grammar::injections(self)
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
            Self::C => 9,
            Self::Cpp => 10,
            Self::Go => 11,
            Self::Java => 12,
            Self::CSharp => 13,
            Self::Ruby => 14,
            Self::Php => 15,
            Self::Lua => 16,
            Self::Yaml => 17,
            Self::Html => 18,
            Self::Css => 19,
            Self::Markdown => 20,
            Self::Ada => 21,
            Self::MarkdownInline => 22,
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
