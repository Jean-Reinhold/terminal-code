---
type: Compatibility Contract
title: Ghostty and Kitty Shortcut Configuration
contract_id: C13
description: Preserve terminal detection, key syntax, conflict derivation, isolated config edits, reload, and undo.
tags: [shortcuts, ghostty, kitty, terminal]
status: draft
risk: critical
owners: [shortcuts]
surfaces: [filesystem, process, terminal]
source_paths: [src/shortcuts/backends/ghostty.ts, src/shortcuts/backends/kitty.ts, src/shortcuts/store.ts, test/shortcuts.test.js]
scenario_ids: []
legacy_test_paths: [test/shortcuts.test.js]
platforms: [macos, linux]
sources:
  - { id: ghostty, resource: ../../../../src/shortcuts/backends/ghostty.ts, title: Ghostty backend }
  - { id: kitty, resource: ../../../../src/shortcuts/backends/kitty.ts, title: Kitty backend }
  - { id: tests, resource: ../../../../test/shortcuts.test.js, title: Shortcut backend suite }
---

# Contract

Preserve environment/binary/config detection, effective/default key parsing, editor↔terminal chord conversion, action docs, harmless/shared/native-tab rules, conflict derivation, moved/unset/emit bindings, one tode-owned include, ancestry-based reload signal, cached invalidation, clean undo, and OS-specific quit behavior.

# Coverage Status

The 48-test shortcut suite maps here and C14. H3 ports backend fixtures and real isolated terminal reload checks.
