---
type: Compatibility Contract
title: Top Level Command Dispatch
contract_id: C17
description: Preserve first-argument command routing, arguments, output, exit mapping, and fallback to open.
tags: [cli, dispatch, commands]
status: draft
implementation_status: rust-production-parity
risk: high
owners: [cli]
surfaces: [cli, process]
source_paths: [src/main.ts, src/import/command.ts, src/skill.ts, src/upgrade.ts, src/uninstall.ts, crates/tode-cli/src/main.rs, crates/tode-cli/src/skill.rs, crates/tode-cli/tests/extensions.rs, crates/tode-cli/tests/open.rs, crates/tode-cli/tests/profile_commands.rs, crates/tode-cli/tests/reuse.rs, crates/tode-cli/tests/shortcuts.rs, crates/tode-cli/tests/skill.rs, crates/tode-cli/tests/timing.rs, crates/tode-cli/tests/uninstall.rs, crates/tode-cli/tests/upgrade.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-cli/src/main.rs, crates/tode-cli/tests/extensions.rs, crates/tode-cli/tests/open.rs, crates/tode-cli/tests/profile_commands.rs, crates/tode-cli/tests/reuse.rs, crates/tode-cli/tests/shortcuts.rs, crates/tode-cli/tests/skill.rs, crates/tode-cli/tests/timing.rs, crates/tode-cli/tests/uninstall.rs, crates/tode-cli/tests/upgrade.rs]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Top-level command dispatch }
  - { id: import, resource: ../../../../src/import/command.ts, title: Import command }
  - { id: rust, resource: ../../../../crates/tode-cli/src/main.rs, title: Rust basic command parser }
  - { id: rust-open, resource: ../../../../crates/tode-cli/tests/open.rs, title: Rust open and shutdown integration }
  - { id: rust-reuse, resource: ../../../../crates/tode-cli/tests/reuse.rs, title: Rust existing-window dispatch integration }
  - { id: rust-extensions, resource: ../../../../crates/tode-cli/tests/extensions.rs, title: Rust extension command integration }
  - { id: rust-profile, resource: ../../../../crates/tode-cli/tests/profile_commands.rs, title: Rust import and theme dispatch integration }
  - { id: rust-uninstall, resource: ../../../../crates/tode-cli/tests/uninstall.rs, title: Rust uninstall dispatch integration }
  - { id: rust-upgrade, resource: ../../../../crates/tode-cli/tests/upgrade.rs, title: Rust upgrade dispatch integration }
  - { id: rust-timing, resource: ../../../../crates/tode-cli/tests/timing.rs, title: Rust timing dispatch integration }
  - { id: rust-skill, resource: ../../../../crates/tode-cli/tests/skill.rs, title: Production Rust live skill dispatch integration }
  - { id: rust-shortcuts, resource: ../../../../crates/tode-cli/tests/shortcuts.rs, title: Rust shortcut first-position dispatch and production branch integration }
---

# Contract

Dispatch version/help/shortcut/import/theme/timing/skill/upgrade/shutdown/uninstall only when they are the first argument, preserve command-specific trailing arguments and special shortcut boot result, and send every other invocation to open parsing. Promise rejection and explicit failures retain `tode: ` stderr and exit semantics.

# Coverage Status

Rust tests cover help/version/shutdown, shortcut setup/undo/admission, open compatibility options, extensions, import/theme/skill/standalone and per-open timing/upgrade/uninstall commands, real new open with the browser bridge, and existing-window reuse. Every named first-position command now dispatches through the production Rust binary.
