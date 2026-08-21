use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{HarnessError, Result};
use crate::lease::{LeaseBroker, SocketLease};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketTranscript {
    pub request: serde_json::Value,
    pub response: serde_json::Value,
    pub request_bytes: u64,
}

#[derive(Debug)]
pub struct SocketPeer {
    lease: SocketLease,
    worker: JoinHandle<Result<SocketTranscript>>,
}

impl SocketPeer {
    pub fn start(
        broker: &mut LeaseBroker,
        name: &str,
        reply: serde_json::Value,
        max_request_bytes: u64,
        timeout: Duration,
    ) -> Result<Self> {
        let lease = broker.lease_socket(name)?;
        let listener = lease.listener()?;
        listener
            .set_nonblocking(true)
            .map_err(|error| HarnessError::io("set Unix listener nonblocking", error))?;
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + timeout;
            let mut connection = loop {
                match listener.accept() {
                    Ok((connection, _)) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            return Err(HarnessError::Process(
                                "Unix socket peer timed out waiting for connection".into(),
                            ));
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => {
                        return Err(HarnessError::io("accept Unix socket connection", error));
                    }
                }
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            connection
                .set_read_timeout(Some(remaining))
                .map_err(|error| HarnessError::io("set Unix socket read timeout", error))?;
            connection
                .set_write_timeout(Some(remaining))
                .map_err(|error| HarnessError::io("set Unix socket write timeout", error))?;
            read_and_reply(&mut connection, reply, max_request_bytes)
        });
        Ok(Self { lease, worker })
    }

    pub fn path(&self) -> &Path {
        self.lease.path()
    }

    pub fn finish(self) -> Result<SocketTranscript> {
        self.worker
            .join()
            .map_err(|_| HarnessError::Process("Unix socket peer thread panicked".into()))?
    }
}

fn read_and_reply(
    connection: &mut UnixStream,
    response: serde_json::Value,
    max_request_bytes: u64,
) -> Result<SocketTranscript> {
    let reader_stream = connection
        .try_clone()
        .map_err(|error| HarnessError::io("clone Unix socket stream", error))?;
    let mut reader = BufReader::new(reader_stream);
    let mut request = Vec::new();
    reader
        .by_ref()
        .take(max_request_bytes + 1)
        .read_until(b'\n', &mut request)
        .map_err(|error| HarnessError::io("read Unix socket JSON line", error))?;
    if request.len() as u64 > max_request_bytes {
        return Err(HarnessError::Invalid(format!(
            "Unix socket request exceeds {max_request_bytes} bytes"
        )));
    }
    if request.last() != Some(&b'\n') {
        return Err(HarnessError::Invalid(
            "Unix socket request ended without newline".into(),
        ));
    }
    let parsed = serde_json::from_slice(&request[..request.len() - 1])
        .map_err(|error| HarnessError::Json(format!("Unix socket request: {error}")))?;
    let mut response_bytes =
        serde_json::to_vec(&response).map_err(|error| HarnessError::Json(error.to_string()))?;
    response_bytes.push(b'\n');
    connection
        .write_all(&response_bytes)
        .map_err(|error| HarnessError::io("write Unix socket JSON reply", error))?;
    connection
        .flush()
        .map_err(|error| HarnessError::io("flush Unix socket JSON reply", error))?;
    Ok(SocketTranscript {
        request: parsed,
        response,
        request_bytes: request.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    use serde_json::json;

    use super::*;

    #[test]
    fn oversized_request_is_rejected() {
        let mut broker = LeaseBroker::new().unwrap();
        let peer = SocketPeer::start(
            &mut broker,
            "oversized",
            json!({"ok": true}),
            4,
            Duration::from_secs(1),
        )
        .unwrap();
        let mut client = UnixStream::connect(peer.path()).unwrap();
        client.write_all(b"12345\n").unwrap();
        let error = peer.finish().unwrap_err().to_string();
        assert!(error.contains("exceeds 4 bytes"), "{error}");
    }

    #[test]
    fn missing_connection_times_out() {
        let mut broker = LeaseBroker::new().unwrap();
        let peer = SocketPeer::start(
            &mut broker,
            "timeout",
            json!({"ok": true}),
            1024,
            Duration::from_millis(20),
        )
        .unwrap();
        let error = peer.finish().unwrap_err().to_string();
        assert!(error.contains("timed out waiting"), "{error}");
    }
}
