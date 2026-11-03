use super::C_PAIRS;
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![LanguageConfig::new("rust", "rust")
        .with_grammar("rust")
        .with_extensions(["rs"])
        .with_root_markers(["Cargo.toml", "rust-project.json"])
        .with_servers(["rust-analyzer"])
        .with_line_comment("//")
        .with_block_comment("/*", "*/")
        .with_auto_pairs(C_PAIRS)]
}
