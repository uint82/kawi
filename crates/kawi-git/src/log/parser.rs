use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogOptions {
    pub path: Option<PathBuf>,
    pub grep: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub oid: String,
    pub short_oid: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub decorations: String,
}

impl LogEntry {
    pub fn display(&self) -> String {
        let mut text = format!("{} {}", self.short_oid, self.subject);
        if !self.decorations.is_empty() {
            text.push_str(" (");
            text.push_str(&self.decorations);
            text.push(')');
        }
        text.push_str(" - ");
        text.push_str(&self.author);
        text.push_str(", ");
        text.push_str(short_date(&self.date));
        text
    }
}

fn short_date(date: &str) -> &str {
    date.split('T').next().unwrap_or(date)
}
