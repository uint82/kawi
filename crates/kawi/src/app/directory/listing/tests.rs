use std::fs;

use crate::app::directory::SortKey;
use crate::test_support::temp_dir;

use super::*;

#[test]
fn name_sort_ignores_entry_kind() {
    let dir = temp_dir("directory_name_sort");
    fs::create_dir_all(dir.join("zdir")).unwrap();
    fs::write(dir.join("alpha.txt"), "").unwrap();
    let sort = SortOptions {
        key: SortKey::Name,
        reverse: false,
        directories_first: false,
    };
    let listing = read_listing(&dir, false, 1, sort, MetadataColumns::default()).unwrap();
    let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
    assert_eq!(names, ["alpha.txt", "zdir/"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn directories_sort_before_files_and_hidden_entries_are_filtered() {
    let dir = temp_dir("directory_listing");
    fs::create_dir_all(dir.join("zeta")).unwrap();
    fs::write(dir.join("alpha.txt"), "").unwrap();
    fs::write(dir.join(".hidden"), "").unwrap();
    fs::write(dir.join("beta10.txt"), "").unwrap();
    fs::write(dir.join("beta2.txt"), "").unwrap();
    let listing = read_listing(
        &dir,
        false,
        1,
        SortOptions::default(),
        MetadataColumns::default(),
    )
    .unwrap();
    let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
    assert_eq!(names, ["zeta/", "alpha.txt", "beta2.txt", "beta10.txt"]);
    let listing = read_listing(
        &dir,
        true,
        1,
        SortOptions::default(),
        MetadataColumns::default(),
    )
    .unwrap();
    assert!(listing.entries.iter().any(|entry| entry.name == ".hidden"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn size_sort_loads_metadata_without_display_columns() {
    let dir = temp_dir("directory_size_sort");
    fs::write(dir.join("small"), "1").unwrap();
    fs::write(dir.join("large"), "1234567890").unwrap();
    fs::write(dir.join("middle"), "12345").unwrap();
    let sort = SortOptions {
        key: SortKey::Size,
        reverse: false,
        directories_first: false,
    };
    let listing = read_listing(&dir, false, 1, sort, MetadataColumns::default()).unwrap();
    let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
    assert_eq!(names, ["small", "middle", "large"]);
    assert!(listing.entries.iter().all(|entry| entry.metadata.is_some()));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn modified_sort_uses_filesystem_timestamps() {
    use std::fs::FileTimes;
    use std::time::{Duration, SystemTime};

    let dir = temp_dir("directory_modified_sort");
    let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    for (name, offset) in [("new", 20), ("old", 0), ("middle", 10)] {
        let path = dir.join(name);
        let file = fs::File::create(&path).unwrap();
        let times = FileTimes::new().set_modified(base + Duration::from_secs(offset));
        file.set_times(times).unwrap();
    }
    let sort = SortOptions {
        key: SortKey::Modified,
        reverse: false,
        directories_first: false,
    };
    let listing = read_listing(&dir, false, 1, sort, MetadataColumns::default()).unwrap();
    let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
    assert_eq!(names, ["old", "middle", "new"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn reverse_sort_keeps_directories_first() {
    let dir = temp_dir("directory_reverse_sort");
    fs::create_dir(dir.join("zdir")).unwrap();
    fs::write(dir.join("small"), "1").unwrap();
    fs::write(dir.join("large"), "1234567890").unwrap();
    let sort = SortOptions {
        key: SortKey::Size,
        reverse: true,
        directories_first: true,
    };
    let listing = read_listing(&dir, false, 1, sort, MetadataColumns::default()).unwrap();
    let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
    assert_eq!(names, ["zdir/", "large", "small"]);
    fs::remove_dir_all(dir).unwrap();
}

fn entry_named<'a>(listing: &'a Listing, name: &str) -> &'a Entry {
    listing
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
}

#[cfg(unix)]
#[test]
fn symlinks_keep_link_kind_and_metadata() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("directory_symlink_kind");
    fs::write(dir.join("target.txt"), "hello").unwrap();
    symlink("target.txt", dir.join("link")).unwrap();
    let columns = MetadataColumns::parse("permissions,size").unwrap();
    let listing = read_listing(&dir, false, 1, SortOptions::default(), columns).unwrap();
    let link = entry_named(&listing, "link");
    assert_eq!(link.kind, EntryKind::Symlink);
    assert_eq!(link.display_name(), "link");
    let text = link
        .metadata
        .as_ref()
        .unwrap()
        .text(columns, listing.metadata_widths)
        .unwrap();
    assert!(text.starts_with('l'));
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinks_to_directories_are_not_directories() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("directory_symlink_sort");
    fs::create_dir(dir.join("real")).unwrap();
    symlink("real", dir.join("link")).unwrap();
    let listing = read_listing(
        &dir,
        false,
        1,
        SortOptions::default(),
        MetadataColumns::default(),
    )
    .unwrap();
    let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
    assert_eq!(names, ["real/", "link"]);
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn special_files_keep_their_kind_and_type_character() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixListener;

    let dir = temp_dir("directory_special_files");
    let _listener = UnixListener::bind(dir.join("sock")).unwrap();
    let fifo = CString::new(dir.join("pipe").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    let columns = MetadataColumns::parse("permissions").unwrap();
    let listing = read_listing(&dir, false, 1, SortOptions::default(), columns).unwrap();
    let socket = entry_named(&listing, "sock");
    assert_eq!(socket.kind, EntryKind::Socket);
    assert!(permission_text(socket, &listing, columns).starts_with('s'));
    let pipe = entry_named(&listing, "pipe");
    assert_eq!(pipe.kind, EntryKind::Fifo);
    assert!(permission_text(pipe, &listing, columns).starts_with('p'));
    fs::remove_dir_all(dir).unwrap();
}

fn permission_text(entry: &Entry, listing: &Listing, columns: MetadataColumns) -> String {
    entry
        .metadata
        .as_ref()
        .unwrap()
        .text(columns, listing.metadata_widths)
        .unwrap()
}

#[test]
fn metadata_is_read_only_when_columns_are_enabled() {
    let dir = temp_dir("directory_metadata");
    fs::write(dir.join("alpha.txt"), "hello").unwrap();
    fs::create_dir_all(dir.join("sub")).unwrap();
    let listing = read_listing(
        &dir,
        false,
        1,
        SortOptions::default(),
        MetadataColumns::default(),
    )
    .unwrap();
    assert!(listing.entries.iter().all(|entry| entry.metadata.is_none()));
    let columns = MetadataColumns::parse("permissions,size,modified").unwrap();
    let listing = read_listing(&dir, false, 1, SortOptions::default(), columns).unwrap();
    let file = listing
        .entries
        .iter()
        .find(|entry| entry.name == "alpha.txt")
        .unwrap();
    let text = file
        .metadata
        .as_ref()
        .unwrap()
        .text(columns, listing.metadata_widths)
        .unwrap();
    assert!(text.starts_with('-'));
    let fields: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(fields.get(1), Some(&"5"));
    let directory = listing
        .entries
        .iter()
        .find(|entry| entry.name == "sub")
        .unwrap();
    let text = directory
        .metadata
        .as_ref()
        .unwrap()
        .text(columns, listing.metadata_widths)
        .unwrap();
    assert!(text.starts_with('d'));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn large_listings_load_metadata_for_every_entry() {
    const ENTRY_COUNT: usize = 600;
    let dir = temp_dir("directory_large_metadata");
    for index in 0..ENTRY_COUNT {
        fs::write(dir.join(format!("file-{index:03}.txt")), "content").unwrap();
    }
    let listing = read_listing(
        &dir,
        false,
        1,
        SortOptions::default(),
        MetadataColumns::all(),
    )
    .unwrap();
    assert_eq!(listing.entries.len(), ENTRY_COUNT);
    assert!(listing.entries.iter().all(|entry| entry.metadata.is_some()));
    fs::remove_dir_all(dir).unwrap();
}
