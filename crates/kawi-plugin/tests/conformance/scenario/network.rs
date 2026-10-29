#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkTarget {
    Allowed,
    Denied,
    Hostname,
    OtherHostname,
    PublicIp,
    Mapped,
    LinkLocal,
    Loopback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkGrant {
    None,
    Local,
    Hostname,
    Public,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NetworkCase {
    pub(crate) command: &'static str,
    pub(crate) target: NetworkTarget,
    pub(crate) grant: NetworkGrant,
}

pub(crate) const PUBLIC_GRANT: &str = "8.8.8.8:443";

pub(crate) fn network_case(id: &str) -> NetworkCase {
    match id {
        "wasm.net.no_grant" => NetworkCase {
            command: "connect",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::None,
        },
        "wasm.net.connection_cap_denied" => NetworkCase {
            command: "connect-many",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.connection_cap_reuse" => NetworkCase {
            command: "connect-reuse",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.restart_closes_socket" => NetworkCase {
            command: "hold",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.connect_allowed" => NetworkCase {
            command: "connect",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.connect_denied" => NetworkCase {
            command: "connect",
            target: NetworkTarget::Denied,
            grant: NetworkGrant::Local,
        },
        "wasm.net.dns_denied" => NetworkCase {
            command: "resolve",
            target: NetworkTarget::Hostname,
            grant: NetworkGrant::Local,
        },
        "wasm.net.dns_other_denied" => NetworkCase {
            command: "resolve",
            target: NetworkTarget::OtherHostname,
            grant: NetworkGrant::Hostname,
        },
        "wasm.net.dns_ip_allowed" => NetworkCase {
            command: "resolve",
            target: NetworkTarget::PublicIp,
            grant: NetworkGrant::Hostname,
        },
        "wasm.net.hostname_allowed" => NetworkCase {
            command: "connect",
            target: NetworkTarget::Hostname,
            grant: NetworkGrant::Hostname,
        },
        "wasm.net.udp_denied" => NetworkCase {
            command: "udp",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.bind_denied" => NetworkCase {
            command: "listen",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.accept_denied" => NetworkCase {
            command: "accept",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.mapped_denied" => NetworkCase {
            command: "connect",
            target: NetworkTarget::Mapped,
            grant: NetworkGrant::Public,
        },
        "wasm.net.link_local_denied" => NetworkCase {
            command: "connect",
            target: NetworkTarget::LinkLocal,
            grant: NetworkGrant::Public,
        },
        "wasm.net.loopback_denied" => NetworkCase {
            command: "connect",
            target: NetworkTarget::Loopback,
            grant: NetworkGrant::Public,
        },
        _ => panic!("missing network case: {id}"),
    }
}
