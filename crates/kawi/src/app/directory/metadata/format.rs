use std::fs::Metadata;

use super::super::entry::EntryKind;
use super::identity::IdentityResolver;

pub(super) const PERMISSION_WIDTH: usize = 10;
pub(super) const SIZE_WIDTH: usize = 7;
const SIZE_UNITS: [&str; 7] = ["B", "K", "M", "G", "T", "P", "E"];
#[cfg(not(unix))]
const UNKNOWN: &str = "?";

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

pub(super) fn format_links(metadata: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.nlink().to_string()
    }
    #[cfg(not(unix))]
    {
        UNKNOWN.to_string()
    }
}

pub(super) fn format_owner(metadata: &Metadata, identities: &mut IdentityResolver) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        identities.owner(metadata.uid()).to_string()
    }
    #[cfg(not(unix))]
    {
        let _ = identities;
        UNKNOWN.to_string()
    }
}

pub(super) fn format_group(metadata: &Metadata, identities: &mut IdentityResolver) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        identities.group(metadata.gid()).to_string()
    }
    #[cfg(not(unix))]
    {
        let _ = identities;
        UNKNOWN.to_string()
    }
}

pub(super) fn format_size(metadata: &Metadata, kind: EntryKind) -> String {
    if kind == EntryKind::Directory {
        "-".to_string()
    } else {
        human_size(metadata.len())
    }
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
}
