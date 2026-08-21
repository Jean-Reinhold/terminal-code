---
type: Compatibility Contract
title: Extension Management
contract_id: C04
description: Preserve extension install, uninstall, list, version, ordering, profile paths, and exit propagation.
tags: [cli, extensions, code-server]
status: draft
risk: high
owners: [cli, profile]
surfaces: [cli, process, filesystem]
source_paths: [src/main.ts, src/profile.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Extension command delegation }
  - { id: profile, resource: ../../../../src/profile.ts, title: Extension profile paths }
---

# Contract

Forward extension operations to the pinned code-server binary with the managed extensions and user-data directories. Uninstalls run before installs, the first nonzero child status stops processing, list output remains quiet on stderr, theme registration follows successful changes, and successful installs print the reopen notice.

# Coverage Status

No current Node test freezes this boundary. H3 requires a Bash/Rust fake code-server argv/stdio/exit scenario matrix.
