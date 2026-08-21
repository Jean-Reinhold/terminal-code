---
type: Compatibility Contract
title: Existing Window IPC
contract_id: C05
description: Preserve TODE_IPC Unix-socket discovery, JSON-line framing, replies, timeouts, and wait behavior.
tags: [ipc, unix-socket, cli, wait]
status: draft
implementation_status: rust-core-parity
risk: critical
owners: [protocol, cli]
surfaces: [unix-socket, cli, process]
source_paths: [src/ipc.ts, src/bridge/extension.ts, crates/tode-core/src/ipc.rs, test/livesync.test.js]
scenario_ids:
  - ipc.window-reuse.success
  - ipc.window-reuse.refused
  - ipc.window-reuse.timeout
  - ipc.window-reuse.wait
legacy_test_paths: [test/livesync.test.js]
rust_test_paths:
  - crates/tode-core/src/ipc.rs
platforms: [macos, linux]
sources:
  - { id: ipc, resource: ../../../../src/ipc.ts, title: IPC client }
  - { id: bridge, resource: ../../../../src/bridge/extension.ts, title: Window socket server }
  - { id: tests, resource: ../../../../test/livesync.test.js, title: Live socket tests }
  - { id: rust, resource: ../../../../crates/tode-core/src/ipc.rs, title: Rust IPC client }
---

# Contract

Use `TODE_IPC` only when it names a socket. Send one UTF-8 JSON request plus newline and accept one JSON reply line. Preserve omitted optional fields, success/refusal/unreadable messages, the 4-second default timeout, and unbounded wait-mode completion.

# Coverage Status

Four Rust scenarios cover success, explicit refusal, bounded timeout, and unbounded wait with sealed process/transcript evidence. Rust unit tests cover unreadable replies and missing sockets. C05 remains draft only because the generated VS Code bridge/server side is not yet ported; the Rust client contract is implemented.
