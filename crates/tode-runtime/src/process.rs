use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerState {
    pub pid: i32,
    pub port: u16,
    #[serde(rename = "injectorPid")]
    pub injector_pid: i32,
    #[serde(rename = "injectorPort")]
    pub injector_port: u16,
    pub version: String,
    #[serde(rename = "startedAt")]
    pub started_at: u128,
}

pub fn read_state(path: &Path) -> Option<ServerState> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn write_state(path: &Path, state: &ServerState) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "state path has no parent")
    })?;
    fs::create_dir_all(parent)?;
    let mut bytes = serde_json::to_vec_pretty(state).expect("server state serializes");
    bytes.push(b'\n');
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

pub fn pid_running(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    match kill(Pid::from_raw(pid), None) {
        Ok(()) | Err(Errno::EPERM) => true,
        Err(_) => false,
    }
}

pub async fn answering(port: u16, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, TcpStream::connect(("127.0.0.1", port)))
        .await
        .is_ok_and(|connection| connection.is_ok())
}

pub async fn current_server(path: &Path, timeout: Duration) -> Option<ServerState> {
    let state = read_state(path)?;
    if !pid_running(state.pid) || !pid_running(state.injector_pid) {
        return None;
    }
    let (upstream, injector) = tokio::join!(
        answering(state.port, timeout),
        answering(state.injector_port, timeout)
    );
    (upstream && injector).then_some(state)
}

pub async fn wait_ready(port: u16, pid: i32, deadline: Duration) -> bool {
    let until = tokio::time::Instant::now() + deadline;
    while tokio::time::Instant::now() < until {
        if answering(port, Duration::from_millis(400)).await {
            return true;
        }
        if !pid_running(pid) {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    false
}

pub fn stop_server(path: &Path) -> bool {
    let Some(state) = read_state(path) else {
        return false;
    };
    let mut stopped = false;
    for pid in [state.injector_pid, state.pid] {
        if pid_running(pid) && kill(Pid::from_raw(pid), Signal::SIGTERM).is_ok() {
            stopped = true;
        }
    }
    let _ = fs::remove_file(path);
    stopped
}

pub fn origin(state: &ServerState) -> String {
    format!("http://127.0.0.1:{}/", state.injector_port)
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use tempfile::TempDir;
    use tokio::net::TcpListener;

    use super::*;

    fn state(pid: i32, port: u16, injector_port: u16) -> ServerState {
        ServerState {
            pid,
            port,
            injector_pid: pid,
            injector_port,
            version: "v1".into(),
            started_at: 123,
        }
    }

    #[test]
    fn state_round_trips_and_origin_uses_injector() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("state/server.json");
        let expected = state(std::process::id() as i32, 3000, 4000);
        write_state(&path, &expected).unwrap();
        assert_eq!(read_state(&path), Some(expected.clone()));
        assert_eq!(origin(&expected), "http://127.0.0.1:4000/");
    }

    #[test]
    fn liveness_rejects_invalid_pids() {
        assert!(pid_running(std::process::id() as i32));
        assert!(!pid_running(0));
        assert!(!pid_running(i32::MAX));
    }

    #[tokio::test]
    async fn current_server_requires_both_listeners() {
        let upstream = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let injector = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let root = TempDir::new().unwrap();
        let path = root.path().join("server.json");
        let expected = state(
            std::process::id() as i32,
            upstream.local_addr().unwrap().port(),
            injector.local_addr().unwrap().port(),
        );
        write_state(&path, &expected).unwrap();
        assert_eq!(
            current_server(&path, Duration::from_millis(100)).await,
            Some(expected)
        );
        drop(injector);
        assert!(
            current_server(&path, Duration::from_millis(20))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn wait_ready_observes_delayed_listener() {
        let reservation = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let address = reservation.local_addr().unwrap();
        drop(reservation);
        let delayed = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            TcpListener::bind(address).await.unwrap()
        });
        assert!(
            wait_ready(
                address.port(),
                std::process::id() as i32,
                Duration::from_secs(1)
            )
            .await
        );
        drop(delayed.await.unwrap());
    }

    #[test]
    fn stop_removes_stale_state_without_signalling() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("server.json");
        write_state(&path, &state(i32::MAX, 1, 2)).unwrap();
        assert!(!stop_server(&path));
        assert!(!path.exists());
    }
}
