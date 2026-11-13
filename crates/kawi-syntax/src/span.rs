use std::ops::Range;

use ropey::Rope;

use crate::language::LanguageId;
use crate::registry::SyntaxHighlight;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxKey {
    pub document: u64,
    pub revision: u64,
    pub top: usize,
    pub end: usize,
    pub language: LanguageId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub highlight: SyntaxHighlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnippetSpan {
    pub start: usize,
    pub end: usize,
    pub highlight: SyntaxHighlight,
}

pub struct SyntaxRequest {
    pub key: SyntaxKey,
    pub text: Rope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxResult {
    pub key: SyntaxKey,
    pub generation: u64,
    pub window: Range<usize>,
    pub spans: Vec<SyntaxSpan>,
}
