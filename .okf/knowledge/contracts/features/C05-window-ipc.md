---
type: Compatibility Contract
title: Existing Window IPC
contract_id: C05
description: Preserve TODE_IPC Unix-socket discovery, JSON-line framing, replies, timeouts, and wait behavior.
tags: [ipc, unix-socket, cli, wait]
status: draft
risk: critical
owners: [protocol, cli]
surfaces: [unix-socket, cli, process]
source_paths: [src/ipc.ts, src/bridge/extension.ts, test/livesync.test.js]
scenario_ids: []
legacy_test_paths: [test/livesync.test.js]
platforms: [macos, linux]
sources:
  - { id: ipc, resource: ../../../../src/ipc.ts, title: IPC client }
  - { id: bridge, resource: ../../../../src/bridge/extension.ts, title: Window socket server }
  - { id: tests, resource: ../../../../test/livesync.test.js, title: Live socket tests }
---

# Contract

Use `TODE_IPC` only when it names a socket. Send one UTF-8 JSON request plus newline and accept one JSON reply line. Preserve omitted optional fields, success/refusal/unreadable messages, the 4-second default timeout, and unbounded wait-mode completion.

# Coverage Status

The live-sync suite covers part of the socket path. H3 must split request/reply/wait/error cases into deterministic Rust socket-peer scenarios.
