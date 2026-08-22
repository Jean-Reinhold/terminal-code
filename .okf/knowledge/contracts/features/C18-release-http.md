---
type: Compatibility Contract
title: Release Worker HTTP API
contract_id: C18
description: Preserve stable/dev installer, latest/pinned manifests, downloads, methods, ranges, HEAD, and status codes.
tags: [release, worker, r2, http]
status: draft
implementation_status: rust-http-core-partial
risk: critical
owners: [release]
surfaces: [http, object-store]
source_paths: [release-worker/src/index.ts, release-worker/wrangler.toml, src/upgrade.ts, crates/tode-core/src/release.rs, crates/tode-release-http/src/lib.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-core/src/release.rs, crates/tode-release-http/src/lib.rs]
platforms: [worker-wasm]
sources:
  - { id: worker, resource: ../../../../release-worker/src/index.ts, title: Current release worker }
  - { id: config, resource: ../../../../release-worker/wrangler.toml, title: R2 worker configuration }
  - { id: rust, resource: ../../../../crates/tode-core/src/release.rs, title: Rust release manifest and target schemas }
  - { id: rust-http, resource: ../../../../crates/tode-release-http/src/lib.rs, title: Transport-neutral Rust release HTTP router and object-store contract }
---

# Contract

Serve root and `/install`-prefixed stable/dev installers, latest and pinned manifests with derived URLs, immutable artifact downloads, GET/HEAD only, exact 404/405/503 bodies/statuses, and reviewed content-length/range/ETag/cache semantics without buffering full artifacts or allowing path/filename mismatch.

# Coverage Status

Rust tests cover target schemas plus root and `/install` stable/pinned installers, latest/pinned derived manifest URLs, immutable downloads, GET/HEAD, single ranges, ETag 304, content length/type/range/cache headers, channel/path rejection, and exact 404/405/416/500/503 bodies/statuses. C18 remains draft until the thin Cloudflare R2/WASM adapter and staged worker scenarios are added.
