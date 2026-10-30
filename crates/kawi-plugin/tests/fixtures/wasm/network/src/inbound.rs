use std::io::{Read, Write};
use std::net::TcpStream;

use crate::kawi::plugin::types::{Argument, Error};

use crate::outbound::write_total;
use crate::{failure, text, INBOUND_CAP, MIXED_CONNECTIONS, OUTBOUND_CAP, READ_CHUNK};

pub(crate) fn read_until_eof(address: &str) -> Result<Argument, Error> {
    let (read, outcome) = read_bytes(address)?;
    Ok(text(format!("{read}:{outcome}")))
}

pub(crate) fn read_strict(address: &str) -> Result<Argument, Error> {
    let (_, outcome) = read_bytes(address)?;
    Err(failure(outcome))
}

pub(crate) fn read_straddle(address: &str) -> Result<Argument, Error> {
    let mut stream = TcpStream::connect(address).map_err(failure)?;
    let mut buffer = [0_u8; READ_CHUNK];
    let prefix = INBOUND_CAP - 1;
    read_total(&mut stream, prefix)?;
    let straddle = stream.read(&mut buffer[..2]).map_err(failure)?;
    let next = match stream.read(&mut buffer[..1]) {
        Ok(0) => "eof".to_string(),
        Ok(count) => format!("ok:{count}"),
        Err(error) => format!("error:{error}"),
    };
    Ok(text(format!("{prefix}:{straddle}:{next}")))
}

fn read_bytes(address: &str) -> Result<(usize, String), Error> {
    let mut stream = TcpStream::connect(address).map_err(failure)?;
    let mut buffer = [0_u8; READ_CHUNK];
    let mut read = 0;
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok((read, "eof".to_string())),
            Ok(count) => read += count,
            Err(error) => return Ok((read, error.to_string())),
        }
    }
}

fn read_total(stream: &mut TcpStream, limit: usize) -> Result<usize, Error> {
    let mut buffer = [0_u8; READ_CHUNK];
    let mut read = 0;
    while read < limit {
        let remaining = (limit - read).min(READ_CHUNK);
        read_exact(stream, &mut buffer[..remaining])?;
        read += remaining;
    }
    Ok(read)
}

fn read_exact(stream: &mut TcpStream, bytes: &mut [u8]) -> Result<(), Error> {
    let mut offset = 0;
    while offset < bytes.len() {
        match stream.read(&mut bytes[offset..]) {
            Ok(0) => return Err(failure("read returned zero")),
            Ok(count) => offset += count,
            Err(error) => return Err(failure(error)),
        }
    }
    Ok(())
}

pub(crate) fn mixed_cap(address: &str) -> Result<Argument, Error> {
    let mut read = 0;
    for _ in 0..MIXED_CONNECTIONS {
        let mut stream = TcpStream::connect(address).map_err(failure)?;
        read += read_total(&mut stream, OUTBOUND_CAP)?;
    }
    let mut written = 0;
    for _ in 0..MIXED_CONNECTIONS {
        let mut stream = TcpStream::connect(address).map_err(failure)?;
        written += write_total(&mut stream, OUTBOUND_CAP)?;
    }
    let mut stream = TcpStream::connect(address).map_err(failure)?;
    let next_write = match stream.write(&[1]) {
        Ok(0) => "zero".to_string(),
        Ok(count) => format!("ok:{count}"),
        Err(_) => "closed".to_string(),
    };
    Ok(text(format!("{read}:{written}:{next_write}")))
}
