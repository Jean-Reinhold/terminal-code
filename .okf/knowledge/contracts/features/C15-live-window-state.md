---
type: Compatibility Contract
title: Live Window State and Startup Marker
contract_id: C15
description: Preserve one-shot startup requests, live theme persistence/fan-out, dead-socket cleanup, and bridge activation.
tags: [bridge, live-sync, socket, startup]
status: draft
risk: high
owners: [protocol, browser]
surfaces: [filesystem, unix-socket, browser]
source_paths: [src/bridge.ts, src/bridge/extension.ts, src/browserglue.ts, src/livesync.ts, test/livesync.test.js, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/livesync.test.js, test/browserglue.test.js]
platforms: [macos, linux]
sources:
  - { id: bridge, resource: ../../../../src/bridge.ts, title: Bridge generation and startup marker }
  - { id: extension, resource: ../../../../src/bridge/extension.ts, title: Bridge activation }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Browser socket fan-out }
---

# Contract

Consume startup open/view/diff state exactly once, apply persisted live theme on activation and every change, send terminal colors to every live window socket, remove refused/dead sockets, ignore junk messages, and preserve window-open/wait semantics through the dependency-free bridge host adapter.

# Coverage Status

Live-sync and browser-glue suites map here. H3 needs multi-socket Rust peers and generated-host-adapter ABI fixtures.
