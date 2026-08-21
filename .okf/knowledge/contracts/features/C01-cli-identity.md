---
type: Compatibility Contract
title: CLI Identity and Basic Invocation
contract_id: C01
description: Preserve help, version, default-current-directory, and basic target invocation identity.
tags: [cli, identity, help, version]
status: draft
implementation_status: legacy-characterized
risk: medium
owners: [cli]
surfaces: [cli, filesystem]
source_paths:
  - README.md
  - src/main.ts
  - src/target.ts
scenario_ids:
  - cli.help
  - cli.version
platforms: [macos, linux]
sources:
  - { id: readme, resource: ../../../../README.md, title: Public CLI usage }
  - { id: main, resource: ../../../../src/main.ts, title: Current CLI implementation }
---

# Contract

* `tode --help` and `tode -h` print the reviewed help text to stdout and exit zero.
* `tode --version` and `tode -v` print the installed `VERSION` content, trimmed, plus one newline; absent/empty receipt falls back to `dev`.
* With no positional target, the current directory is the requested target.
* Error output is prefixed `tode: ` and exits nonzero where the command rejects input.

# Initial Executable Evidence

* [Help scenario](../../../../harness/scenarios/cli/help.scenario.jsonc)
* [Version scenario](../../../../harness/scenarios/cli/version.scenario.jsonc)

C01 remains draft until the Rust product CLI exists and these scenarios run differentially. The current slice characterizes the legacy target and proves deterministic evidence/replay.
