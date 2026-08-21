---
type: Compatibility Contract
title: Verified Upgrade Transaction
contract_id: C19
description: Preserve latest/pinned lookup, check outcomes, verified fetch, atomic swap, receipts, and rollback safety.
tags: [upgrade, release, atomicity, rollback]
status: draft
implementation_status: rust-artifact-partial
risk: critical
owners: [release, runtime]
surfaces: [cli, http, filesystem, process]
source_paths: [src/upgrade.ts, src/runtime/release.ts, scripts/install.sh, scripts/release.sh, crates/tode-core/src/release.rs, crates/tode-runtime/src/artifact.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-core/src/release.rs, crates/tode-runtime/src/artifact.rs]
platforms: [macos, linux]
sources:
  - { id: upgrade, resource: ../../../../src/upgrade.ts, title: Current self-upgrade }
  - { id: runtime, resource: ../../../../src/runtime/release.ts, title: Verified runtime fetch }
  - { id: rust, resource: ../../../../crates/tode-core/src/release.rs, title: Rust build selection and installed receipts }
  - { id: rust-artifact, resource: ../../../../crates/tode-runtime/src/artifact.rs, title: Rust verified artifacts, safe extraction, and atomic swap }
---

# Contract

Preserve target manifest selection, not-install/current/available/upgraded outcomes, progress output, declared size/SHA verification before unpack, staged sibling extraction, atomic rename over the complete install, receipt timing/content, server stop after durable success, and previous installation survival on every pre-swap failure.

# Coverage Status

Rust tests cover schemas/receipts plus exact streamed size/SHA verification, failed-download cleanup, safe bounded extraction, link rejection, and atomic swap rollback/restoration. C19 remains draft until the staged upgrade outcome/receipt transaction and interruption matrix are wired to the production CLI.
