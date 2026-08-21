---
type: Compatibility Contract
title: VS Code Compatible Editor Import
contract_id: C12
description: Preserve editor discovery and import of settings, keybindings, snippets, tasks, and extensions.
tags: [import, editors, profile, extensions]
status: draft
implementation_status: rust-import-parity
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

Four Rust integration tests cover settings precedence/reports, keybinding deduplication, snippet/task copying, extension registry/copy, existing registry retention, missing/unsafe folders, and symlink rejection. The import pipeline is ported. C12 remains draft until editor discovery, progress callbacks, managed theme registration, embedded UI, and production command wiring are complete.
