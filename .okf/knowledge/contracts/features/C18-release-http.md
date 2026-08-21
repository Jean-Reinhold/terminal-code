---
type: Compatibility Contract
title: Release Worker HTTP API
contract_id: C18
description: Preserve stable/dev installer, latest/pinned manifests, downloads, methods, ranges, HEAD, and status codes.
tags: [release, worker, r2, http]
status: draft
implementation_status: rust-schema-partial
risk: critical
owners: [release]
surfaces: [http, object-store]
source_paths: [release-worker/src/index.ts, release-worker/wrangler.toml, src/upgrade.ts, crates/tode-core/src/release.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-core/src/release.rs]
platforms: [worker-wasm]
sources:
  - { id: worker, resource: ../../../../release-worker/src/index.ts, title: Current release worker }
  - { id: config, resource: ../../../../release-worker/wrangler.toml, title: R2 worker configuration }
  - { id: rust, resource: ../../../../crates/tode-core/src/release.rs, title: Rust release manifest and target schemas }
---

# Contract

Serve root and `/install`-prefixed stable/dev installers, latest and pinned manifests with derived URLs, immutable artifact downloads, GET/HEAD only, exact 404/405/503 bodies/statuses, and reviewed content-length/range/ETag/cache semantics without buffering full artifacts or allowing path/filename mismatch.

# Coverage Status

Rust tests cover target-key mapping, manifest build selection/missing-target errors, and stable/dev latest paths. C18 remains draft until the Rust/WASM worker and staged HTTP route/range/HEAD/cache scenarios replace the current worker.
