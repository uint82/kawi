use super::*;
use std::process::Stdio;

#[test]
fn write_access_tracks_the_supported_abi() {
    assert_eq!(write_access(1) & ACCESS_FS_REFER, 0);
    assert_eq!(write_access(1) & ACCESS_FS_TRUNCATE, 0);
    assert_ne!(write_access(ABI_REFER) & ACCESS_FS_REFER, 0);
    assert_ne!(write_access(ABI_TRUNCATE) & ACCESS_FS_TRUNCATE, 0);
}

#[test]
fn handled_access_tracks_the_supported_abi() {
    assert_eq!(handled_access(1) & ACCESS_FS_REFER, 0);
    assert_eq!(handled_access(1) & ACCESS_FS_TRUNCATE, 0);
    assert_ne!(handled_access(ABI_REFER) & ACCESS_FS_REFER, 0);
    assert_ne!(handled_access(ABI_TRUNCATE) & ACCESS_FS_TRUNCATE, 0);
}

#[test]
#[cfg(target_os = "linux")]
fn denies_reads_outside_the_policy() {
    if !status().is_enforced() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("kawi-landlock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let allowed = dir.join("allowed.txt");
    let denied = dir.join("denied.txt");
    std::fs::write(&allowed, "allowed\n").unwrap();
    std::fs::write(&denied, "denied\n").unwrap();
    let shell = crate::find_executable("sh").expect("sh");
    let mut policy = SandboxPolicy {
        read_only: ["/usr", "/lib", "/lib64", "/etc/ld.so.cache", "/dev/urandom"]
            .iter()
            .map(PathBuf::from)
            .collect(),
        read_write: vec![PathBuf::from("/dev/null")],
        execute: vec![shell.clone()],
    };
    policy.read_only.push(PathBuf::from("/proc/self"));
    policy.read_only.push(allowed.clone());
    if let Some(interpreter) = crate::executable_interpreter(&shell) {
        policy.execute.push(interpreter);
    }
    assert!(run_shell(&shell, &allowed, &policy));
    assert!(!run_shell(&shell, &denied, &policy));
    std::fs::remove_dir_all(dir).unwrap();
}

fn run_shell(shell: &std::path::Path, path: &std::path::Path, policy: &SandboxPolicy) -> bool {
    let mut command = Command::new(shell);
    command
        .arg("-c")
        .arg(format!("read value < {}", path.display()))
        .stderr(Stdio::null());
    assert!(super::configure(&mut command, policy).is_enforced());
    command.status().expect("shell").success()
}

#[test]
fn merges_rules_for_the_same_path() {
    let policy = SandboxPolicy {
        read_only: vec![PathBuf::from("/tmp")],
        read_write: vec![PathBuf::from("/tmp")],
        execute: Vec::new(),
    };
    let rules = rules(&policy, ABI_TRUNCATE);
    assert_eq!(rules.len(), 1);
    assert_ne!(rules[0].1 & ACCESS_FS_WRITE_FILE, 0);
}
