---
type: Compatibility Contract
title: VS Code CLI Compatibility Flags
contract_id: C03
description: Preserve accepted ignored flags and warning-only unsupported extension-isolation flags.
tags: [cli, compatibility, flags]
status: draft
implementation_status: rust-parser-partial
risk: medium
owners: [cli]
surfaces: [cli]
source_paths: [src/main.ts, crates/tode-cli/src/main.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-cli/src/main.rs]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Current CLI parser }
  - { id: rust, resource: ../../../../crates/tode-cli/src/main.rs, title: Rust open-option parser }
---

# Contract

Flags in `IGNORED` and `IGNORED_WITH_VALUE` are consumed without becoming unknown-option failures. `--disable-extensions` and `--disable-extension` are accepted but emit the existing explanatory warning. Missing values, unknown flags, ordering, stderr text, and exit behavior remain observable.

# Coverage Status

Rust tests cover goto/add/diff/new/reuse/wait/review/split/size parsing and invalid values. C03 remains draft until ignored VS Code flags and warning-only extension-isolation flags are ported exactly.
