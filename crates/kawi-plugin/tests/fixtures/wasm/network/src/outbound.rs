use std::io::{self, IoSlice, Write};
use std::net::TcpStream;

use crate::kawi::plugin::types::{Argument, Error};

use crate::{
    failure, text, OUTBOUND_CAP, SESSION_CLOSE_BYTES, SESSION_CLOSE_CONNECTIONS,
    SESSION_CONNECTIONS, WRITE_CHUNK, WRITE_LIMIT,
};

pub(crate) fn write_until_error(address: &str) -> Result<Argument, Error> {
    let (written, outcome) = write_bytes(address)?;
    Ok(text(format!("{written}:{outcome}")))
}

pub(crate) fn write_strict(address: &str) -> Result<Argument, Error> {
    let (_, outcome) = write_bytes(address)?;
    Err(failure(outcome))
}

pub(crate) fn write_straddle(address: &str) -> Result<Argument, Error> {
    let mut stream = TcpStream::connect(address).map_err(failure)?;
    let chunk = [0_u8; WRITE_CHUNK];
    let prefix = OUTBOUND_CAP - 1;
    let mut written = 0;
    while written + WRITE_CHUNK <= prefix {
        write_exact(&mut stream, &chunk)?;
        written += WRITE_CHUNK;
    }
    write_exact(&mut stream, &chunk[..prefix - written])?;
    let straddle = stream.write(&[1, 2]).map_err(failure)?;
    let next = match stream.write(&[3]) {
        Ok(count) => format!("ok:{count}"),
        Err(error) => format!("error:{error}"),
    };
    Ok(text(format!("{prefix}:{straddle}:{next}")))
}

fn write_exact(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), Error> {
    let mut offset = 0;
    while offset < bytes.len() {
        match stream.write(&bytes[offset..]) {
            Ok(0) => return Err(failure("write returned zero")),
            Ok(count) => offset += count,
            Err(error) => return Err(failure(error)),
        }
    }
    Ok(())
}

pub(crate) fn write_vectored_cap(address: &str) -> Result<Argument, Error> {
    let (written, outcome) = write_bytes_with(address, |stream, bytes| {
        let (first, second) = bytes.split_at(bytes.len() / 2);
        let slices = [IoSlice::new(first), IoSlice::new(second)];
        stream.write_vectored(&slices)
    })?;
    Ok(text(format!("{written}:{outcome}")))
}

fn write_bytes(address: &str) -> Result<(usize, String), Error> {
    write_bytes_with(address, TcpStream::write)
}

fn write_bytes_with(
    address: &str,
    write: impl FnMut(&mut TcpStream, &[u8]) -> io::Result<usize>,
) -> Result<(usize, String), Error> {
    let mut stream = TcpStream::connect(address).map_err(failure)?;
    write_stream(&mut stream, WRITE_LIMIT, write)
}

fn write_stream(
    stream: &mut TcpStream,
    limit: usize,
    mut write: impl FnMut(&mut TcpStream, &[u8]) -> io::Result<usize>,
) -> Result<(usize, String), Error> {
    let chunk = [0_u8; WRITE_CHUNK];
    let mut written = 0;
    while written < limit {
        let remaining = (limit - written).min(WRITE_CHUNK);
        match write(stream, &chunk[..remaining]) {
            Ok(0) => return Ok((written, "zero".to_string())),
            Ok(count) => written += count,
            Err(error) => return Ok((written, error.to_string())),
        }
    }
    Ok((written, "none".to_string()))
}

pub(crate) fn write_session(address: &str) -> Result<Argument, Error> {
    let mut results = Vec::with_capacity(SESSION_CONNECTIONS);
    for _ in 0..SESSION_CONNECTIONS {
        let mut stream = TcpStream::connect(address).map_err(failure)?;
        let (written, outcome) = write_stream(&mut stream, WRITE_LIMIT, TcpStream::write)?;
        results.push(format!("{written}:{outcome}"));
    }
    Ok(text(results.join(",")))
}

pub(crate) fn write_close(address: &str) -> Result<Argument, Error> {
    let mut results = Vec::with_capacity(SESSION_CLOSE_CONNECTIONS + 1);
    for _ in 0..=SESSION_CLOSE_CONNECTIONS {
        let mut stream = TcpStream::connect(address).map_err(failure)?;
        let (written, outcome) = write_stream(&mut stream, SESSION_CLOSE_BYTES, TcpStream::write)?;
        results.push(format!("{written}:{outcome}"));
    }
    Ok(text(results.join(",")))
}

pub(crate) fn write_total(stream: &mut TcpStream, limit: usize) -> Result<usize, Error> {
    let chunk = [0_u8; WRITE_CHUNK];
    let mut written = 0;
    while written < limit {
        let remaining = (limit - written).min(WRITE_CHUNK);
        match stream.write(&chunk[..remaining]) {
            Ok(0) => return Err(failure("write returned zero")),
            Ok(count) => written += count,
            Err(error) => return Err(failure(error)),
        }
    }
    Ok(written)
}
