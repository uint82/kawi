use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GrammarError;
use crate::install;
use crate::pack::Pack;
use crate::version::Version;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackRootKind {
    User,
    Bundled,
    System,
}

impl PackRootKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Bundled => "bundled",
            Self::System => "system",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackRoot {
    pub path: PathBuf,
    pub kind: PackRootKind,
}

impl PackRoot {
    pub fn user(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: PackRootKind::User,
        }
    }

    pub fn bundled(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: PackRootKind::Bundled,
        }
    }

    pub fn system(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: PackRootKind::System,
        }
    }
}

#[derive(Debug)]
pub struct PackStatus {
    pub id: String,
    pub version: String,
    pub root: PackRootKind,
    pub path: PathBuf,
    pub state: PackState,
}

#[derive(Debug)]
pub enum PackState {
    Available,
    Valid,
    Invalid(String),
}

impl PackState {
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

#[derive(Debug)]
pub struct InstalledPack {
    pub id: String,
    pub version: String,
    pub path: PathBuf,
}

pub struct GrammarStore {
    roots: Vec<PackRoot>,
}

impl GrammarStore {
    pub fn new(roots: Vec<PackRoot>) -> Self {
        Self { roots }
    }

    pub fn roots(&self) -> &[PackRoot] {
        &self.roots
    }

    pub fn discover(&self) -> Vec<PackStatus> {
        let mut statuses = Vec::new();
        for root in &self.roots {
            discover_root(root, &mut statuses);
        }
        sort_statuses(&mut statuses);
        statuses
    }

    pub fn status(&self) -> Vec<PackStatus> {
        self.discover()
            .into_iter()
            .map(|mut status| {
                if matches!(status.state, PackState::Available) {
                    status.state = match Pack::open(&status.path).and_then(|pack| pack.verify()) {
                        Ok(()) => PackState::Valid,
                        Err(error) => PackState::Invalid(error.to_string()),
                    };
                }
                status
            })
            .collect()
    }

    pub fn best_packs(&self) -> Vec<Pack> {
        let mut best: BTreeMap<String, PackStatus> = BTreeMap::new();
        for status in self.status() {
            if !status.state.is_usable() {
                continue;
            }
            match best.get(&status.id) {
                Some(current)
                    if compare_versions(&status.version, &current.version) != Ordering::Greater => {
                }
                _ => {
                    best.insert(status.id.clone(), status);
                }
            }
        }
        best.into_values()
            .filter_map(|status| Pack::open(&status.path).ok())
            .collect()
    }

    pub fn install(&self, source: &Path) -> Result<InstalledPack, GrammarError> {
        install::install(&self.roots, source)
    }
}

fn discover_root(root: &PackRoot, statuses: &mut Vec<PackStatus>) {
    let Ok(entries) = fs::read_dir(&root.path) else {
        return;
    };
    for id_entry in entries.flatten() {
        let Ok(file_type) = id_entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || is_hidden(&id_entry) {
            continue;
        }
        let id_path = id_entry.path();
        let id = id_entry.file_name().to_string_lossy().into_owned();
        let Ok(versions) = fs::read_dir(&id_path) else {
            continue;
        };
        for version_entry in versions.flatten() {
            let Ok(file_type) = version_entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() || is_hidden(&version_entry) {
                continue;
            }
            let version = version_entry.file_name().to_string_lossy().into_owned();
            let path = version_entry.path();
            let state = inspect_pack(&path, &id, &version);
            statuses.push(PackStatus {
                id: id.clone(),
                version,
                root: root.kind,
                state,
                path,
            });
        }
    }
}

fn inspect_pack(path: &Path, id: &str, version: &str) -> PackState {
    match Pack::open(path) {
        Ok(pack) if pack.manifest().id == id && pack.manifest().version == version => {
            PackState::Available
        }
        Ok(pack) => PackState::Invalid(format!(
            "manifest declares {} {}, directory is {id} {version}",
            pack.manifest().id,
            pack.manifest().version
        )),
        Err(error) => PackState::Invalid(error.to_string()),
    }
}

fn is_hidden(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

fn sort_statuses(statuses: &mut [PackStatus]) {
    statuses.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| compare_versions(&right.version, &left.version))
            .then_with(|| root_order(left.root).cmp(&root_order(right.root)))
    });
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    match (Version::parse(left), Version::parse(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

fn root_order(kind: PackRootKind) -> u8 {
    match kind {
        PackRootKind::User => 0,
        PackRootKind::Bundled => 1,
        PackRootKind::System => 2,
    }
}
