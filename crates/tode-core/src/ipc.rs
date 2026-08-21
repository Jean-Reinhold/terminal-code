use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::OpenFile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub files: Vec<OpenFile>,
    pub folders: Vec<String>,
    pub add: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("the tode window did not answer")]
    Timeout,
    #[error("the window sent something unreadable")]
    Unreadable,
    #[error("{0}")]
    Refused(String),
    #[error("{0}")]
    Io(String),
}

#[derive(Debug, Deserialize)]
struct Reply {
    ok: Option<bool>,
    error: Option<String>,
}

pub fn send_to_extension(
    socket: &Path,
    request: &OpenRequest,
    timeout: Option<Duration>,
) -> Result<(), IpcError> {
    let mut connection =
        UnixStream::connect(socket).map_err(|error| IpcError::Io(error.to_string()))?;
    connection
        .set_read_timeout(timeout)
        .map_err(|error| IpcError::Io(error.to_string()))?;
    connection
        .set_write_timeout(timeout)
        .map_err(|error| IpcError::Io(error.to_string()))?;
    let mut request_bytes =
        serde_json::to_vec(request).map_err(|error| IpcError::Io(error.to_string()))?;
    request_bytes.push(b'\n');
    connection.write_all(&request_bytes).map_err(map_io)?;
    connection.flush().map_err(map_io)?;

    let mut reply_line = String::new();
    BufReader::new(connection)
        .read_line(&mut reply_line)
        .map_err(map_io)?;
    let reply: Reply =
        serde_json::from_str(reply_line.trim_end()).map_err(|_| IpcError::Unreadable)?;
    if reply.ok == Some(true) {
        Ok(())
    } else {
        Err(IpcError::Refused(
            reply.error.unwrap_or_else(|| "the window refused".into()),
        ))
    }
}

fn map_io(error: std::io::Error) -> IpcError {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        IpcError::Timeout
    } else {
        IpcError::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    fn request() -> OpenRequest {
        OpenRequest {
            files: Vec::new(),
            folders: vec!["/workspace".into()],
            add: false,
            wait: None,
            diff: None,
            view: None,
            theme: None,
        }
    }

    fn serve(
        response: &'static [u8],
        delay: Duration,
    ) -> (TempDir, std::path::PathBuf, thread::JoinHandle<String>) {
        let root = TempDir::new().unwrap();
        let path = root.path().join("window.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let worker = thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(connection.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            thread::sleep(delay);
            let _ = connection.write_all(response);
            line
        });
        (root, path, worker)
    }

    #[test]
    fn sends_one_json_line_and_accepts_success() {
        let (_root, path, worker) = serve(b"{\"ok\":true}\n", Duration::ZERO);
        send_to_extension(&path, &request(), Some(Duration::from_secs(1))).unwrap();
        let line = worker.join().unwrap();
        assert!(line.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(value["folders"][0], "/workspace");
        assert_eq!(value["add"], false);
        assert!(value.get("wait").is_none());
    }

    #[test]
    fn preserves_refusal_and_unreadable_errors() {
        let (_root, path, worker) = serve(b"{\"ok\":false,\"error\":\"nope\"}\n", Duration::ZERO);
        assert_eq!(
            send_to_extension(&path, &request(), Some(Duration::from_secs(1)))
                .unwrap_err()
                .to_string(),
            "nope"
        );
        worker.join().unwrap();

        let (_root, path, worker) = serve(b"not-json\n", Duration::ZERO);
        assert_eq!(
            send_to_extension(&path, &request(), Some(Duration::from_secs(1)))
                .unwrap_err()
                .to_string(),
            "the window sent something unreadable"
        );
        worker.join().unwrap();
    }

    #[test]
    fn maps_read_timeout_to_window_timeout() {
        let (_root, path, worker) = serve(b"{\"ok\":true}\n", Duration::from_millis(100));
        assert_eq!(
            send_to_extension(&path, &request(), Some(Duration::from_millis(10)))
                .unwrap_err()
                .to_string(),
            "the tode window did not answer"
        );
        worker.join().unwrap();
    }

    #[test]
    fn missing_socket_reports_io_error() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("missing.sock");
        let error = send_to_extension(&path, &request(), Some(Duration::from_millis(10)))
            .unwrap_err()
            .to_string();
        assert!(!error.is_empty());
        assert!(!path.exists());
        fs::remove_dir_all(root.path()).unwrap();
    }
}
