---
type: Compatibility Contract
title: CLI Identity and Basic Invocation
contract_id: C01
description: Preserve help, version, default-current-directory, and basic target invocation identity.
tags: [cli, identity, help, version]
status: draft
implementation_status: rust-identity-parity
risk: medium
owners: [cli]
surfaces: [cli, filesystem]
source_paths:
  - README.md
  - src/main.ts
  - src/target.ts
  - crates/tode-core/src/cli.rs
scenario_ids:
  - cli.help
  - cli.version
rust_test_paths:
  - crates/tode-core/src/cli.rs
platforms: [macos, linux]
sources:
  - { id: readme, resource: ../../../../README.md, title: Public CLI usage }
  - { id: main, resource: ../../../../src/main.ts, title: Current CLI implementation }
  - { id: rust, resource: ../../../../crates/tode-core/src/cli.rs, title: Rust CLI identity implementation }
---

# Contract

* `tode --help` and `tode -h` print the reviewed help text to stdout and exit zero.
* `tode --version` and `tode -v` print the installed `VERSION` content, trimmed, plus one newline; absent/empty receipt falls back to `dev`.
* With no positional target, the current directory is the requested target.
* Error output is prefixed `tode: ` and exits nonzero where the command rejects input.

# Initial Executable Evidence

* [Help scenario](../../../../harness/scenarios/cli/help.scenario.jsonc)
* [Version scenario](../../../../harness/scenarios/cli/version.scenario.jsonc)

The Rust `tode-contract-cli` help and version scenarios match exact snapshots captured from the legacy CLI, and no harness target executes Node. C01 remains draft until the production M6 `tode` binary uses this identity code and wires default target/error behavior.
