---
type: Compatibility Contract
title: VS Code CLI Compatibility Flags
contract_id: C03
description: Preserve accepted ignored flags and warning-only unsupported extension-isolation flags.
tags: [cli, compatibility, flags]
status: draft
risk: medium
owners: [cli]
surfaces: [cli]
source_paths: [src/main.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Current CLI parser }
---

# Contract

Flags in `IGNORED` and `IGNORED_WITH_VALUE` are consumed without becoming unknown-option failures. `--disable-extensions` and `--disable-extension` are accepted but emit the existing explanatory warning. Missing values, unknown flags, ordering, stderr text, and exit behavior remain observable.

# Coverage Status

No current Node test covers this contract. H3 must add Bash/Rust black-box scenarios before C03 can become stable.
