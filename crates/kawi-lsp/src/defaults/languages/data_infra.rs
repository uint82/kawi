use super::{C_PAIRS, C_PAIRS_SINGLE};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("sql", "sql")
            .with_spaces(2)
            .with_extensions(["sql", "dsql"])
            .with_servers(["sqls"])
            .with_indent_after_keywords([
                "select",
                "from",
                "where",
                "by",
                "having",
                "limit",
                "offset",
                "on",
                "using",
                "set",
                "values",
                "returning",
                "begin",
                "case",
                "when",
                "then",
                "else",
                "loop",
                "do",
                "repeat",
                "exception",
            ])
            .with_indent_block_openers([
                "begin",
                "case",
                "if",
                "loop",
                "for",
                "while",
                "repeat",
                "exception",
            ])
            .with_indent_block_closers(["end"])
            .with_indent_align_keywords([
                "select",
                "from",
                "where",
                "group",
                "order",
                "having",
                "limit",
                "offset",
                "union",
                "except",
                "intersect",
                "set",
                "values",
                "returning",
                "with",
                "insert",
                "update",
                "delete",
                "merge",
                "join",
                "exception",
                "until",
            ])
            .with_indent_branch_keywords(["when", "else", "elseif", "elsif"])
            .with_indent_branch_openers(["case", "exception"])
            .with_indent_ignore_case(true)
            .with_indent_keywords_in_brackets(true)
            .with_line_comment("--")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("protobuf", "proto")
            .with_spaces(2)
            .with_extensions(["proto"])
            .with_root_markers(["buf.yaml", "buf.gen.yaml"])
            .with_servers(["buf"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("dockerfile", "dockerfile")
            .with_spaces(4)
            .with_filenames(["Dockerfile", "Containerfile", "dockerfile", "containerfile"])
            .with_root_markers(["Dockerfile", "Containerfile"])
            .with_servers(["docker-langserver"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("terraform", "terraform")
            .with_spaces(2)
            .with_extensions(["tf", "tfvars"])
            .with_root_markers(["terraform.tfstate", ".terraform"])
            .with_servers(["terraform-ls"])
            .with_line_comment("#")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("hcl", "hcl")
            .with_spaces(2)
            .with_extensions(["hcl", "nomad"])
            .with_servers(["terraform-ls"])
            .with_line_comment("#")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("nix", "nix")
            .with_spaces(2)
            .with_extensions(["nix"])
            .with_servers(["nixd"])
            .with_line_comment("#")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("cmake", "cmake")
            .with_spaces(2)
            .with_extensions(["cmake"])
            .with_filenames(["CMakeLists.txt"])
            .with_servers(["cmake-language-server"])
            .with_line_comment("#")
            .with_block_comment("#[[", "]]")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("just", "just")
            .with_spaces(4)
            .with_extensions(["just"])
            .with_filenames(["justfile", "Justfile", ".justfile", ".Justfile"])
            .with_servers(["just-lsp"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("systemd", "systemd")
            .with_spaces(4)
            .with_extensions([
                "service",
                "socket",
                "timer",
                "mount",
                "target",
                "path",
                "slice",
                "scope",
                "swap",
                "device",
                "automount",
                "nspawn",
                "network",
                "volume",
                "container",
                "build",
                "image",
                "kube",
            ])
            .with_servers(["systemd-lsp"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("pascal", "pascal")
            .with_spaces(2)
            .with_extensions(["pas", "lpr", "lfm"])
            .with_servers(["pasls"])
            .with_line_comment("//")
            .with_block_comment("{", "}")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("fortran", "fortran")
            .with_spaces(3)
            .with_extensions(["f", "for", "f90", "f95", "f03"])
            .with_root_markers(["fpm.toml"])
            .with_servers(["fortls"])
            .with_line_comment("!")
            .with_auto_pairs(C_PAIRS),
    ]
}
