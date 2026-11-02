#[cfg(unix)]
use std::ffi::CStr;

#[cfg(unix)]
use std::collections::HashMap;

#[cfg(unix)]
const DEFAULT_NSS_BUFFER_SIZE: usize = 1024;
#[cfg(unix)]
const MAX_NSS_BUFFER_SIZE: usize = 1024 * 1024;

#[cfg(unix)]
pub(super) struct IdentityResolver {
    owners: HashMap<u32, String>,
    groups: HashMap<u32, String>,
}

#[cfg(unix)]
impl IdentityResolver {
    pub(super) fn new() -> Self {
        Self {
            owners: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    pub(super) fn owner(&mut self, uid: u32) -> &str {
        self.owners.entry(uid).or_insert_with(|| lookup_owner(uid))
    }

    pub(super) fn group(&mut self, gid: u32) -> &str {
        self.groups.entry(gid).or_insert_with(|| lookup_group(gid))
    }
}

#[cfg(not(unix))]
pub(super) struct IdentityResolver;

#[cfg(not(unix))]
impl IdentityResolver {
    pub(super) fn new() -> Self {
        Self
    }
}

#[cfg(unix)]
fn lookup_owner(uid: u32) -> String {
    lookup_name::<libc::passwd>(uid, libc::_SC_GETPW_R_SIZE_MAX).unwrap_or_else(|| uid.to_string())
}

#[cfg(unix)]
fn lookup_group(gid: u32) -> String {
    lookup_name::<libc::group>(gid, libc::_SC_GETGR_R_SIZE_MAX).unwrap_or_else(|| gid.to_string())
}

#[cfg(unix)]
fn lookup_name<R: NssRecord>(id: u32, size_key: libc::c_int) -> Option<String> {
    let mut size = nss_buffer_size(size_key);
    loop {
        let mut buffer = vec![0 as libc::c_char; size];
        let mut record = std::mem::MaybeUninit::<R>::uninit();
        let mut result = std::ptr::null_mut();
        let status = R::lookup(
            id,
            record.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        );
        if status == libc::ERANGE && size < MAX_NSS_BUFFER_SIZE {
            size = size.saturating_mul(2).min(MAX_NSS_BUFFER_SIZE);
            continue;
        }
        if status != 0 || result.is_null() {
            return None;
        }
        let record = unsafe { record.assume_init() };
        return record_name(&record);
    }
}

#[cfg(unix)]
fn nss_buffer_size(size_key: libc::c_int) -> usize {
    let size = unsafe { libc::sysconf(size_key) };
    usize::try_from(size)
        .ok()
        .filter(|size| *size > 0)
        .unwrap_or(DEFAULT_NSS_BUFFER_SIZE)
        .min(MAX_NSS_BUFFER_SIZE)
}

#[cfg(unix)]
fn record_name<R: NssRecord>(record: &R) -> Option<String> {
    let name = record.name();
    if name.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(unix)]
trait NssRecord: Sized {
    fn lookup(
        id: u32,
        record: *mut Self,
        buffer: *mut libc::c_char,
        length: usize,
        result: *mut *mut Self,
    ) -> libc::c_int;

    fn name(&self) -> *const libc::c_char;
}

#[cfg(unix)]
impl NssRecord for libc::passwd {
    fn lookup(
        id: u32,
        record: *mut Self,
        buffer: *mut libc::c_char,
        length: usize,
        result: *mut *mut Self,
    ) -> libc::c_int {
        unsafe { libc::getpwuid_r(id, record, buffer, length, result) }
    }

    fn name(&self) -> *const libc::c_char {
        self.pw_name
    }
}

#[cfg(unix)]
impl NssRecord for libc::group {
    fn lookup(
        id: u32,
        record: *mut Self,
        buffer: *mut libc::c_char,
        length: usize,
        result: *mut *mut Self,
    ) -> libc::c_int {
        unsafe { libc::getgrgid_r(id, record, buffer, length, result) }
    }

    fn name(&self) -> *const libc::c_char {
        self.gr_name
    }
}
