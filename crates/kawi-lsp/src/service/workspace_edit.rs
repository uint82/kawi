use super::*;
use crate::text;
use crate::workspace;
use kawi_buffer::{content_hash, validate_text_edits, TextEdit, TextEditError};
use lsp_types::{DocumentChanges, OneOf, TextDocumentEdit, WorkspaceEdit as ProtocolWorkspaceEdit};
use ropey::Rope;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

impl Manager {
    pub(super) fn parse_workspace_edit(
        &self,
        value: Value,
        encoding: &PositionEncodingKind,
    ) -> Result<WorkspaceEdit, String> {
        let edit = serde_json::from_value::<ProtocolWorkspaceEdit>(value)
            .map_err(|error| format!("Invalid workspace edit: {error}"))?;
        if let Some(changes) = edit.document_changes {
            return match changes {
                DocumentChanges::Edits(edits) => self.versioned_edits(edits, encoding),
                DocumentChanges::Operations(_) => {
                    Err("Workspace edit resource operations are not supported".to_string())
                }
            };
        }
        let Some(changes) = edit.changes else {
            return Err("Workspace edit contained no changes".to_string());
        };
        self.plain_edits(changes.into_iter().collect(), encoding)
    }

    fn versioned_edits(
        &self,
        documents: Vec<TextDocumentEdit>,
        encoding: &PositionEncodingKind,
    ) -> Result<WorkspaceEdit, String> {
        let mut seen = HashSet::new();
        let mut edits = Vec::new();
        for document in documents {
            let path = document_path(&document.text_document.uri)?;
            if !seen.insert(path.clone()) {
                return Err(format!("Workspace edit repeats {}", path.display()));
            }
            self.check_version(&path, document.text_document.version)?;
            let text = self.document_text(&path)?;
            let converted = convert_edits(&text, &document.edits, encoding)?;
            edits.push(document_edit(path, text, converted)?);
        }
        finish_edit(edits)
    }

    fn plain_edits(
        &self,
        mut documents: Vec<(lsp_types::Uri, Vec<lsp_types::TextEdit>)>,
        encoding: &PositionEncodingKind,
    ) -> Result<WorkspaceEdit, String> {
        documents.sort_by_key(|(uri, _)| uri.to_string());
        let mut edits = Vec::new();
        for (uri, changes) in documents {
            let path = document_path(&uri)?;
            let text = self.document_text(&path)?;
            let edits_in = changes.into_iter().map(OneOf::Left).collect::<Vec<_>>();
            let converted = convert_edits(&text, &edits_in, encoding)?;
            edits.push(document_edit(path, text, converted)?);
        }
        finish_edit(edits)
    }

    fn check_version(&self, path: &std::path::Path, version: Option<i32>) -> Result<(), String> {
        let Some(version) = version else {
            return Ok(());
        };
        let Some(document) = self.documents.get(path) else {
            return Ok(());
        };
        if document.version == version {
            return Ok(());
        }
        Err(format!(
            "Workspace edit version {} does not match open version {}: {}",
            version,
            document.version,
            path.display()
        ))
    }

