use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("awk-language-server", "awk-language-server", &[]),
    ("bash-language-server", "bash-language-server", &["start"]),
    ("elixir-ls", "elixir-ls", &[]),
    ("erlang-ls", "erlang_ls", &[]),
    ("fish-lsp", "fish-lsp", &["start"]),
    ("intelephense", "intelephense", &["--stdio"]),
    (
        "julia",
        "julia",
        &[
            "--startup-file=no",
            "--history-file=no",
            "--quiet",
            "-e",
            "using LanguageServer; runserver()",
        ],
    ),
    ("lua-language-server", "lua-language-server", &[]),
    ("perlnavigator", "perlnavigator", &["--stdio"]),
    ("pyright", "pyright-langserver", &["--stdio"]),
    (
        "r",
        "R",
        &["--vanilla", "--no-echo", "-e", "languageserver::run()"],
    ),
    ("ruby-lsp", "ruby-lsp", &[]),
    ("ruff", "ruff", &["server"]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
