---
type: Compatibility Contract
title: Complete Safe Uninstall
contract_id: C20
description: Preserve confirmation, managed shutdown, owned path/config/font/shim cleanup, and retained user boundaries.
tags: [uninstall, cleanup, safety]
status: draft
implementation_status: rust-production-parity
risk: critical
owners: [cli, runtime, profile]
surfaces: [cli, filesystem, process, terminal]
source_paths: [src/uninstall.ts, src/runtime/paths.ts, src/shortcuts/backends/ghostty.ts, src/shortcuts/backends/kitty.ts, crates/tode-profile/src/uninstall.rs, crates/tode-cli/tests/uninstall.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-profile/src/uninstall.rs, crates/tode-cli/tests/uninstall.rs]
platforms: [macos, linux]
sources:
  - { id: uninstall, resource: ../../../../src/uninstall.ts, title: Current uninstall command }
  - { id: paths, resource: ../../../../src/runtime/paths.ts, title: Managed path ownership }
  - { id: rust, resource: ../../../../crates/tode-profile/src/uninstall.rs, title: Rust safe uninstall service }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/uninstall.rs, title: Production Rust uninstall integration }
---

# Contract

Prompt unless `--yes`, abort without changes on refusal, stop managed activities, remove only owned install/data/state/cache/runtime/font/shim and terminal include files, reload affected terminals, preserve unrelated configs/files, report removed/absent outcomes, and never escape the resolved managed roots.

# Coverage Status

Rust service/integration tests cover `--yes`, daemon stop, owned install/data/state/cache/shim/font cleanup, byte-matched font protection, VERSION-gated install removal, Ghostty/Kitty owned file/include cleanup, unrelated config preservation, and idempotence. C20 remains draft for interactive TTY confirmation and real terminal reload certification.
