---
type: Compatibility Contract
title: Shortcut Decision Convergence
contract_id: C14
description: Preserve wizard conflict state, claimant decisions, safe chord selection, convergence, and byte idempotence.
tags: [shortcuts, state-machine, convergence, idempotence]
status: draft
implementation_status: rust-production-parity
risk: critical
owners: [shortcuts]
surfaces: [filesystem, browser, terminal]
source_paths: [src/shortcuts/wizard.ts, src/shortcuts/holds.ts, src/shortcuts/imported.ts, crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, crates/tode-profile/src/shortcut_manager.rs, crates/tode-runtime/src/shortcut_manager.rs, crates/tode-cli/src/main.rs, crates/tode-cli/tests/shortcuts.rs, test/shortcuts.test.js, test/shortcuts-loop.test.js]
scenario_ids: []
legacy_test_paths: [test/shortcuts.test.js, test/shortcuts-loop.test.js]
rust_test_paths: [crates/tode-core/src/shortcuts.rs, crates/tode-profile/src/shortcuts.rs, crates/tode-profile/src/shortcut_manager.rs, crates/tode-runtime/src/shortcut_manager.rs, crates/tode-cli/tests/shortcuts.rs]
platforms: [macos, linux]
sources:
  - { id: wizard, resource: ../../../../src/shortcuts/wizard.ts, title: Shortcut manager state machine }
  - { id: loop, resource: ../../../../test/shortcuts-loop.test.js, title: Adversarial closed-loop test }
  - { id: rust, resource: ../../../../crates/tode-core/src/shortcuts.rs, title: Rust persisted decision and binding behavior }
  - { id: rust-service, resource: ../../../../crates/tode-profile/src/shortcuts.rs, title: Rust holder discovery, shared convergence, decisions, and keybinding reconciliation }
  - { id: rust-manager, resource: ../../../../crates/tode-profile/src/shortcut_manager.rs, title: Rust manager rows, occupancy, decision staging, and confirmation state machine }
  - { id: rust-server, resource: ../../../../crates/tode-runtime/src/shortcut_manager.rs, title: Token-scoped Rust embedded manager HTTP protocol and offline switchyard page }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/src/main.rs, title: Rust TTY manager and terminal-browser orchestration }
---

# Contract

Represent terminal/import/builtin/extension claimants consistently, reject occupied move targets, stage decisions without contradictory duplicate facts, apply every resolved row, and guarantee that reopening shows zero unresolved conflicts. A second apply changes no byte and undo restores owned changes.

# Coverage Status

Rust tests cover decision bindings, live holder refresh, shared convergence, manager rows, canonical moved-target occupancy, exact staged claim identity/translation, uniform foreign/default/extension removal masking, twin cleanup, persistence/apply/reopen, cyclic claimant termination, bounded token-scoped HTTP state/taken/decide/confirm/done, live ancestry reload, and production CLI admission. The translated adversarial oracle passes unset/keep/editor-move plus all 48 terminal-move and 29 contested-move offsets across 144 candidates, proves fresh reopen has zero unresolved rows, proves second apply byte idempotence, and observes late import/unregistered extension growth in the same session. A real Chrome smoke routed/applied/closed successfully. C14 remains draft only for isolated real-terminal/browser scenario certification.
