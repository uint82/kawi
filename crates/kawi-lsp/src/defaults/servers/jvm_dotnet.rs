use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("clojure-lsp", "clojure-lsp", &[]),
    ("csharp-ls", "csharp-ls", &[]),
    ("fsautocomplete", "fsautocomplete", &[]),
    ("jdtls", "jdtls", &[]),
    ("kotlin-language-server", "kotlin-language-server", &[]),
    ("metals", "metals", &[]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
