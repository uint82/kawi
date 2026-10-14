pub struct Register {
    text: String,
}

impl Register {
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl Default for Register {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let register = Register::new();
        assert!(register.is_empty());
    }

    #[test]
    fn stores_round_trip() {
        let mut register = Register::new();
        register.set_text("bcd".to_string());
        assert_eq!(register.text(), "bcd");
        assert!(!register.is_empty());
    }
}
