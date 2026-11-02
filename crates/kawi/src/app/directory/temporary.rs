use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::fs::DirectoryFs;

const MAX_ATTEMPTS: usize = 10_000;

pub(super) struct TemporaryPath<'a> {
    parent: &'a Path,
    name: String,
    marker: &'a str,
    index: usize,
    origin: &'a Path,
}

impl<'a> TemporaryPath<'a> {
    pub(super) fn new(origin: &'a Path, parent: &'a Path, marker: &'a str, index: usize) -> Self {
        let name = origin
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "item".to_string());
        Self {
            parent,
            name,
            marker,
            index,
            origin,
        }
    }
}

pub(super) fn unique_path(
    spec: &TemporaryPath<'_>,
    reserved: &BTreeSet<PathBuf>,
    fs: &impl DirectoryFs,
) -> Result<PathBuf, String> {
    for attempt in 0..MAX_ATTEMPTS {
        let candidate = spec.parent.join(format!(
            ".{}.{}-{}-{attempt}",
            spec.name, spec.marker, spec.index
        ));
        if !fs.exists(&candidate) && !reserved.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(format!(
        "cannot find a temporary name for {}",
        spec.origin.display()
    ))
}
