#![cfg(feature = "wasm-grammars")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PACKS: [&str; 2] = ["adl/0.7.0", "typescript/0.23.2"];

struct Archive {
    root: PathBuf,
    name: String,
}

impl Archive {
    fn new(label: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "kawi_release_archive_{}_{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("dist")).expect("create dist");
        for directory in ["data", "config", "state", "cache"] {
            fs::create_dir_all(root.join(directory)).expect("create state dir");
        }
        Self {
            root,
            name: format!("kawi-v{}-smoke", env!("CARGO_PKG_VERSION")),
        }
    }

    fn package(&self) -> PathBuf {
        self.root.join("dist").join(&self.name)
    }

    fn archive(&self) -> PathBuf {
        self.root.join("dist").join(format!("{}.tar.gz", self.name))
    }

    fn extracted(&self) -> PathBuf {
        self.root.join("extracted")
    }

    fn build(&self) {
        let package = self.package();
        fs::create_dir_all(package.join("share/kawi/grammars")).expect("create share");
        copy_binary(&package.join("kawi"));
        for pack in PACKS {
            copy_pack(&package.join("share/kawi/grammars").join(pack), pack);
        }
        copy_release_files(&package);
        create_archive(self);
        fs::create_dir_all(self.extracted()).expect("create extract dir");
        extract_archive(self);
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.extracted().join(&self.name).join("kawi"));
        command
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("XDG_CACHE_HOME", self.root.join("cache"));
        command
    }
}

impl Drop for Archive {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_binary(target: &Path) {
    fs::copy(env!("CARGO_BIN_EXE_kawi"), target).expect("copy kawi binary");
}

fn copy_pack(target: &Path, pack: &str) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-grammar/packs")
        .join(pack);
    copy_tree(&source, target);
}

fn copy_release_files(package: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["LICENSE", "README.md"] {
        fs::copy(root.join(name), package.join(name)).expect("copy release file");
    }
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create target");
    for entry in fs::read_dir(source).expect("read source") {
        let entry = entry.expect("source entry");
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("source type").is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), &destination).expect("copy file");
        }
    }
}

fn create_archive(archive: &Archive) {
    let status = Command::new("tar")
        .arg("-C")
        .arg(archive.root.join("dist"))
        .arg("-czf")
        .arg(archive.archive())
        .arg(&archive.name)
        .status()
        .expect("run tar");
    assert!(status.success(), "tar create failed");
}

fn extract_archive(archive: &Archive) {
    let status = Command::new("tar")
        .arg("-C")
        .arg(archive.extracted())
        .arg("-xzf")
        .arg(archive.archive())
        .status()
        .expect("run tar");
    assert!(status.success(), "tar extract failed");
}

fn stdout(output: Output) -> String {
    assert!(output.status.success(), "{:?}", output.stderr);
    String::from_utf8(output.stdout).expect("utf-8 output")
}

#[test]
fn bundled_packs_load_from_the_release_archive() {
    let archive = Archive::new("packs");
    archive.build();

    let status = archive
        .command()
        .args(["grammar", "status"])
        .output()
        .expect("run grammar status");
    let status = stdout(status);

    assert!(status.contains("adl 0.7.0 [bundled] valid"));
    assert!(status.contains("typescript 0.23.2 [bundled] valid"));
}
