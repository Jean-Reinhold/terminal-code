---
type: Compatibility Contract
title: Existing Window IPC
contract_id: C05
description: Preserve TODE_IPC Unix-socket discovery, JSON-line framing, replies, timeouts, and wait behavior.
tags: [ipc, unix-socket, cli, wait]
status: draft
implementation_status: rust-production-parity
risk: critical
owners: [protocol, cli]
surfaces: [unix-socket, cli, process]
source_paths: [src/ipc.ts, src/bridge/extension.ts, crates/tode-core/src/ipc.rs, crates/tode-profile/src/bridge.rs, crates/tode-cli/tests/open.rs, crates/tode-cli/tests/reuse.rs, test/livesync.test.js]
scenario_ids:
  - ipc.window-reuse.success
  - ipc.window-reuse.refused
  - ipc.window-reuse.timeout
  - ipc.window-reuse.wait
legacy_test_paths: [test/livesync.test.js]
rust_test_paths:
  - crates/tode-core/src/ipc.rs
  - crates/tode-profile/src/bridge.rs
  - crates/tode-cli/tests/open.rs
  - crates/tode-cli/tests/reuse.rs
platforms: [macos, linux]
sources:
  - { id: ipc, resource: ../../../../src/ipc.ts, title: IPC client }
  - { id: bridge, resource: ../../../../src/bridge/extension.ts, title: Window socket server }
  - { id: tests, resource: ../../../../test/livesync.test.js, title: Live socket tests }
  - { id: rust, resource: ../../../../crates/tode-core/src/ipc.rs, title: Rust IPC client }
  - { id: rust-reuse, resource: ../../../../crates/tode-cli/tests/reuse.rs, title: Production Rust goto wait review IPC reuse }
  - { id: rust-bridge, resource: ../../../../crates/tode-profile/src/bridge.rs, title: Rust-generated dependency-free VS Code socket bridge }
  - { id: rust-open, resource: ../../../../crates/tode-cli/tests/open.rs, title: Production bridge install and startup-marker launch integration }
---

# Contract

Use `TODE_IPC` only when it names a socket. Send one UTF-8 JSON request plus newline and accept one JSON reply line. Preserve omitted optional fields, success/refusal/unreadable messages, the 4-second default timeout, and unbounded wait-mode completion.

# Coverage Status

Four harness scenarios and Rust integration/unit tests cover production goto/wait/review reuse, framing, omitted fields, success, refusal, unreadable reply, bounded timeout, unbounded wait, missing sockets, generated server activation, TODE_IPC export, request replies, and socket cleanup. The generated extension was activated eagerly by real code-server; `lsof` observed its authoritative Unix listener, production `tode --goto --review abi.rs:3:2` received `{ok:true}`, and Chrome observed `abi.rs`, Source Control selected, and `Ln 3, Col 2`.
