use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

const SECTIONS: &[(&str, &[&str])] = &[
    ("base", &["foreground", "background"]),
    ("search_match", &["foreground", "background"]),
    ("search_current", &["foreground", "background"]),
    ("substitute", &["foreground", "background"]),
    ("selection", &["foreground", "background"]),
    ("bracket.match", &["foreground", "background"]),
    ("status", &["foreground", "background"]),
    ("gutter", &["line_number", "non_text"]),
    (
        "git",
        &[
            "add",
            "change",
            "delete",
            "staged_add",
            "staged_change",
            "staged_delete",
        ],
    ),
    ("diagnostics", &["error", "warning", "information", "hint"]),
    (
        "syntax",
        &[
            "comment",
            "keyword",
            "string",
            "number",
            "constant",
            "function",
            "type",
            "variable",
            "property",
            "operator",
            "punctuation",
            "attribute",
            "tag",
            "invalid",
        ],
    ),
];

fn theme_text() -> String {
    let mut text = String::new();
    for (section, fields) in SECTIONS {
        text.push_str(&format!("[{section}]\n"));
        for field in *fields {
            text.push_str(&format!("{field} = \"#010203\"\n"));
        }
        text.push('\n');
    }
    text
}

fn temp_dir(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "kawi_theme_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn float_background_defaults_to_the_base_background() {
    let theme = Theme::from_toml("personal", &theme_text()).unwrap();
    assert_eq!(theme.float_background, theme.base.background);
}

#[test]
fn code_background_defaults_to_the_float_background() {
    let text = format!("float_background = \"#0a0b0c\"\n{}", theme_text());
    let theme = Theme::from_toml("personal", &text).unwrap();
    assert_eq!(theme.code_background, theme.float_background);
}

#[test]
fn bracket_match_defaults_to_selection() {
    let text = theme_text().replace(
        "[bracket.match]\nforeground = \"#010203\"\nbackground = \"#010203\"\n\n",
        "",
    );
    let theme = Theme::from_toml("personal", &text).unwrap();
    assert_eq!(theme.bracket_match, theme.selection);
}

#[test]
fn loads_a_custom_bracket_match() {
    let text = theme_text().replace(
        "[bracket.match]\nforeground = \"#010203\"\nbackground = \"#010203\"\n",
        "[bracket.match]\nforeground = \"#112233\"\nbackground = \"#445566\"\n",
    );
    let theme = Theme::from_toml("personal", &text).unwrap();
    assert_eq!(
        theme.bracket_match.foreground,
        Color::Rgb {
            r: 17,
            g: 34,
            b: 51
        }
    );
    assert_eq!(
        theme.bracket_match.background,
        Color::Rgb {
            r: 68,
            g: 85,
            b: 102
        }
    );
}

#[test]
fn loads_plugin_decoration_overrides() {
    let text = format!(
        "{}\n[plugin_decorations.markers.todo]\nforeground = \"#112233\"\nfill = \"foreground\"\nitalic = true\n",
        theme_text()
    );
    let theme = Theme::from_toml("personal", &text).unwrap();
    let value = theme
        .plugin_decorations
        .get("markers", "todo")
        .expect("override");
    assert_eq!(
        value.foreground,
        Some(Color::Rgb {
            r: 17,
            g: 34,
            b: 51
        })
    );
    assert_eq!(value.fill, Some(crate::frame::PluginFill::Foreground));
    assert_eq!(value.italic, Some(true));
}

#[test]
fn loads_a_custom_code_background() {
    let text = format!("code_background = \"#0a0b0c\"\n{}", theme_text());
    let theme = Theme::from_toml("personal", &text).unwrap();
    assert_eq!(
        theme.code_background,
        Color::Rgb {
            r: 10,
            g: 11,
            b: 12,
        }
    );
}

