pub fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_the_matching_form() {
        assert_eq!(count_label(1, "match", "matches"), "1 match");
        assert_eq!(count_label(2, "file", "files"), "2 files");
        assert_eq!(count_label(0, "file", "files"), "0 files");
    }
}
