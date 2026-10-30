use crate::kawi::plugin::types::{Argument, Error};
use wasi::io::streams::{InputStream, OutputStream};
use wasi::sockets::network::{
    ErrorCode as WasiErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress,
};
use wasi::sockets::tcp::TcpSocket;

use crate::{failure, text, SPLICE_PREFIX, WRITE_CHUNK};

struct WasiConnection {
    input: InputStream,
    output: OutputStream,
    _socket: TcpSocket,
}

pub(crate) fn splice_cap(address: &str) -> Result<Argument, Error> {
    let source = wasi_connect(address)?;
    let spare = wasi_connect(address)?;
    let destination = wasi_connect(address)?;
    let mut spliced = 0_u64;
    while spliced < SPLICE_PREFIX as u64 {
        let remaining = SPLICE_PREFIX as u64 - spliced;
        let request = remaining.min(WRITE_CHUNK as u64);
        let count = destination
            .output
            .blocking_splice(&source.input, request)
            .map_err(failure)?;
        if count == 0 {
            return Err(failure("splice returned zero"));
        }
        spliced += count;
    }
    let straddle = destination
        .output
        .blocking_splice(&source.input, 2)
        .map_err(failure)?;
    let next = match destination.output.blocking_splice(&spare.input, 1) {
        Ok(count) => format!("ok:{count}"),
        Err(_) => "closed".to_string(),
    };
    Ok(text(format!("{spliced}:{straddle}:{next}")))
}

fn wasi_connect(address: &str) -> Result<WasiConnection, Error> {
    let port = parse_port(address)?;
    let network = wasi::sockets::instance_network::instance_network();
    let socket = wasi::sockets::tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4)
        .map_err(|error| failure(format!("create socket: {}", error.name())))?;
    let remote = IpSocketAddress::Ipv4(Ipv4SocketAddress {
        port,
        address: (127, 0, 0, 1),
    });
    match socket.start_connect(&network, remote) {
        Ok(()) | Err(WasiErrorCode::WouldBlock) => {}
        Err(error) => return Err(failure(format!("start connect: {}", error.name()))),
    }
    let (input, output) = finish_wasi_connect(&socket)?;
    Ok(WasiConnection {
        input,
        output,
        _socket: socket,
    })
}

fn finish_wasi_connect(socket: &TcpSocket) -> Result<(InputStream, OutputStream), Error> {
    loop {
        match socket.finish_connect() {
            Ok(streams) => return Ok(streams),
            Err(WasiErrorCode::WouldBlock) => socket.subscribe().block(),
            Err(error) => return Err(failure(format!("finish connect: {}", error.name()))),
        }
    }
}

fn parse_port(address: &str) -> Result<u16, Error> {
    address
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
        .ok_or_else(|| failure("invalid socket address"))
}
