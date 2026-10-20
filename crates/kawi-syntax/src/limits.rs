pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_LINE_BYTES: usize = 8 * 1024;
pub const MAX_CAPTURES: usize = 20_000;

pub fn file_size_supported(bytes: usize) -> bool {
    bytes <= MAX_FILE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_files_up_to_the_limit() {
        assert!(file_size_supported(0));
        assert!(file_size_supported(MAX_FILE_BYTES));
        assert!(!file_size_supported(MAX_FILE_BYTES.saturating_add(1)));
    }
}
