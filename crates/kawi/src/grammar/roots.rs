use std::path::{Path, PathBuf};

use kawi_grammar::{GrammarStore, PackRoot};

const SHARE_GRAMMARS: &str = "share/kawi/grammars";

pub(crate) fn store(user_root: &Path) -> GrammarStore {
    store_with_bundled(user_root, bundled_root())
}

fn bundled_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    bundled_root_from(&exe)
}

fn bundled_root_from(exe: &Path) -> Option<PathBuf> {
    let directory = exe.parent()?;
    [
        directory.join(SHARE_GRAMMARS),
        directory.join("..").join(SHARE_GRAMMARS),
    ]
    .into_iter()
    .find_map(existing_directory)
}

fn existing_directory(path: PathBuf) -> Option<PathBuf> {
    path.canonicalize().ok().filter(|path| path.is_dir())
}

fn store_with_bundled(user_root: &Path, bundled: Option<PathBuf>) -> GrammarStore {
    let mut roots = vec![PackRoot::user(user_root)];
    if let Some(path) = bundled.filter(|path| path != user_root) {
        roots.push(PackRoot::bundled(path));
    }
    GrammarStore::new(roots)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use kawi_grammar::PackRootKind;

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "kawi_grammar_roots_{}_{}_{name}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn finds_share_next_to_the_binary() {
        let dir = TempDir::new("adjacent");
        let exe = dir.path.join("kawi");
        let share = dir.path.join(SHARE_GRAMMARS);
        fs::create_dir_all(&share).expect("create share");

        assert_eq!(
            bundled_root_from(&exe),
            Some(share.canonicalize().expect("canonical share"))
        );
    }

    #[test]
    fn finds_share_beside_bin() {
        let dir = TempDir::new("bin");
        let exe = dir.path.join("bin/kawi");
        let share = dir.path.join(SHARE_GRAMMARS);
        fs::create_dir_all(&share).expect("create share");
        fs::create_dir_all(exe.parent().expect("binary parent")).expect("create bin");

        assert_eq!(
            bundled_root_from(&exe),
            Some(share.canonicalize().expect("canonical share"))
        );
    }

    #[test]
    fn ignores_a_missing_or_non_directory_share() {
        let dir = TempDir::new("missing");
        let exe = dir.path.join("kawi");
        assert_eq!(bundled_root_from(&exe), None);

        fs::create_dir_all(dir.path.join("share/kawi")).expect("create parent");
        fs::write(dir.path.join(SHARE_GRAMMARS), "not a directory").expect("write file");
        assert_eq!(bundled_root_from(&exe), None);
    }

    #[test]
    fn store_without_bundled_has_only_the_user_root() {
        let dir = TempDir::new("user-only");
        let store = store_with_bundled(&dir.path, None);
        assert_eq!(store.roots().len(), 1);
        assert_eq!(store.roots()[0].kind, PackRootKind::User);
    }

    #[test]
    fn store_puts_the_bundled_root_after_the_user_root() {
        let dir = TempDir::new("bundled");
        let user = dir.path.join("user");
        let bundled = dir.path.join("bundled");
        let store = store_with_bundled(&user, Some(bundled.clone()));

        assert_eq!(store.roots().len(), 2);
        assert_eq!(store.roots()[0].path, user);
        assert_eq!(store.roots()[0].kind, PackRootKind::User);
        assert_eq!(store.roots()[1].path, bundled);
        assert_eq!(store.roots()[1].kind, PackRootKind::Bundled);
    }

    #[test]
    fn store_deduplicates_a_bundled_root_that_matches_the_user_root() {
        let dir = TempDir::new("deduplicate");
        let store = store_with_bundled(&dir.path, Some(dir.path.clone()));
        assert_eq!(store.roots().len(), 1);
    }
}
