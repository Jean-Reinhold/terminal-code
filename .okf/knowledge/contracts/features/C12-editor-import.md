---
type: Compatibility Contract
title: VS Code Compatible Editor Import
contract_id: C12
description: Preserve editor discovery and import of settings, keybindings, snippets, tasks, and extensions.
tags: [import, editors, profile, extensions]
status: draft
implementation_status: rust-import-service-parity
risk: critical
owners: [import, profile]
surfaces: [filesystem, process, browser]
source_paths: [src/import/run.ts, src/import/editors.ts, src/import/command.ts, src/import/web.ts, crates/tode-profile/src/import.rs, test/import.test.js]
scenario_ids: []
legacy_test_paths: [test/import.test.js]
rust_test_paths: [crates/tode-profile/src/import.rs]
platforms: [macos, linux]
sources:
  - { id: run, resource: ../../../../src/import/run.ts, title: Import pipeline }
  - { id: editors, resource: ../../../../src/import/editors.ts, title: Editor discovery }
  - { id: tests, resource: ../../../../test/import.test.js, title: Import page test }
  - { id: rust, resource: ../../../../crates/tode-profile/src/import.rs, title: Rust settings, keybindings, snippets, tasks, and extensions import }
---

# Contract

Discover compatible editors in existing precedence, import settings without overriding managed values, merge keybindings, copy snippets/tasks, copy valid extensions with progress, preserve skip reasons, register the managed theme, and report exact imported/kept/copied/skipped outcomes through CLI and embedded page flows.

# Coverage Status

Six Rust integration tests cover editor/extension discovery, absolute-XDG precedence, content summaries, progress events, settings precedence/reports, keybinding deduplication, snippet/task copying, extension registry/copy, existing registry retention, missing/unsafe folders, and symlink rejection. The non-UI import service is ported. C12 remains draft until managed theme registration, production command wiring, and the embedded UI are complete.
