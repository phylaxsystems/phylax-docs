use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};

/// Parse a [SocketAddr] from a `str`.
///
/// The following formats are checked:
///
/// - If the value can be parsed as a `u16` or starts with `:` it is considered a port, and the
/// hostname is set to `localhost`.
/// - If the value contains `:` it is assumed to be the format `<host>:<port>`
/// - Otherwise it is assumed to be a hostname
///
/// An error is returned if the value is empty.
///
/// This is taken from reth and is distributed under the MIT license.
/// source: https://github.com/paradigmxyz/reth/blob/b823cc01778fd364a78279a2979f84ef79f954eb/bin/reth/src/args/utils.rs#L87
pub fn parse_socket_address(value: &str) -> eyre::Result<SocketAddr, SocketAddressParsingError> {
    if value.is_empty() {
        return Err(SocketAddressParsingError::Empty);
    }

    if let Some(port) = value.strip_prefix(':').or_else(|| value.strip_prefix("localhost:")) {
        let port: u16 = port.parse()?;
        return Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
    }
    if let Ok(port) = value.parse::<u16>() {
        return Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
    }
    value
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| SocketAddressParsingError::Parse(value.to_string()))
}

/// Error thrown while parsing a socket address.
#[derive(thiserror::Error, Debug)]
pub enum SocketAddressParsingError {
    /// Failed to convert the string into a socket addr
    #[error("Cannot parse socket address: {0}")]
    Io(#[from] std::io::Error),
    /// Input must not be empty
    #[error("Cannot parse socket address from empty string")]
    Empty,
    /// Failed to parse the address
    #[error("Could not parse socket address from {0}")]
    Parse(String),
    /// Failed to parse port
    #[error("Could not parse port: {0}")]
    Port(#[from] std::num::ParseIntError),
}
