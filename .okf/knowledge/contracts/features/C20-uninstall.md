---
type: Compatibility Contract
title: Complete Safe Uninstall
contract_id: C20
description: Preserve confirmation, managed shutdown, owned path/config/font/shim cleanup, and retained user boundaries.
tags: [uninstall, cleanup, safety]
status: draft
risk: critical
owners: [cli, runtime, profile]
surfaces: [cli, filesystem, process, terminal]
source_paths: [src/uninstall.ts, src/runtime/paths.ts, src/shortcuts/backends/ghostty.ts, src/shortcuts/backends/kitty.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [macos, linux]
sources:
  - { id: uninstall, resource: ../../../../src/uninstall.ts, title: Current uninstall command }
  - { id: paths, resource: ../../../../src/runtime/paths.ts, title: Managed path ownership }
---

# Contract

Prompt unless `--yes`, abort without changes on refusal, stop managed activities, remove only owned install/data/state/cache/runtime/font/shim and terminal include files, reload affected terminals, preserve unrelated configs/files, report removed/absent outcomes, and never escape the resolved managed roots.

# Coverage Status

No current uninstall test exists. H3/H5 require S2 sandbox scenarios for confirmation, idempotence, permissions, symlink/path attacks, partial state, process cleanup, and exact retained/removed trees.
