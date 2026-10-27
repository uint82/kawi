use toml_edit::Item;

use kawi_tui::statusbar::{ModeKind, SegmentId};

use super::load::Loader;

pub(super) fn load(loader: &mut Loader<'_>, item: &Item) {
    let Some(table) = item.as_table() else {
        loader.warn(
            item.span(),
            "ui.statusline",
            item.to_string(),
            "section ignored, expected a table",
        );
        return;
    };
    for (key, item) in table.iter() {
        match key {
            "left" => {
                if let Some(ids) = parse_list(loader, item, "ui.statusline.left") {
                    loader.config.statusline.left = ids;
                }
            }
            "center" => {
                if let Some(ids) = parse_list(loader, item, "ui.statusline.center") {
                    loader.config.statusline.center = ids;
                }
            }
            "right" => {
                if let Some(ids) = parse_list(loader, item, "ui.statusline.right") {
                    loader.config.statusline.right = ids;
                }
            }
            "separator" => load_separator(loader, item),
            "color_modes" => load_color_modes(loader, item),
            "mode" => load_mode_labels(loader, item),
            _ => loader.warn(
                item.span(),
                format!("ui.statusline.{key}"),
                item.to_string(),
                "ignored",
            ),
        }
    }
}

fn parse_list(loader: &mut Loader<'_>, item: &Item, key: &str) -> Option<Vec<SegmentId>> {
    let Some(array) = item.as_array() else {
        loader.warn(item.span(), key, item.to_string(), "default element list");
        return None;
    };
    let mut ids = Vec::with_capacity(array.len());
    for value in array.iter() {
        let Some(name) = value.as_str() else {
            loader.warn(value.span(), key, value.to_string(), "ignored element");
            continue;
        };
        match SegmentId::parse(name) {
            Some(id) => ids.push(id),
            None => loader.warn(value.span(), key, value.to_string(), "ignored element"),
        }
    }
    Some(ids)
}

fn load_separator(loader: &mut Loader<'_>, item: &Item) {
    let Some(value) = item.as_str() else {
        loader.warn(
            item.span(),
            "ui.statusline.separator",
            item.to_string(),
            "default separator",
        );
        return;
    };
    loader.config.statusline.separator = value.to_string();
}

fn load_color_modes(loader: &mut Loader<'_>, item: &Item) {
    let Some(value) = item.as_bool() else {
        loader.warn(
            item.span(),
            "ui.statusline.color_modes",
            item.to_string(),
            "default color mode",
        );
        return;
    };
    loader.config.statusline.color_modes = value;
}

fn load_mode_labels(loader: &mut Loader<'_>, item: &Item) {
    let Some(table) = item.as_table() else {
        loader.warn(
            item.span(),
            "ui.statusline.mode",
            item.to_string(),
            "default mode labels",
        );
        return;
    };
    for (key, item) in table.iter() {
        let Some(kind) = ModeKind::parse(key) else {
            loader.warn(
                item.span(),
                format!("ui.statusline.mode.{key}"),
                item.to_string(),
                "ignored",
            );
            continue;
        };
        let Some(value) = item.as_str() else {
            loader.warn(
                item.span(),
                format!("ui.statusline.mode.{key}"),
                item.to_string(),
                "default mode label",
            );
            continue;
        };
        loader
            .config
            .statusline
            .mode_labels
            .insert(kind, value.to_string());
    }
}
