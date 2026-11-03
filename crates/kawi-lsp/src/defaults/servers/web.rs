use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("astro-ls", "astro-ls", &["--stdio"]),
    (
        "graphql-language-service",
        "graphql-lsp",
        &["server", "-m", "stream"],
    ),
    ("lemminx", "lemminx", &[]),
    (
        "prisma-language-server",
        "prisma-language-server",
        &["--stdio"],
    ),
    ("svelteserver", "svelteserver", &["--stdio"]),
    ("taplo", "taplo", &["lsp", "stdio"]),
    (
        "typescript-language-server",
        "typescript-language-server",
        &["--stdio"],
    ),
    (
        "vscode-css-language-server",
        "vscode-css-language-server",
        &["--stdio"],
    ),
    (
        "vscode-html-language-server",
        "vscode-html-language-server",
        &["--stdio"],
    ),
    (
        "vscode-json-language-server",
        "vscode-json-language-server",
        &["--stdio"],
    ),
    ("vuels", "vue-language-server", &["--stdio"]),
    ("yaml-language-server", "yaml-language-server", &["--stdio"]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
