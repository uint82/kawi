use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("cl-lsp", "cl-lsp", &[]),
    ("elm-language-server", "elm-language-server", &[]),
    ("gleam", "gleam", &["lsp"]),
    (
        "haskell-language-server",
        "haskell-language-server-wrapper",
        &["--lsp"],
    ),
    ("ocamllsp", "ocamllsp", &[]),
    (
        "purescript-language-server",
        "purescript-language-server",
        &["--stdio"],
    ),
    ("racket", "racket", &["-l", "racket-langserver"]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