#[test]
fn loads_a_custom_theme() {
    let dir = temp_dir("load");
    fs::write(dir.join("personal.toml"), theme_text()).unwrap();
    let theme = Theme::load("personal", &dir).unwrap();
    assert_eq!(theme.name, "personal");
    assert_eq!(theme.base.background, Color::Rgb { r: 1, g: 2, b: 3 });
    assert_eq!(
        theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground),
        Some(Color::Rgb { r: 1, g: 2, b: 3 })
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn theme_files_override_builtins() {
    let dir = temp_dir("override");
    let text = theme_text().replacen("background = \"#010203\"", "background = \"#123456\"", 1);
    fs::write(dir.join("default.toml"), text).unwrap();
    let theme = Theme::load("default", &dir).unwrap();
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 18,
            g: 52,
            b: 86
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_fallback_when_theme_files_are_missing() {
    let dir = temp_dir("fallback");
    let theme = Theme::load("default", &dir).unwrap();
    assert_eq!(theme.name, "default");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 16,
            g: 23,
            b: 26
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_fallback_when_theme_files_are_invalid() {
    let dir = temp_dir("invalid_override");
    fs::write(dir.join("default.toml"), "broken").unwrap();
    let theme = Theme::load("default", &dir).unwrap();
    assert_eq!(theme.name, "default");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn missing_theme_reports_not_found() {
    let dir = temp_dir("missing");
    let error = Theme::load("personal", &dir).unwrap_err();
    assert!(error.is_not_found());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rejects_unsafe_theme_names() {
    let dir = temp_dir("unsafe");
    assert!(Theme::load("../personal", &dir).is_err());
    assert!(Theme::load("personal/evil", &dir).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn applies_palette_indirection_and_style_attributes() {
    let text = format!(
        "{}\n[palette]\naccent = \"#112233\"\n",
        theme_text().replace(
            "keyword = \"#010203\"",
            "keyword = { fg = \"accent\", bold = true, italic = true, underline = true }"
        )
    );
    let theme = Theme::from_toml("personal", &text).unwrap();
    let style = theme.syntax.named("keyword").expect("keyword");
    assert_eq!(
        style.foreground,
        Some(Color::Rgb {
            r: 17,
            g: 34,
            b: 51
        })
    );
    assert!(style.bold);
    assert!(style.italic);
    assert!(style.underline);
}

#[test]
fn applies_nested_capture_styles() {
    let text = theme_text().replace(
        "keyword = \"#010203\"",
        "keyword = \"#010203\"\n\"keyword.return\" = { fg = \"#445566\", bold = true }",
    );
    let theme = Theme::from_toml("personal", &text).unwrap();
    let style = theme.syntax.named("keyword.return").expect("subcapture");
    assert_eq!(
        style.foreground,
        Some(Color::Rgb {
            r: 68,
            g: 85,
            b: 102
        })
    );
    assert!(style.bold);
    assert_ne!(
        style.foreground,
        theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground)
    );
}

#[test]
fn undefined_subcaptures_fall_back_to_the_base_style() {
    let theme = Theme::from_toml("personal", &theme_text()).unwrap();
    assert_eq!(
        theme.syntax.named("keyword.return"),
        theme.syntax.named("keyword")
    );
}

#[test]
fn rejects_unknown_fields() {
    let text = format!("extra = 1\n{}", theme_text());
    let error = Theme::from_toml("personal", &text).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn rejects_missing_colors() {
    let text = theme_text().replacen("foreground = \"#010203\"\n", "", 1);
    let error = Theme::from_toml("personal", &text).unwrap_err();
    assert!(error.to_string().contains("missing field"));
    assert!(error.to_string().contains("line"));
}

#[test]
fn rejects_invalid_colors() {
    let text = theme_text().replacen("background = \"#010203\"", "background = \"#zzzzzz\"", 1);
    let error = Theme::from_toml("personal", &text).unwrap_err();
    assert!(error.to_string().contains("base.background"));
}

#[test]
fn lists_builtin_and_custom_themes() {
    let dir = temp_dir("available");
    fs::write(dir.join("personal.toml"), theme_text()).unwrap();
    fs::write(dir.join("broken.toml"), "invalid").unwrap();
    fs::write(dir.join("invalid name.toml"), "").unwrap();
    fs::write(dir.join("notes.txt"), "").unwrap();
    let names = Theme::available(&dir);
    assert!(names.contains(&"default".to_string()));
    assert!(names.contains(&"personal".to_string()));
    assert!(!names.contains(&"broken".to_string()));
    assert!(!names.contains(&"invalid name".to_string()));
    assert!(!names.contains(&"notes".to_string()));
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn validates_custom_and_builtin_themes() {
    let dir = temp_dir("validation");
    fs::write(dir.join("personal.toml"), theme_text()).unwrap();
    assert_eq!(Theme::validate("personal", &dir), Ok(()));
    assert_eq!(Theme::validate("default", &dir), Ok(()));
    assert!(Theme::validate("missing", &dir).is_err());
}

#[test]
fn validation_rejects_an_invalid_custom_builtin_override() {
    let dir = temp_dir("invalid_builtin");
    fs::write(dir.join("default.toml"), "not valid toml").unwrap();
    assert!(Theme::validate("default", &dir).is_err());
}

#[test]
fn parses_theme_package_metadata() {
    let text = format!(
        "[package]\nname = \"personal\"\nversion = \"1.2.0\"\ndescription = \"Personal palette\"\nauthor = \"Ada\"\n{}",
        theme_text()
    );
    let theme = Theme::from_toml("personal", &text).unwrap();
    let package = theme.package.expect("package");
    assert_eq!(package.name, "personal");
    assert_eq!(package.version, "1.2.0");
    assert_eq!(package.description.as_deref(), Some("Personal palette"));
    assert_eq!(package.author.as_deref(), Some("Ada"));
}

#[test]
fn rejects_unknown_package_fields() {
    let text = format!(
        "[package]\nname = \"personal\"\nversion = \"1.0.0\"\nunknown = true\n{}",
        theme_text()
    );
    assert!(Theme::from_toml("personal", &text).is_err());
}

#[test]
fn loads_theme_packages_from_directories() {
    let dir = temp_dir("package_dir");
    let package = dir.join("personal");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("theme.toml"), theme_text()).unwrap();
    assert_eq!(Theme::validate("personal", &dir), Ok(()));
    assert_eq!(
        Theme::load("personal", &dir).expect("theme").name,
        "personal"
    );
    assert!(Theme::available(&dir).contains(&"personal".to_string()));
}
