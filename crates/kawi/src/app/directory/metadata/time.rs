use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const MODIFIED_WIDTH: usize = 12;
pub(super) const MISSING_MODIFIED: &str = "????????????";

#[cfg(unix)]
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
#[cfg(unix)]
const UNKNOWN_MONTH: &str = "???";
#[cfg(unix)]
const TM_YEAR_BASE: i32 = 1900;
#[cfg(unix)]
const SIX_MONTHS_SECONDS: i64 = 15_778_476;
#[cfg(unix)]
const FUTURE_TOLERANCE_SECONDS: i64 = 3_600;

pub(super) fn format_modified(time: SystemTime, now: SystemTime) -> String {
    modified_text(seconds_since_epoch(time), seconds_since_epoch(now))
}

fn seconds_since_epoch(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(error) => -i64::try_from(error.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

#[cfg(unix)]
fn modified_text(seconds: i64, now: i64) -> String {
    let Some(time) = local_time(seconds) else {
        return MISSING_MODIFIED.to_string();
    };
    let month = month_name(time.tm_mon);
    let day = time.tm_mday;
    if is_recent(seconds, now) {
        format!("{month} {day:>2} {:02}:{:02}", time.tm_hour, time.tm_min)
    } else {
        let year = time.tm_year + TM_YEAR_BASE;
        format!("{month} {day:>2}  {year:04}")
    }
}

#[cfg(unix)]
fn local_time(seconds: i64) -> Option<libc::tm> {
    let seconds = seconds as libc::time_t;
    let mut time = std::mem::MaybeUninit::<libc::tm>::uninit();
    let result = unsafe { libc::localtime_r(&seconds, time.as_mut_ptr()) };
    if result.is_null() {
        None
    } else {
        Some(unsafe { time.assume_init() })
    }
}

#[cfg(unix)]
fn is_recent(seconds: i64, now: i64) -> bool {
    seconds >= now.saturating_sub(SIX_MONTHS_SECONDS)
        && seconds <= now.saturating_add(FUTURE_TOLERANCE_SECONDS)
}

#[cfg(unix)]
fn month_name(month: libc::c_int) -> &'static str {
    usize::try_from(month)
        .ok()
        .and_then(|index| MONTHS.get(index))
        .copied()
        .unwrap_or(UNKNOWN_MONTH)
}

#[cfg(not(unix))]
fn modified_text(_: i64, _: i64) -> String {
    MISSING_MODIFIED.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn recent_timestamps_match_ls() {
        let text = modified_text(0, 0);
        assert_eq!(text.len(), MODIFIED_WIDTH);
        assert_eq!(&text[3..4], " ");
        assert_eq!(&text[9..10], ":");
    }

    #[cfg(unix)]
    #[test]
    fn old_timestamps_match_ls() {
        let text = modified_text(0, SIX_MONTHS_SECONDS + 1);
        assert_eq!(text.len(), MODIFIED_WIDTH);
        assert_eq!(&text[6..8], "  ");
        assert!(!text.contains(':'));
    }
}
