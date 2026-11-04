use super::{C_PAIRS_SINGLE, SCRIPT_PAIRS};
use crate::LanguageConfig;

pub(super) fn languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig::new("python", "python")
            .with_spaces(4)
            .with_grammar("python")
            .with_extensions(["py", "pyi", "pyw"])
            .with_indent_after_suffixes([":"])
            .with_root_markers([
                "pyproject.toml",
                "pyrightconfig.json",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
                "poetry.lock",
            ])
            .with_servers(["pyright", "ruff"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("ruby", "ruby")
            .with_spaces(2)
            .with_extensions(["rb", "rake", "gemspec", "rbi", "rbs"])
            .with_filenames([
                "Gemfile",
                "Rakefile",
                "Guardfile",
                "Capfile",
                "Vagrantfile",
                "Podfile",
                "Brewfile",
            ])
            .with_root_markers(["Gemfile", "Rakefile", ".ruby-version"])
            .with_servers(["ruby-lsp"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("php", "php")
            .with_spaces(4)
            .with_extensions(["php", "phtml", "php4", "php5", "ctp"])
            .with_root_markers(["composer.json", "index.php"])
            .with_servers(["intelephense"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("perl", "perl")
            .with_spaces(4)
            .with_extensions(["pl", "pm", "psgi", "t"])
            .with_servers(["perlnavigator"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("lua", "lua")
            .with_spaces(2)
            .with_extensions(["lua", "rockspec"])
            .with_root_markers([".luarc.json", ".luacheckrc", ".stylua.toml", "selene.toml"])
            .with_servers(["lua-language-server"])
            .with_block_comment("--[[", "--]]")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("bash", "shellscript")
            .with_spaces(2)
            .with_grammar("bash")
            .with_extensions(["sh", "bash", "zsh", "ash", "dash", "ksh", "mksh"])
            .with_indent_after_keywords(["then", "do", "{"])
            .with_filenames([
                ".bashrc",
                ".zshrc",
                ".profile",
                ".bash_profile",
                ".bash_login",
                ".bash_logout",
                ".zshenv",
                ".zprofile",
                ".zlogin",
                ".zlogout",
            ])
            .with_servers(["bash-language-server"])
            .with_line_comment("#")
            .with_auto_pairs(SCRIPT_PAIRS),
        LanguageConfig::new("fish", "fish")
            .with_spaces(4)
            .with_extensions(["fish"])
            .with_servers(["fish-lsp"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("elixir", "elixir")
            .with_spaces(2)
            .with_extensions(["ex", "exs"])
            .with_root_markers(["mix.exs", "mix.lock"])
            .with_servers(["elixir-ls"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("erlang", "erlang")
            .with_spaces(4)
            .with_extensions(["erl", "hrl", "app"])
            .with_root_markers(["rebar.config", "rebar.lock"])
            .with_servers(["erlang-ls"])
            .with_line_comment("%%")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("julia", "julia")
            .with_spaces(4)
            .with_extensions(["jl"])
            .with_root_markers(["Manifest.toml", "Project.toml"])
            .with_servers(["julia"])
            .with_line_comment("#")
            .with_block_comment("#=", "=#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("r", "r")
            .with_spaces(2)
            .with_extensions(["r"])
            .with_root_markers(["DESCRIPTION", "renv.lock"])
            .with_servers(["r"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
        LanguageConfig::new("awk", "awk")
            .with_spaces(4)
            .with_extensions(["awk", "gawk", "nawk", "mawk"])
            .with_servers(["awk-language-server"])
            .with_line_comment("#")
            .with_auto_pairs(C_PAIRS_SINGLE),
    ]
}
