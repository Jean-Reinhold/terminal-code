---
type: Compatibility Contract
title: Live Window State and Startup Marker
contract_id: C15
description: Preserve one-shot startup requests, live theme persistence/fan-out, dead-socket cleanup, and bridge activation.
tags: [bridge, live-sync, socket, startup]
status: draft
implementation_status: rust-generated-bridge-partial
risk: high
owners: [protocol, browser]
surfaces: [filesystem, unix-socket, browser]
source_paths: [src/bridge.ts, src/bridge/extension.ts, src/browserglue.ts, src/livesync.ts, crates/tode-profile/src/bridge.rs, crates/tode-cli/tests/open.rs, test/livesync.test.js, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/livesync.test.js, test/browserglue.test.js]
rust_test_paths: [crates/tode-profile/src/bridge.rs, crates/tode-cli/tests/open.rs]
platforms: [macos, linux]
sources:
  - { id: bridge, resource: ../../../../src/bridge.ts, title: Bridge generation and startup marker }
  - { id: extension, resource: ../../../../src/bridge/extension.ts, title: Bridge activation }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Browser socket fan-out }
  - { id: rust-bridge, resource: ../../../../crates/tode-profile/src/bridge.rs, title: Rust bridge manifest/source/registry and one-shot marker }
  - { id: rust-open, resource: ../../../../crates/tode-cli/tests/open.rs, title: Production new-window goto/review/diff marker integration }
---

# Contract

Consume startup open/view/diff state exactly once, apply persisted live theme on activation and every change, send terminal colors to every live window socket, remove refused/dead sockets, ignore junk messages, and preserve window-open/wait semantics through the dependency-free bridge host adapter.

# Coverage Status

Rust tests cover idempotent bridge manifest/source/registry installation, one-shot marker serialization with freshness time, and production folder/goto/review/diff launches. Generated activation code consumes fresh markers once, watches/persists live themes, hosts bounded JSON-line IPC, exports TODE_IPC, waits for tabs, and cleans sockets. C15 remains draft until generated-host ABI, multi-socket theme fan-out, and dead-socket execution scenarios are added.
