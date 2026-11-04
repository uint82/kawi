use super::{C_PAIRS, C_PAIRS_SINGLE, SCRIPT_PAIRS};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("markdown", "markdown")
            .with_spaces(4)
            .with_extensions(["md", "markdown", "mdx"])
            .with_root_markers([".marksman.toml"])
            .with_servers(["marksman"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("latex", "latex")
            .with_spaces(2)
            .with_extensions(["tex", "sty", "cls", "dtx", "ins", "bbx", "cbx"])
            .with_servers(["texlab"])
            .with_line_comment("%")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("bibtex", "bibtex")
            .with_spaces(2)
            .with_extensions(["bib"])
            .with_servers(["texlab"])
            .with_line_comment("%")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("typst", "typst")
            .with_spaces(2)
            .with_extensions(["typ", "typst"])
            .with_servers(["tinymist"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
    ]
}
