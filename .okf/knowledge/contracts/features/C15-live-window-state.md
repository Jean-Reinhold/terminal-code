---
type: Compatibility Contract
title: Live Window State and Startup Marker
contract_id: C15
description: Preserve one-shot startup requests, live theme persistence/fan-out, dead-socket cleanup, and bridge activation.
tags: [bridge, live-sync, socket, startup]
status: draft
implementation_status: rust-production-host-partial
risk: high
owners: [protocol, browser]
surfaces: [filesystem, unix-socket, browser]
source_paths: [src/bridge.ts, src/bridge/extension.ts, src/browserglue.ts, src/livesync.ts, crates/tode-profile/src/bridge.rs, crates/tode-runtime/src/browser_bridge.rs, crates/tode-runtime/src/bin/tode-theme-bridge.rs, crates/tode-runtime/tests/theme_bridge_command.rs, crates/tode-cli/tests/open.rs, test/livesync.test.js, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/livesync.test.js, test/browserglue.test.js]
rust_test_paths: [crates/tode-profile/src/bridge.rs, crates/tode-runtime/src/browser_bridge.rs, crates/tode-runtime/tests/theme_bridge_command.rs, crates/tode-cli/tests/open.rs]
platforms: [macos, linux]
sources:
  - { id: bridge, resource: ../../../../src/bridge.ts, title: Bridge generation and startup marker }
  - { id: extension, resource: ../../../../src/bridge/extension.ts, title: Bridge activation }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Browser socket fan-out }
  - { id: rust-bridge, resource: ../../../../crates/tode-profile/src/bridge.rs, title: Rust bridge manifest/source/registry and one-shot marker }
  - { id: rust-open, resource: ../../../../crates/tode-cli/tests/open.rs, title: Production new-window goto/review/diff marker integration }
  - { id: rust-browser, resource: ../../../../crates/tode-runtime/src/browser_bridge.rs, title: Rust-generated theme capture/helper/fan-out host adapter }
  - { id: rust-helper, resource: ../../../../crates/tode-runtime/tests/theme_bridge_command.rs, title: Production Rust live-theme conversion helper }
---

# Contract

Consume startup open/view/diff state exactly once, apply persisted live theme on activation and every change, send terminal colors to every live window socket, remove refused/dead sockets, ignore junk messages, and preserve window-open/wait semantics through the dependency-free bridge host adapter.

# Coverage Status

Rust tests cover idempotent bridge installation, fresh one-shot startup markers, production folder/goto/review/diff launches, raw-color validation/fallbacks, full Rust theme generation, generated Electron capture/helper invocation, real multi-socket JSON-line success/refusal/stale removal, and production helper fan-out. Real code-server eagerly activated the generated extension; its authoritative Unix listener was observed, production goto+review received success, and Chrome confirmed file/SCM/line/column state. C15 remains draft only until a real terminal theme-change event is driven through terminal-browser and automated as a harness scenario.
