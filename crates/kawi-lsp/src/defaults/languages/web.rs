use super::{C_PAIRS_SINGLE, HTML_PAIRS, SCRIPT_PAIRS};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
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
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("jsx", "javascript")
            .with_extensions(["jsx"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("typescript", "typescript")
            .with_extensions(["ts", "mts", "cts"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("tsx", "typescriptreact")
            .with_extensions(["tsx"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("html", "html")
            .with_extensions(["html", "htm"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(HTML_PAIRS),
        LanguageConfig::new("css", "css")
            .with_extensions(["css"])
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
    ]
}
