use super::*;
use std::fs;

fn temp_file(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_lsp_config_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("languages.toml");
    fs::write(&path, text).expect("write");
    path
}

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn server() -> BTreeMap<String, ServerConfig> {
    [(
        "rust-analyzer".to_string(),
        ServerConfig::command("rust-analyzer"),
    )]
    .into_iter()
    .collect()
}

fn rust() -> LanguageConfig {
    LanguageConfig::new("rust", "rust")
        .with_extensions(["rs"])
        .with_servers(["rust-analyzer"])
}

#[test]
fn built_in_defaults_cover_rust_and_python() {
    let languages = Languages::load(Path::new("/nonexistent/languages.toml"));
    assert_eq!(
        languages.language("rust").expect("rust").language_id,
        "rust"
    );
    assert_eq!(
        languages.language("python").expect("python").servers,
        vec!["pyright", "ruff"]
    );
}

#[test]
fn user_languages_preserve_unspecified_defaults() {
    let path = temp_file(
        "override",
        "[[language]]\nname = \"python\"\nservers = [\"pyright\", \"ruff\"]\n",
    );
    let languages = Languages::load(&path);
    let python = languages.language("python").expect("python");
    assert_eq!(python.extensions, vec!["py", "pyi", "pyw"]);
    assert_eq!(python.servers, vec!["pyright", "ruff"]);
}

