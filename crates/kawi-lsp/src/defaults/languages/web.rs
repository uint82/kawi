use super::{C_PAIRS, C_PAIRS_SINGLE, HTML_PAIRS, SCRIPT_PAIRS};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("toml", "toml")
            .with_spaces(2)
            .with_extensions(["toml"])
            .with_filenames(["Cargo.toml", "Cargo.lock"])
            .with_servers(["taplo"])
            .with_line_comment("#")
            .with_auto_pairs([('[', ']'), ('{', '}'), ('\'', '\''), ('"', '"')]),
        LanguageConfig::new("jsonc", "jsonc")
            .with_spaces(2)
            .with_extensions(["jsonc"])
            .with_filenames(["tsconfig.json", "jsconfig.json", ".devcontainer.json"])
            .with_servers(["vscode-json-language-server"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS),
        LanguageConfig::new("json", "json")
            .with_spaces(2)
            .with_extensions(["json"])
            .with_servers(["vscode-json-language-server"])
            .with_auto_pairs([('[', ']'), ('{', '}'), ('"', '"')]),
        LanguageConfig::new("javascript", "javascript")
            .with_spaces(2)
            .with_extensions(["js", "mjs", "cjs"])
            .with_root_markers(["package.json", "jsconfig.json"])
            .with_servers(["typescript-language-server"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("jsx", "javascript")
            .with_spaces(2)
            .with_extensions(["jsx"])
            .with_root_markers(["package.json", "jsconfig.json"])
            .with_servers(["typescript-language-server"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("typescript", "typescript")
            .with_spaces(2)
            .with_extensions(["ts", "mts", "cts"])
            .with_root_markers(["package.json", "tsconfig.json"])
            .with_servers(["typescript-language-server"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("tsx", "typescriptreact")
            .with_spaces(2)
            .with_extensions(["tsx"])
            .with_root_markers(["package.json", "tsconfig.json"])
            .with_servers(["typescript-language-server"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("html", "html")
            .with_spaces(2)
            .with_extensions(["html", "htm", "xhtml", "xht"])
            .with_servers(["vscode-html-language-server"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(HTML_PAIRS),
        LanguageConfig::new("css", "css")
            .with_spaces(2)
            .with_extensions(["css"])
            .with_servers(["vscode-css-language-server"])
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("scss", "scss")
            .with_spaces(2)
            .with_extensions(["scss"])
            .with_servers(["vscode-css-language-server"])
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("less", "less")
            .with_spaces(2)
            .with_extensions(["less"])
            .with_servers(["vscode-css-language-server"])
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("yaml", "yaml")
            .with_spaces(2)
            .with_extensions(["yaml", "yml"])
            .with_servers(["yaml-language-server"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("xml", "xml")
            .with_spaces(2)
            .with_extensions([
                "xml", "xsd", "xsl", "xslt", "svg", "plist", "csproj", "fsproj", "vcxproj", "xaml",
                "props", "targets",
            ])
            .with_servers(["lemminx"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(HTML_PAIRS),
        LanguageConfig::new("graphql", "graphql")
            .with_spaces(2)
            .with_extensions(["graphql", "gql", "graphqls"])
            .with_root_markers([".graphqlrc", "graphql.config.js", "package.json"])
            .with_servers(["graphql-language-service"])
            .with_line_comment("#")
            .with_block_comment("\"\"\"", "\"\"\"")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("vue", "vue")
            .with_spaces(2)
            .with_extensions(["vue"])
            .with_root_markers(["package.json"])
            .with_servers(["vuels"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(HTML_PAIRS),
        LanguageConfig::new("svelte", "svelte")
            .with_spaces(2)
            .with_extensions(["svelte"])
            .with_root_markers(["package.json", "svelte.config.js"])
            .with_servers(["svelteserver"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(HTML_PAIRS),
        LanguageConfig::new("astro", "astro")
            .with_spaces(2)
            .with_extensions(["astro"])
            .with_root_markers(["package.json", "astro.config.mjs", "astro.config.ts"])
            .with_servers(["astro-ls"])
            .with_block_comment("<!--", "-->")
            .with_auto_pairs(HTML_PAIRS),
        LanguageConfig::new("prisma", "prisma")
            .with_spaces(2)
            .with_extensions(["prisma"])
            .with_root_markers(["package.json"])
            .with_servers(["prisma-language-server"])
            .with_line_comment("//")
            .with_auto_pairs(C_PAIRS),
    ]
}
