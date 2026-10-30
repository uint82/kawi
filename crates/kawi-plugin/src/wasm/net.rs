use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use wasmtime_wasi::sockets::SocketAddrUse;

use crate::capability::{HostPort, Scope};
use crate::NETWORK_WARNING_PREFIX;

const RESOLUTION_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONNECTIONS: usize = 64;
const TEST_LOOPBACK_HOSTNAME: bool = cfg!(all(feature = "wasm-test-network", debug_assertions));

#[derive(Clone)]
pub(crate) struct NetworkPolicy {
    allowed: BTreeSet<SocketAddr>,
    names: BTreeMap<String, Vec<IpAddr>>,
    active_connections: Arc<Mutex<BTreeSet<u32>>>,
    warnings: Vec<String>,
    name_lookup: bool,
    enabled: bool,
}

impl NetworkPolicy {
    pub(crate) fn from_scope(scope: Option<&Scope>) -> Self {
        match scope {
            Some(Scope::Hosts(hosts)) => Self::from_hosts_with(hosts, resolve_grant),
            _ => Self::deny(),
        }
    }

    pub(crate) fn from_hosts_with(
        hosts: &[String],
        resolve: impl Fn(&HostPort) -> io::Result<Vec<SocketAddr>>,
    ) -> Self {
        let mut policy = Self {
            allowed: BTreeSet::new(),
            names: BTreeMap::new(),
            active_connections: Arc::new(Mutex::new(BTreeSet::new())),
            warnings: Vec::new(),
            name_lookup: false,
            enabled: !hosts.is_empty(),
        };
        for host in hosts.iter().filter_map(|host| HostPort::parse(host).ok()) {
            match host.ip() {
                Some(ip) => policy.allow_explicit(ip, host.port()),
                None => policy.allow_resolved(&host, &resolve),
            }
        }
        policy
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn allows_name_lookup(&self) -> bool {
        self.name_lookup
    }

    pub(crate) fn warnings(&self) -> &[String] {
        &self.warnings
    }

    #[cfg(feature = "fuzz")]
    pub(crate) fn allowed(&self) -> &BTreeSet<SocketAddr> {
        &self.allowed
    }

    #[cfg(feature = "fuzz")]
    pub(crate) fn names(&self) -> &BTreeMap<String, Vec<IpAddr>> {
        &self.names
    }

    pub(crate) fn resolve_name(&self, name: &str) -> Result<Vec<IpAddr>, io::Error> {
        if !self.name_lookup {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "name lookup is not granted",
            ));
        }
        if let Ok(address) = name.parse::<IpAddr>() {
            return Ok(vec![address]);
        }
        self.names
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::PermissionDenied, "name is not granted"))
    }

    pub(crate) fn allows(&self, address: SocketAddr, use_: SocketAddrUse) -> bool {
        match use_ {
            SocketAddrUse::TcpBind => {
                self.enabled && address.ip().is_unspecified() && address.port() == 0
            }
            SocketAddrUse::TcpConnect => self.allowed.contains(&address),
            _ => false,
        }
    }

    pub(crate) fn is_tracked(&self, handle: u32) -> bool {
        self.lock_connections().contains(&handle)
    }

    pub(crate) fn claim_connection(&self, handle: u32) -> bool {
        let mut active = self.lock_connections();
        if active.contains(&handle) {
            return true;
        }
        if active.len() >= MAX_CONNECTIONS {
            return false;
        }
        active.insert(handle);
        true
    }

    pub(crate) fn release_connection(&self, handle: u32) {
        self.lock_connections().remove(&handle);
    }

    fn lock_connections(&self) -> std::sync::MutexGuard<'_, BTreeSet<u32>> {
        self.active_connections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn deny() -> Self {
        Self {
            allowed: BTreeSet::new(),
            names: BTreeMap::new(),
            active_connections: Arc::new(Mutex::new(BTreeSet::new())),
            warnings: Vec::new(),
            name_lookup: false,
            enabled: false,
        }
    }

    fn allow_explicit(&mut self, ip: IpAddr, port: u16) {
        if is_connectable_explicit(ip) {
            self.allowed.insert(SocketAddr::new(ip, port));
        }
    }

    fn allow_resolved(
        &mut self,
        host: &HostPort,
        resolve: &impl Fn(&HostPort) -> io::Result<Vec<SocketAddr>>,
    ) {
        self.name_lookup = true;
        let key = host.host().to_ascii_lowercase();
        let mut allowed_ips = BTreeSet::new();
        let mut resolution_failed = false;
        match resolve(host) {
            Ok(addresses) => {
                for address in addresses {
                    if is_allowed_resolved(host, address.ip()) {
                        allowed_ips.insert(address.ip());
                        self.allowed
                            .insert(SocketAddr::new(address.ip(), host.port()));
                    }
                }
            }
            Err(error) => {
                resolution_failed = true;
                self.warnings
                    .push(format!("{NETWORK_WARNING_PREFIX}{}: {error}", host.host()));
            }
        }
        if allowed_ips.is_empty() && !resolution_failed {
            self.warnings.push(format!(
                "{NETWORK_WARNING_PREFIX}{}: no permitted addresses",
                host.host()
            ));
        }
        self.names.insert(key, allowed_ips.into_iter().collect());
    }
}

