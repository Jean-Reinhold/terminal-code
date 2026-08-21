use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use hyper::header::ACCEPT;

use crate::{
    CodeServerConfig, Injector, InjectorConfig, ManagedCodeServer, ManagedProcessError,
    ServerState, now_unix_ms, start_code_server, write_state,
};

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub code_server: CodeServerConfig,
    pub injector_listen: SocketAddr,
    pub css_file: PathBuf,
    pub font_file: Option<PathBuf>,
    pub injector_hold: Duration,
    pub state_file: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error(transparent)]
    CodeServer(#[from] ManagedProcessError),
    #[error("start injector: {0}")]
    Injector(#[from] crate::injector::InjectorError),
    #[error("write daemon state: {0}")]
    State(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Daemon {
    code_server: ManagedCodeServer,
    injector: Injector,
    state_file: PathBuf,
    pub state: ServerState,
}

impl Daemon {
    pub async fn start(config: DaemonConfig) -> Result<Self, DaemonError> {
        let code_server = start_code_server(&config.code_server).await?;
        let injector = match Injector::start(InjectorConfig {
            listen: config.injector_listen,
            upstream: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), code_server.port),
            css_file: config.css_file,
            font_file: config.font_file,
            hold: config.injector_hold,
        })
        .await
        {
            Ok(injector) => injector,
            Err(error) => {
                let _ = code_server.shutdown();
                return Err(DaemonError::Injector(error));
            }
        };
        let state = ServerState {
            pid: code_server.pid,
            port: code_server.port,
            injector_pid: std::process::id() as i32,
            injector_port: injector.address().port(),
            version: code_server.version.clone(),
            started_at: now_unix_ms(),
        };
        if let Err(error) = write_state(&config.state_file, &state) {
            injector.shutdown().await;
            let _ = code_server.shutdown();
            return Err(DaemonError::State(error));
        }
        let daemon = Self {
            code_server,
            injector,
            state_file: config.state_file,
            state,
        };
        daemon.warm_up().await;
        Ok(daemon)
    }

    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}/", self.state.injector_port)
    }

    pub async fn warm_up(&self) {
        let client = reqwest::Client::new();
        let Ok(response) = client
            .get(self.origin())
            .header(ACCEPT.as_str(), "text/html")
            .send()
            .await
        else {
            return;
        };
        let Ok(html) = response.text().await else {
            return;
        };
        let assets = asset_paths(&html);
        for asset in assets.into_iter().take(4) {
            let url = if asset.starts_with('/') {
                format!("{}{}", self.origin().trim_end_matches('/'), asset)
            } else {
                format!("{}{asset}", self.origin())
            };
            let _ = client.get(url).send().await;
        }
    }

    pub async fn shutdown(self) -> Result<(), DaemonError> {
        self.injector.shutdown().await;
        self.code_server.shutdown()?;
        match fs::remove_file(&self.state_file) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(DaemonError::State(error)),
        }
    }
}

fn asset_paths(html: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for marker in ["src=\"", "href=\""] {
        let mut rest = html;
        while let Some(start) = rest.find(marker) {
            rest = &rest[start + marker.len()..];
            let Some(end) = rest.find('"') else { break };
            let path = &rest[..end];
            if path.ends_with(".js") || path.ends_with(".css") {
                paths.push(path.to_owned());
            }
            rest = &rest[end + 1..];
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_script_and_style_assets() {
        assert_eq!(
            asset_paths(
                r#"<script src="/a.js"></script><link href="b.css"><img src="x.png"><a href="page">"#
            ),
            ["/a.js", "b.css"]
        );
    }
}
