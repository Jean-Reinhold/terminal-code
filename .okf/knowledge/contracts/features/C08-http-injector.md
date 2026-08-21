---
type: Compatibility Contract
title: Code Server HTTP and WebSocket Injector
contract_id: C08
description: Preserve HTML CSS injection, font serving, passthrough, readiness, errors, and WebSocket upgrades.
tags: [http, websocket, injector, css]
status: draft
implementation_status: rust-test-parity
risk: high
owners: [runtime]
surfaces: [http, websocket, filesystem]
source_paths: [src/codeserver/inject.ts, crates/tode-runtime/src/injector.rs, test/inject.test.js]
scenario_ids: []
legacy_test_paths: [test/inject.test.js]
rust_test_paths: [crates/tode-runtime/src/injector.rs]
platforms: [macos, linux]
sources:
  - { id: injector, resource: ../../../../src/codeserver/inject.ts, title: Current injector }
  - { id: tests, resource: ../../../../test/inject.test.js, title: Injector regression suite }
  - { id: rust, resource: ../../../../crates/tode-runtime/src/injector.rs, title: Rust injector implementation and tests }
---

# Contract

Request uncompressed upstream HTML, inject managed CSS before `</head>` or into headless documents, correct content length, preserve non-HTML bodies, serve the managed font, forward upstream identity, proxy WebSocket upgrades, wait during boot, and return a plain controlled error when upstream is unavailable. Never inject script.

# Coverage Status

Seven Rust tests cover all 14 legacy injector behaviors: exact CSS/no-script/watermark, HTML and no-head injection, content length and encoding correction, upstream header rewrites and identity encoding, non-HTML/missing-CSS passthrough, font/cache serving, controlled 502, startup readiness hold, and buffered-head WebSocket upgrade bridging. C08 remains draft until these run as harness scenarios and the Rust injector is wired into the production runtime.
