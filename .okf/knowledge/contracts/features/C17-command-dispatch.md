---
type: Compatibility Contract
title: Top Level Command Dispatch
contract_id: C17
description: Preserve first-argument command routing, arguments, output, exit mapping, and fallback to open.
tags: [cli, dispatch, commands]
status: draft
risk: high
owners: [cli]
surfaces: [cli, process]
source_paths: [src/main.ts, src/import/command.ts, src/skill.ts, src/upgrade.ts, src/uninstall.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Top-level command dispatch }
  - { id: import, resource: ../../../../src/import/command.ts, title: Import command }
---

# Contract

Dispatch version/help/shortcut/import/theme/timing/skill/upgrade/shutdown/uninstall only when they are the first argument, preserve command-specific trailing arguments and special shortcut boot result, and send every other invocation to open parsing. Promise rejection and explicit failures retain `tode: ` stderr and exit semantics.

# Coverage Status

No current Node test freezes the complete table. H3 adds a Bash/Rust command matrix before the Rust CLI becomes the production entry point.
