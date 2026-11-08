use std::ops::Range;

use kawi_completion::Item;
use kawi_picker::{FuzzyMatch, FuzzyMatcher};

pub(crate) struct FilteredItem {
    pub(super) index: usize,
    pub(super) highlights: Vec<Range<usize>>,
}

pub(super) fn filter(items: &[Item], query: &str) -> Vec<FilteredItem> {
    let matcher = FuzzyMatcher::new(query);
    let mut matched: Vec<(usize, FuzzyMatch)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matcher
                .matches(item.filter_key())
                .map(|matched| (index, matched))
        })
        .collect();
    matched.sort_by(|(left_index, left), (right_index, right)| {
        right
            .score()
            .cmp(&left.score())
            .then_with(|| {
                items[*left_index]
                    .sort_key()
                    .cmp(items[*right_index].sort_key())
            })
            .then_with(|| left_index.cmp(right_index))
    });
    matched
        .into_iter()
        .map(|(index, matched)| FilteredItem {
            index,
            highlights: highlights(&matcher, &items[index], &matched),
        })
        .collect()
}

fn highlights(matcher: &FuzzyMatcher, item: &Item, matched: &FuzzyMatch) -> Vec<Range<usize>> {
    if item.filter_key() == item.label.as_str() {
        return matched.ranges().to_vec();
    }
    matcher
        .matches(&item.label)
        .map(|matched| matched.ranges().to_vec())
        .unwrap_or_default()
}
