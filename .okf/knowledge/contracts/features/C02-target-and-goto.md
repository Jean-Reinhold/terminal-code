---
type: Compatibility Contract
title: Open Target and Goto Parsing
contract_id: C02
description: Preserve cwd-relative file/folder/new-file resolution and file-line-column parsing.
tags: [cli, target, goto, paths]
status: draft
implementation_status: rust-snapshot-parity
risk: high
owners: [cli]
surfaces: [cli, filesystem]
source_paths:
  - src/target.ts
  - src/ipc.ts
  - crates/tode-core/src/target.rs
  - test/target.test.js
scenario_ids:
  - cli.target-file
  - cli.target-folder
  - cli.target-missing
  - cli.goto
legacy_test_paths:
  - test/target.test.js
rust_test_paths:
  - crates/tode-core/src/target.rs
platforms: [macos, linux]
sources:
  - { id: target, resource: ../../../../src/target.ts, title: Target resolution }
  - { id: ipc, resource: ../../../../src/ipc.ts, title: Goto parsing }
  - { id: tests, resource: ../../../../test/target.test.js, title: Existing target regression tests }
  - { id: rust, resource: ../../../../crates/tode-core/src/target.rs, title: Rust target and goto implementation }
---

# Contract

* Resolve a supplied target against the process cwd.
* Existing directory returns `{ folder: <absolute>, file: null }`.
* Existing non-directory returns `{ folder: null, file: <absolute> }`.
* Missing path is treated as a new file, not an error.
* A goto argument matching `path:line[:column]` returns numeric line and column; omitted column defaults to one.
* If the complete argument already names an existing path, do not split numeric suffixes from it.

# Initial Executable Evidence

* [Existing-file scenario](../../../../harness/scenarios/cli/target-file.scenario.jsonc)
* [Existing-folder scenario](../../../../harness/scenarios/cli/target-folder.scenario.jsonc)
* [Missing-file scenario](../../../../harness/scenarios/cli/target-missing.scenario.jsonc)
* [Goto scenario](../../../../harness/scenarios/cli/goto.scenario.jsonc)

The Rust `tode-core` implementation is executed through the Rust `tode-contract-probe` binary. All four scenarios match exact snapshots captured from the legacy exports; no JavaScript test wrapper remains.
