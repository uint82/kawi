use crate::highlight::{Document, Runtime};
use crate::language::LanguageId;
use crate::span::SyntaxSpan;

use super::super::support::request_for;

pub(super) fn runtime_for(language: LanguageId) -> Runtime {
    Runtime::new(language).expect("runtime")
}

pub(super) fn update(
    document: &mut Document,
    runtime: &Runtime,
    language: LanguageId,
    text: &str,
    revision: u64,
) -> Vec<SyntaxSpan> {
    let end = text.lines().count();
    document
        .update(&request_for(language, text, 0, end, revision), runtime)
        .1
}
