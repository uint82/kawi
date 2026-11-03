use super::{C_PAIRS_SINGLE, SCRIPT_PAIRS};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("python", "python")
            .with_grammar("python")
            .with_extensions(["py", "pyi", "pyw"])
            .with_indent_after_suffixes([":"])
            .with_root_markers([
                "pyproject.toml",
                "pyrightconfig.json",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
            ])
            .with_servers(["pyright"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("bash", "shellscript")
            .with_grammar("bash")
            .with_extensions(["sh", "bash", "zsh"])
            .with_indent_after_keywords(["then", "do", "{"])
            .with_filenames([".bashrc", ".zshrc", ".profile"])
            .with_line_comment("#")
            .with_auto_pairs(SCRIPT_PAIRS),
    ]
}
