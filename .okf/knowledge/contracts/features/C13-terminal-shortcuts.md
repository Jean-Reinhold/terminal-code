---
type: Compatibility Contract
title: Ghostty and Kitty Shortcut Configuration
contract_id: C13
description: Preserve terminal detection, key syntax, conflict derivation, isolated config edits, reload, and undo.
tags: [shortcuts, ghostty, kitty, terminal]
status: draft
implementation_status: rust-provider-command-partial
risk: critical
owners: [shortcuts]
surfaces: [filesystem, process, terminal]
source_paths: [src/shortcuts/backends/ghostty.ts, src/shortcuts/backends/kitty.ts, src/shortcuts/store.ts, crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, crates/tode-cli/src/main.rs, crates/tode-cli/tests/shortcuts.rs, test/shortcuts.test.js]
scenario_ids: []
legacy_test_paths: [test/shortcuts.test.js]
rust_test_paths: [crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, crates/tode-cli/tests/shortcuts.rs]
platforms: [macos, linux]
sources:
  - { id: ghostty, resource: ../../../../src/shortcuts/backends/ghostty.ts, title: Ghostty backend }
  - { id: kitty, resource: ../../../../src/shortcuts/backends/kitty.ts, title: Kitty backend }
  - { id: tests, resource: ../../../../test/shortcuts.test.js, title: Shortcut backend suite }
  - { id: rust, resource: ../../../../crates/tode-core/src/shortcuts.rs, title: Rust chord and terminal config transforms }
  - { id: rust-service, resource: ../../../../crates/tode-profile/src/shortcuts.rs, title: Rust terminal provider scan/apply/undo service }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/shortcuts.rs, title: Rust shortcut detection/readiness/undo/no-conflict/non-TTY command integration }
---

# Contract

Preserve environment/binary/config detection, effective/default key parsing, editor↔terminal chord conversion, action docs, harmless/shared/native-tab rules, conflict derivation, moved/unset/emit bindings, one tode-owned include, ancestry-based reload signal, cached invalidation, clean undo, and OS-specific quit behavior.

# Coverage Status

Rust tests cover chord conversion, Ghostty/Kitty config transforms, provider detection/readiness, real executable keymap ingestion, effective conflict discovery, shared Kitty convergence, byte-idempotent owned-file writes, clean undo without a terminal CLI, unsupported/no-conflict/non-TTY command behavior, and foreign editor-binding preservation. C13 remains draft until action documentation, ancestry-based live reload, and isolated real-terminal scenarios are ported.
