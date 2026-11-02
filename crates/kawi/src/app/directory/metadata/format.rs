use std::fs::Metadata;

use super::super::entry::EntryKind;
use super::identity::IdentityResolver;

pub(super) const PERMISSION_WIDTH: usize = 10;
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
        EntryKind::Fifo => 'p',
        EntryKind::Socket => 's',
        EntryKind::BlockDevice => 'b',
        EntryKind::CharDevice => 'c',
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

pub(super) fn format_size(metadata: &Metadata) -> String {
    metadata.len().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn unix_modes_are_formatted_like_ls() {
        assert_eq!(format_mode(EntryKind::File, 0o755), "-rwxr-xr-x");
        assert_eq!(format_mode(EntryKind::File, 0o4755), "-rwsr-xr-x");
        assert_eq!(format_mode(EntryKind::Directory, 0o2755), "drwxr-sr-x");
        assert_eq!(format_mode(EntryKind::Directory, 0o1755), "drwxr-xr-t");
        assert_eq!(format_mode(EntryKind::Symlink, 0o777), "lrwxrwxrwx");
        assert_eq!(format_mode(EntryKind::Fifo, 0o644), "prw-r--r--");
        assert_eq!(format_mode(EntryKind::Socket, 0o755), "srwxr-xr-x");
        assert_eq!(format_mode(EntryKind::BlockDevice, 0o660), "brw-rw----");
        assert_eq!(format_mode(EntryKind::CharDevice, 0o666), "crw-rw-rw-");
    }
}
