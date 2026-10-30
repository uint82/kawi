wit_bindgen::generate!({
    path: "../../../../../kawi-plugin-sdk/wit",
    world: "host",
});

mod inbound;
mod outbound;
mod splice;

use std::io;
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::Mutex;

use exports::kawi::plugin::lifecycle::Guest;
use kawi::plugin::types::{Argument, Error, Event};

struct Plugin;

static HELD: Mutex<Option<TcpStream>> = Mutex::new(None);

pub(crate) const CAPABILITY_DENIED: i32 = -32012;
pub(crate) const NETWORK_CAPABILITY: &str = "net.connect";
pub(crate) const INBOUND_CAP: usize = 8 * 1024 * 1024;
pub(crate) const OUTBOUND_CAP: usize = 8 * 1024 * 1024;
pub(crate) const WRITE_LIMIT: usize = 9 * 1024 * 1024;
pub(crate) const WRITE_CHUNK: usize = 256 * 1024;
pub(crate) const READ_CHUNK: usize = 256 * 1024;
pub(crate) const SESSION_CONNECTIONS: usize = 9;
pub(crate) const MIXED_CONNECTIONS: usize = 4;
pub(crate) const SESSION_CLOSE_CONNECTIONS: usize = 16;
pub(crate) const SESSION_CLOSE_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const SPLICE_PREFIX: usize = OUTBOUND_CAP - 1;

pub(crate) fn failure(error: impl std::fmt::Display) -> Error {
    Error {
        code: CAPABILITY_DENIED,
        message: error.to_string(),
        capability: Some(NETWORK_CAPABILITY.to_string()),
    }
}

pub(crate) fn text(value: impl Into<String>) -> Argument {
    Argument::Text(value.into())
}

fn argument(args: &[Argument]) -> Result<&str, Error> {
    match args.first() {
        Some(Argument::Text(value)) => Ok(value),
        _ => Err(failure("missing text argument")),
    }
}

impl Guest for Plugin {
    fn init(_host_protocol: u32) -> Result<(), Error> {
        Ok(())
    }

    fn invoke_command(id: String, args: Vec<Argument>) -> Result<Argument, Error> {
        if id == "ping" {
            return Ok(text("pong"));
        }
        let address = argument(&args)?;
        match id.as_str() {
            "connect" => TcpStream::connect(address)
                .map(|_| text("connected"))
                .map_err(failure),
            "connect-many" => {
                let mut streams = Vec::new();
                for _ in 0..65 {
                    streams.push(TcpStream::connect(address).map_err(failure)?);
                }
                Ok(text("connected"))
            }
            "connect-reuse" => {
                for _ in 0..100 {
                    TcpStream::connect(address).map_err(failure)?;
                }
                Ok(text("reused"))
            }
            "write-cap" => outbound::write_until_error(address),
            "write-strict" => outbound::write_strict(address),
            "write-straddle" => outbound::write_straddle(address),
            "write-vectored" => outbound::write_vectored_cap(address),
            "write-session" => outbound::write_session(address),
            "write-close" => outbound::write_close(address),
            "splice-cap" => splice::splice_cap(address),
            "read-cap" => inbound::read_until_eof(address),
            "read-straddle" => inbound::read_straddle(address),
            "read-strict" => inbound::read_strict(address),
            "mixed-cap" => inbound::mixed_cap(address),
            "hold" => TcpStream::connect(address)
                .map(|stream| {
                    *HELD.lock().expect("held socket lock") = Some(stream);
                    text("held")
                })
                .map_err(failure),
            "resolve" => address
                .to_socket_addrs()
                .map(|addresses| text(format!("{}", addresses.count())))
                .map_err(failure),
            "resolve-list" => address
                .to_socket_addrs()
                .map(|addresses| {
                    text(
                        addresses
                            .map(|address| address.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                })
                .map_err(failure),
            "udp" => UdpSocket::bind(address)
                .map(|_| text("bound"))
                .map_err(failure),
            "listen" => TcpListener::bind(address)
                .map(|_| text("listening"))
                .map_err(failure),
            "accept" => TcpListener::bind(address)
                .and_then(|listener| listener.accept().map(|_| ()))
                .map(|_| text("accepted"))
                .map_err(failure),
            _ => Err(failure(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown probe",
            ))),
        }
    }

    fn on_event(_event: Event) {}
}

export!(Plugin);
