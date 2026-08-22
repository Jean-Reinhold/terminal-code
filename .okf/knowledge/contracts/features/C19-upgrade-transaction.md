---
type: Compatibility Contract
title: Verified Upgrade Transaction
contract_id: C19
description: Preserve latest/pinned lookup, check outcomes, verified fetch, atomic swap, receipts, and rollback safety.
tags: [upgrade, release, atomicity, rollback]
status: draft
implementation_status: rust-production-check-partial
risk: critical
owners: [release, runtime]
surfaces: [cli, http, filesystem, process]
source_paths: [src/upgrade.ts, src/runtime/release.ts, scripts/install.sh, scripts/release.sh, crates/tode-core/src/release.rs, crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/upgrade.rs, crates/tode-cli/tests/upgrade.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-core/src/release.rs, crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/upgrade.rs, crates/tode-cli/tests/upgrade.rs]
platforms: [macos, linux]
sources:
  - { id: upgrade, resource: ../../../../src/upgrade.ts, title: Current self-upgrade }
  - { id: runtime, resource: ../../../../src/runtime/release.ts, title: Verified runtime fetch }
  - { id: rust, resource: ../../../../crates/tode-core/src/release.rs, title: Rust build selection and installed receipts }
  - { id: rust-artifact, resource: ../../../../crates/tode-runtime/src/artifact.rs, title: Rust verified artifacts, safe extraction, and atomic swap }
  - { id: rust-upgrade, resource: ../../../../crates/tode-runtime/src/upgrade.rs, title: Rust verified staged upgrade transaction }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/upgrade.rs, title: Production Rust upgrade check integration }
---

# Contract

Preserve target manifest selection, not-install/current/available/upgraded outcomes, progress output, declared size/SHA verification before unpack, staged sibling extraction, atomic rename over the complete install, receipt timing/content, server stop after durable success, and previous installation survival on every pre-swap failure.

# Coverage Status

Rust tests cover schemas/receipts, current/available/upgraded outcomes, production `--check`, exact download verification, failed-download preservation, safe staged extraction, VERSION/CHANNEL staging, complete swap, and old-file removal. C19 remains draft until full production upgraded-output/daemon-stop and interruption-at-every-checkpoint scenarios are certified.
