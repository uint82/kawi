use std::fs;
use std::path::PathBuf;

use crossterm::style::Color;
use kawi_buffer::Buffer;
use kawi_lsp::{CursorPosition, DocumentColor};
use kawi_tui::frame::DecorationSpan;

use super::lsp;
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

fn spans_for(app: &App, source: &[&str]) -> Vec<DecorationSpan> {
    let lines = source
        .iter()
        .map(|line| (*line).to_string())
        .collect::<Vec<_>>();
    let rows = (0..lines.len()).collect::<Vec<_>>();
    app.color_preview_spans(app.editor.buffer_number(), &lines, &rows)
}

fn spans(app: &App, text: &str) -> Vec<DecorationSpan> {
    spans_for(app, &[text])
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

#[test]
fn css_named_colors_are_limited_to_declaration_values() {
    let line = ".red { color: white; content: \"red\"; }";
    let app = app("css_named", "style.css", line, "");
    let found = spans(&app, line);
    assert_eq!(found.len(), 1);
    let start = line.find("white").expect("white");
    assert_eq!(found[0].start, start);
    assert_eq!(found[0].end, start + "white".len());
}

#[test]
fn css_rgb_function_produces_color_span() {
    let line = "color: rgb(110 208 240 / 0.12);";
    let app = app("css_rgb", "style.css", line, "");
    let found = spans(&app, line);
    assert_eq!(found.len(), 1);
    let start = line.find("rgb").expect("rgb");
    assert_eq!(found[0].start, start);
    assert_eq!(found[0].end, start + "rgb(110 208 240 / 0.12)".len());
}

#[test]
fn html_style_attribute_supports_named_and_rgb_colors() {
    let line = "<div style=\"color: red; background: rgb(0 0 0 / 0.35)\">red</div>";
    let app = app("html_style", "index.html", line, "");
    let found = spans(&app, line);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].start, line.find("red").expect("red"));
    assert_eq!(found[1].start, line.find("rgb").expect("rgb"));
}

#[test]
fn html_text_colors_are_ignored() {
    let line = "<p>red rgb(0 0 0)</p>";
    let app = app("html_text", "index.html", line, "");
    assert!(spans(&app, line).is_empty());
}

#[test]
fn html_style_block_supports_named_colors() {
    let source = ["<style>", ".x { color: red; }", "</style>"];
    let app = app("html_block", "index.html", &source.join("\n"), "");
    let found = spans_for(&app, &source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].row, 1);
    assert_eq!(found[0].start, source[1].find("red").expect("red"));
}

#[test]
fn hex_identifier_suffix_is_ignored() {
    let line = "color: #abcdefghi;";
    let app = app("hex_suffix", "style.css", line, "");
    assert!(spans(&app, line).is_empty());
}

#[test]
fn unicode_prefix_uses_character_columns() {
    let line = "é color: #fff;";
    let app = app("unicode_prefix", "style.css", line, "");
    let found = spans(&app, line);
    assert_eq!(found.len(), 1);
    let start = line.find("#fff").expect("color");
    assert_eq!(found[0].start, line[..start].chars().count());
    assert_eq!(found[0].end, found[0].start + "#fff".len());
}

#[test]
fn red_previews_use_white_foreground() {
    let app = app("red_foreground", "style.css", "", "");
    for color in ["#ff0000", "#fb4934"] {
        let line = format!("color: {color};");
        let found = spans(&app, &line);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].style.foreground,
            Color::Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
    }
}

#[test]
fn bright_green_preview_uses_black_foreground() {
    let line = "color: #00ff00;";
    let app = app("green_foreground", "style.css", line, "");
    let found = spans(&app, line);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].style.foreground, Color::Rgb { r: 0, g: 0, b: 0 });
}

fn lsp_color(
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
    red: u8,
    green: u8,
    blue: u8,
) -> DocumentColor {
    DocumentColor {
        start: CursorPosition {
            line: start_line,
            column: start_column,
        },
        end: CursorPosition {
            line: end_line,
            column: end_column,
        },
        red,
        green,
        blue,
        alpha: 255,
    }
}

#[test]
fn lsp_spans_use_provider_ranges() {
    let colors = vec![lsp_color(0, 0, 0, 4, 255, 0, 0)];
    let lines = vec!["#fff".to_string()];
    let found = lsp::spans(&colors, &lines, &[0], Color::Black);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].row, 0);
    assert_eq!(found[0].start, 0);
    assert_eq!(found[0].end, 4);
    assert_eq!(found[0].style.background, Color::Rgb { r: 255, g: 0, b: 0 });
}

#[test]
fn lsp_spans_clip_multiline_ranges_per_row() {
    let colors = vec![lsp_color(0, 2, 1, 2, 0, 0, 255)];
    let lines = vec!["abcdef".to_string(), "ghijkl".to_string()];
    let found = lsp::spans(&colors, &lines, &[0, 1], Color::Black);
    assert_eq!(found.len(), 2);
    assert_eq!((found[0].row, found[0].start, found[0].end), (0, 2, 6));
    assert_eq!((found[1].row, found[1].start, found[1].end), (1, 0, 2));
}

#[test]
fn lsp_colors_take_precedence_over_the_scanner() {
    let mut app = app("lsp_precedence", "style.css", "color: #ff0000;\n", "");
    let path = std::path::Path::new("style.css");
    app.lsp.set_color_preview_colors_for_test(
        path,
        app.editor.buffer_revision(),
        vec![lsp_color(0, 0, 0, 6, 0, 0, 255)],
    );
    let found = spans(&app, "color: #ff0000;");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].start, 0);
    assert_eq!(found[0].end, 6);
    assert_eq!(found[0].style.background, Color::Rgb { r: 0, g: 0, b: 255 });
}
