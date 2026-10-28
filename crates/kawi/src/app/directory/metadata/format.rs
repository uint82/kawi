use std::fs::Metadata;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::entry::EntryKind;

const PERMISSION_WIDTH: usize = 10;
const SIZE_WIDTH: usize = 7;
const MODIFIED_WIDTH: usize = 16;
const SIZE_UNITS: [&str; 7] = ["B", "K", "M", "G", "T", "P", "E"];
pub(super) const MISSING_MODIFIED: &str = "????????????????";
pub(super) const TEXT_WIDTH: usize = PERMISSION_WIDTH + 1 + SIZE_WIDTH + 1 + MODIFIED_WIDTH;

pub(super) fn format_permissions(metadata: &Metadata, kind: EntryKind) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        format_mode(kind, metadata.permissions().mode())
    }
    #[cfg(not(unix))]
    {
        format!("{}?????????", type_char(kind))
    }
}

#[cfg(unix)]
fn format_mode(kind: EntryKind, mode: u32) -> String {
    let mut text = String::with_capacity(PERMISSION_WIDTH);
    text.push(type_char(kind));
    text.push(permission_char(mode, 0o400, 'r', '-'));
    text.push(permission_char(mode, 0o200, 'w', '-'));
    text.push(special_exec_char(
        mode & 0o4000 != 0,
        mode & 0o100 != 0,
        's',
        'S',
    ));
    text.push(permission_char(mode, 0o040, 'r', '-'));
    text.push(permission_char(mode, 0o020, 'w', '-'));
    text.push(special_exec_char(
        mode & 0o2000 != 0,
        mode & 0o010 != 0,
        's',
        'S',
    ));
    text.push(permission_char(mode, 0o004, 'r', '-'));
    text.push(permission_char(mode, 0o002, 'w', '-'));
    text.push(special_exec_char(
        mode & 0o1000 != 0,
        mode & 0o001 != 0,
        't',
        'T',
    ));
    text
}

fn type_char(kind: EntryKind) -> char {
    match kind {
        EntryKind::File => '-',
        EntryKind::Directory => 'd',
        EntryKind::Symlink => 'l',
    }
}

#[cfg(unix)]
fn permission_char(mode: u32, bit: u32, present: char, absent: char) -> char {
    if mode & bit == bit {
        present
    } else {
        absent
    }
}

#[cfg(unix)]
fn special_exec_char(special: bool, executable: bool, set: char, unset: char) -> char {
    match (special, executable) {
        (false, true) => 'x',
        (false, false) => '-',
        (true, true) => set,
        (true, false) => unset,
    }
}

pub(super) fn format_size(metadata: &Metadata, kind: EntryKind) -> String {
    let size = if kind == EntryKind::Directory {
        "-".to_string()
    } else {
        human_size(metadata.len())
    };
    format!("{size:>width$}", width = SIZE_WIDTH)
}

fn human_size(size: u64) -> String {
    if size < 1024 {
        return format!("{size}B");
    }
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < SIZE_UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.1}{}", SIZE_UNITS[unit])
    } else {
        format!("{value:.0}{}", SIZE_UNITS[unit])
    }
}

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

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(999), "999B");
        assert_eq!(human_size(1024), "1.0K");
        assert_eq!(human_size(1536), "1.5K");
        assert_eq!(human_size(10 * 1024), "10K");
        assert_eq!(human_size(1024 * 1024), "1.0M");
    }

    #[cfg(unix)]
    #[test]
    fn unix_modes_are_formatted_like_ls() {
        assert_eq!(format_mode(EntryKind::File, 0o755), "-rwxr-xr-x");
        assert_eq!(format_mode(EntryKind::File, 0o4755), "-rwsr-xr-x");
        assert_eq!(format_mode(EntryKind::Directory, 0o2755), "drwxr-sr-x");
        assert_eq!(format_mode(EntryKind::Directory, 0o1755), "drwxr-xr-t");
        assert_eq!(format_mode(EntryKind::Symlink, 0o777), "lrwxrwxrwx");
    }

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
