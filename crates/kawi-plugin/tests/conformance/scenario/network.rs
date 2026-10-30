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

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const OTHER_PUBLIC_ANSWER: &str = "1.1.1.1:443";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const PRIVATE_ANSWER: &str = "10.0.0.1:443";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const LOOPBACK_ANSWER: &str = "127.0.0.1:443";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const LINK_LOCAL_ANSWER: &str = "169.254.169.254:443";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const LINK_LOCAL_V6_ANSWER: &str = "[fe80::1%1]:443";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const MAPPED_LOOPBACK_ANSWER: &str = "[::ffff:127.0.0.1]:443";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const SCRIPTED_PORT: u16 = 443;

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const REBIND_PRIVATE_HOST: &str = "rebind-private.conformance.test";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const REBIND_PUBLIC_HOST: &str = "rebind-public.conformance.test";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const MIXED_ANSWERS_HOST: &str = "mixed-answers.conformance.test";
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const ALL_PRIVATE_HOST: &str = "all-private.conformance.test";

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolverScript {
    pub(crate) host: &'static str,
    pub(crate) port: u16,
    pub(crate) answers: &'static [&'static [&'static str]],
}

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
const MIXED_ANSWER: &[&str] = &[
    PUBLIC_GRANT,
    PRIVATE_ANSWER,
    LOOPBACK_ANSWER,
    LINK_LOCAL_ANSWER,
    LINK_LOCAL_V6_ANSWER,
    MAPPED_LOOPBACK_ANSWER,
];

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
pub(crate) fn resolver_script(id: &str) -> Option<ResolverScript> {
    match id {
        "wasm.net.rebind_private" => Some(ResolverScript {
            host: REBIND_PRIVATE_HOST,
            port: SCRIPTED_PORT,
            answers: &[&[PUBLIC_GRANT], &[PRIVATE_ANSWER]],
        }),
        "wasm.net.rebind_public" => Some(ResolverScript {
            host: REBIND_PUBLIC_HOST,
            port: SCRIPTED_PORT,
            answers: &[&[PUBLIC_GRANT], &[OTHER_PUBLIC_ANSWER]],
        }),
        "wasm.net.mixed_answers" => Some(ResolverScript {
            host: MIXED_ANSWERS_HOST,
            port: SCRIPTED_PORT,
            answers: &[MIXED_ANSWER; 5],
        }),
        "wasm.net.all_private" => Some(ResolverScript {
            host: ALL_PRIVATE_HOST,
            port: SCRIPTED_PORT,
            answers: &[&[PRIVATE_ANSWER]],
        }),
        _ => None,
    }
}

pub(crate) fn network_endpoint(_id: &str, port: u16) -> String {
    #[cfg(all(feature = "wasm-test-network", debug_assertions))]
    if let Some(script) = resolver_script(_id) {
        return format!("{}:{}", script.host, script.port);
    }
    format!("localhost:{port}")
}

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
        "wasm.net.outbound_cap" => NetworkCase {
            command: "write-cap",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.outbound_cap_no_reader" => NetworkCase {
            command: "write-strict",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.outbound_cap_straddle" => NetworkCase {
            command: "write-straddle",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.outbound_cap_vectored" => NetworkCase {
            command: "write-vectored",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.outbound_session_cap" => NetworkCase {
            command: "write-session",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.inbound_cap" => NetworkCase {
            command: "read-cap",
            target: NetworkTarget::Allowed,
            grant: NetworkGrant::Local,
        },
        "wasm.net.inbound_cap_no_sender" => NetworkCase {
            command: "read-strict",
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
        "wasm.net.rebind_private" => NetworkCase {
            command: "resolve-list",
            target: NetworkTarget::Hostname,
            grant: NetworkGrant::Hostname,
        },
        "wasm.net.rebind_public" => NetworkCase {
            command: "resolve-list",
            target: NetworkTarget::Hostname,
            grant: NetworkGrant::Hostname,
        },
        "wasm.net.mixed_answers" => NetworkCase {
            command: "resolve-list",
            target: NetworkTarget::Hostname,
            grant: NetworkGrant::Hostname,
        },
        "wasm.net.all_private" => NetworkCase {
            command: "resolve-list",
            target: NetworkTarget::Hostname,
            grant: NetworkGrant::Hostname,
        },
        _ => panic!("missing network case: {id}"),
    }
}
