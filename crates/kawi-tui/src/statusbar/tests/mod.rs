use super::*;

mod layout;
mod segments;
mod text;

fn info<'a>(filename: &'a str) -> StatusInfo<'a> {
    StatusInfo {
        filename,
        dirty: false,
        readonly: false,
        row: 0,
        col: 0,
        virtcol: 0,
        line_empty: false,
        empty_buffer: false,
        mode: "NORMAL",
        mode_kind: ModeKind::Normal,
        lsp: None,
        diagnostics: None,
        diagnostics_severity: None,
        workspace_diagnostics: None,
        workspace_diagnostics_severity: None,
        external: None,
        git: None,
        filetype: None,
        indent: None,
        encoding: None,
        eol: None,
        selection: None,
        macro_recording: None,
        percent: None,
        plugins: &[],
    }
}

fn texts(line: &StatusLine) -> Vec<&str> {
    line.spans
        .iter()
        .filter(|span| span.id.is_some())
        .map(|span| span.text.as_str())
        .collect()
}

fn ids(line: &StatusLine) -> Vec<SegmentId> {
    line.spans.iter().filter_map(|span| span.id).collect()
}
