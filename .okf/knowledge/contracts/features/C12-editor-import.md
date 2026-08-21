---
type: Compatibility Contract
title: VS Code Compatible Editor Import
contract_id: C12
description: Preserve editor discovery and import of settings, keybindings, snippets, tasks, and extensions.
tags: [import, editors, profile, extensions]
status: draft
risk: critical
owners: [import, profile]
surfaces: [filesystem, process, browser]
source_paths: [src/import/run.ts, src/import/editors.ts, src/import/command.ts, src/import/web.ts, test/import.test.js]
scenario_ids: []
legacy_test_paths: [test/import.test.js]
platforms: [macos, linux]
sources:
  - { id: run, resource: ../../../../src/import/run.ts, title: Import pipeline }
  - { id: editors, resource: ../../../../src/import/editors.ts, title: Editor discovery }
  - { id: tests, resource: ../../../../test/import.test.js, title: Import page test }
---

# Contract

Discover compatible editors in existing precedence, import settings without overriding managed values, merge keybindings, copy snippets/tasks, copy valid extensions with progress, preserve skip reasons, register the managed theme, and report exact imported/kept/copied/skipped outcomes through CLI and embedded page flows.

# Coverage Status

The single import-page integration test maps here and C22. H3 needs isolated editor trees for every state/file type and data-loss/error boundary.
