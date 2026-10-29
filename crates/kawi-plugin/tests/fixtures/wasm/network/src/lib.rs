wit_bindgen::generate!({
    path: "../../../../wit",
    world: "host",
});

use std::io::{self, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::Mutex;

use exports::kawi::plugin::lifecycle::Guest;
use kawi::plugin::types::{Argument, Error, Event};

struct Plugin;

static HELD: Mutex<Option<TcpStream>> = Mutex::new(None);

const CAPABILITY_DENIED: i32 = -32012;
const NETWORK_CAPABILITY: &str = "net.connect";
const WRITE_LIMIT: usize = 9 * 1024 * 1024;
const WRITE_CHUNK: usize = 64 * 1024;

fn failure(error: impl std::fmt::Display) -> Error {
    Error {
        code: CAPABILITY_DENIED,
        message: error.to_string(),
        capability: Some(NETWORK_CAPABILITY.to_string()),
    }
}

fn text(value: impl Into<String>) -> Argument {
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
            "write-cap" => write_until_error(address),
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

fn write_until_error(address: &str) -> Result<Argument, Error> {
    let mut stream = TcpStream::connect(address).map_err(failure)?;
    let chunk = [0_u8; WRITE_CHUNK];
    let mut written = 0;
    let mut outcome = "none".to_string();
    while written < WRITE_LIMIT {
        match stream.write(&chunk) {
            Ok(0) => {
                outcome = "zero".to_string();
                break;
            }
            Ok(count) => written += count,
            Err(error) => {
                outcome = error.to_string();
                break;
            }
        }
    }
    Ok(text(format!("{written}:{outcome}")))
}

export!(Plugin);
