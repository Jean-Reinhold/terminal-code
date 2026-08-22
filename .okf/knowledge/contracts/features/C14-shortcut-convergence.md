---
type: Compatibility Contract
title: Shortcut Decision Convergence
contract_id: C14
description: Preserve wizard conflict state, claimant decisions, safe chord selection, convergence, and byte idempotence.
tags: [shortcuts, state-machine, convergence, idempotence]
status: draft
implementation_status: rust-state-machine-partial
risk: critical
owners: [shortcuts]
surfaces: [filesystem, browser, terminal]
source_paths: [src/shortcuts/wizard.ts, src/shortcuts/holds.ts, src/shortcuts/imported.ts, crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, crates/tode-profile/src/shortcut_manager.rs, test/shortcuts.test.js, test/shortcuts-loop.test.js]
scenario_ids: []
legacy_test_paths: [test/shortcuts.test.js, test/shortcuts-loop.test.js]
rust_test_paths: [crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, crates/tode-profile/src/shortcut_manager.rs]
platforms: [macos, linux]
sources:
  - { id: wizard, resource: ../../../../src/shortcuts/wizard.ts, title: Shortcut manager state machine }
  - { id: loop, resource: ../../../../test/shortcuts-loop.test.js, title: Adversarial closed-loop test }
  - { id: rust, resource: ../../../../crates/tode-core/src/shortcuts.rs, title: Rust persisted decision and binding behavior }
  - { id: rust-service, resource: ../../../../crates/tode-profile/src/shortcuts.rs, title: Rust holder discovery, shared convergence, decisions, and keybinding reconciliation }
  - { id: rust-manager, resource: ../../../../crates/tode-profile/src/shortcut_manager.rs, title: Rust manager rows, occupancy, decision staging, and confirmation state machine }
---

# Contract

Represent terminal/import/builtin/extension claimants consistently, reject occupied move targets, stage decisions without contradictory duplicate facts, apply every resolved row, and guarantee that reopening shows zero unresolved conflicts. A second apply changes no byte and undo restores owned changes.

# Coverage Status

Rust tests cover persisted claimant removals/moves, imported editor overrides, platform quit/hint guards, fallback filtering, live provider holder discovery, Kitty shared auto-apply, foreign keybinding preservation, byte idempotence, manager rows, moved-target occupancy, terminal/import/claim decision enrichment, twin cleanup, persistence/apply/reopen, and cyclic claimant termination. C14 remains draft until the embedded HTTP manager and CLI/browser admission are ported and the full legacy adversarial corpus is translated.
