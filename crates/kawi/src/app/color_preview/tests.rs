use std::fs;
use std::path::PathBuf;

use crossterm::style::Color;
use kawi_buffer::Buffer;
use kawi_tui::frame::DecorationSpan;

use crate::app::App;
use crate::keymap::Flavor;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_color_preview_{}_{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn app(name: &str, path: &str, text: &str, languages: &str) -> App {
    let mut buffer = Buffer::from_string(text);
    buffer.path = Some(path.to_string());
    let mut app = App::new(buffer, Flavor::Vim);
    let dir = temp_dir(name);
    let languages_path = dir.join("languages.toml");
    fs::write(&languages_path, languages).expect("write languages");
    app.lsp.load(&languages_path, app.events.wake());
    app
}

fn spans(app: &App, text: &str) -> Vec<DecorationSpan> {
    let lines = vec![text.to_string()];
    app.color_preview_spans(app.editor.buffer_number(), &lines, &[0])
}

#[test]
fn css_buffer_produces_color_span() {
    let app = app("css_span", "style.css", "a { color: #000000; }\n", "");
    let found = spans(&app, "a { color: #000000; }");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].start, 11);
    assert_eq!(found[0].end, 18);
    assert_eq!(found[0].style.background, Color::Rgb { r: 0, g: 0, b: 0 });
    assert_eq!(
        found[0].style.foreground,
        Color::Rgb {
            r: 255,
            g: 255,
            b: 255
        }
    );
}

#[test]
fn global_option_disables_color_preview() {
    let mut app = app("global_option", "style.css", "color: #abcdef;\n", "");
    app.color_preview = false;
    assert!(spans(&app, "color: #abcdef;").is_empty());
}

#[test]
fn unsupported_language_disables_color_preview() {
    let app = app(
        "unsupported",
        "main.rs",
        "const COLOR: &str = \"#abcdef\";\n",
        "",
    );
    assert!(spans(&app, "const COLOR: &str = \"#abcdef\";").is_empty());
}

#[test]
fn language_config_can_enable_color_preview() {
    let app = app(
        "enable_language",
        "main.rs",
        "const COLOR: &str = \"#abcdef\";\n",
        "[[language]]\nname = \"rust\"\ncolor_preview = true\n",
    );
    let found = spans(&app, "const COLOR: &str = \"#abcdef\";");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].start, 21);
    assert_eq!(found[0].end, 28);
}

#[test]
fn language_config_can_disable_color_preview() {
    let app = app(
        "disable_language",
        "style.css",
        "color: #abcdef;\n",
        "[[language]]\nname = \"css\"\ncolor_preview = false\n",
    );
    assert!(spans(&app, "color: #abcdef;").is_empty());
}
