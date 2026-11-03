use super::segment::{
    Section, Segment, SegmentId, SegmentStyle, StatusBarConfig, BADGE_PADDING, REQUIRED_PRIORITY,
};
use super::{GitStatusInfo, PluginSegment, StatusInfo};

const FILE_PRIORITY: u8 = 200;
const SELECTION_PRIORITY: u8 = 190;
const MACRO_PRIORITY: u8 = 185;
const MODIFIED_PRIORITY: u8 = 180;
const READONLY_PRIORITY: u8 = 175;
const DIAGNOSTICS_PRIORITY: u8 = 170;
const WORKSPACE_DIAGNOSTICS_PRIORITY: u8 = 165;
const GIT_PRIORITY: u8 = 160;
const LSP_PRIORITY: u8 = 150;
const DEBUG_PRIORITY: u8 = 148;
const PLUGIN_PRIORITY: u8 = 155;
const PERCENT_PRIORITY: u8 = 140;
const FILETYPE_PRIORITY: u8 = 130;
const INDENT_PRIORITY: u8 = 120;
const ENCODING_PRIORITY: u8 = 110;
const EOL_PRIORITY: u8 = 100;
const FILE_MIN_WIDTH: usize = 12;

struct SegmentSpec {
    style: SegmentStyle,
    priority: u8,
    flex: bool,
    min_width: usize,
}

pub fn build_segments(
    info: &StatusInfo<'_>,
    active: bool,
    config: &StatusBarConfig,
) -> Vec<Segment> {
    let mut segments = Vec::new();
    push_section(
        &mut segments,
        info,
        active,
        config,
        Section::Left,
        &config.left,
    );
    push_section(
        &mut segments,
        info,
        active,
        config,
        Section::Center,
        &config.center,
    );
    push_section(
        &mut segments,
        info,
        active,
        config,
        Section::Right,
        &config.right,
    );
    segments
}

fn push_section(
    segments: &mut Vec<Segment>,
    info: &StatusInfo<'_>,
    active: bool,
    config: &StatusBarConfig,
    section: Section,
    ids: &[SegmentId],
) {
    for id in ids {
        if *id == SegmentId::Plugins {
            push_plugin_segments(segments, info.plugins, active, section);
            continue;
        }
        if !active && !matches!(id, SegmentId::File | SegmentId::Modified) {
            continue;
        }
        let Some(text) = segment_text(info, *id, active, config) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let spec = segment_spec(*id, info, config);
        let padding = if spec.style.is_badge() {
            BADGE_PADDING
        } else {
            0
        };
        segments.push(Segment {
            id: *id,
            text,
            style: spec.style,
            priority: spec.priority,
            section,
            flex: spec.flex,
            min_width: spec.min_width,
            padding,
        });
    }
}

fn push_plugin_segments(
    segments: &mut Vec<Segment>,
    plugins: &[PluginSegment],
    active: bool,
    section: Section,
) {
    if !active {
        return;
    }
    for plugin in plugins {
        if plugin.text.is_empty() {
            continue;
        }
        segments.push(Segment {
            id: SegmentId::Plugins,
            text: plugin.text.clone(),
            style: SegmentStyle::Plugin(plugin.style),
            priority: PLUGIN_PRIORITY,
            section,
            flex: false,
            min_width: 0,
            padding: BADGE_PADDING,
        });
    }
}

fn segment_text(
    info: &StatusInfo<'_>,
    id: SegmentId,
    active: bool,
    config: &StatusBarConfig,
) -> Option<String> {
    match id {
        SegmentId::Mode => active.then(|| mode_text(info, config)),
        SegmentId::File => Some(file_text(info)),
        SegmentId::Modified => info.dirty.then(|| "[+]".to_string()),
        SegmentId::ReadOnly => info.readonly.then(|| "[readonly]".to_string()),
        SegmentId::Diagnostics => info.diagnostics.map(str::to_string),
        SegmentId::WorkspaceDiagnostics => info.workspace_diagnostics.map(str::to_string),
        SegmentId::Git => info.git.as_ref().map(git_text),
        SegmentId::Lsp => info.lsp.map(str::to_string),
        SegmentId::Debug => info.debug.map(str::to_string),
        SegmentId::Selection => info.selection.map(str::to_string),
        SegmentId::Macro => info
            .macro_recording
            .map(|register| format!("recording @{register}")),
        SegmentId::FileType => info.filetype.map(str::to_string),
        SegmentId::Indent => info.indent.map(str::to_string),
        SegmentId::Encoding => info.encoding.map(str::to_string),
        SegmentId::Eol => info.eol.map(str::to_string),
        SegmentId::Percent => info.percent.map(|percent| format!("{percent}%")),
        SegmentId::Position => active.then(|| position_text(info)),
        SegmentId::Plugins => None,
    }
}

