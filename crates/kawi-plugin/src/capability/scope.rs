use std::collections::BTreeSet;
use std::fmt;
#[cfg(feature = "wasm")]
use std::net::IpAddr;
use std::net::Ipv6Addr;
use std::path::Path;

use serde_json::{Map, Value};

use super::command;

mod parse;

pub(super) use parse::parse_request_scope;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct HostPort {
    host: String,
    port: u16,
}

impl HostPort {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let (raw_host, port) = value
            .rsplit_once(':')
            .ok_or_else(|| "hosts must include a port".to_string())?;
        let port = port
            .parse::<u16>()
            .map_err(|_| "hosts must use host:port".to_string())?;
        let is_ipv6 = raw_host.starts_with('[') && raw_host.ends_with(']');
        let host = if is_ipv6 {
            &raw_host[1..raw_host.len() - 1]
        } else {
            raw_host
        };
        if host.is_empty() || port == 0 {
            return Err("hosts must use host:port".to_string());
        }
        if host.contains(['[', ']']) {
            return Err("invalid host brackets".to_string());
        }
        if is_ipv6 && host.parse::<Ipv6Addr>().is_err() {
            return Err("bracketed hosts must be IPv6 addresses".to_string());
        }
        if host.contains('*') {
            return Err("host wildcards are not supported".to_string());
        }
        if host.contains(':') && !is_ipv6 {
            return Err("IPv6 hosts must use brackets".to_string());
        }
        Ok(Self {
            host: host.to_ascii_lowercase(),
            port,
        })
    }

    #[cfg(feature = "wasm")]
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    #[cfg(feature = "wasm")]
    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    #[cfg(feature = "wasm")]
    pub(crate) fn ip(&self) -> Option<IpAddr> {
        self.host.parse().ok()
    }
}

impl fmt::Display for HostPort {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            write!(formatter, "[{}]:{}", self.host, self.port)
        } else {
            write!(formatter, "{}:{}", self.host, self.port)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Unscoped,
    Paths(Vec<String>),
    Commands(Vec<String>),
    EditorCommands(Vec<String>),
    Hosts(Vec<String>),
}

impl Scope {
    pub fn covers(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unscoped, Self::Unscoped) => true,
            (Self::Paths(outer), Self::Paths(inner)) => inner
                .iter()
                .all(|inner| outer.iter().any(|outer| path_within(inner, outer))),
            (Self::Commands(outer), Self::Commands(inner)) => command::covers(outer, inner),
            (Self::EditorCommands(outer), Self::EditorCommands(inner)) => {
                inner.iter().all(|inner| outer.contains(inner))
            }
            (Self::Hosts(outer), Self::Hosts(inner)) => {
                inner.iter().all(|inner| outer.contains(inner))
            }
            _ => false,
        }
    }

    pub fn to_json_object(&self) -> Map<String, Value> {
        let mut object = Map::new();
        match self {
            Self::Unscoped => {}
            Self::Paths(paths) => {
                object.insert("paths".to_string(), string_array(paths));
            }
            Self::Commands(commands) | Self::EditorCommands(commands) => {
                object.insert("commands".to_string(), string_array(commands));
            }
            Self::Hosts(hosts) => {
                object.insert("hosts".to_string(), string_array(hosts));
            }
        }
        object
    }
}

fn string_array(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

pub fn intersect_scope(requested: &Scope, granted: &Scope) -> Option<Scope> {
    match (requested, granted) {
        (Scope::Unscoped, Scope::Unscoped) => Some(Scope::Unscoped),
        (Scope::Paths(requested), Scope::Paths(granted)) => intersect_paths(requested, granted),
        (Scope::Commands(requested), Scope::Commands(granted)) => {
            intersect_commands(requested, granted)
        }
        (Scope::EditorCommands(requested), Scope::EditorCommands(granted)) => {
            intersect_editor_commands(requested, granted)
        }
        (Scope::Hosts(requested), Scope::Hosts(granted)) => intersect_hosts(requested, granted),
        _ => None,
    }
}

fn intersect_commands(requested: &[String], granted: &[String]) -> Option<Scope> {
    command::intersect(requested, granted).map(Scope::Commands)
}

fn intersect_editor_commands(requested: &[String], granted: &[String]) -> Option<Scope> {
    exact_values(requested, granted).map(Scope::EditorCommands)
}

fn intersect_hosts(requested: &[String], granted: &[String]) -> Option<Scope> {
    exact_values(requested, granted).map(Scope::Hosts)
}

fn exact_values(requested: &[String], granted: &[String]) -> Option<Vec<String>> {
    let requested = requested.iter().collect::<BTreeSet<_>>();
    let values = granted
        .iter()
        .filter(|value| requested.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

fn intersect_paths(requested: &[String], granted: &[String]) -> Option<Scope> {
    let values = granted
        .iter()
        .filter(|granted| {
            requested
                .iter()
                .any(|requested| path_within(granted, requested))
        })
        .cloned()
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(Scope::Paths(values))
}

fn path_within(granted: &str, requested: &str) -> bool {
    if granted == requested {
        return true;
    }
    let Ok(relative) = Path::new(granted).strip_prefix(requested) else {
        return false;
    };
    !relative.as_os_str().is_empty()
}
