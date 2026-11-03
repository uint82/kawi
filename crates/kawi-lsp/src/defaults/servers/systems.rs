use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("asm-lsp", "asm-lsp", &[]),
    ("clangd", "clangd", &[]),
    ("crystalline", "crystalline", &["--stdio"]),
    ("glsl_analyzer", "glsl_analyzer", &[]),
    ("gopls", "gopls", &[]),
    ("nimlangserver", "nimlangserver", &[]),
    ("rust-analyzer", "rust-analyzer", &[]),
    ("serve-d", "serve-d", &[]),
    ("solc", "solc", &["--lsp"]),
    ("sourcekit-lsp", "sourcekit-lsp", &[]),
    ("verible-verilog-ls", "verible-verilog-ls", &[]),
    ("vhdl_ls", "vhdl_ls", &[]),
    ("wgsl-analyzer", "wgsl-analyzer", &[]),
    ("zls", "zls", &[]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
