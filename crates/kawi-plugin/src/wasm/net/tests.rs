use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wasmtime_wasi::sockets::SocketAddrUse;

use super::*;

const PUBLIC_V4: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const OTHER_PUBLIC_V4: Ipv4Addr = Ipv4Addr::new(1, 1, 1, 1);
const PUBLIC_V6: Ipv6Addr = Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111);

fn hosts(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn address(ip: impl Into<IpAddr>, port: u16) -> SocketAddr {
    SocketAddr::new(ip.into(), port)
}

fn fixed_resolver(addresses: Vec<SocketAddr>) -> impl Fn(&HostPort) -> io::Result<Vec<SocketAddr>> {
    move |_| Ok(addresses.clone())
}

fn unreachable_resolver(_: &HostPort) -> io::Result<Vec<SocketAddr>> {
    unreachable!()
}

#[test]
fn denies_without_a_scope() {
    let policy = NetworkPolicy::from_scope(None);
    assert!(!policy.is_enabled());
    assert!(!policy.allows_name_lookup());
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpConnect));
    assert!(!policy.allows(address(Ipv4Addr::UNSPECIFIED, 0), SocketAddrUse::TcpBind));
}

#[test]
fn uses_the_host_scope_and_rejects_other_scopes() {
    let policy = NetworkPolicy::from_scope(Some(&Scope::Hosts(vec!["127.0.0.1:80".to_string()])));
    assert!(policy.is_enabled());
    assert!(policy.allows(address(Ipv4Addr::LOCALHOST, 80), SocketAddrUse::TcpConnect));
    assert!(!NetworkPolicy::from_scope(Some(&Scope::Unscoped)).is_enabled());
}

#[test]
fn allows_explicit_private_and_loopback_grants() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["127.0.0.1:8080", "10.0.0.1:443", "[::1]:8080"]),
        unreachable_resolver,
    );
    assert!(policy.allows(
        address(Ipv4Addr::LOCALHOST, 8080),
        SocketAddrUse::TcpConnect
    ));
    assert!(policy.allows(
        address(Ipv4Addr::new(10, 0, 0, 1), 443),
        SocketAddrUse::TcpConnect
    ));
    assert!(policy.allows(
        address(Ipv6Addr::LOCALHOST, 8080),
        SocketAddrUse::TcpConnect
    ));
    assert!(!policy.allows(
        address(Ipv4Addr::LOCALHOST, 8081),
        SocketAddrUse::TcpConnect
    ));
    assert!(!policy.allows_name_lookup());
}

#[test]
fn pins_dns_answers_and_rejects_rebinding() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    assert!(policy.allows_name_lookup());
    assert!(policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpConnect));
    assert!(!policy.allows(address(Ipv4Addr::LOCALHOST, 443), SocketAddrUse::TcpConnect));
    assert!(!policy.allows(address(OTHER_PUBLIC_V4, 443), SocketAddrUse::TcpConnect));
}

#[test]
fn resolves_only_granted_names() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    assert_eq!(
        policy.resolve_name("example.com").unwrap(),
        vec![IpAddr::V4(PUBLIC_V4)]
    );
    assert!(policy.resolve_name("other.example").is_err());
    assert_eq!(
        policy.resolve_name("8.8.8.8").unwrap(),
        vec![IpAddr::V4(PUBLIC_V4)]
    );
}

#[test]
fn denies_name_lookup_without_a_hostname_grant() {
    let policy = NetworkPolicy::from_hosts_with(&hosts(&["8.8.8.8:443"]), unreachable_resolver);
    assert!(!policy.allows_name_lookup());
    assert!(policy.resolve_name("8.8.8.8").is_err());
    assert!(policy.resolve_name("example.com").is_err());
}

