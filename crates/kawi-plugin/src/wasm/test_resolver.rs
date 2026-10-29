use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::capability::HostPort;

struct Script {
    answers: VecDeque<Vec<SocketAddr>>,
    calls: usize,
}

static SCRIPTS: Mutex<BTreeMap<String, Script>> = Mutex::new(BTreeMap::new());

pub fn install_resolver_script(host: &str, answers: Vec<Vec<SocketAddr>>) {
    lock_scripts().insert(
        host.to_ascii_lowercase(),
        Script {
            answers: answers.into(),
            calls: 0,
        },
    );
}

pub fn remove_resolver_script(host: &str) {
    lock_scripts().remove(&host.to_ascii_lowercase());
}

pub fn resolver_call_count(host: &str) -> usize {
    lock_scripts()
        .get(&host.to_ascii_lowercase())
        .map_or(0, |script| script.calls)
}

pub(crate) fn resolve_scripted(host: &HostPort) -> Option<io::Result<Vec<SocketAddr>>> {
    let mut scripts = lock_scripts();
    let script = scripts.get_mut(host.host())?;
    script.calls += 1;
    Some(
        script
            .answers
            .pop_front()
            .ok_or_else(|| io::Error::other("test resolver script exhausted")),
    )
}

fn lock_scripts() -> MutexGuard<'static, BTreeMap<String, Script>> {
    SCRIPTS.lock().unwrap_or_else(PoisonError::into_inner)
}
