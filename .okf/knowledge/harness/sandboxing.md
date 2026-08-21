---
type: Sandbox Specification
title: Hermetic Sandbox and Capability Levels
description: Isolation rules for user files, processes, ports, sockets, networks, clocks, terminals, browsers, and release state.
tags: [harness, sandbox, isolation, macos, linux]
status: draft
sources:
  - id: paths
    resource: ../contracts/state-and-protocols.md
    title: Current state and protocol paths
  - id: uninstall
    resource: ../../../src/uninstall.ts
    title: Current destructive uninstall boundary
  - id: runtime
    resource: ../../../src/codeserver/server.ts
    title: Current process lifecycle
---

# Safety Objective

No harness scenario may mutate a developer's real home, XDG directories, terminal configuration, install root, running editor, release channel, R2 bucket, or network service. The runner proves containment before target spawn and proves cleanup after target exit.

# Isolation Levels

| Level | Use | Guarantees |
|---|---|---|
| S0 — in-process | pure parsers, color math, schema/oracle tests | no child process/network; immutable fixture inputs |
| S1 — process sandbox | most CLI, state, IPC, injector, import, shortcut fixtures | unique HOME/XDG/workspace/install roots, process group, leased loopback, controlled peers |
| S2 — hardened VM/container | archive/install/upgrade/uninstall, outbound-denial, hostile symlinks, security campaigns | OS boundary plus read-only source/artifact mounts and explicit egress policy |
| S3 — hardware-backed | real Ghostty/Kitty, Kitty graphics protocol, terminal-browser rendering, native browser/app integration | dedicated ephemeral machine/user session, exclusive device/window, post-run reset attestation |

A scenario declares the minimum level. A worker cannot claim stronger containment than its attested environment provides. Linux user namespaces/container isolation may provide S2; macOS hard network/filesystem isolation requires a dedicated VM or disposable CI host rather than relying on deprecated/partial sandbox facilities.

# Filesystem Layout

Each attempt receives:

```text
<worker-root>/<run-id>/<scenario-id>/<attempt>/
  seed/               # read-only materialized fixture
  left/               # legacy target writable clone
  right/              # Rust target writable clone
  shared/             # only declared local peer state
  leases/             # socket/port/resource lease metadata
  observations/       # temporary unsealed captures
  logs/
  teardown/
```

Targets never share a writable HOME. Differential runs clone identical seed trees and compare after execution.

# Path Containment

* Create the sandbox under a worker-owned root whose device/inode and permissions are verified.
* Resolve scenario paths as normalized relative components; reject absolute paths, empty roots, `..`, NUL, and platform-specific alternate separators where relevant.
* Use directory-handle-relative operations and no-follow semantics for security-sensitive writes/removals.
* Reject fixture symlinks escaping the seed; preserve reviewed internal symlinks only when policy permits.
* Before destructive operations, verify every target path remains beneath the open sandbox root by handle, not string prefix.
* Mount or copy source/build artifacts read-only.
* Set `HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `XDG_BIN_HOME`, and `TODE_INSTALL_ROOT` to target-specific absolute sandbox paths.
* Strip inherited environment by default; add only target manifest/scenario allowlisted variables.

A preflight canary file in real HOME and a worker sentinel outside the sandbox are hashed before/after destructive campaigns. Any change is a critical sandbox failure.

# Process Isolation

Every target/local peer starts in a dedicated process group/session. The worker records parent/child events, executable digest, argv (redacted where required), cwd, environment-key names, start/end monotonic timestamps, signal/exit, and resource summary.

Teardown:

1. request adapter-specific graceful shutdown;
2. signal process group;
3. wait bounded grace period;
4. kill remaining descendants;
5. scan process table for tagged/leaked descendants;
6. capture open socket/file evidence where supported;
7. fail the scenario if leaks remain.

PID files and running-state fixtures never refer to unrelated host PIDs. Stale/alive process scenarios use harness-owned child processes.

# Ports and Unix Sockets

A central broker creates leases. For TCP, it binds the listener before returning a lease and transfers or proxies the open listener; it never uses “find a free port, close it, then bind.” For Unix sockets it allocates a short path under a worker-level prefix to respect macOS path-length limits, records owner/mode, and removes stale entries only inside the sandbox.

Scenario values reference lease IDs, not numeric ports or raw socket paths. Observations normalize leases through registry rules.

# Network

Default: no network for S0 and loopback-only controlled peers for S1. DNS names resolve only through the harness registry. Scenario URLs are typed references to local peers or approved staged services, never arbitrary strings.

S2 uses VM/container firewall policy to deny outbound traffic except explicit endpoints. S1 cannot claim hard egress denial on every macOS host; security/release scenarios requiring proof schedule on S2.

Local peer adapters emulate:

* code-server readiness/down/slow/malformed behavior;
* artifact downloads, truncation, size/hash mismatch, ranges, redirects, disconnects;
* release manifest/R2 transactions;
* HTTP/WebSocket injector upstreams;
* agent provider failures using recorded provider protocol fixtures, never real credentials.

# Clock and Randomness

Harness code uses injected monotonic/wall-clock providers and seeded randomness. External upstream binaries may not support virtual time; their scenarios compare bounded relations or run inside a VM with supported time control. Scenario/run manifests record seed and clock mode.

Agents never choose an unrecorded random seed. Property/fuzz campaigns retain every failing seed/input digest.

# PTY and Terminal

Synthetic PTY scenarios allocate a fresh pseudoterminal, script exact input bytes and OSC replies, and capture raw bytes plus decoded events. They set reviewed TERM/terminal-program variables rather than inheriting the operator terminal.

Real Ghostty/Kitty scenarios are S3: dedicated configuration roots, separate processes, exclusive locks, known versions, no user's terminal ancestry, controlled reload signal, and post-run config/process reset.

# Browser and Display

Browser scenarios use an ephemeral browser profile, fixed viewport/scale/locale/timezone/fonts, controlled network, and clean storage. S3 terminal-browser rendering uses a dedicated display/session and records terminal/browser/code-server versions plus graphics capability.

# Secrets and Personal Data

Scenarios use opaque secret handles resolved only inside workers. Values are never serialized into plans, agent prompts, argv reports, logs, screenshots, or artifacts. Redaction runs before sealing and before agent access; a deterministic secret scanner blocks publication on matches.

Agent tasks receive minimal redacted artifacts, not complete run directories.

# Disk and Artifact Budgets

Policy caps fixture expansion, archive extraction count/bytes, logs, screenshots, process count, memory, CPU time, wall time, open files, and artifact upload. Archive extractors reject absolute paths, parent traversal, special devices, unsafe links, duplicate conflicting entries, and expansion beyond budget.

# Crash Recovery

Worker startup scans owned sandboxes and lease records. It validates run ownership, terminates tagged orphan process groups, releases expired listeners/resources, seals crash evidence, and deletes only sandboxes past retention policy. Unknown ownership stops cleanup and alerts; it never guesses.

# Containment Acceptance Tests

The harness cannot call itself safe until tests prove:

* absolute/parent/symlink/hardlink escape attempts are rejected;
* malicious archive entries cannot escape;
* uninstall/upgrade cannot address real paths despite hostile environment/fixtures;
* port races cannot attach a foreign service;
* descendants and sockets are removed after crash/timeout/cancel;
* secret canaries never enter artifacts or agent prompts;
* S1 reports lack of hard egress rather than claiming S2 guarantees;
* S3 resets terminal config/process/display state after forced failure.
