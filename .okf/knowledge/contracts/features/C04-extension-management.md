---
type: Compatibility Contract
title: Extension Management
contract_id: C04
description: Preserve extension install, uninstall, list, version, ordering, profile paths, and exit propagation.
tags: [cli, extensions, code-server]
status: draft
implementation_status: rust-cli-parity
risk: high
owners: [cli, profile]
surfaces: [cli, process, filesystem]
source_paths: [src/main.ts, src/profile.ts, crates/tode-cli/src/main.rs, crates/tode-cli/tests/extensions.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-cli/src/main.rs, crates/tode-cli/tests/extensions.rs]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Extension command delegation }
  - { id: profile, resource: ../../../../src/profile.ts, title: Extension profile paths }
  - { id: rust, resource: ../../../../crates/tode-cli/tests/extensions.rs, title: Rust extension management integration }
---

# Contract

Forward extension operations to the pinned code-server binary with the managed extensions and user-data directories. Uninstalls run before installs, the first nonzero child status stops processing, list output remains quiet on stderr, theme registration follows successful changes, and successful installs print the reopen notice.

# Coverage Status

Rust parser/integration tests cover repeated install/uninstall values, uninstall-before-install ordering, managed profile directories, list/show-versions, reopen output, missing values, and child status propagation. C04 is ported; it remains draft until harness black-box scenarios and real pinned code-server certification.
