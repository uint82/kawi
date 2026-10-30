#![no_main]

use std::net::{IpAddr, SocketAddr};

use kawi_plugin::fuzz::{
    host_port, host_port_ip, is_public_address, network_allows_bind, network_allows_connect,
    network_policy, network_resolve_name,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines();
    let hosts = list(lines.next().unwrap_or_default());
    let answers = answers(lines.next().unwrap_or_default());

    for host in &hosts {
        if let Ok((display, port)) = host_port(host) {
            assert!(port > 0);
            assert!(!display.contains('*'));
            let (again, again_port) = host_port(&display).expect("round trip");
            assert_eq!(again, display);
            assert_eq!(again_port, port);
        }
    }

    let summary = network_policy(&hosts, &answers);
    assert_eq!(summary.enabled, !hosts.is_empty());
    if summary.name_lookup {
        assert!(!summary.names.is_empty());
    }

    for address in &summary.allowed {
        let explicit = hosts.iter().any(|host| {
            host_port_ip(host).is_some_and(|ip| ip == address.ip())
                && host_port(host).is_ok_and(|(_, port)| port == address.port())
        });
        let resolved = hosts.iter().any(|host| {
            let Ok((_, port)) = host_port(host) else {
                return false;
            };
            host_port_ip(host).is_none()
                && port == address.port()
                && answers.iter().any(|answer| answer.ip() == address.ip())
        });
        assert!(explicit || resolved);
        if resolved && !explicit {
            assert!(is_public_address(address.ip()));
        }
        assert!(network_allows_connect(&hosts, &answers, *address));
    }

    for (name, addresses) in &summary.names {
        assert_eq!(name, &name.to_ascii_lowercase());
        assert!(addresses.iter().all(|ip| is_public_address(*ip)));
    }

    let bind = SocketAddr::new(IpAddr::from([0_u8; 4]), 0);
    assert_eq!(network_allows_bind(&hosts, &answers, bind), summary.enabled);
    assert!(!network_allows_bind(
        &hosts,
        &answers,
        SocketAddr::new(IpAddr::from([0_u8; 4]), 1)
    ));

    for name in ["example.com", "EXAMPLE.COM", "127.0.0.1", "::1"] {
        let resolved = network_resolve_name(&hosts, &answers, name);
        if !summary.name_lookup {
            assert!(resolved.is_err());
            continue;
        }
        if let Ok(ip) = name.parse::<IpAddr>() {
            assert_eq!(resolved, Ok(vec![ip]));
        } else if let Some(addresses) = summary.names.get(&name.to_ascii_lowercase()) {
            assert_eq!(resolved, Ok(addresses.clone()));
        } else {
            assert!(resolved.is_err());
        }
    }
});

fn list(line: &str) -> Vec<String> {
    line.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn answers(line: &str) -> Vec<SocketAddr> {
    line.split(',')
        .filter_map(|value| value.trim().parse().ok())
        .collect()
}
