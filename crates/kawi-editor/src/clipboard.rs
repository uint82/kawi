#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardKind {
    Clipboard,
    Primary,
}

pub trait Clipboard {
    fn read(&mut self, kind: ClipboardKind) -> Option<String>;
    fn write(&mut self, kind: ClipboardKind, text: &str);
}
