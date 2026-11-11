use kawi_buffer::Buffer;
use kawi_syntax::LanguageId;

use super::*;
use crate::editing::{
    default_match_pairs, default_pairs, BlockComment, CommentTokens, LanguageEditing, Pair,
};

const ALL_PAIRS: [Pair; 4] = [
    Pair::new('(', ')'),
    Pair::new('[', ']'),
    Pair::new('{', '}'),
    Pair::new('<', '>'),
];

fn editing(line_comment: &str, block_comment: (&str, &str)) -> LanguageEditing {
    LanguageEditing {
        comments: CommentTokens {
            line: Some(line_comment.to_string()),
            block: Some(BlockComment::new(block_comment.0, block_comment.1)),
        },
        pairs: default_pairs(),
        match_pairs: default_match_pairs(),
        ..LanguageEditing::default()
    }
}

fn rust_editing() -> LanguageEditing {
    editing("//", ("/*", "*/"))
}

fn python_editing() -> LanguageEditing {
    editing("#", ("\"\"\"", "\"\"\""))
}

fn bash_editing() -> LanguageEditing {
    editing("#", ("", ""))
}

fn find(
    buffer: &Buffer,
    editing: &LanguageEditing,
    language: Option<LanguageId>,
    pairs: &[Pair],
    position: Position,
    bound: usize,
) -> Option<Position> {
    find_match(&MatchRequest {
        buffer,
        position,
        language,
        editing,
        pairs,
        bound,
    })
}

fn find_result(
    buffer: &Buffer,
    editing: &LanguageEditing,
    language: Option<LanguageId>,
    pairs: &[Pair],
    position: Position,
    bound: usize,
) -> Option<MatchResult> {
    find_match_result(&MatchRequest {
        buffer,
        position,
        language,
        editing,
        pairs,
        bound,
    })
}

fn position_of(text: &str, target: char, occurrence: usize) -> Position {
    let mut row = 0usize;
    let mut col = 0usize;
    let mut found = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            row = row.saturating_add(1);
            col = 0;
            continue;
        }
        if ch == target {
            if found == occurrence {
                return (row, col);
            }
            found = found.saturating_add(1);
        }
        col = col.saturating_add(1);
    }
    panic!("character occurrence");
}

mod languages;
mod limits;
mod matching;
