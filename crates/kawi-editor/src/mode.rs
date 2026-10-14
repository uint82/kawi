#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
}

impl Mode {
    pub fn is_insert(self) -> bool {
        matches!(self, Self::Insert)
    }

    pub fn is_visual(self) -> bool {
        matches!(self, Self::Visual)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Command => "COMMAND",
            Self::Visual => "VISUAL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_detects_insert() {
        assert!(Mode::Insert.is_insert());
    }

    #[test]
    fn visual_detects_visual() {
        assert!(Mode::Visual.is_visual());
        assert!(!Mode::Normal.is_visual());
    }

    #[test]
    fn normal_labels_normal() {
        assert_eq!(Mode::Normal.label(), "NORMAL");
    }

    #[test]
    fn insert_labels_insert() {
        assert_eq!(Mode::Insert.label(), "INSERT");
    }

    #[test]
    fn visual_labels_visual() {
        assert_eq!(Mode::Visual.label(), "VISUAL");
    }
}
