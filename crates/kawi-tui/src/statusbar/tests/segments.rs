use super::super::build_segments;
use super::super::layout::layout;
use super::super::segment::{ModeKind, SegmentId, StatusBarConfig};
use super::super::{GitStatusInfo, StatusInfo};
use super::info;

fn line_texts(status: &StatusInfo<'_>, width: usize, config: &StatusBarConfig) -> Vec<String> {
    let segments = build_segments(status, true, config);
    let line = layout(&segments, width, &config.separator);
    line.spans
        .into_iter()
        .filter(|span| span.id.is_some())
        .map(|span| span.text)
        .collect()
}

#[test]
fn default_layout_contains_the_documented_elements() {
    let mut status = info("src/main.rs");
    status.dirty = true;
    status.diagnostics = Some("E1 W2");
    status.diagnostics_severity = Some(super::super::DiagnosticSeverity::Error);
    status.git = Some(GitStatusInfo {
        branch: "main",
        operation: None,
        ahead: 1,
        behind: 2,
    });
    status.lsp = Some("lsp: rust-analyzer");
    status.filetype = Some("rust");
    status.indent = Some("4 spaces");
    status.percent = Some(42);
    let texts = line_texts(&status, 120, &StatusBarConfig::default());
    assert_eq!(
        texts,
        vec![
            "-- NORMAL --",
            "src/main.rs",
            "[+]",
            "main +1 -2",
            "rust",
            "1:1",
        ]
    );
}

#[test]
fn visual_and_recording_states_add_transient_elements() {
    let mut status = info("src/main.rs");
    status.mode = "VISUAL";
    status.mode_kind = ModeKind::Visual;
    status.selection = Some("3 lines");
    status.macro_recording = Some('q');
    let config = StatusBarConfig {
        right: vec![
            SegmentId::Selection,
            SegmentId::Macro,
            SegmentId::FileType,
            SegmentId::Encoding,
            SegmentId::Position,
        ],
        ..StatusBarConfig::default()
    };
    let texts = line_texts(&status, 120, &config);
    assert!(texts.contains(&"-- VISUAL --".to_string()));
    assert!(texts.contains(&"3 lines".to_string()));
    assert!(texts.contains(&"recording @q".to_string()));
}

#[test]
fn readonly_files_show_the_readonly_indicator() {
    let mut status = info("src/main.rs");
    status.readonly = true;
    let texts = line_texts(&status, 120, &StatusBarConfig::default());
    assert!(texts.contains(&"[readonly]".to_string()));
}

#[test]
fn inactive_windows_show_only_the_file_and_modified_state() {
    let mut status = info("src/main.rs");
    status.dirty = true;
    status.readonly = true;
    status.mode = "INSERT";
    status.mode_kind = ModeKind::Insert;
    status.diagnostics = Some("E1");
    status.diagnostics_severity = Some(super::super::DiagnosticSeverity::Error);
    let segments = build_segments(&status, false, &StatusBarConfig::default());
    let ids: Vec<SegmentId> = segments.iter().map(|segment| segment.id).collect();
    assert_eq!(ids, vec![SegmentId::File, SegmentId::Modified]);
}

#[test]
fn configured_mode_labels_replace_the_keymap_label() {
    let mut config = StatusBarConfig::default();
    config.mode_labels.insert(ModeKind::Normal, "N".to_string());
    let texts = line_texts(&info("a.txt"), 80, &config);
    assert_eq!(texts.first().map(String::as_str), Some("N"));
}

#[test]
fn disabling_colors_keeps_labels_without_badges() {
    let config = StatusBarConfig {
        color_modes: false,
        ..StatusBarConfig::default()
    };
    let mut status = info("a.txt");
    status.dirty = true;
    status.readonly = true;
    status.diagnostics = Some("E1");
    status.diagnostics_severity = Some(super::super::DiagnosticSeverity::Error);
    status.git = Some(GitStatusInfo {
        branch: "main",
        operation: None,
        ahead: 0,
        behind: 0,
    });
    status.selection = Some("1 char");
    status.macro_recording = Some('q');
    let segments = build_segments(&status, true, &config);
    assert!(segments
        .iter()
        .all(|segment| segment.style == super::super::SegmentStyle::Default));
    let mode = segments
        .iter()
        .find(|segment| segment.id == SegmentId::Mode)
        .expect("mode");
    assert_eq!(mode.text, "-- NORMAL --");
    assert_eq!(mode.padding, 0);
}

#[test]
fn element_lists_control_order_and_visibility() {
    let config = StatusBarConfig {
        left: vec![SegmentId::File, SegmentId::Mode],
        right: vec![SegmentId::Position],
        ..StatusBarConfig::default()
    };
    let texts = line_texts(&info("a.txt"), 80, &config);
    assert_eq!(texts, vec!["a.txt", "-- NORMAL --", "1:1"]);
}

#[test]
fn workspace_diagnostics_are_a_separate_element() {
    let mut status = info("a.txt");
    status.diagnostics = Some("E1");
    status.diagnostics_severity = Some(super::super::DiagnosticSeverity::Error);
    status.workspace_diagnostics = Some("workspace E2");
    status.workspace_diagnostics_severity = Some(super::super::DiagnosticSeverity::Error);
    let mut config = StatusBarConfig::default();
    config.left.push(SegmentId::WorkspaceDiagnostics);
    let texts = line_texts(&status, 120, &config);
    assert!(texts.contains(&"workspace E2".to_string()));
}

#[test]
fn insert_mode_shows_the_insert_badge() {
    let mut status = info("src/main.rs");
    status.mode = "INSERT";
    status.mode_kind = ModeKind::Insert;
    let texts = line_texts(&status, 80, &StatusBarConfig::default());
    assert_eq!(texts.first().map(String::as_str), Some("-- INSERT --"));
}

#[test]
fn plugin_segments_render_in_the_plugins_element() {
    let plugins = [
        super::super::PluginSegment {
            text: "index".to_string(),
            style: crate::frame::DecorationStyle::Info,
        },
        super::super::PluginSegment {
            text: "lint".to_string(),
            style: crate::frame::DecorationStyle::Warning,
        },
    ];
    let mut status = info("a.txt");
    status.plugins = &plugins;
    let texts = line_texts(&status, 120, &StatusBarConfig::default());
    assert!(texts.contains(&"index".to_string()));
    assert!(texts.contains(&"lint".to_string()));
}

#[test]
fn plugin_segments_hide_in_inactive_windows() {
    let plugins = [super::super::PluginSegment {
        text: "index".to_string(),
        style: crate::frame::DecorationStyle::Info,
    }];
    let mut status = info("a.txt");
    status.plugins = &plugins;
    let segments = build_segments(&status, false, &StatusBarConfig::default());
    assert!(!segments
        .iter()
        .any(|segment| segment.id == SegmentId::Plugins));
}

#[test]
fn plugin_segments_follow_the_configured_element_order() {
    let plugins = [super::super::PluginSegment {
        text: "index".to_string(),
        style: crate::frame::DecorationStyle::Info,
    }];
    let mut status = info("a.txt");
    status.plugins = &plugins;
    let config = StatusBarConfig {
        left: vec![SegmentId::File, SegmentId::Plugins, SegmentId::Mode],
        right: vec![SegmentId::Position],
        ..StatusBarConfig::default()
    };
    let texts = line_texts(&status, 80, &config);
    assert_eq!(texts, vec!["a.txt", "index", "-- NORMAL --", "1:1"]);
}
