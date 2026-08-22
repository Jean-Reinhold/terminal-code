---
type: Compatibility Contract
title: Existing Window IPC
contract_id: C05
description: Preserve TODE_IPC Unix-socket discovery, JSON-line framing, replies, timeouts, and wait behavior.
tags: [ipc, unix-socket, cli, wait]
status: draft
implementation_status: rust-production-reuse
risk: critical
owners: [protocol, cli]
surfaces: [unix-socket, cli, process]
source_paths: [src/ipc.ts, src/bridge/extension.ts, crates/tode-core/src/ipc.rs, crates/tode-cli/tests/reuse.rs, test/livesync.test.js]
scenario_ids:
  - ipc.window-reuse.success
  - ipc.window-reuse.refused
  - ipc.window-reuse.timeout
  - ipc.window-reuse.wait
legacy_test_paths: [test/livesync.test.js]
rust_test_paths:
  - crates/tode-core/src/ipc.rs
  - crates/tode-cli/tests/reuse.rs
platforms: [macos, linux]
sources:
  - { id: ipc, resource: ../../../../src/ipc.ts, title: IPC client }
  - { id: bridge, resource: ../../../../src/bridge/extension.ts, title: Window socket server }
  - { id: tests, resource: ../../../../test/livesync.test.js, title: Live socket tests }
  - { id: rust, resource: ../../../../crates/tode-core/src/ipc.rs, title: Rust IPC client }
  - { id: rust-reuse, resource: ../../../../crates/tode-cli/tests/reuse.rs, title: Production Rust goto wait review IPC reuse }
---

# Contract

Use `TODE_IPC` only when it names a socket. Send one UTF-8 JSON request plus newline and accept one JSON reply line. Preserve omitted optional fields, success/refusal/unreadable messages, the 4-second default timeout, and unbounded wait-mode completion.

# Coverage Status

Four harness scenarios and Rust integration/unit tests cover production goto/wait/review reuse, framing, omitted fields, success, refusal, unreadable reply, bounded timeout, unbounded wait, and missing sockets. C05 remains draft only because the generated VS Code bridge/server side is not yet ported.
