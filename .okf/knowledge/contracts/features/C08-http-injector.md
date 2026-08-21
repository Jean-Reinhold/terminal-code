---
type: Compatibility Contract
title: Code Server HTTP and WebSocket Injector
contract_id: C08
description: Preserve HTML CSS injection, font serving, passthrough, readiness, errors, and WebSocket upgrades.
tags: [http, websocket, injector, css]
status: draft
risk: high
owners: [runtime]
surfaces: [http, websocket, filesystem]
source_paths: [src/codeserver/inject.ts, test/inject.test.js]
scenario_ids: []
legacy_test_paths: [test/inject.test.js]
platforms: [macos, linux]
sources:
  - { id: injector, resource: ../../../../src/codeserver/inject.ts, title: Current injector }
  - { id: tests, resource: ../../../../test/inject.test.js, title: Injector regression suite }
---

# Contract

Request uncompressed upstream HTML, inject managed CSS before `</head>` or into headless documents, correct content length, preserve non-HTML bodies, serve the managed font, forward upstream identity, proxy WebSocket upgrades, wait during boot, and return a plain controlled error when upstream is unavailable. Never inject script.

# Coverage Status

All 14 injector tests map here. H3 ports them into Rust HTTP/WebSocket peer scenarios before the Node suite can be removed.
