use crate::{LanguageConfig, ServerConfig};
use std::collections::BTreeMap;

pub fn servers() -> BTreeMap<String, ServerConfig> {
    [
        (
            "rust-analyzer".to_string(),
            ServerConfig::command("rust-analyzer"),
        ),
        (
            "pyright".to_string(),
            ServerConfig::command("pyright-langserver").with_args(["--stdio"]),
        ),
        (
            "ruff".to_string(),
            ServerConfig::command("ruff").with_args(["server"]),
        ),
    ]
    .into_iter()
    .collect()
}

pub fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("rust", "rust")
            .with_grammar("rust")
            .with_extensions(["rs"])
            .with_root_markers(["Cargo.toml", "rust-project.json"])
            .with_servers(["rust-analyzer"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs([('(', ')'), ('[', ']'), ('{', '}'), ('"', '"')]),
        LanguageConfig::new("python", "python")
            .with_grammar("python")
            .with_extensions(["py", "pyi", "pyw"])
            .with_root_markers([
                "pyproject.toml",
                "pyrightconfig.json",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
            ])
            .with_servers(["pyright"])
            .with_line_comment("#")
            .with_auto_pairs([('(', ')'), ('[', ']'), ('{', '}'), ('\'', '\''), ('"', '"')]),
        LanguageConfig::new("toml", "toml")
            .with_extensions(["toml"])
            .with_filenames(["Cargo.toml", "Cargo.lock"])
            .with_line_comment("#")
            .with_auto_pairs([('[', ']'), ('{', '}'), ('\'', '\''), ('"', '"')]),
        LanguageConfig::new("json", "json")
            .with_extensions(["json", "jsonc"])
            .with_auto_pairs([('[', ']'), ('{', '}'), ('"', '"')]),
        LanguageConfig::new("javascript", "javascript")
            .with_extensions(["js", "mjs", "cjs"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('\'', '\''),
                ('"', '"'),
                ('`', '`'),
            ]),
        LanguageConfig::new("jsx", "javascript")
            .with_extensions(["jsx"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('\'', '\''),
                ('"', '"'),
                ('`', '`'),
            ]),
        LanguageConfig::new("typescript", "typescript")
            .with_extensions(["ts", "mts", "cts"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('\'', '\''),
                ('"', '"'),
                ('`', '`'),
            ]),
        LanguageConfig::new("tsx", "typescriptreact")
            .with_extensions(["tsx"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('\'', '\''),
                ('"', '"'),
                ('`', '`'),
            ]),
        LanguageConfig::new("bash", "shellscript")
            .with_extensions(["sh", "bash", "zsh"])
            .with_filenames([".bashrc", ".zshrc", ".profile"])
            .with_line_comment("#")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('\'', '\''),
                ('"', '"'),
                ('`', '`'),
            ]),
        LanguageConfig::new("html", "html")
            .with_extensions(["html", "htm"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('<', '>'),
                ('\'', '\''),
                ('"', '"'),
            ]),
        LanguageConfig::new("css", "css")
            .with_extensions(["css"])
            .with_block_comment("/*", "*/")
            .with_auto_pairs([('(', ')'), ('[', ']'), ('{', '}'), ('\'', '\''), ('"', '"')]),
        LanguageConfig::new("markdown", "markdown")
            .with_extensions(["md", "markdown"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs([
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('\'', '\''),
                ('"', '"'),
                ('`', '`'),
            ]),
    ]
}
