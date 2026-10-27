use super::super::layout::layout;
use super::super::segment::{Section, Segment, SegmentId, SegmentStyle};
use super::super::text;
use super::super::{build_segments, StatusBarConfig};
use super::{ids, info, texts};

const POOL: &[&str] = &[
    "a",
    "plain",
    "\u{4e2d}\u{6587}",
    "\u{1f642}",
    "e\u{301}",
    "\t",
    "src/chapters/17.markdown",
    "\u{1f980}",
    "x",
];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as usize
    }
}

fn random_segments(rng: &mut Rng) -> Vec<Segment> {
    let count = 1 + rng.next() % 12;
    (0..count)
        .map(|_| {
            let text = POOL[rng.next() % POOL.len()].repeat(1 + rng.next() % 3);
            let id = SegmentId::ALL[rng.next() % SegmentId::ALL.len()];
            let section = [Section::Left, Section::Center, Section::Right][rng.next() % 3];
            Segment {
                id,
                text,
                style: SegmentStyle::Default,
                priority: (rng.next() % 250) as u8,
                section,
                flex: id == SegmentId::File,
                min_width: 4,
                padding: 0,
            }
        })
        .collect()
}

fn assert_inside(line: &super::super::StatusLine, width: usize) {
    assert_eq!(line.width, width);
    let mut previous_end = 0;
    for span in &line.spans {
        assert!(span.x >= previous_end, "spans overlap");
        assert!(
            span.x.saturating_add(span.width) <= width,
            "span escapes the bar"
        );
        assert_eq!(
            text::display_width(&span.text).saturating_add(span.padding.saturating_mul(2)),
            span.width
        );
        assert!(!span.text.chars().any(char::is_control));
        assert!(!span.text.contains('\u{1b}'));
        previous_end = span.x + span.width;
    }
}

#[test]
fn rendered_width_never_exceeds_the_bar() {
    let mut rng = Rng(7);
    for width in 0..=200 {
        let segments = random_segments(&mut rng);
        let line = layout(&segments, width, " ");
        assert_inside(&line, width);
    }
}

#[test]
fn position_and_mode_survive_at_their_minimum() {
    let config = StatusBarConfig::default();
    let status = info("a.txt");
    for width in 5..=200 {
        let segments = build_segments(&status, true, &config);
        let line = layout(&segments, width, &config.separator);
        let ids = ids(&line);
        assert!(ids.contains(&SegmentId::Mode), "mode missing at {width}");
        assert!(
            ids.contains(&SegmentId::Position),
            "position missing at {width}"
        );
        assert_inside(&line, width);
    }
}

#[test]
fn drops_follow_priority_and_prefer_the_right_section() {
    let segments = vec![
        segment(SegmentId::Mode, "M", 255, Section::Left),
        segment(SegmentId::File, "file.rs", 200, Section::Left),
        segment(SegmentId::Diagnostics, "E1", 170, Section::Left),
        segment(SegmentId::FileType, "rust", 130, Section::Right),
        segment(SegmentId::Position, "1:1", 255, Section::Right),
    ];
    let line = layout(&segments, 16, " ");
    let line_ids = ids(&line);
    assert!(!line_ids.contains(&SegmentId::FileType));
    assert!(line_ids.contains(&SegmentId::Diagnostics));
    assert!(line_ids.contains(&SegmentId::Mode));
    assert!(line_ids.contains(&SegmentId::Position));
}

#[test]
fn right_section_drops_first_on_equal_priority() {
    let segments = vec![
        segment(SegmentId::Mode, "M", 255, Section::Left),
        segment(SegmentId::Indent, "4 spaces", 100, Section::Left),
        segment(SegmentId::FileType, "rust", 100, Section::Right),
        segment(SegmentId::Position, "1:1", 255, Section::Right),
    ];
    let line = layout(&segments, 14, " ");
    let line_ids = ids(&line);
    assert!(line_ids.contains(&SegmentId::Indent));
    assert!(!line_ids.contains(&SegmentId::FileType));
}

#[test]
fn truncates_the_file_only_after_dropping_optional_segments() {
    let segments = vec![
        segment(SegmentId::Mode, "NORMAL", 255, Section::Left),
        segment(
            SegmentId::File,
            "src/chapters/17.markdown",
            200,
            Section::Left,
        ),
        segment(SegmentId::Diagnostics, "E1 W2", 170, Section::Left),
        segment(SegmentId::Position, "1:1", 255, Section::Right),
    ];
    let line = layout(&segments, 24, " ");
    let line_ids = ids(&line);
    assert!(!line_ids.contains(&SegmentId::Diagnostics));
    let file = line
        .spans
        .iter()
        .find(|span| span.id == Some(SegmentId::File))
        .expect("file");
    assert!(file.text.starts_with(text::ELLIPSIS));
    assert!(file.text.ends_with(".markdown"));
}

