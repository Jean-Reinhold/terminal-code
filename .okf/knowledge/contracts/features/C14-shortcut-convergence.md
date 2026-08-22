---
type: Compatibility Contract
title: Shortcut Decision Convergence
contract_id: C14
description: Preserve wizard conflict state, claimant decisions, safe chord selection, convergence, and byte idempotence.
tags: [shortcuts, state-machine, convergence, idempotence]
status: draft
implementation_status: rust-convergence-service-partial
risk: critical
owners: [shortcuts]
surfaces: [filesystem, browser, terminal]
source_paths: [src/shortcuts/wizard.ts, src/shortcuts/holds.ts, src/shortcuts/imported.ts, crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, test/shortcuts.test.js, test/shortcuts-loop.test.js]
scenario_ids: []
legacy_test_paths: [test/shortcuts.test.js, test/shortcuts-loop.test.js]
rust_test_paths: [crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs]
platforms: [macos, linux]
sources:
  - { id: wizard, resource: ../../../../src/shortcuts/wizard.ts, title: Shortcut manager state machine }
  - { id: loop, resource: ../../../../test/shortcuts-loop.test.js, title: Adversarial closed-loop test }
  - { id: rust, resource: ../../../../crates/tode-core/src/shortcuts.rs, title: Rust persisted decision and binding behavior }
  - { id: rust-service, resource: ../../../../crates/tode-profile/src/shortcuts.rs, title: Rust holder discovery, shared convergence, decisions, and keybinding reconciliation }
---

# Contract

Represent terminal/import/builtin/extension claimants consistently, reject occupied move targets, stage decisions without contradictory duplicate facts, apply every resolved row, and guarantee that reopening shows zero unresolved conflicts. A second apply changes no byte and undo restores owned changes.

# Coverage Status

Rust tests cover persisted claimant removals/moves, imported editor overrides, platform quit/hint guards, fallback filtering, live provider holder discovery, Kitty shared auto-apply, foreign keybinding preservation, byte idempotence, and undo. C14 remains draft until `managerRows`, `chordTaken`, twin/mirror decision staging, embedded manager admission, and the adversarial closed loop are ported.