#[test]
fn mixed_answers_pin_only_public_addresses() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![
            address(PUBLIC_V4, 443),
            address(Ipv4Addr::LOCALHOST, 443),
        ]),
    );
    assert!(policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpConnect));
    assert!(!policy.allows(address(Ipv4Addr::LOCALHOST, 443), SocketAddrUse::TcpConnect));
    assert_eq!(
        policy.resolve_name("example.com").unwrap(),
        vec![IpAddr::V4(PUBLIC_V4)]
    );
}

#[test]
fn failed_resolution_denies_connections() {
    let policy = NetworkPolicy::from_hosts_with(&hosts(&["example.com:443"]), |_| {
        Err(io::Error::other("resolver failure"))
    });
    assert!(policy.allows_name_lookup());
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpConnect));
    assert!(policy.resolve_name("example.com").unwrap().is_empty());
    assert!(policy
        .warnings()
        .iter()
        .any(|warning| warning.contains("example.com")));
    assert!(policy
        .warnings()
        .iter()
        .any(|warning| warning.contains("resolver failure")));
}

#[test]
fn resolves_each_hostname_once_per_policy() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443", "example.com:8443"]),
        move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(vec![address(PUBLIC_V4, 443)])
        },
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let _ = policy.resolve_name("example.com");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn resolver_timeout_fails_closed() {
    let host = HostPort::parse("example.com:443").unwrap();
    let start = Instant::now();
    let result = resolve_with_timeout(&host, Duration::from_millis(10), |_| {
        thread::sleep(Duration::from_secs(1));
        Ok(Vec::new())
    });
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
#[cfg(not(feature = "wasm-test-network"))]
fn production_denies_loopback_hostnames() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["localhost:443"]),
        fixed_resolver(vec![address(Ipv4Addr::LOCALHOST, 443)]),
    );
    assert!(!policy.allows(address(Ipv4Addr::LOCALHOST, 443), SocketAddrUse::TcpConnect));
    assert!(policy.resolve_name("localhost").unwrap().is_empty());
}

#[test]
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
fn test_override_allows_loopback_hostnames() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["localhost:443"]),
        fixed_resolver(vec![address(Ipv4Addr::LOCALHOST, 443)]),
    );
    assert!(policy.allows(address(Ipv4Addr::LOCALHOST, 443), SocketAddrUse::TcpConnect));
    assert_eq!(
        policy.resolve_name("localhost").unwrap(),
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]
    );
}

#[test]
fn rejects_private_and_special_dns_answers() {
    let addresses = vec![
        address(Ipv4Addr::LOCALHOST, 443),
        address(Ipv4Addr::new(10, 0, 0, 1), 443),
        address(Ipv4Addr::new(169, 254, 1, 1), 443),
        address(Ipv4Addr::new(100, 64, 0, 1), 443),
        address(Ipv4Addr::new(198, 18, 0, 1), 443),
        address(Ipv4Addr::new(192, 0, 2, 1), 443),
        address(Ipv4Addr::new(224, 0, 0, 1), 443),
        address(Ipv4Addr::new(240, 0, 0, 1), 443),
        address(Ipv6Addr::LOCALHOST, 443),
        address(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0x2001, 0x0002, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0x2001, 0x0010, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0x2001, 0x0020, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0x2002, 0x0808, 0x0808, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 1), 443),
        address(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x0a00, 1), 443),
    ];
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(addresses.clone()),
    );
    assert!(policy.allows_name_lookup());
    assert!(addresses
        .iter()
        .all(|address| !policy.allows(*address, SocketAddrUse::TcpConnect)));
}

#[test]
fn caps_connections_per_session() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    for handle in 0..MAX_CONNECTIONS as u32 {
        assert!(policy.claim_connection(handle));
    }
    assert!(!policy.claim_connection(MAX_CONNECTIONS as u32));
    let cloned = policy.clone();
    assert!(!cloned.claim_connection(MAX_CONNECTIONS as u32));
}

#[test]
fn closed_connections_free_slots() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    for handle in 0..MAX_CONNECTIONS as u32 {
        assert!(policy.claim_connection(handle));
    }
    assert!(!policy.claim_connection(MAX_CONNECTIONS as u32));
    policy.release_connection(0);
    assert!(policy.claim_connection(MAX_CONNECTIONS as u32));
}