fn resolve_grant(host: &HostPort) -> io::Result<Vec<SocketAddr>> {
    #[cfg(all(feature = "wasm-test-network", debug_assertions))]
    if let Some(result) = super::test_resolver::resolve_scripted(host) {
        return result;
    }
    resolve_host(host)
}

fn resolve_host(host: &HostPort) -> io::Result<Vec<SocketAddr>> {
    resolve_with_timeout(host, RESOLUTION_TIMEOUT, |host| {
        (host.host(), host.port())
            .to_socket_addrs()
            .map(Iterator::collect)
    })
}

fn resolve_with_timeout(
    host: &HostPort,
    timeout: Duration,
    resolve: impl FnOnce(&HostPort) -> io::Result<Vec<SocketAddr>> + Send + 'static,
) -> io::Result<Vec<SocketAddr>> {
    let host = host.clone();
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("kawi-dns".to_string())
        .spawn(move || {
            let _ = sender.send(resolve(&host));
        })
        .map_err(|error| io::Error::other(error.to_string()))?;
    receiver.recv_timeout(timeout).unwrap_or_else(|_| {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "dns resolution timed out",
        ))
    })
}

fn is_connectable_explicit(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => !(ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast()),
        IpAddr::V6(ip) => !(ip.is_unspecified() || ip.is_multicast()),
    }
}

fn is_allowed_resolved(host: &HostPort, ip: IpAddr) -> bool {
    is_public_ip(ip) || (TEST_LOOPBACK_HOSTNAME && host.host() == "localhost" && ip.is_loopback())
}

pub(crate) fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || is_shared_ipv4(ip)
        || is_benchmarking_ipv4(ip)
        || is_reserved_ipv4(ip))
}

fn is_shared_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_benchmarking_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 198 && (18..=19).contains(&octets[1])
}

fn is_reserved_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 0
        || octets[0] >= 240
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    is_global_unicast_ipv6(ip)
        && !is_documentation_ipv6(ip)
        && !is_benchmarking_ipv6(ip)
        && !is_orchid(ip)
        && !is_6to4(ip)
        && !is_teredo(ip)
}

fn is_global_unicast_ipv6(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xe000 == 0x2000
}

fn is_documentation_ipv6(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
}

fn is_benchmarking_ipv6(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0002
}

fn is_orchid(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && (0x0010..=0x002f).contains(&ip.segments()[1])
}

fn is_6to4(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2002
}

fn is_teredo(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0
}

#[cfg(test)]
mod tests;