    fn document_text(&self, path: &std::path::Path) -> Result<Rope, String> {
        if let Some(document) = self.documents.get(path) {
            return Ok(document.text.clone());
        }
        let text = fs::read_to_string(path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        Ok(Rope::from_str(&text))
    }
}

fn document_path(uri: &lsp_types::Uri) -> Result<PathBuf, String> {
    workspace::path_from_uri(uri)
        .map(|path| workspace::absolute(&path))
        .ok_or_else(|| format!("Workspace edit has an invalid file URI: {uri:?}"))
}

fn convert_edits(
    text: &Rope,
    edits: &[OneOf<lsp_types::TextEdit, lsp_types::AnnotatedTextEdit>],
    encoding: &PositionEncodingKind,
) -> Result<Vec<TextEdit>, String> {
    let mut converted: Vec<TextEdit> = edits
        .iter()
        .map(|edit| {
            let edit = match edit {
                OneOf::Left(edit) => edit,
                OneOf::Right(edit) => &edit.text_edit,
            };
            convert_edit(text, edit, encoding)
        })
        .collect::<Result<_, _>>()?;
    converted.sort_by_key(|edit| (edit.start, edit.end));
    Ok(converted)
}

fn convert_edit(
    text: &Rope,
    edit: &lsp_types::TextEdit,
    encoding: &PositionEncodingKind,
) -> Result<TextEdit, String> {
    let start = text::char_offset(text, edit.range.start, encoding)
        .ok_or_else(|| "Workspace edit range is out of bounds".to_string())?;
    let end = text::char_offset(text, edit.range.end, encoding)
        .ok_or_else(|| "Workspace edit range is out of bounds".to_string())?;
    Ok(TextEdit {
        start,
        end,
        text: edit.new_text.clone(),
    })
}

fn document_edit(path: PathBuf, text: Rope, edits: Vec<TextEdit>) -> Result<DocumentEdit, String> {
    validate_text_edits(text.len_chars(), &edits).map_err(edit_error)?;
    Ok(DocumentEdit {
        path,
        original_hash: content_hash(text.to_string().as_bytes()),
        edits,
    })
}

fn edit_error(error: TextEditError) -> String {
    match error {
        TextEditError::InvalidRange => "Workspace edit range is invalid".to_string(),
        TextEditError::Overlapping => "Workspace edit ranges overlap".to_string(),
    }
}

fn finish_edit(documents: Vec<DocumentEdit>) -> Result<WorkspaceEdit, String> {
    if documents.is_empty() {
        return Err("Workspace edit contained no changes".to_string());
    }
    Ok(WorkspaceEdit { documents })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Position, PositionEncodingKind};

    fn text(value: &str) -> Rope {
        Rope::from_str(value)
    }

    fn protocol_edit(start: (u32, u32), end: (u32, u32), value: &str) -> lsp_types::TextEdit {
        lsp_types::TextEdit {
            range: lsp_types::Range::new(
                Position::new(start.0, start.1),
                Position::new(end.0, end.1),
            ),
            new_text: value.to_string(),
        }
    }

    #[test]
    fn strict_offsets_reject_out_of_bounds_ranges() {
        let text = text("one\ntwo\n");
        let edits = vec![OneOf::Left(protocol_edit((0, 0), (4, 0), "x"))];
        assert!(convert_edits(&text, &edits, &PositionEncodingKind::UTF16).is_err());
    }

    #[test]
    fn converts_ranges_to_character_offsets() {
        let text = text("a𝄞b");
        let edits = vec![OneOf::Left(protocol_edit((0, 3), (0, 3), "x"))];
        let converted = convert_edits(&text, &edits, &PositionEncodingKind::UTF16).expect("edits");
        assert_eq!(converted[0].start, 2);
        assert_eq!(converted[0].end, 2);
    }

    #[test]
    fn sorts_protocol_edits_by_offset() {
        let text = text("abcdef");
        let edits = vec![
            OneOf::Left(protocol_edit((0, 4), (0, 5), "X")),
            OneOf::Left(protocol_edit((0, 1), (0, 2), "Y")),
        ];
        let converted = convert_edits(&text, &edits, &PositionEncodingKind::UTF16).expect("edits");
        assert_eq!(converted[0].start, 1);
        assert_eq!(converted[1].start, 4);
        assert!(document_edit(PathBuf::from("/tmp/x"), text, converted).is_ok());
    }

    #[test]
    fn rejects_overlapping_edits() {
        let text = text("abcdef");
        let edits = vec![
            OneOf::Left(protocol_edit((0, 0), (0, 3), "x")),
            OneOf::Left(protocol_edit((0, 2), (0, 4), "y")),
        ];
        let converted = convert_edits(&text, &edits, &PositionEncodingKind::UTF16).expect("edits");
        assert!(document_edit(PathBuf::from("/tmp/x"), text, converted).is_err());
    }
}
