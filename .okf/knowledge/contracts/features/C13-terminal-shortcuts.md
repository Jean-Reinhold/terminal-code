---
type: Compatibility Contract
title: Ghostty and Kitty Shortcut Configuration
contract_id: C13
description: Preserve terminal detection, key syntax, conflict derivation, isolated config edits, reload, and undo.
tags: [shortcuts, ghostty, kitty, terminal]
status: draft
implementation_status: rust-transform-parity
risk: critical
owners: [shortcuts]
surfaces: [filesystem, process, terminal]
source_paths: [src/shortcuts/backends/ghostty.ts, src/shortcuts/backends/kitty.ts, src/shortcuts/store.ts, crates/tode-core/src/shortcuts.rs, test/shortcuts.test.js]
scenario_ids: []
legacy_test_paths: [test/shortcuts.test.js]
rust_test_paths: [crates/tode-core/src/shortcuts.rs]
platforms: [macos, linux]
sources:
  - { id: ghostty, resource: ../../../../src/shortcuts/backends/ghostty.ts, title: Ghostty backend }
  - { id: kitty, resource: ../../../../src/shortcuts/backends/kitty.ts, title: Kitty backend }
  - { id: tests, resource: ../../../../test/shortcuts.test.js, title: Shortcut backend suite }
  - { id: rust, resource: ../../../../crates/tode-core/src/shortcuts.rs, title: Rust chord and terminal config transforms }
---

# Contract

Preserve environment/binary/config detection, effective/default key parsing, editor↔terminal chord conversion, action docs, harmless/shared/native-tab rules, conflict derivation, moved/unset/emit bindings, one tode-owned include, ancestry-based reload signal, cached invalidation, clean undo, and OS-specific quit behavior.

# Coverage Status

Eight Rust tests cover canonical/user chord normalization, Ghostty aliases/prefixes/file/include/emit behavior, and Kitty aliases/shared copy rebind/file/include behavior. C13 remains draft until effective-keymap parsing, conflict derivation, atomic filesystem apply/undo, ancestry signals, and real isolated terminal reload are ported.
