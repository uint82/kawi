use crossterm::style::Color;
use std::collections::BTreeSet;

pub(super) fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub(super) fn syntax_keys(source: &str) -> BTreeSet<String> {
    table_keys(source, "syntax")
}

pub(super) fn table_keys(source: &str, section: &str) -> BTreeSet<String> {
    document(source)
        .get(section)
        .and_then(toml::Value::as_table)
        .expect("section table")
        .keys()
        .cloned()
        .collect()
}

pub(super) fn leaf_keys(source: &str, section: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let document = document(source);
    let table = document
        .get(section)
        .and_then(toml::Value::as_table)
        .expect("section table");
    collect_leaf_keys("", table, &mut keys);
    keys
}

fn document(source: &str) -> toml::Value {
    toml::from_str(source).expect("embedded theme")
}

fn collect_leaf_keys(prefix: &str, table: &toml::Table, keys: &mut BTreeSet<String>) {
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::Table(nested) => collect_leaf_keys(&path, nested, keys),
            _ => {
                keys.insert(path);
            }
        }
    }
}

pub(super) fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let foreground = luminance(foreground);
    let background = luminance(background);
    let (lighter, darker) = if foreground > background {
        (foreground, background)
    } else {
        (background, foreground)
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn luminance(color: Color) -> f64 {
    let (r, g, b) = match color {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => return 0.0,
    };
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}
