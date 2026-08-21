use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::error::{HarnessError, Result};

#[derive(Debug)]
pub struct LeaseBroker {
    root: TempDir,
    next_socket: u64,
}

#[derive(Debug)]
pub struct PortLease {
    listener: TcpListener,
}

#[derive(Debug)]
pub struct SocketLease {
    listener: UnixListener,
    path: PathBuf,
}

impl LeaseBroker {
    pub fn new() -> Result<Self> {
        let root = tempfile::Builder::new()
            .prefix("th-")
            .tempdir()
            .map_err(|error| HarnessError::io("create lease broker root", error))?;
        Ok(Self {
            root,
            next_socket: 0,
        })
    }

    pub fn lease_port(&self) -> Result<PortLease> {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .map_err(|error| HarnessError::io("bind held loopback port", error))?;
        Ok(PortLease { listener })
    }

    pub fn lease_socket(&mut self, name: &str) -> Result<SocketLease> {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(HarnessError::Invalid(format!(
                "invalid socket lease name: {name}"
            )));
        }
        self.next_socket += 1;
        let path = self
            .root
            .path()
            .join(format!("{}-{name}.sock", self.next_socket));
        let listener = UnixListener::bind(&path).map_err(|error| {
            HarnessError::io(format!("bind held Unix socket {}", path.display()), error)
        })?;
        Ok(SocketLease { listener, path })
    }
}

impl PortLease {
    pub fn port(&self) -> Result<u16> {
        self.listener
            .local_addr()
            .map(|address| address.port())
            .map_err(|error| HarnessError::io("read leased port", error))
    }

    pub fn listener(&self) -> Result<TcpListener> {
        self.listener
            .try_clone()
            .map_err(|error| HarnessError::io("clone leased port listener", error))
    }
}

impl SocketLease {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn listener(&self) -> Result<UnixListener> {
        self.listener
            .try_clone()
            .map_err(|error| HarnessError::io("clone leased Unix listener", error))
    }
}

impl Drop for SocketLease {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
    use std::os::unix::net::UnixStream;

    use super::*;

    #[test]
    fn port_remains_reserved_until_lease_drops() {
        let broker = LeaseBroker::new().unwrap();
        let lease = broker.lease_port().unwrap();
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, lease.port().unwrap());
        assert!(TcpListener::bind(address).is_err());
        drop(lease);
        assert!(TcpListener::bind(address).is_ok());
    }

    #[test]
    fn unix_socket_path_is_short_held_and_removed() {
        let mut broker = LeaseBroker::new().unwrap();
        let lease = broker.lease_socket("window").unwrap();
        let path = lease.path().to_owned();
        assert!(path.as_os_str().len() < 100);
        assert!(UnixStream::connect(&path).is_ok());
        drop(lease);
        assert!(!path.exists());
    }
}
