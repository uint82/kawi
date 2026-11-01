use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const MODIFIED_WIDTH: usize = 16;
pub(super) const MISSING_MODIFIED: &str = "????????????????";

pub(super) fn format_modified(time: SystemTime) -> String {
    modified_text(seconds_since_epoch(time))
}

fn seconds_since_epoch(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(error) => -i64::try_from(error.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

#[cfg(unix)]
fn modified_text(seconds: i64) -> String {
    let seconds = seconds as libc::time_t;
    let mut time = std::mem::MaybeUninit::<libc::tm>::uninit();
    let result = unsafe { libc::localtime_r(&seconds, time.as_mut_ptr()) };
    if result.is_null() {
        return MISSING_MODIFIED.to_string();
    }
    let time = unsafe { time.assume_init() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        time.tm_year + 1900,
        time.tm_mon + 1,
        time.tm_mday,
        time.tm_hour,
        time.tm_min
    )
}

#[cfg(not(unix))]
fn modified_text(_: i64) -> String {
    MISSING_MODIFIED.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn modified_timestamps_have_a_stable_width() {
        let text = modified_text(0);
        assert_eq!(text.len(), MODIFIED_WIDTH);
        assert_eq!(&text[4..5], "-");
        assert_eq!(&text[7..8], "-");
        assert_eq!(&text[10..11], " ");
        assert_eq!(&text[13..14], ":");
    }
}
