use super::{C_PAIRS, C_PAIRS_SINGLE};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("java", "java")
            .with_extensions(["java", "jav", "pde"])
            .with_root_markers([
                "pom.xml",
                "build.gradle",
                "build.gradle.kts",
                "settings.gradle",
                "settings.gradle.kts",
            ])
            .with_servers(["jdtls"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("kotlin", "kotlin")
            .with_extensions(["kt", "kts"])
            .with_root_markers(["settings.gradle", "settings.gradle.kts", "build.gradle.kts"])
            .with_servers(["kotlin-language-server"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("scala", "scala")
            .with_extensions(["scala", "sbt", "sc"])
            .with_root_markers([
                "build.sbt",
                "build.sc",
                "build.gradle",
                "build.gradle.kts",
                "pom.xml",
            ])
            .with_servers(["metals"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("clojure", "clojure")
            .with_extensions(["clj", "cljs", "cljc", "edn"])
            .with_root_markers(["project.clj", "deps.edn", "build.boot", "shadow-cljs.edn"])
            .with_servers(["clojure-lsp"])
            .with_line_comment(";")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("csharp", "csharp")
            .with_extensions(["cs", "csx", "cake"])
            .with_root_markers([
                "global.json",
                "Directory.Build.props",
                "Directory.Build.targets",
            ])
            .with_servers(["csharp-ls"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("fsharp", "fsharp")
            .with_extensions(["fs", "fsi", "fsx", "fsscript"])
            .with_root_markers([
                "global.json",
                "Directory.Build.props",
                "Directory.Build.targets",
            ])
            .with_servers(["fsautocomplete"])
            .with_line_comment("//")
            .with_block_comment("(*", "*)")
            .with_auto_pairs(C_PAIRS),
    ]
}
