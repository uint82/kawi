use super::SCRIPT_PAIRS;
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![LanguageConfig::new("markdown", "markdown")
        .with_extensions(["md", "markdown"])
        .with_block_comment("<!--", "-->")
        .with_auto_pairs(SCRIPT_PAIRS)]
}