fn mode_text(info: &StatusInfo<'_>, config: &StatusBarConfig) -> String {
    config
        .mode_labels
        .get(&info.mode_kind)
        .cloned()
        .unwrap_or_else(|| info.mode.to_string())
}

fn file_text(info: &StatusInfo<'_>) -> String {
    let mut text = info.filename.to_string();
    if let Some(external) = info.external {
        text.push_str(" [");
        text.push_str(external);
        text.push(']');
    }
    text
}

fn git_text(git: &GitStatusInfo<'_>) -> String {
    let mut text = String::new();
    if let Some(operation) = git.operation {
        text.push_str(operation);
        text.push(' ');
    }
    text.push_str(git.branch);
    if git.ahead > 0 {
        text.push_str(&format!(" +{}", git.ahead));
    }
    if git.behind > 0 {
        text.push_str(&format!(" -{}", git.behind));
    }
    text
}

fn position_text(info: &StatusInfo<'_>) -> String {
    let line = if info.empty_buffer {
        0
    } else {
        info.row.saturating_add(1)
    };
    let col = if info.line_empty {
        0
    } else {
        info.col.saturating_add(1)
    };
    let virtcol = info.virtcol.saturating_add(1);
    if col == virtcol {
        format!("{line}:{col}")
    } else {
        format!("{line}:{col}-{virtcol}")
    }
}

fn segment_spec(id: SegmentId, info: &StatusInfo<'_>, config: &StatusBarConfig) -> SegmentSpec {
    let style = segment_style(id, info, config);
    let (priority, flex, min_width) = match id {
        SegmentId::Mode | SegmentId::Position => (REQUIRED_PRIORITY, false, 0),
        SegmentId::File => (FILE_PRIORITY, true, FILE_MIN_WIDTH),
        SegmentId::Selection => (SELECTION_PRIORITY, false, 0),
        SegmentId::Macro => (MACRO_PRIORITY, false, 0),
        SegmentId::Modified => (MODIFIED_PRIORITY, false, 0),
        SegmentId::ReadOnly => (READONLY_PRIORITY, false, 0),
        SegmentId::Diagnostics => (DIAGNOSTICS_PRIORITY, false, 0),
        SegmentId::WorkspaceDiagnostics => (WORKSPACE_DIAGNOSTICS_PRIORITY, false, 0),
        SegmentId::Git => (GIT_PRIORITY, false, 0),
        SegmentId::Lsp => (LSP_PRIORITY, false, 0),
        SegmentId::Debug => (DEBUG_PRIORITY, false, 0),
        SegmentId::Percent => (PERCENT_PRIORITY, false, 0),
        SegmentId::FileType => (FILETYPE_PRIORITY, false, 0),
        SegmentId::Indent => (INDENT_PRIORITY, false, 0),
        SegmentId::Encoding => (ENCODING_PRIORITY, false, 0),
        SegmentId::Eol => (EOL_PRIORITY, false, 0),
        SegmentId::Plugins => (PLUGIN_PRIORITY, false, 0),
    };
    SegmentSpec {
        style,
        priority,
        flex,
        min_width,
    }
}

fn segment_style(id: SegmentId, info: &StatusInfo<'_>, config: &StatusBarConfig) -> SegmentStyle {
    if !config.color_modes {
        return SegmentStyle::Default;
    }
    match id {
        SegmentId::Mode => SegmentStyle::Mode(info.mode_kind),
        SegmentId::Modified => SegmentStyle::Modified,
        SegmentId::ReadOnly => SegmentStyle::ReadOnly,
        SegmentId::Diagnostics => info
            .diagnostics_severity
            .map_or(SegmentStyle::Default, SegmentStyle::Diagnostic),
        SegmentId::WorkspaceDiagnostics => info
            .workspace_diagnostics_severity
            .map_or(SegmentStyle::Default, SegmentStyle::Diagnostic),
        SegmentId::Git => SegmentStyle::Git,
        SegmentId::Lsp => SegmentStyle::Lsp,
        SegmentId::Debug => SegmentStyle::Debug,
        SegmentId::Selection => SegmentStyle::Selection,
        SegmentId::Macro => SegmentStyle::Macro,
        SegmentId::FileType => SegmentStyle::FileType,
        SegmentId::Indent => SegmentStyle::Indent,
        SegmentId::Encoding => SegmentStyle::Encoding,
        SegmentId::Eol => SegmentStyle::Eol,
        SegmentId::Percent => SegmentStyle::Percent,
        SegmentId::Position => SegmentStyle::Position,
        SegmentId::File | SegmentId::Plugins => SegmentStyle::Default,
    }
}
