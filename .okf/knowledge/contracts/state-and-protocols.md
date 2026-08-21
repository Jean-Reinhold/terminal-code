---
type: Protocol Contract
title: State and Protocol Compatibility
description: Persisted paths, wire formats, host adapters, and release HTTP interfaces.
tags: [protocol, state, ipc, release]
status: draft
sources:
  - id: paths
    resource: ../../../src/runtime/paths.ts
    title: XDG and install paths
  - id: ipc
    resource: ../../../src/ipc.ts
    title: Window IPC schema and framing
  - id: osc
    resource: ../../../src/terminal/osc.ts
    title: Terminal OSC protocol
  - id: release
    resource: ../../../release-worker/src/index.ts
    title: Release HTTP API
---

# Canonical Locations

Environment variables are honored only when their values are absolute, matching current behavior.

| Logical state | Current location |
|---|---|
| Install | `${TODE_INSTALL_ROOT}` or `~/.local/lib/tode` |
| Data | `${XDG_DATA_HOME}` or `~/.local/share`, then `tode/` |
| State | `${XDG_STATE_HOME}` or `~/.local/state`, then `tode/` |
| Cache | `${XDG_CACHE_HOME}` or `~/.cache`, then `tode/` |
| Runtime | data `runtime/` |
| Logs | state `logs/` |
| Browser data/state/cache | tode data/state/cache `browser/` subtrees |
| Shim | `${XDG_BIN_HOME}` or `~/.local/bin/tode` |

Stable files include `server.json`, `inject.css`, `injector.port`, `palette.json`, `live-theme.json`, `startup-open.json`, `shortcuts.json`, and `keybindings.tode.json`. Their names, ownership, and read compatibility stay stable through cutover. Schema evolution requires additive tolerant reads, one atomic write of the canonical version, and an explicit rollback reader.

# Window IPC

Transport: Unix domain socket at `TODE_IPC` when the path exists and is a socket.

Request framing: one UTF-8 JSON object followed by `\n`.

```json
{
  "files": [{ "path": "/absolute/file", "line": 1, "column": 1 }],
  "folders": ["/absolute/folder"],
  "add": false,
  "wait": false,
  "diff": ["/a", "/b"],
  "view": "scm",
  "theme": {}
}
```

Optional properties remain omitted rather than serialized as null. Reply framing is one JSON line: `{ "ok": true }` or `{ "ok": false, "error": "..." }`. Default timeout is 4,000 ms; wait-mode uses no timeout. Unreadable JSON maps to `the window sent something unreadable`; missing `ok` maps to the reply error or `the window refused`.

# Startup and Live State

* `startup-open.json` carries a partial open request for the first extension activation, then is consumed once.
* `live-theme.json` is applied on activation and every change without a workbench reload.
* Browser main/preload glue forwards color changes to every live window socket, removes dead sockets, and records workbench timing marks.
* Bridge extension generation remains dependency-free at runtime. Its host adapter is versioned with its manifest and contains no independent domain policy.

# Terminal OSC

The palette query reads background, foreground, and ANSI slots 0–15. Replies may end with BEL or ST and may use any hexadecimal component width; each component scales independently into 0–255. Incomplete replies retain answered values and fill missing values from the existing fallback palette. Idle and hard-cap timeouts remain observable compatibility inputs.

# HTTP Injection

The injector proxies HTTP and WebSocket traffic to code-server. For HTML it requests uncompressed upstream content, injects the managed CSS before `</head>` or into a headless document, corrects content length, serves the managed font route, and avoids script injection. Non-HTML bodies pass unchanged. An unavailable upstream returns a plain controlled error rather than crashing the process.

# Release Manifest

Stored manifest:

```json
{
  "version": "vX.Y.Z",
  "channel": "stable",
  "published": "ISO-8601 timestamp",
  "platforms": {
    "darwin-arm64": { "file": "...", "sha256": "...", "size": 0 }
  }
}
```

Served manifests add a per-platform `url`; latest manifests also add `install`.

Existing routes, both at root and beneath `/install` where applicable:

* `/` and `/dev`: stable/dev installer.
* `/latest.json` and `/dev/latest.json`: channel latest manifest.
* `/v/<version>`: pinned installer.
* `/v/<version>/manifest.json`: pinned manifest.
* `/dl/<channel>/<version>/<file>`: artifact download.

Only GET and HEAD are accepted. Unknown channels/routes return 404; missing latest returns 503; other methods return 405. Download content length, ranges, ETag/cache headers, and HEAD semantics must be frozen from the worker before replacement.

# Compatibility Rules

1. Define protocol structs in `tode-protocol` with deny-unknown disabled for readers and controlled omission for writers.
2. Keep JSON field names and newline framing exact.
3. Preserve absolute-path semantics and macOS Unix-socket length constraints.
4. Write state atomically and never reuse a real user directory in tests.
5. Version the bridge/loader ABI separately from the Rust crate version when host compatibility changes.
