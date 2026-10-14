#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_labels_normal() {
        assert_eq!(Mode::Normal.label(), "NORMAL");
    }

    #[test]
    fn insert_labels_insert() {
        assert_eq!(Mode::Insert.label(), "INSERT");
    }
}
