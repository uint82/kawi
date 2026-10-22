use super::*;
use crate::text;
use crate::workspace::{self, path_from_uri};
use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, Location, LocationLink, MarkedString,
    MarkupContent, Position, PositionEncodingKind,
};
use ropey::Rope;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn definition_locations(
    documents: &HashMap<PathBuf, Document>,
    value: Value,
    encoding: &PositionEncodingKind,
) -> Vec<TextLocation> {
    let Ok(response) = serde_json::from_value::<GotoDefinitionResponse>(value) else {
        return Vec::new();
    };
    let mut cache = HashMap::new();
    match response {
        GotoDefinitionResponse::Scalar(location) => {
            resolve_location(documents, &mut cache, &location, encoding)
                .into_iter()
                .collect()
        }
        GotoDefinitionResponse::Array(locations) => locations
            .iter()
            .filter_map(|location| resolve_location(documents, &mut cache, location, encoding))
            .collect(),
        GotoDefinitionResponse::Link(links) => links
            .iter()
            .filter_map(|link| resolve_link(documents, &mut cache, link, encoding))
            .collect(),
    }
}

pub(super) fn reference_locations(
    documents: &HashMap<PathBuf, Document>,
    value: Value,
    encoding: &PositionEncodingKind,
) -> Vec<TextLocation> {
    let Ok(locations) = serde_json::from_value::<Vec<Location>>(value) else {
        return Vec::new();
    };
    let mut cache = HashMap::new();
    locations
        .iter()
        .filter_map(|location| resolve_location(documents, &mut cache, location, encoding))
        .collect()
}

fn resolve_link(
    documents: &HashMap<PathBuf, Document>,
    cache: &mut HashMap<PathBuf, Rope>,
    link: &LocationLink,
    encoding: &PositionEncodingKind,
) -> Option<TextLocation> {
    let path = path_for_uri(&link.target_uri)?;
    let start = link.target_selection_range.start;
    resolve_position(documents, cache, &path, start, encoding)
}

fn resolve_location(
    documents: &HashMap<PathBuf, Document>,
    cache: &mut HashMap<PathBuf, Rope>,
    location: &Location,
    encoding: &PositionEncodingKind,
) -> Option<TextLocation> {
    let path = path_for_uri(&location.uri)?;
    resolve_position(documents, cache, &path, location.range.start, encoding)
}

fn resolve_position(
    documents: &HashMap<PathBuf, Document>,
    cache: &mut HashMap<PathBuf, Rope>,
    path: &Path,
    position: Position,
    encoding: &PositionEncodingKind,
) -> Option<TextLocation> {
    if !cache.contains_key(path) {
        let rope = documents
            .get(path)
            .map(|document| document.text.clone())
            .or_else(|| {
                fs::read_to_string(path)
                    .ok()
                    .map(|text| Rope::from_str(&text))
            })?;
        cache.insert(path.to_path_buf(), rope);
    }
    let text = cache.get(path)?;
    let line = text::clamped_line(text, position.line);
    let column = text::line_character(
        text,
        u32::try_from(line).unwrap_or(u32::MAX),
        position.character,
        encoding,
    );
    Some(TextLocation {
        path: path.to_path_buf(),
        line: u32::try_from(line).unwrap_or(u32::MAX),
        column,
    })
}

fn path_for_uri(uri: &lsp_types::Uri) -> Option<PathBuf> {
    path_from_uri(uri).map(|path| workspace::absolute(&path))
}

pub(super) fn sort_locations(locations: &mut Vec<TextLocation>) {
    locations.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.column.cmp(&right.column))
    });
    locations.dedup();
}

pub(super) fn hover_lines(value: Value) -> Option<Vec<String>> {
    let hover = serde_json::from_value::<Hover>(value).ok()?;
    let mut lines = match hover.contents {
        HoverContents::Scalar(marked) => marked_lines(&marked),
        HoverContents::Array(items) => items.iter().flat_map(marked_lines).collect(),
        HoverContents::Markup(markup) => markup_lines(&markup),
    };
    trim_empty_lines(&mut lines);
    Some(lines)
}

fn marked_lines(marked: &MarkedString) -> Vec<String> {
    match marked {
        MarkedString::String(text) => plain_lines(text),
        MarkedString::LanguageString(value) => plain_lines(&value.value),
    }
}

fn markup_lines(markup: &MarkupContent) -> Vec<String> {
    if markup.kind == lsp_types::MarkupKind::Markdown {
        markdown_lines(&markup.value)
    } else {
        plain_lines(&markup.value)
    }
}

fn plain_lines(text: &str) -> Vec<String> {
    text.lines().map(sanitize_line).collect()
}

