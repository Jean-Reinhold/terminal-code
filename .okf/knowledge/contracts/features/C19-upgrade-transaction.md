---
type: Compatibility Contract
title: Verified Upgrade Transaction
contract_id: C19
description: Preserve latest/pinned lookup, check outcomes, verified fetch, atomic swap, receipts, and rollback safety.
tags: [upgrade, release, atomicity, rollback]
status: draft
risk: critical
owners: [release, runtime]
surfaces: [cli, http, filesystem, process]
source_paths: [src/upgrade.ts, src/runtime/release.ts, scripts/install.sh, scripts/release.sh]
scenario_ids: []
legacy_test_paths: []
platforms: [macos, linux]
sources:
  - { id: upgrade, resource: ../../../../src/upgrade.ts, title: Current self-upgrade }
  - { id: runtime, resource: ../../../../src/runtime/release.ts, title: Verified runtime fetch }
---

# Contract

Preserve target manifest selection, not-install/current/available/upgraded outcomes, progress output, declared size/SHA verification before unpack, staged sibling extraction, atomic rename over the complete install, receipt timing/content, server stop after durable success, and previous installation survival on every pre-swap failure.

# Coverage Status

No current test covers the transaction. H3/H5 require local release peers and interruption at every fetch/unpack/swap/receipt checkpoint.
