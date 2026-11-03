use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("buf", "buf", &["lsp", "serve", "--timeout", "0"]),
    ("cmake-language-server", "cmake-language-server", &[]),
    ("docker-langserver", "docker-langserver", &["--stdio"]),
    ("fortls", "fortls", &[]),
    ("just-lsp", "just-lsp", &[]),
    ("nixd", "nixd", &[]),
    ("pasls", "pasls", &[]),
    ("sqls", "sqls", &[]),
    ("systemd-lsp", "systemd-lsp", &[]),
    ("terraform-ls", "terraform-ls", &["serve"]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