fn sanitize_line(line: &str) -> String {
    line.chars()
        .map(|character| {
            if character == '\t' || character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

fn markdown_lines(text: &str) -> Vec<String> {
    let mut in_fence = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            lines.push(sanitize_line(line));
        } else if trimmed != "---" {
            lines.push(strip_markdown(line));
        }
    }
    lines
}

fn strip_markdown(line: &str) -> String {
    let mut text = line.to_string();
    for marker in ["**", "__", "`", "~~"] {
        text = text.replace(marker, "");
    }
    let trimmed = text.trim_start();
    if let Some(rest) = trimmed.strip_prefix('#') {
        text = rest.trim_start().to_string();
    } else if let Some(rest) = trimmed.strip_prefix("> ") {
        text = rest.to_string();
    }
    sanitize_line(text.trim_end())
}

fn trim_empty_lines(lines: &mut Vec<String>) {
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
}

pub(super) fn join_hovers(hovers: Vec<Vec<String>>) -> Vec<String> {
    let mut contents = Vec::new();
    for lines in hovers {
        if lines.is_empty() {
            continue;
        }
        if !contents.is_empty() {
            contents.push(String::new());
        }
        contents.extend(lines);
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(path: &Path, text: &str) -> Document {
        Document {
            uri: crate::workspace::file_uri(path).expect("uri"),
            language_id: "rust".to_string(),
            text: Rope::from_str(text),
            version: 1,
            clients: Vec::new(),
        }
    }

    fn documents(path: &Path, text: &str) -> HashMap<PathBuf, Document> {
        HashMap::from([(path.to_path_buf(), document(path, text))])
    }

    #[test]
    fn resolves_utf16_columns_to_character_columns() {
        let path = PathBuf::from("/tmp/kawi_navigation_utf16.rs");
        let documents = documents(&path, "a𝄞b\n");
        let mut cache = HashMap::new();
        let location = resolve_position(
            &documents,
            &mut cache,
            &path,
            Position::new(0, 3),
            &PositionEncodingKind::UTF16,
        )
        .expect("location");
        assert_eq!(location.column, 2);
    }

    #[test]
    fn clamps_missing_lines_to_the_document_end() {
        let path = PathBuf::from("/tmp/kawi_navigation_clamp.rs");
        let documents = documents(&path, "one\ntwo\n");
        let mut cache = HashMap::new();
        let location = resolve_position(
            &documents,
            &mut cache,
            &path,
            Position::new(99, 0),
            &PositionEncodingKind::UTF16,
        )
        .expect("location");
        assert_eq!(location.line, 2);
    }

    #[test]
    fn definition_responses_become_text_locations() {
        let path = PathBuf::from("/tmp/kawi_navigation_definition.rs");
        let documents = documents(&path, "one\ntwo\n");
        let uri = crate::workspace::file_uri(&path).expect("uri");
        let value = serde_json::json!({
            "uri": uri,
            "range": {
                "start": {"line": 1, "character": 0},
                "end": {"line": 1, "character": 3}
            }
        });
        let locations = definition_locations(&documents, value, &PositionEncodingKind::UTF16);
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].line, 1);
        assert_eq!(locations[0].column, 0);
    }

    #[test]
    fn locations_are_sorted_and_deduplicated() {
        let mut locations = vec![
            TextLocation {
                path: PathBuf::from("/tmp/b.rs"),
                line: 1,
                column: 0,
            },
            TextLocation {
                path: PathBuf::from("/tmp/a.rs"),
                line: 2,
                column: 0,
            },
            TextLocation {
                path: PathBuf::from("/tmp/b.rs"),
                line: 1,
                column: 0,
            },
        ];
        sort_locations(&mut locations);
        assert_eq!(
            locations,
            vec![
                TextLocation {
                    path: PathBuf::from("/tmp/a.rs"),
                    line: 2,
                    column: 0,
                },
                TextLocation {
                    path: PathBuf::from("/tmp/b.rs"),
                    line: 1,
                    column: 0,
                },
            ]
        );
    }

    #[test]
    fn markdown_is_flattened_for_the_pager() {
        let lines = markdown_lines("# Title\n\n```rust\nfn main()\n```\n---\n`value`");
        assert_eq!(lines, vec!["Title", "", "fn main()", "value"]);
    }

    #[test]
    fn control_characters_are_removed_from_hover_text() {
        assert_eq!(plain_lines("safe\x1b[31mtext"), vec!["safe [31mtext"]);
    }

    #[test]
    fn hover_merges_multiple_parts() {
        let contents = join_hovers(vec![vec!["one".to_string()], vec!["two".to_string()]]);
        assert_eq!(contents, vec!["one", "", "two"]);
    }
}
