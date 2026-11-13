use ropey::{Rope, RopeSlice};
use tree_sitter::{Parser, Point, Tree};

pub(crate) fn parse_rope(parser: &mut Parser, text: &Rope, old: Option<&Tree>) -> Option<Tree> {
    parse_slice(parser, text.byte_slice(..), old)
}

pub(crate) fn parse_slice(
    parser: &mut Parser,
    text: RopeSlice<'_>,
    old: Option<&Tree>,
) -> Option<Tree> {
    let mut read = |offset: usize, _: Point| -> &str {
        if offset >= text.len_bytes() {
            ""
        } else {
            text.byte_slice(offset..).chunks().next().unwrap_or("")
        }
    };
    parser.parse_with_options(&mut read, old, None)
}
