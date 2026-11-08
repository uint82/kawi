use kawi_completion::{CompletionKind, Item};
use kawi_tui::frame::CompletionLine;
use kawi_tui::text::{display_width, sanitize_text};

const DETAIL_GAP: usize = 2;
const KIND_GAP: usize = 2;
const LIST_PADDING: usize = 2;

pub(super) struct DisplayMetrics {
    pub(super) list_width: usize,
    pub(super) kind_width: usize,
}

pub(super) fn metrics(items: &[Item]) -> DisplayMetrics {
    let kind_width = items
        .iter()
        .filter_map(display_kind)
        .map(kind_width)
        .max()
        .unwrap_or(0);
    let primary_width = items.iter().map(primary_width).max().unwrap_or(0);
    DisplayMetrics {
        list_width: primary_width + kind_space(kind_width) + LIST_PADDING,
        kind_width,
    }
}

pub(super) fn display_line(item: &Item) -> CompletionLine {
    CompletionLine {
        label: sanitize_text(&item.label),
        detail: detail(item).map(sanitize_text),
        kind: display_kind(item),
        deprecated: item.deprecated,
    }
}

fn detail(item: &Item) -> Option<&str> {
    item.detail.as_deref().filter(|detail| !detail.is_empty())
}

fn primary_width(item: &Item) -> usize {
    display_width(&item.label) + detail(item).map_or(0, |detail| DETAIL_GAP + display_width(detail))
}

fn display_kind(item: &Item) -> Option<CompletionKind> {
    match item.kind {
        Some(CompletionKind::Text) | None if item.snippet => Some(CompletionKind::Snippet),
        kind => kind,
    }
}

fn kind_width(kind: CompletionKind) -> usize {
    display_width(kind.name())
}

fn kind_space(kind_width: usize) -> usize {
    if kind_width == 0 {
        0
    } else {
        KIND_GAP + kind_width
    }
}
