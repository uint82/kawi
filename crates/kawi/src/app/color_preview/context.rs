mod markup;
#[cfg(test)]
mod tests;
mod util;
mod value;

use std::ops::Range;

use super::mode::PreviewMode;

pub(crate) use markup::MarkupState;

pub(super) fn preview_ranges(
    line: &str,
    mode: PreviewMode,
    state: &mut MarkupState,
) -> Vec<Range<usize>> {
    match mode {
        PreviewMode::Stylesheet => value::ranges(line, false),
        PreviewMode::Markup => markup::ranges(line, state),
        PreviewMode::Plain => std::iter::once(0..line.len()).collect(),
    }
}