#[test]
fn connection_caps_are_per_policy() {
    let first = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    let second = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    for handle in 0..MAX_CONNECTIONS as u32 {
        assert!(first.claim_connection(handle));
    }
    assert!(!first.claim_connection(MAX_CONNECTIONS as u32));
    assert!(second.claim_connection(0));
}

#[test]
fn allows_only_tcp_connect_and_ephemeral_bind() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    assert!(policy.allows(address(Ipv4Addr::UNSPECIFIED, 0), SocketAddrUse::TcpBind));
    assert!(policy.allows(address(Ipv6Addr::UNSPECIFIED, 0), SocketAddrUse::TcpBind));
    assert!(!policy.allows(address(Ipv4Addr::LOCALHOST, 0), SocketAddrUse::TcpBind));
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpListen));
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpAccept));
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::UdpBind));
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::UdpSend));
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::UdpReceive));
}

#[test]
fn allows_only_hostname_grants_to_resolve_names() {
    let ip_policy = NetworkPolicy::from_hosts_with(&hosts(&["8.8.8.8:443"]), unreachable_resolver);
    assert!(!ip_policy.allows_name_lookup());
    let host_policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V4, 443)]),
    );
    assert!(host_policy.allows_name_lookup());
}

#[test]
fn rejects_explicit_unspecified_and_multicast_grants() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["0.0.0.0:80", "224.0.0.1:80", "[ff02::1]:80"]),
        unreachable_resolver,
    );
    assert!(policy.is_enabled());
    assert!(!policy.allows(
        address(Ipv4Addr::UNSPECIFIED, 80),
        SocketAddrUse::TcpConnect
    ));
    assert!(!policy.allows(
        address(Ipv4Addr::new(224, 0, 0, 1), 80),
        SocketAddrUse::TcpConnect
    ));
    assert!(!policy.allows(
        address(Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1), 80),
        SocketAddrUse::TcpConnect
    ));
}

#[test]
fn allows_resolved_global_ipv6_answers() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&["example.com:443"]),
        fixed_resolver(vec![address(PUBLIC_V6, 443)]),
    );
    assert!(policy.allows(address(PUBLIC_V6, 443), SocketAddrUse::TcpConnect));
}

#[test]
fn ignores_invalid_host_entries() {
    let policy = NetworkPolicy::from_hosts_with(
        &hosts(&[
            "not-a-host",
            "example.com:0",
            "*.example.com:443",
            "::1:443",
        ]),
        unreachable_resolver,
    );
    assert!(policy.is_enabled());
    assert!(!policy.allows_name_lookup());
    assert!(!policy.allows(address(PUBLIC_V4, 443), SocketAddrUse::TcpConnect));
}

#[test]
fn classifies_public_and_non_public_addresses() {
    let cases = [
        (IpAddr::V4(PUBLIC_V4), true),
        (IpAddr::V4(OTHER_PUBLIC_V4), true),
        (IpAddr::V6(PUBLIC_V6), true),
        (IpAddr::V4(Ipv4Addr::UNSPECIFIED), false),
        (IpAddr::V4(Ipv4Addr::LOCALHOST), false),
        (IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(198, 18, 0, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(192, 88, 99, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1)), false),
        (IpAddr::V4(Ipv4Addr::new(240, 0, 0, 1)), false),
        (IpAddr::V4(Ipv4Addr::BROADCAST), false),
        (IpAddr::V6(Ipv6Addr::UNSPECIFIED), false),
        (IpAddr::V6(Ipv6Addr::LOCALHOST), false),
        (
            IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x0002, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x0010, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x0020, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0x2002, 0x0808, 0x0808, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 1)),
            false,
        ),
        (
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x0a00, 1)),
            false,
        ),
    ];
    for (address, expected) in cases {
        assert_eq!(is_public_ip(address), expected, "{address}");
    }
}