#[test]
fn drops_the_file_before_dropping_position() {
    let segments = vec![
        segment(SegmentId::Mode, "NORMAL", 255, Section::Left),
        segment(
            SegmentId::File,
            "src/chapters/17.markdown",
            200,
            Section::Left,
        ),
        segment(SegmentId::Position, "1:1", 255, Section::Right),
    ];
    let line = layout(&segments, 5, " ");
    let line_ids = ids(&line);
    assert!(line_ids.contains(&SegmentId::Mode));
    assert!(line_ids.contains(&SegmentId::Position));
    assert!(!line_ids.contains(&SegmentId::File));
    let narrow = layout(&segments, 3, " ");
    assert_eq!(ids(&narrow), vec![SegmentId::Position]);
}

#[test]
fn sections_join_with_exactly_one_separator() {
    let segments = vec![
        segment(SegmentId::Mode, "NORMAL", 255, Section::Left),
        segment(SegmentId::File, "a.txt", 200, Section::Left),
        segment(SegmentId::Position, "1:1", 255, Section::Left),
    ];
    let line = layout(&segments, 40, " | ");
    let separators: Vec<&str> = line
        .spans
        .iter()
        .filter(|span| span.id.is_none())
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(separators, vec![" | ", " | "]);
}

#[test]
fn sanitizes_control_characters_and_escape_sequences() {
    let segments = vec![
        segment(
            SegmentId::File,
            "bad\u{1b}[31mname\t.txt",
            200,
            Section::Left,
        ),
        segment(
            SegmentId::Diagnostics,
            "E1\u{1b}]0;title\u{7}",
            170,
            Section::Left,
        ),
    ];
    let line = layout(&segments, 40, " ");
    assert_eq!(texts(&line), vec!["badname .txt", "E1"]);
}

#[test]
fn clips_without_splitting_wide_characters() {
    let segments = vec![segment(
        SegmentId::Position,
        "\u{4e2d}\u{4e2d}",
        255,
        Section::Left,
    )];
    let line = layout(&segments, 3, "");
    assert_inside(&line, 3);
    assert_eq!(texts(&line), vec!["\u{4e2d} "]);
}

fn segment(id: SegmentId, text: &str, priority: u8, section: Section) -> Segment {
    Segment {
        id,
        text: text.to_string(),
        style: SegmentStyle::Default,
        priority,
        section,
        flex: id == SegmentId::File,
        min_width: 4,
        padding: 0,
    }
}

#[test]
fn long_file_names_keep_the_basename_and_extension() {
    let segments = vec![segment(
        SegmentId::File,
        "src/chapters/17.markdown",
        200,
        Section::Left,
    )];
    let line = layout(&segments, 12, "");
    assert_eq!(texts(&line), vec!["\u{2026}17.markdown"]);
}

#[test]
fn layout_is_independent_of_theme() {
    let segments = vec![segment(SegmentId::Mode, "NORMAL", 255, Section::Left)];
    let first = layout(&segments, 40, " ");
    let second = layout(&segments, 40, " ");
    assert_eq!(first, second);
}

#[test]
fn badges_get_horizontal_padding_when_space_allows() {
    let config = StatusBarConfig {
        color_modes: true,
        ..StatusBarConfig::default()
    };
    let mut status = info("a.txt");
    status.git = Some(super::super::GitStatusInfo {
        branch: "main",
        operation: None,
        ahead: 0,
        behind: 0,
    });
    let segments = build_segments(&status, true, &config);
    let line = layout(&segments, 80, &config.separator);
    for id in [SegmentId::Mode, SegmentId::Git] {
        let span = line
            .spans
            .iter()
            .find(|span| span.id == Some(id))
            .expect("badge");
        assert_eq!(span.padding, 1);
        assert_eq!(span.width, text::display_width(&span.text) + 2);
    }
    let file = line
        .spans
        .iter()
        .find(|span| span.id == Some(SegmentId::File))
        .expect("file");
    assert_eq!(file.padding, 0);
}

#[test]
fn padding_is_removed_before_badge_text_is_shortened() {
    let config = StatusBarConfig {
        color_modes: true,
        ..StatusBarConfig::default()
    };
    let segments = build_segments(&info("a.txt"), true, &config);
    let line = layout(&segments, 10, &config.separator);
    let mode = line
        .spans
        .iter()
        .find(|span| span.id == Some(SegmentId::Mode))
        .expect("mode");
    assert_eq!(mode.padding, 0);
    assert_eq!(mode.text, "-");
}
