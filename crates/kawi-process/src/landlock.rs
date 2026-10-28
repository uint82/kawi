use std::collections::BTreeMap;
use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::sandbox::SandboxPolicy;

const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1;
const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;

const ACCESS_FS_EXECUTE: u64 = 1 << 0;
const ACCESS_FS_WRITE_FILE: u64 = 1 << 1;
const ACCESS_FS_READ_FILE: u64 = 1 << 2;
const ACCESS_FS_READ_DIR: u64 = 1 << 3;
const ACCESS_FS_REMOVE_DIR: u64 = 1 << 4;
const ACCESS_FS_REMOVE_FILE: u64 = 1 << 5;
const ACCESS_FS_MAKE_CHAR: u64 = 1 << 6;
const ACCESS_FS_MAKE_DIR: u64 = 1 << 7;
const ACCESS_FS_MAKE_REG: u64 = 1 << 8;
const ACCESS_FS_MAKE_SOCK: u64 = 1 << 9;
const ACCESS_FS_MAKE_FIFO: u64 = 1 << 10;
const ACCESS_FS_MAKE_BLOCK: u64 = 1 << 11;
const ACCESS_FS_MAKE_SYM: u64 = 1 << 12;
const ACCESS_FS_REFER: u64 = 1 << 13;
const ACCESS_FS_TRUNCATE: u64 = 1 << 14;

const ABI_REFER: i32 = 2;
const ABI_TRUNCATE: i32 = 3;

#[repr(C)]
struct RulesetAttr {
    handled_access_fs: u64,
}

#[repr(C)]
struct PathBeneathAttr {
    allowed_access: u64,
    parent_fd: i32,
}

pub(super) fn supported() -> bool {
    abi().is_some()
}

pub(super) fn configure(command: &mut Command, policy: &SandboxPolicy) -> bool {
    let Some(abi) = abi() else {
        return false;
    };
    let rules = rules(policy, abi);
    unsafe {
        command.pre_exec(move || restrict(abi, &rules));
    }
    true
}

fn abi() -> Option<i32> {
    static ABI: OnceLock<Option<i32>> = OnceLock::new();
    *ABI.get_or_init(probe)
}

fn probe() -> Option<i32> {
    let result = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<RulesetAttr>(),
            0usize,
            LANDLOCK_CREATE_RULESET_VERSION,
        )
    };
    let abi = i32::try_from(result).ok()?;
    (abi >= 1).then_some(abi)
}

fn rules(policy: &SandboxPolicy, abi: i32) -> Vec<(CString, u64)> {
    let mut merged = BTreeMap::new();
    for path in &policy.read_only {
        merge_rule(&mut merged, path, read_access());
    }
    for path in &policy.read_write {
        merge_rule(&mut merged, path, write_access(abi));
    }
    for path in &policy.execute {
        merge_rule(&mut merged, path, execute_access());
    }
    merged
        .into_iter()
        .filter_map(|(path, access)| {
            let path_c = CString::new(path.as_os_str().as_bytes()).ok()?;
            Some((path_c, access_for(&path, access)))
        })
        .collect()
}

fn access_for(path: &std::path::Path, access: u64) -> u64 {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => access,
        Ok(metadata) if metadata.is_file() => {
            access
                & (ACCESS_FS_EXECUTE
                    | ACCESS_FS_WRITE_FILE
                    | ACCESS_FS_READ_FILE
                    | ACCESS_FS_TRUNCATE)
        }
        _ => access & (ACCESS_FS_WRITE_FILE | ACCESS_FS_READ_FILE),
    }
}

fn merge_rule(rules: &mut BTreeMap<PathBuf, u64>, path: &Path, access: u64) {
    *rules.entry(path.to_path_buf()).or_default() |= access;
}

fn read_access() -> u64 {
    ACCESS_FS_READ_FILE | ACCESS_FS_READ_DIR
}

fn execute_access() -> u64 {
    ACCESS_FS_EXECUTE | ACCESS_FS_READ_FILE
}

fn write_access(abi: i32) -> u64 {
    let mut access = ACCESS_FS_WRITE_FILE
        | ACCESS_FS_READ_FILE
        | ACCESS_FS_READ_DIR
        | ACCESS_FS_REMOVE_DIR
        | ACCESS_FS_REMOVE_FILE
        | ACCESS_FS_MAKE_CHAR
        | ACCESS_FS_MAKE_DIR
        | ACCESS_FS_MAKE_REG
        | ACCESS_FS_MAKE_SOCK
        | ACCESS_FS_MAKE_FIFO
        | ACCESS_FS_MAKE_BLOCK
        | ACCESS_FS_MAKE_SYM;
    if abi >= ABI_REFER {
        access |= ACCESS_FS_REFER;
    }
    if abi >= ABI_TRUNCATE {
        access |= ACCESS_FS_TRUNCATE;
    }
    access
}

fn handled_access(abi: i32) -> u64 {
    let mut access = ACCESS_FS_EXECUTE
        | ACCESS_FS_WRITE_FILE
        | ACCESS_FS_READ_FILE
        | ACCESS_FS_READ_DIR
        | ACCESS_FS_REMOVE_DIR
        | ACCESS_FS_REMOVE_FILE
        | ACCESS_FS_MAKE_CHAR
        | ACCESS_FS_MAKE_DIR
        | ACCESS_FS_MAKE_REG
        | ACCESS_FS_MAKE_SOCK
        | ACCESS_FS_MAKE_FIFO
        | ACCESS_FS_MAKE_BLOCK
        | ACCESS_FS_MAKE_SYM;
    if abi >= ABI_REFER {
        access |= ACCESS_FS_REFER;
    }
    if abi >= ABI_TRUNCATE {
        access |= ACCESS_FS_TRUNCATE;
    }
    access
}

fn restrict(abi: i32, rules: &[(CString, u64)]) -> io::Result<()> {
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let attr = RulesetAttr {
        handled_access_fs: handled_access(abi),
    };
    let ruleset_fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &attr,
            std::mem::size_of::<RulesetAttr>(),
            0u32,
        )
    };
    if ruleset_fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let ruleset_fd = ruleset_fd as i32;
    for (path, access) in rules {
        if let Err(error) = add_rule(ruleset_fd, path, *access) {
            unsafe {
                libc::close(ruleset_fd);
            }
            return Err(error);
        }
    }
    let result = unsafe { libc::syscall(libc::SYS_landlock_restrict_self, ruleset_fd, 0u32) };
    unsafe {
        libc::close(ruleset_fd);
    }
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn add_rule(ruleset_fd: i32, path: &CString, access: u64) -> io::Result<()> {
    let path_fd = unsafe { libc::open(path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
    if path_fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let attr = PathBeneathAttr {
        allowed_access: access,
        parent_fd: path_fd,
    };
    let result = unsafe {
        libc::syscall(
            libc::SYS_landlock_add_rule,
            ruleset_fd,
            LANDLOCK_RULE_PATH_BENEATH,
            &attr,
            0u32,
        )
    };
    unsafe {
        libc::close(path_fd);
    }
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
