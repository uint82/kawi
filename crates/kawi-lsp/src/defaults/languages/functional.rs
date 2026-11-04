use super::{C_PAIRS, C_PAIRS_SINGLE};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("haskell", "haskell")
            .with_spaces(2)
            .with_extensions(["hs", "hsc", "hs-boot"])
            .with_root_markers(["Setup.hs", "stack.yaml", "cabal.project", "hie.yaml"])
            .with_servers(["haskell-language-server"])
            .with_line_comment("--")
            .with_block_comment("{-", "-}")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("ocaml", "ocaml")
            .with_spaces(2)
            .with_extensions(["ml", "mli"])
            .with_root_markers(["dune-project", "dune"])
            .with_servers(["ocamllsp"])
            .with_block_comment("(*", "*)")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("elm", "elm")
            .with_spaces(4)
            .with_extensions(["elm"])
            .with_root_markers(["elm.json"])
            .with_servers(["elm-language-server"])
            .with_line_comment("--")
            .with_block_comment("{-", "-}")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("purescript", "purescript")
            .with_spaces(2)
            .with_extensions(["purs"])
            .with_root_markers(["spago.yaml", "spago.dhall", "bower.json"])
            .with_servers(["purescript-language-server"])
            .with_line_comment("--")
            .with_block_comment("{-", "-}")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("racket", "racket")
            .with_spaces(2)
            .with_extensions(["rkt", "rktd", "rktl", "scrbl"])
            .with_servers(["racket"])
            .with_line_comment(";")
            .with_block_comment("#|", "|#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("commonlisp", "commonlisp")
            .with_spaces(2)
            .with_extensions(["lisp", "asd", "cl", "l", "lsp", "ny", "sexp"])
            .with_servers(["cl-lsp"])
            .with_line_comment(";")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("gleam", "gleam")
            .with_spaces(2)
            .with_extensions(["gleam"])
            .with_root_markers(["gleam.toml"])
            .with_servers(["gleam"])
            .with_line_comment("//")
            .with_auto_pairs(C_PAIRS),
    ]
}
