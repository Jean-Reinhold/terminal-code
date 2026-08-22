use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredBuild {
    pub file: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredManifest {
    pub version: String,
    pub channel: String,
    pub published: String,
    pub platforms: BTreeMap<String, StoredBuild>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    pub body: Arc<[u8]>,
    pub content_type: String,
    pub etag: String,
}

pub trait ObjectStore {
    fn get(&self, key: &str) -> Option<StoredObject>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub origin: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Arc<[u8]>,
}

impl Response {
    fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: BTreeMap::from([("content-type".into(), "text/plain; charset=utf-8".into())]),
            body: Arc::from(body.as_bytes()),
        }
    }
}

pub fn handle(request: &Request, store: &impl ObjectStore) -> Response {
    if request.method != "GET" && request.method != "HEAD" {
        return Response::text(405, "method not allowed\n");
    }
    let requested = request.path.trim_end_matches('/');
    let prefix = if requested == "/install" || requested.starts_with("/install/") {
        "/install"
    } else {
        ""
    };
    let path = &requested[prefix.len()..];
    let segments: Vec<_> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let base = format!("{}{}", request.origin.trim_end_matches('/'), prefix);
    let mut response = match segments.as_slice() {
        ["dl", channel, version, file] if channel_valid(channel) => {
            download(request, store, channel, version, file)
        }
        ["latest.json"] => manifest_response(store, "stable/latest.json", &base, true),
        ["dev", "latest.json"] => manifest_response(store, "dev/latest.json", &base, true),
        ["v", version] => pinned_installer(store, version, &base),
        ["v", version, "manifest.json"] => pinned_manifest(store, version, &base),
        [] => latest_installer(store, "stable", &base),
        ["dev"] => latest_installer(store, "dev", &base),
        _ => Response::text(404, "not found\n"),
    };
    if request.method == "HEAD" {
        response.body = Arc::from([]);
    }
    response
}

fn channel_valid(channel: &str) -> bool {
    matches!(channel, "stable" | "dev")
}

fn read_manifest(store: &impl ObjectStore, key: &str) -> Option<StoredManifest> {
    serde_json::from_slice(&store.get(key)?.body).ok()
}

fn public_manifest(manifest: &StoredManifest, base: &str, install: bool) -> serde_json::Value {
    let platforms: serde_json::Map<_, _> = manifest
        .platforms
        .iter()
        .map(|(target, build)| {
            (
                target.clone(),
                serde_json::json!({
                    "file": build.file,
                    "sha256": build.sha256,
                    "size": build.size,
                    "url": format!("{base}/dl/{}/{}/{}", manifest.channel, manifest.version, build.file),
                }),
            )
        })
        .collect();
    let mut value = serde_json::json!({
        "version": manifest.version,
        "channel": manifest.channel,
        "published": manifest.published,
        "platforms": platforms,
    });
    if install {
        value["install"] = serde_json::json!(if manifest.channel == "dev" {
            format!("{base}/dev")
        } else {
            base.to_owned()
        });
    }
    value
}

fn manifest_response(store: &impl ObjectStore, key: &str, base: &str, install: bool) -> Response {
    let Some(manifest) = read_manifest(store, key) else {
        return Response::text(404, "not found\n");
    };
    json_response(&public_manifest(&manifest, base, install))
}

fn pinned_manifest(store: &impl ObjectStore, version: &str, base: &str) -> Response {
    for channel in ["stable", "dev"] {
        let key = format!("{channel}/{version}/manifest.json");
        if let Some(manifest) = read_manifest(store, &key) {
            return json_response(&public_manifest(&manifest, base, false));
        }
    }
    Response::text(404, "not found\n")
}

fn latest_installer(store: &impl ObjectStore, channel: &str, base: &str) -> Response {
    let Some(manifest) = read_manifest(store, &format!("{channel}/latest.json")) else {
        return Response::text(503, "no release published yet\n");
    };
    installer(store, &manifest, base)
}

fn pinned_installer(store: &impl ObjectStore, version: &str, base: &str) -> Response {
    for channel in ["stable", "dev"] {
        let key = format!("{channel}/{version}/manifest.json");
        if let Some(manifest) = read_manifest(store, &key) {
            return installer(store, &manifest, base);
        }
    }
    Response::text(404, "not found\n")
}

