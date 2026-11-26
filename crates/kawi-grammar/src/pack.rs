use std::io;
use std::path::{Path, PathBuf};

use crate::error::GrammarError;
use crate::file;
use crate::hash;
use crate::limits;
use crate::manifest::{Manifest, MANIFEST_FILE};

#[derive(Debug)]
pub struct Pack {
    root: PathBuf,
    manifest: Manifest,
}

#[derive(Debug)]
pub struct PackContents {
    pub wasm: Vec<u8>,
    pub highlights: String,
    pub injections: String,
    pub locals: String,
}

impl Pack {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, GrammarError> {
        let root = root.into();
        let root = root
            .canonicalize()
            .map_err(|source| GrammarError::io(&root, source))?;
        if !root.is_dir() {
            return Err(GrammarError::NotADirectory(root));
        }
        let manifest = Manifest::load(&root.join(MANIFEST_FILE))?;
        Ok(Self { root, manifest })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn load_contents(&self) -> Result<PackContents, GrammarError> {
        self.manifest.ensure_host_abi()?;
        let (_, wasm) = self.read_file(
            &self.manifest.grammar,
            &self.manifest.grammar_sha256,
            limits::MAX_GRAMMAR_BYTES,
        )?;
        let highlights = self.read_query(
            &self.manifest.queries.highlights,
            &self.manifest.queries.highlights_sha256,
        )?;
        let injections = self.read_optional_query(
            self.manifest.queries.injections.as_deref(),
            self.manifest.queries.injections_sha256.as_deref(),
        )?;
        let locals = self.read_optional_query(
            self.manifest.queries.locals.as_deref(),
            self.manifest.queries.locals_sha256.as_deref(),
        )?;
        Ok(PackContents {
            wasm,
            highlights,
            injections,
            locals,
        })
    }

    pub fn verify(&self) -> Result<(), GrammarError> {
        self.load_contents().map(|_| ())
    }

    fn read_optional_query(
        &self,
        path: Option<&Path>,
        sha256: Option<&str>,
    ) -> Result<String, GrammarError> {
        match (path, sha256) {
            (Some(path), Some(sha256)) => self.read_query(path, sha256),
            (None, None) => Ok(String::new()),
            _ => Err(GrammarError::InvalidManifest(
                "query path and hash must be present together".to_string(),
            )),
        }
    }

    fn read_query(&self, relative: &Path, sha256: &str) -> Result<String, GrammarError> {
        let (path, bytes) = self.read_file(relative, sha256, limits::MAX_QUERY_BYTES)?;
        String::from_utf8(bytes).map_err(|source| GrammarError::InvalidUtf8 { path, source })
    }

    fn read_file(
        &self,
        relative: &Path,
        sha256: &str,
        limit: u64,
    ) -> Result<(PathBuf, Vec<u8>), GrammarError> {
        let path = self.canonical_file(relative)?;
        let bytes = file::read_bounded(&path, limit)?;
        hash::verify_sha256(&path, &bytes, sha256)?;
        Ok((path, bytes))
    }

    fn canonical_file(&self, relative: &Path) -> Result<PathBuf, GrammarError> {
        let path = self.root.join(relative);
        let canonical = path.canonicalize().map_err(|source| match source.kind() {
            io::ErrorKind::NotFound => GrammarError::MissingFile(path.clone()),
            _ => GrammarError::io(&path, source),
        })?;
        if !canonical.starts_with(&self.root) {
            return Err(GrammarError::UnsafePath(relative.display().to_string()));
        }
        if !canonical.is_file() {
            return Err(GrammarError::NotAFile(canonical));
        }
        Ok(canonical)
    }
}
