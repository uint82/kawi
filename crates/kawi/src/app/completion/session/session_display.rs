use kawi_completion::Item;

pub(super) fn list_width(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| display_item(item).chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2)
}

pub(super) fn display_item(item: &Item) -> String {
    let mut display = item.label.clone();
    if let Some(detail) = item.detail.as_deref().filter(|detail| !detail.is_empty()) {
        display.push_str("  ");
        display.push_str(detail);
    }
    if item.snippet {
        display.push_str(" [snippet]");
    }
    if item.deprecated {
        display.push_str(" [deprecated]");
    }
    display
}