fn installer(store: &impl ObjectStore, manifest: &StoredManifest, base: &str) -> Response {
    let key = format!("{}/{}/install.sh", manifest.channel, manifest.version);
    let Some(object) = store.get(&key) else {
        return Response::text(500, "installer missing for this release\n");
    };
    let Ok(source) = std::str::from_utf8(&object.body) else {
        return Response::text(500, "installer missing for this release\n");
    };
    let table = manifest
        .platforms
        .iter()
        .map(|(target, build)| {
            format!(
                "{target} {base}/dl/{}/{}/{} {} {}",
                manifest.channel, manifest.version, build.file, build.sha256, build.size
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = source
        .replace("__PLATFORMS__", &table)
        .replace("__VERSION__", &manifest.version)
        .replace("__CHANNEL__", &manifest.channel);
    Response {
        status: 200,
        headers: BTreeMap::from([
            (
                "content-type".into(),
                "text/x-shellscript; charset=utf-8".into(),
            ),
            ("cache-control".into(), "no-store".into()),
        ]),
        body: Arc::from(body.into_bytes()),
    }
}

fn json_response(value: &serde_json::Value) -> Response {
    let mut body = serde_json::to_vec_pretty(value).expect("public manifest serializes");
    body.push(b'\n');
    Response {
        status: 200,
        headers: BTreeMap::from([
            ("content-type".into(), "application/json".into()),
            ("cache-control".into(), "no-store".into()),
        ]),
        body: Arc::from(body),
    }
}

fn download(
    request: &Request,
    store: &impl ObjectStore,
    channel: &str,
    version: &str,
    file: &str,
) -> Response {
    let Some(object) = store.get(&format!("{channel}/{version}/{file}")) else {
        return Response::text(404, "not found\n");
    };
    if request
        .headers
        .get("if-none-match")
        .is_some_and(|etag| etag == &object.etag)
    {
        return Response {
            status: 304,
            headers: download_headers(&object, object.body.len()),
            body: Arc::from([]),
        };
    }
    if let Some(range) = request.headers.get("range") {
        let Some((start, end)) = parse_range(range, object.body.len()) else {
            return Response::text(416, "range not satisfiable\n");
        };
        let body: Arc<[u8]> = Arc::from(&object.body[start..=end]);
        let mut headers = download_headers(&object, body.len());
        headers.insert(
            "content-range".into(),
            format!("bytes {start}-{end}/{}", object.body.len()),
        );
        return Response {
            status: 206,
            headers,
            body,
        };
    }
    Response {
        status: 200,
        headers: download_headers(&object, object.body.len()),
        body: object.body,
    }
}

fn download_headers(object: &StoredObject, length: usize) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("content-type".into(), object.content_type.clone()),
        ("content-length".into(), length.to_string()),
        ("etag".into(), object.etag.clone()),
        (
            "cache-control".into(),
            "public, max-age=31536000, immutable".into(),
        ),
        ("accept-ranges".into(), "bytes".into()),
    ])
}

