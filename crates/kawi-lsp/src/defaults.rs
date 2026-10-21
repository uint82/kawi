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
            .with_servers(["rust-analyzer"]),
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
            .with_servers(["pyright"]),
    ]
}
