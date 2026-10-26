use kawi_editor::Editor;
use kawi_picker::PickerKind;
use ropey::Rope;
use std::path::PathBuf;

use crate::app::picker::PickerSession;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PickerPreviewTarget {
    pub(crate) key: PreviewKey,
    pub(crate) path: PathBuf,
    pub(crate) text: Option<Rope>,
    pub(crate) focus_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewKey {
    kind: PickerKind,
    path: PathBuf,
    focus_line: usize,
    buffer: Option<usize>,
}

impl PickerPreviewTarget {
    fn file(kind: PickerKind, path: PathBuf, focus_line: usize) -> Self {
        Self {
            key: PreviewKey {
                kind,
                path: path.clone(),
                focus_line,
                buffer: None,
            },
            path,
            text: None,
            focus_line,
        }
    }

    fn buffer(kind: PickerKind, path: PathBuf, number: usize, text: Rope) -> Self {
        Self {
            key: PreviewKey {
                kind,
                path: path.clone(),
                focus_line: 0,
                buffer: Some(number),
            },
            path,
            text: Some(text),
            focus_line: 0,
        }
    }
}

pub(crate) fn picker_preview_target(
    session: &PickerSession,
    editor: &Editor,
) -> Option<PickerPreviewTarget> {
    let picker = session.picker();
    let row = picker.rows().get(picker.selected()?)?;
    let kind = picker.kind();
    if kind == PickerKind::Buffers {
        return buffer_target(kind, row.buffer?, editor);
    }
    if !has_file_preview(kind) {
        return None;
    }
    let path = session.root().join(&row.path);
    let focus_line = row.line.unwrap_or(0).saturating_sub(1);
    Some(PickerPreviewTarget::file(kind, path, focus_line))
}

fn buffer_target(kind: PickerKind, number: usize, editor: &Editor) -> Option<PickerPreviewTarget> {
    let path = PathBuf::from(editor.buffer_path(number)?);
    let text = editor.buffer_snapshot(number)?;
    Some(PickerPreviewTarget::buffer(kind, path, number, text))
}

pub(crate) fn has_file_preview(kind: PickerKind) -> bool {
    matches!(
        kind,
        PickerKind::Files
            | PickerKind::Grep
            | PickerKind::Buffers
            | PickerKind::Locations
            | PickerKind::Symbols
            | PickerKind::WorkspaceSymbols
            | PickerKind::GitCommitFiles
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;
    use kawi_picker::PickerRow;

    fn row(path: &str, line: Option<usize>, buffer: Option<usize>) -> PickerRow {
        PickerRow {
            display: path.to_string(),
            path: path.to_string(),
            line,
            column: None,
            buffer,
            secondary: None,
            deprecated: false,
            id: None,
        }
    }

    #[test]
    fn resolves_a_file_target_relative_to_the_picker_root() {
        let root = std::env::temp_dir().join("kawi_preview_target");
        let session = PickerSession::locations(
            root.clone(),
            "Locations",
            vec![row("src/main.rs", Some(12), None)],
        );
        let editor = Editor::new(Buffer::from_string(""));
        let target = picker_preview_target(&session, &editor).expect("target");
        assert_eq!(target.path, root.join("src/main.rs"));
        assert_eq!(target.focus_line, 11);
        assert!(target.text.is_none());
    }

    #[test]
    fn resolves_a_buffer_target_from_the_editor_snapshot() {
        let root = std::env::temp_dir().join("kawi_preview_buffer");
        let path = root.join("main.rs");
        let text = "one\ntwo\n";
        let mut buffer = Buffer::from_string(text);
        buffer.path = Some(path.to_string_lossy().into_owned());
        let editor = Editor::new(buffer);
        let session = PickerSession::buffers(root, vec![(1, "main.rs".to_string())]);
        let target = picker_preview_target(&session, &editor).expect("target");
        assert_eq!(target.path, path);
        assert_eq!(
            target.text.as_ref().map(|text| text.to_string()),
            Some(text.to_string())
        );
    }

    #[test]
    fn skips_pickers_without_a_selected_row() {
        let root = std::env::temp_dir().join("kawi_preview_empty");
        let session = PickerSession::locations(root, "Locations", Vec::new());
        let editor = Editor::new(Buffer::from_string(""));
        assert!(picker_preview_target(&session, &editor).is_none());
    }
}