#[test]
fn user_languages_replace_explicit_fields() {
    let path = temp_file(
        "explicit",
        "[[language]]\nname = \"rust\"\nextensions = [\".rs\"]\nservers = []\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.extensions, vec!["rs"]);
    assert!(rust.servers.is_empty());
}

#[test]
fn user_languages_can_be_added() {
    let path = temp_file(
        "new_language",
        "[[language]]\nname = \"lua\"\nextensions = [\"lua\"]\nservers = []\n",
    );
    let languages = Languages::load(&path);
    let lua = languages.language("lua").expect("lua");
    assert_eq!(lua.language_id, "lua");
    assert_eq!(lua.extensions, vec!["lua"]);
}

#[test]
fn user_languages_set_editor_overrides() {
    let path = temp_file(
        "editor_overrides",
        "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.indent_override.shiftwidth, Some(2));
    assert_eq!(rust.indent_override.expandtab, Some(true));
    assert_eq!(rust.indent, IndentConfig::spaces(4));
    assert!(languages.warnings().is_empty());
}

#[test]
fn user_languages_set_tabstop_override() {
    let path = temp_file(
        "tabstop_override",
        "[[language]]\nname = \"rust\"\ntabstop = 2\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.indent_override.tabstop, Some(2));
    assert_eq!(rust.indent_override.shiftwidth, None);
    assert_eq!(rust.indent, IndentConfig::spaces(4));
    assert!(languages.warnings().is_empty());
}

#[test]
fn zero_tabstop_warns_and_is_ignored() {
    let path = temp_file(
        "zero_tabstop",
        "[[language]]\nname = \"rust\"\ntabstop = 0\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.indent_override.tabstop, None);
    assert_eq!(languages.warnings().len(), 1);
    assert!(languages.warnings()[0]
        .message()
        .contains("tabstop must be an integer from 1"));
}

#[test]
fn user_languages_set_formatter() {
    let path = temp_file(
        "formatter",
        "[[language]]\nname = \"rust\"\nformatter = { command = \"rustfmt\", args = [\"--edition\", \"2021\"], env = { RUSTFMT = \"1\" }, stdin = false }\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    let formatter = rust.formatter.as_ref().expect("formatter");
    assert_eq!(formatter.command, "rustfmt");
    assert_eq!(formatter.args, vec!["--edition", "2021"]);
    assert_eq!(formatter.env.get("RUSTFMT").map(String::as_str), Some("1"));
    assert!(!formatter.stdin);
    assert_eq!(
        languages
            .formatter_for_path(Path::new("main.rs"))
            .expect("formatter")
            .command,
        "rustfmt"
    );
    assert_eq!(languages.formatters().count(), 1);
    assert!(languages.warnings().is_empty());
}

#[test]
fn user_languages_disable_formatter() {
    let path = temp_file(
        "formatter_disabled",
        "[[language]]\nname = \"rust\"\nformatter = false\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert!(rust.formatter.is_none());
    assert!(languages.warnings().is_empty());
}

#[test]
fn disabled_languages_hide_formatters() {
    let path = temp_file(
        "formatter_disabled_language",
        "[[language]]\nname = \"rust\"\nenabled = false\nformatter = { command = \"rustfmt\" }\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert!(rust.formatter.is_some());
    assert_eq!(languages.formatters().count(), 0);
    assert!(languages.formatter_for_path(Path::new("main.rs")).is_none());
}

#[test]
fn invalid_formatter_warns_and_is_ignored() {
    let path = temp_file(
        "formatter_invalid",
        "[[language]]\nname = \"rust\"\nformatter = { args = [\"--check\"] }\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert!(rust.formatter.is_none());
    assert_eq!(languages.warnings().len(), 1);
    assert!(languages.warnings()[0]
        .message()
        .contains("formatter command must be a non-empty string"));
}

#[test]
fn unknown_formatter_fields_warn_and_are_ignored() {
    let path = temp_file(
        "formatter_unknown",
        "[[language]]\nname = \"rust\"\nformatter = { commnad = \"rustfmt\" }\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert!(rust.formatter.is_none());
    assert_eq!(languages.warnings().len(), 1);
    assert!(languages.warnings()[0].message().contains("commnad"));
}

#[test]
fn built_in_languages_set_indent_rules() {
    let languages = Languages::load(Path::new("/nonexistent/languages.toml"));
    let python = languages.language("python").expect("python");
    assert_eq!(python.indent_after_suffixes, vec![":"]);
    assert!(python.indent_after_keywords.is_empty());
    assert!(python.indent_brackets);
    let bash = languages.language("bash").expect("bash");
    assert_eq!(bash.indent_after_keywords, vec!["then", "do", "{"]);
    assert!(bash.indent_after_suffixes.is_empty());
}

#[test]
fn user_languages_set_indent_overrides() {
    let path = temp_file(
        "indent_overrides",
        "[[language]]\nname = \"python\"\nindent_after_suffixes = [\";\"]\nindent_after_keywords = [\"begin\"]\nindent_brackets = false\n",
    );
    let languages = Languages::load(&path);
    let python = languages.language("python").expect("python");
    assert_eq!(python.indent_after_suffixes, vec![";"]);
    assert_eq!(python.indent_after_keywords, vec!["begin"]);
    assert!(!python.indent_brackets);
    assert!(languages.warnings().is_empty());
}

#[test]
fn user_languages_can_clear_indent_rules() {
    let path = temp_file(
        "clear_indent_rules",
        "[[language]]\nname = \"python\"\nindent_after_suffixes = []\nindent_after_keywords = []\n",
    );
    let languages = Languages::load(&path);
    let python = languages.language("python").expect("python");
    assert!(python.indent_after_suffixes.is_empty());
    assert!(python.indent_after_keywords.is_empty());
}

#[test]
fn invalid_indent_overrides_warn_and_are_ignored() {
    let path = temp_file(
        "invalid_indent_overrides",
        "[[language]]\nname = \"python\"\nindent_after_suffixes = [\":\", 1]\nindent_brackets = \"yes\"\n",
    );
    let languages = Languages::load(&path);
    let python = languages.language("python").expect("python");
    assert_eq!(python.indent_after_suffixes, vec![":"]);
    assert!(python.indent_brackets);
    assert_eq!(languages.warnings().len(), 2);
    assert!(languages.warnings()[0]
        .message()
        .contains("indent_after_suffixes must be an array"));
    assert!(languages.warnings()[1]
        .message()
        .contains("indent_brackets must be a boolean"));
}

#[test]
fn new_languages_set_editor_overrides() {
    let path = temp_file(
        "new_editor_overrides",
        "[[language]]\nname = \"lua\"\nextensions = [\"lua\"]\nshiftwidth = 3\nexpandtab = false\n",
    );
    let languages = Languages::load(&path);
    let lua = languages.language("lua").expect("lua");
    assert_eq!(lua.indent_override.shiftwidth, Some(3));
    assert_eq!(lua.indent_override.expandtab, Some(false));
}

#[test]
fn invalid_editor_overrides_warn_and_are_ignored() {
    let path = temp_file(
        "invalid_editor_overrides",
        "[[language]]\nname = \"rust\"\nshiftwidth = -1\nexpandtab = \"yes\"\ntabstop = \"wide\"\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.indent_override, IndentConfig::default());
    assert_eq!(rust.indent, IndentConfig::spaces(4));
    assert_eq!(languages.warnings().len(), 3);
    assert!(languages.warnings()[0]
        .message()
        .contains("shiftwidth must be an integer"));
    assert!(languages.warnings()[1]
        .message()
        .contains("expandtab must be a boolean"));
    assert!(languages.warnings()[2]
        .message()
        .contains("tabstop must be an integer"));
}

#[test]
fn user_servers_preserve_unspecified_defaults() {
    let path = temp_file(
        "server_override",
        "[server.pyright]\nargs = [\"--stdio\", \"--verbose\"]\n",
    );
    let languages = Languages::load(&path);
    assert_eq!(
        languages.servers.get("pyright").expect("pyright").command,
        "pyright-langserver"
    );
    assert_eq!(
        languages.servers.get("pyright").expect("pyright").args,
        vec!["--stdio", "--verbose"]
    );
}

#[test]
fn tracks_user_configured_servers() {
    let path = temp_file(
        "user_server_tracking",
        "[server.rust-analyzer]\nargs = [\"--log-file\", \"/tmp/ra.log\"]\n",
    );
    let languages = Languages::load(&path);
    assert!(languages.server_is_user_configured("rust-analyzer"));
    assert!(!languages.server_is_user_configured("pyright"));
}

#[test]
fn missing_server_references_warn() {
    let path = temp_file(
        "missing_server",
        "[[language]]\nname = \"rust\"\nservers = [\"missing\"]\n",
    );
    let languages = Languages::load(&path);
    assert!(languages
        .warnings()
        .iter()
        .any(|warning| warning.message().contains("server missing")));
}

#[test]
fn invalid_file_falls_back_to_defaults() {
    let path = temp_file("invalid", "history = \n");
    let languages = Languages::load(&path);
    assert_eq!(languages.warnings().len(), 1);
    assert!(languages.warnings()[0].line().is_some());
    assert!(languages.language("rust").is_some());
}

#[test]
fn built_in_languages_define_editing_tokens() {
    let languages = Languages::load(Path::new("/nonexistent/languages.toml"));
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.line_comment.as_deref(), Some("//"));
    assert_eq!(
        rust.block_comment.as_ref().map(|block| block.open.as_str()),
        Some("/*")
    );
    assert!(rust
        .auto_pairs
        .as_ref()
        .is_some_and(|pairs| pairs.contains(&crate::PairConfig::new('(', ')'))));
    assert!(rust.match_pairs.is_none());
    let markdown = languages.language("markdown").expect("markdown");
    assert_eq!(
        markdown
            .block_comment
            .as_ref()
            .map(|block| block.close.as_str()),
        Some("-->")
    );
}

#[test]
fn user_languages_override_editing_tokens() {
    let path = temp_file(
        "editing_tokens",
        "[[language]]\nname = \"rust\"\nline_comment = \"#\"\nblock_comment = [\"<!--\", \"-->\"]\nauto_pairs = [[\"<\", \">\"]]\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.line_comment.as_deref(), Some("#"));
    assert_eq!(
        rust.block_comment.as_ref().map(|block| block.open.as_str()),
        Some("<!--")
    );
    assert_eq!(
        rust.auto_pairs,
        Some(vec![crate::PairConfig::new('<', '>')])
    );
    assert!(languages.warnings().is_empty());
}

#[test]
fn user_languages_override_match_pairs() {
    let path = temp_file(
        "match_pairs",
        "[[language]]\nname = \"rust\"\nmatch_pairs = [[\"(\", \")\"], [\"<\", \">\"]]\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(
        rust.match_pairs,
        Some(vec![
            crate::PairConfig::new('(', ')'),
            crate::PairConfig::new('<', '>'),
        ])
    );
    assert!(languages.warnings().is_empty());
}

#[test]
fn invalid_match_pairs_warn_and_are_ignored() {
    let path = temp_file(
        "match_pairs_invalid",
        "[[language]]\nname = \"rust\"\nmatch_pairs = [[\"'\", \"'\"]]\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert!(rust.match_pairs.is_none());
    assert_eq!(languages.warnings().len(), 1);
    assert!(languages.warnings()[0]
        .message()
        .contains("match_pairs must be an array"));
}

#[test]
fn empty_editing_configuration_disables_defaults() {
    let path = temp_file(
        "editing_empty",
        "[[language]]\nname = \"rust\"\nline_comment = \"\"\nblock_comment = []\nauto_pairs = []\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.line_comment, None);
    assert_eq!(rust.block_comment, None);
    assert_eq!(rust.auto_pairs, Some(Vec::new()));
}

#[test]
fn invalid_editing_configuration_warns_and_keeps_defaults() {
    let path = temp_file(
        "editing_invalid",
        "[[language]]\nname = \"rust\"\nline_comment = \"//\\n\"\nblock_comment = [\"/*\"]\nauto_pairs = [[\"ab\", \")\"]]\n",
    );
    let languages = Languages::load(&path);
    let rust = languages.language("rust").expect("rust");
    assert_eq!(rust.line_comment.as_deref(), Some("//"));
    assert_eq!(languages.warnings().len(), 3);
}

#[test]
fn missing_binary_is_reported_for_the_language() {
    let languages = Languages::from_parts(server(), vec![rust()], BTreeMap::new());
    assert_eq!(
        languages.missing_binary_status(Some(Path::new("src/main.rs"))),
        Some("lsp: rust-analyzer not found in PATH".to_string())
    );
    assert_eq!(
        languages.missing_binary_status(Some(Path::new("notes.txt"))),
        None
    );
}

#[test]
fn available_binary_has_no_status() {
    let mut available = BTreeMap::new();
    available.insert(
        "rust-analyzer".to_string(),
        PathBuf::from("/bin/rust-analyzer"),
    );
    let languages = Languages::from_parts(server(), vec![rust()], available);
    assert_eq!(
        languages.missing_binary_status(Some(Path::new("src/main.rs"))),
        None
    );
}

#[test]
fn server_accessors_report_configuration_and_usage() {
    let mut available = BTreeMap::new();
    available.insert(
        "rust-analyzer".to_string(),
        PathBuf::from("/bin/rust-analyzer"),
    );
    let languages = Languages::from_parts(server(), vec![rust()], available);
    assert!(languages.server_is_used("rust-analyzer"));
    assert!(!languages.server_is_used("pyright"));
    assert_eq!(
        languages
            .languages_using("rust-analyzer")
            .map(|language| language.name.as_str())
            .collect::<Vec<_>>(),
        vec!["rust"]
    );
    assert_eq!(languages.languages_using("pyright").count(), 0);
    assert_eq!(
        languages.server_path("rust-analyzer"),
        Some(Path::new("/bin/rust-analyzer"))
    );
    assert!(languages
        .servers()
        .any(|(name, config)| name == "rust-analyzer" && config.enabled));
}

#[test]
fn refresh_availability_resolves_newly_installed_binaries() {
    let path = temp_file("refresh", "");
    let server = path.parent().unwrap().join("late-server");
    fs::write(
        &path,
        format!(
            "[server.late]\ncommand = {:?}\n\n[[language]]\nname = \"rust\"\nservers = [\"late\"]\n",
            server.to_str().unwrap()
        ),
    )
    .unwrap();
    let mut languages = Languages::load(&path);
    assert!(languages.server_path("late").is_none());
    executable(&server);
    languages.refresh_availability();
    assert_eq!(languages.server_path("late"), Some(server.as_path()));
}

#[test]
fn optional_ruff_is_reported_when_configured() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "pyright".to_string(),
        ServerConfig::command("pyright-langserver"),
    );
    servers.insert("ruff".to_string(), ServerConfig::command("ruff"));
    let languages = Languages::from_parts(
        servers,
        vec![LanguageConfig::new("python", "python")
            .with_extensions(["py"])
            .with_servers(["pyright", "ruff"])],
        BTreeMap::new(),
    );
    assert_eq!(
        languages.missing_binary_status(Some(Path::new("main.py"))),
        Some("lsp: pyright, ruff not found in PATH".to_string())
    );
}

#[test]
fn disabled_servers_do_not_report_status() {
    let mut servers = server();
    servers.get_mut("rust-analyzer").unwrap().enabled = false;
    let languages = Languages::from_parts(servers, vec![rust()], BTreeMap::new());
    assert_eq!(
        languages.missing_binary_status(Some(Path::new("src/main.rs"))),
        None
    );
}

#[test]
fn pyright_is_the_python_default() {
    let mut servers = server();
    servers.insert(
        "pyright".to_string(),
        ServerConfig::command("pyright-langserver").with_args(["--stdio"]),
    );
    let languages = Languages::from_parts(
        servers,
        vec![LanguageConfig::new("python", "python")
            .with_extensions(["py"])
            .with_servers(["pyright"])],
        BTreeMap::new(),
    );
    assert_eq!(
        languages.missing_binary_status(Some(Path::new("main.py"))),
        Some("lsp: pyright not found in PATH".to_string())
    );
}