fn parse_range(value: &str, size: usize) -> Option<(usize, usize)> {
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<usize>().ok()?.min(size);
        return (suffix > 0).then_some((size - suffix, size - 1));
    }
    let start = start.parse::<usize>().ok()?;
    if start >= size {
        return None;
    }
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<usize>().ok()?.min(size - 1)
    };
    (start <= end).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryStore(BTreeMap<String, StoredObject>);

    impl ObjectStore for MemoryStore {
        fn get(&self, key: &str) -> Option<StoredObject> {
            self.0.get(key).cloned()
        }
    }

    fn store() -> MemoryStore {
        let manifest = StoredManifest {
            version: "v2".into(),
            channel: "stable".into(),
            published: "2026-01-01T00:00:00Z".into(),
            platforms: BTreeMap::from([(
                "darwin-arm64".into(),
                StoredBuild {
                    file: "tode-darwin-arm64.tar.gz".into(),
                    sha256: "abc".into(),
                    size: 10,
                },
            )]),
        };
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let object = |body: Vec<u8>, content_type: &str, etag: &str| StoredObject {
            body: Arc::from(body),
            content_type: content_type.into(),
            etag: etag.into(),
        };
        MemoryStore(BTreeMap::from([
            (
                "stable/latest.json".into(),
                object(manifest_bytes.clone(), "application/json", "m1"),
            ),
            (
                "stable/v2/manifest.json".into(),
                object(manifest_bytes, "application/json", "m2"),
            ),
            (
                "stable/v2/install.sh".into(),
                object(
                    b"V=__VERSION__ C=__CHANNEL__\n__PLATFORMS__\n".to_vec(),
                    "text/x-shellscript",
                    "i1",
                ),
            ),
            (
                "stable/v2/tode-darwin-arm64.tar.gz".into(),
                object(b"0123456789".to_vec(), "application/gzip", "a1"),
            ),
        ]))
    }

    fn request(method: &str, path: &str) -> Request {
        Request {
            method: method.into(),
            path: path.into(),
            origin: "https://tode.test".into(),
            headers: BTreeMap::new(),
        }
    }

    #[test]
    fn serves_root_prefixed_and_pinned_installers() {
        let store = store();
        for path in ["/", "/install", "/v/v2", "/install/v/v2"] {
            let response = handle(&request("GET", path), &store);
            assert_eq!(response.status, 200, "{path}");
            let text = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(text.contains("V=v2 C=stable"));
            assert!(text.contains(if path.starts_with("/install") {
                "https://tode.test/install/dl/stable/v2/tode-darwin-arm64.tar.gz"
            } else {
                "https://tode.test/dl/stable/v2/tode-darwin-arm64.tar.gz"
            }));
        }
    }

    #[test]
    fn derives_latest_and_pinned_manifest_urls() {
        let store = store();
        let latest = handle(&request("GET", "/install/latest.json"), &store);
        let value: serde_json::Value = serde_json::from_slice(&latest.body).unwrap();
        assert_eq!(value["install"], "https://tode.test/install");
        assert_eq!(
            value["platforms"]["darwin-arm64"]["url"],
            "https://tode.test/install/dl/stable/v2/tode-darwin-arm64.tar.gz"
        );
        let pinned = handle(&request("GET", "/v/v2/manifest.json"), &store);
        let value: serde_json::Value = serde_json::from_slice(&pinned.body).unwrap();
        assert!(value.get("install").is_none());
    }

    #[test]
    fn download_get_head_range_etag_and_cache_are_exact() {
        let store = store();
        let path = "/dl/stable/v2/tode-darwin-arm64.tar.gz";
        let full = handle(&request("GET", path), &store);
        assert_eq!(full.status, 200);
        assert_eq!(&*full.body, b"0123456789");
        assert_eq!(full.headers["content-length"], "10");
        assert_eq!(
            full.headers["cache-control"],
            "public, max-age=31536000, immutable"
        );
        let head = handle(&request("HEAD", path), &store);
        assert_eq!(head.status, 200);
        assert!(head.body.is_empty());
        assert_eq!(head.headers["content-length"], "10");
        let mut ranged = request("GET", path);
        ranged.headers.insert("range".into(), "bytes=2-5".into());
        let ranged = handle(&ranged, &store);
        assert_eq!(ranged.status, 206);
        assert_eq!(&*ranged.body, b"2345");
        assert_eq!(ranged.headers["content-range"], "bytes 2-5/10");
        let mut cached = request("GET", path);
        cached.headers.insert("if-none-match".into(), "a1".into());
        assert_eq!(handle(&cached, &store).status, 304);
    }

    #[test]
    fn rejects_methods_channels_ranges_and_missing_releases() {
        let store = store();
        assert_eq!(handle(&request("POST", "/"), &store).status, 405);
        assert_eq!(handle(&request("GET", "/dl/beta/v2/a"), &store).status, 404);
        let mut bad_range = request("GET", "/dl/stable/v2/tode-darwin-arm64.tar.gz");
        bad_range
            .headers
            .insert("range".into(), "bytes=99-100".into());
        assert_eq!(handle(&bad_range, &store).status, 416);
        assert_eq!(handle(&request("GET", "/missing"), &store).status, 404);
        assert_eq!(
            handle(&request("GET", "/"), &MemoryStore::default()).status,
            503
        );
    }

    #[test]
    fn missing_installer_is_500_with_exact_body() {
        let mut store = store();
        store.0.remove("stable/v2/install.sh");
        let response = handle(&request("GET", "/"), &store);
        assert_eq!(response.status, 500);
        assert_eq!(&*response.body, b"installer missing for this release\n");
    }
}
