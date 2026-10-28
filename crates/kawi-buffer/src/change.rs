#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub byte_column: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BufferChangeOrigin {
    #[default]
    User,
    Undo,
    Plugin(String),
    ExternalReload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferChange {
    pub start: TextPosition,
    pub end: TextPosition,
    pub text: String,
    pub origin: BufferChangeOrigin,
}
