use super::*;

#[test]
fn custom_theme_files_are_valid_config_values() {
    let dir = temp_dir("custom_theme");
    let themes = dir.join("themes");
    fs::create_dir_all(&themes).unwrap();
    fs::write(
        themes.join("personal.toml"),
        crate::test_support::custom_theme(),
    )
    .unwrap();
    let path = dir.join("config.toml");
    fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
    let loaded = load(&path);
    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded.config.get("theme"),
        Some(&OptionValue::Text("personal".to_string()))
    );
    assert_eq!(
        kawi_tui::theme::Theme::load("personal", &themes)
            .unwrap()
            .name,
        "personal"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn invalid_custom_themes_fall_back_with_a_warning() {
    let dir = temp_dir("invalid_theme");
    let themes = dir.join("themes");
    fs::create_dir_all(&themes).unwrap();
    fs::write(
        themes.join("personal.toml"),
        "[base]\nforeground = \"#zzzzzz\"\n",
    )
    .unwrap();
    let path = dir.join("config.toml");
    fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
    let loaded = load(&path);
    assert_eq!(
        loaded.config.get("theme"),
        Some(&OptionValue::Text("default".to_string()))
    );
    assert_eq!(loaded.warnings.len(), 1);
    assert!(loaded.warnings[0]
        .format(&path)
        .contains("ui.theme: rejected \"personal\": using default"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn missing_custom_themes_fall_back_with_a_warning() {
    let dir = temp_dir("missing_theme");
    let path = dir.join("config.toml");
    fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
    let loaded = load(&path);
    assert_eq!(
        loaded.config.get("theme"),
        Some(&OptionValue::Text("default".to_string()))
    );
    assert_eq!(loaded.warnings.len(), 1);
    let _ = fs::remove_dir_all(dir);
}
