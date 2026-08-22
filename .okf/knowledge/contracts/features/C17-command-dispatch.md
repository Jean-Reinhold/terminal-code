---
type: Compatibility Contract
title: Top Level Command Dispatch
contract_id: C17
description: Preserve first-argument command routing, arguments, output, exit mapping, and fallback to open.
tags: [cli, dispatch, commands]
status: draft
implementation_status: rust-basic-dispatch
risk: high
owners: [cli]
surfaces: [cli, process]
source_paths: [src/main.ts, src/import/command.ts, src/skill.ts, src/upgrade.ts, src/uninstall.ts, crates/tode-cli/src/main.rs, crates/tode-cli/tests/open.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-cli/src/main.rs, crates/tode-cli/tests/open.rs]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Top-level command dispatch }
  - { id: import, resource: ../../../../src/import/command.ts, title: Import command }
  - { id: rust, resource: ../../../../crates/tode-cli/src/main.rs, title: Rust basic command parser }
  - { id: rust-open, resource: ../../../../crates/tode-cli/tests/open.rs, title: Rust open and shutdown integration }
---

# Contract

Dispatch version/help/shortcut/import/theme/timing/skill/upgrade/shutdown/uninstall only when they are the first argument, preserve command-specific trailing arguments and special shortcut boot result, and send every other invocation to open parsing. Promise rejection and explicit failures retain `tode: ` stderr and exit semantics.

# Coverage Status

Rust tests cover help/version/shutdown/single-target dispatch, unknown/multiple rejection, and real open/shutdown flow. C17 remains draft until shortcut/import/theme/timing/skill/upgrade/uninstall and the full VS Code-compatible option table are wired.
