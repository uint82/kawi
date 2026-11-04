use super::{C_PAIRS, C_PAIRS_SINGLE};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("rust", "rust")
            .with_spaces(4)
            .with_grammar("rust")
            .with_extensions(["rs"])
            .with_root_markers(["Cargo.toml", "rust-project.json"])
            .with_servers(["rust-analyzer"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("c", "c")
            .with_spaces(4)
            .with_extensions(["c"])
            .with_root_markers(["compile_commands.json", "CMakeLists.txt", "Makefile"])
            .with_servers(["clangd"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("cpp", "cpp")
            .with_spaces(4)
            .with_extensions([
                "cc", "cpp", "cxx", "c++", "h", "hh", "hpp", "hxx", "ipp", "tpp", "ixx", "txx",
                "ino", "cu", "cuh", "cppm", "h++", "ii", "inl",
            ])
            .with_root_markers(["compile_commands.json", "CMakeLists.txt", "Makefile"])
            .with_servers(["clangd"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("objective-c", "objective-c")
            .with_spaces(4)
            .with_extensions(["m", "mm"])
            .with_root_markers(["compile_commands.json", "CMakeLists.txt"])
            .with_servers(["clangd"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("go", "go")
            .with_tabs(8)
            .with_extensions(["go"])
            .with_root_markers(["go.work", "go.mod"])
            .with_servers(["gopls"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("gomod", "gomod")
            .with_tabs(8)
            .with_filenames(["go.mod"])
            .with_root_markers(["go.work", "go.mod"])
            .with_servers(["gopls"])
            .with_line_comment("//")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("gowork", "gowork")
            .with_tabs(8)
            .with_filenames(["go.work"])
            .with_root_markers(["go.work"])
            .with_servers(["gopls"])
            .with_line_comment("//")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("zig", "zig")
            .with_spaces(4)
            .with_extensions(["zig", "zon"])
            .with_root_markers(["build.zig", "build.zig.zon"])
            .with_servers(["zls"])
            .with_line_comment("//")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("nim", "nim")
            .with_spaces(2)
            .with_extensions(["nim", "nims", "nimble"])
            .with_root_markers(["nim.cfg", "config.nims"])
            .with_servers(["nimlangserver"])
            .with_line_comment("#")
            .with_block_comment("#[", "]#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("d", "d")
            .with_spaces(4)
            .with_extensions(["d", "di"])
            .with_root_markers(["dub.json", "dub.sdl"])
            .with_servers(["serve-d"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("swift", "swift")
            .with_spaces(4)
            .with_extensions(["swift", "swiftinterface"])
            .with_root_markers(["Package.swift"])
            .with_servers(["sourcekit-lsp"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("crystal", "crystal")
            .with_spaces(2)
            .with_extensions(["cr"])
            .with_root_markers(["shard.yml", "shard.lock"])
            .with_servers(["crystalline"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("glsl", "glsl")
            .with_spaces(4)
            .with_extensions(["glsl", "vert", "tesc", "tese", "geom", "frag", "comp"])
            .with_servers(["glsl_analyzer"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("wgsl", "wgsl")
            .with_spaces(4)
            .with_extensions(["wgsl"])
            .with_servers(["wgsl-analyzer"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("asm", "asm")
            .with_tabs(8)
            .with_extensions(["asm", "s", "nasm"])
            .with_servers(["asm-lsp"])
            .with_line_comment(";")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("verilog", "verilog")
            .with_spaces(2)
            .with_extensions(["v", "vh"])
            .with_servers(["verible-verilog-ls"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("systemverilog", "systemverilog")
            .with_spaces(2)
            .with_extensions(["sv", "svh"])
            .with_servers(["verible-verilog-ls"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("vhdl", "vhdl")
            .with_spaces(2)
            .with_extensions(["vhd", "vhdl"])
            .with_servers(["vhdl_ls"])
            .with_line_comment("--")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("solidity", "solidity")
            .with_spaces(4)
            .with_extensions(["sol"])
            .with_root_markers([
                "foundry.toml",
                "hardhat.config.js",
                "hardhat.config.ts",
                "truffle-config.js",
            ])
            .with_servers(["solc"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
    ]
}
