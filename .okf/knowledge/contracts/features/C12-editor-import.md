---
type: Compatibility Contract
title: VS Code Compatible Editor Import
contract_id: C12
description: Preserve editor discovery and import of settings, keybindings, snippets, tasks, and extensions.
tags: [import, editors, profile, extensions]
status: draft
implementation_status: rust-production-parity
risk: critical
owners: [import, profile]
surfaces: [filesystem, process, browser]
source_paths: [src/import/run.ts, src/import/editors.ts, src/import/command.ts, src/import/web.ts, crates/tode-profile/src/import.rs, crates/tode-runtime/src/import_manager.rs, crates/tode-cli/tests/profile_commands.rs, test/import.test.js]
scenario_ids: []
legacy_test_paths: [test/import.test.js]
rust_test_paths: [crates/tode-profile/src/import.rs, crates/tode-runtime/src/import_manager.rs, crates/tode-cli/tests/profile_commands.rs]
platforms: [macos, linux]
sources:
  - { id: run, resource: ../../../../src/import/run.ts, title: Import pipeline }
  - { id: editors, resource: ../../../../src/import/editors.ts, title: Editor discovery }
  - { id: tests, resource: ../../../../test/import.test.js, title: Import page test }
  - { id: rust, resource: ../../../../crates/tode-profile/src/import.rs, title: Rust settings, keybindings, snippets, tasks, and extensions import }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/profile_commands.rs, title: Production Rust import command integration }
  - { id: rust-page, resource: ../../../../crates/tode-runtime/src/import_manager.rs, title: Token-scoped Rust embedded import page and protocol }
---

# Contract

Discover compatible editors in existing precedence, import settings without overriding managed values, merge keybindings, copy snippets/tasks, copy valid extensions with progress, preserve skip reasons, register the managed theme, and report exact imported/kept/copied/skipped outcomes through CLI and embedded page flows.

# Coverage Status

Rust service/runtime/CLI tests cover editor discovery/selection, non-TTY listing, settings precedence/report, keybindings/snippets/tasks, extension progress/copy safety/skip reasons, managed-theme registration, token-scoped embedded state/import/done, invalid editor rejection, and profile output. Real Chrome verified responsive apply/progress/report/continue and cancel flows; both production TTY sessions exited 0.
