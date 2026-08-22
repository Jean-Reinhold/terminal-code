---
type: Compatibility Contract
title: Embedded Import and Shortcut Pages
contract_id: C22
description: Preserve tokenized local pages, state rendering, actions, progress, apply, cancel, completion, and navigation.
tags: [web, embedded, import, shortcuts]
status: draft
implementation_status: rust-production-parity
risk: high
owners: [web, import, shortcuts]
surfaces: [browser, http, filesystem]
source_paths: [src/pages/import/app.tsx, src/pages/shortcuts/app.tsx, src/webui/pages.ts, src/webui/tokens.ts, src/import/web.ts, src/shortcuts/web.ts, crates/tode-runtime/src/import_manager.rs, crates/tode-runtime/src/shortcut_manager.rs, test/import.test.js]
scenario_ids: []
legacy_test_paths: [test/import.test.js]
rust_test_paths: [crates/tode-runtime/src/import_manager.rs, crates/tode-runtime/src/shortcut_manager.rs]
platforms: [macos, linux, browser]
sources:
  - { id: import-page, resource: ../../../../src/pages/import/app.tsx, title: Import page }
  - { id: shortcut-page, resource: ../../../../src/pages/shortcuts/app.tsx, title: Shortcut page }
  - { id: server, resource: ../../../../src/webui/pages.ts, title: Local page server }
  - { id: rust-import, resource: ../../../../crates/tode-runtime/src/import_manager.rs, title: Rust token-scoped embedded import page and protocol }
  - { id: rust-shortcuts, resource: ../../../../crates/tode-runtime/src/shortcut_manager.rs, title: Rust token-scoped embedded shortcut page and protocol }
---

# Contract

Serve reviewed built pages only through unguessable scoped tokens, reject invalid/expired access, preserve initial/session state, editor selection, import progress/report/done flow, shortcut row/claim/collision decisions, free-chord feedback, apply/cancel/confirmation, reload/navigation, responsive styling, keyboard operation, and accessible state.

# Coverage Status
Rust tests cover 128-bit scoped URL rejection/admission, bounded JSON bodies, import editor validation/state/report/done, shortcut collision validation/decisions/confirmation/completion, and persisted apply. Real Chrome at compact terminal-pane viewports verified import progress/report/continue/cancel and shortcut route/apply/close; footer overlap found during smoke was removed before certification. C22 remains draft only until these real-browser cases are promoted into harness scenarios.
