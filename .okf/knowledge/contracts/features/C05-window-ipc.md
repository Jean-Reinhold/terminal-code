---
type: Compatibility Contract
title: Existing Window IPC
contract_id: C05
description: Preserve TODE_IPC Unix-socket discovery, JSON-line framing, replies, timeouts, and wait behavior.
tags: [ipc, unix-socket, cli, wait]
status: draft
implementation_status: harness-peer-only
risk: critical
owners: [protocol, cli]
surfaces: [unix-socket, cli, process]
source_paths: [src/ipc.ts, src/bridge/extension.ts, test/livesync.test.js]
scenario_ids:
  - ipc.window-reuse.success
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

The [Rust success scenario](../../../../harness/scenarios/ipc/window-reuse-success.scenario.jsonc) proves held socket leasing, typed `TODE_IPC` injection, bounded JSON-line request/reply, process output, transcript artifact, and replay. C05 remains draft until H3 adds refusal, unreadable reply, default timeout, and wait-completion scenarios against the Rust IPC client.
