---
type: Compatibility Contract
title: Release Worker HTTP API
contract_id: C18
description: Preserve stable/dev installer, latest/pinned manifests, downloads, methods, ranges, HEAD, and status codes.
tags: [release, worker, r2, http]
status: draft
risk: critical
owners: [release]
surfaces: [http, object-store]
source_paths: [release-worker/src/index.ts, release-worker/wrangler.toml, src/upgrade.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [worker-wasm]
sources:
  - { id: worker, resource: ../../../../release-worker/src/index.ts, title: Current release worker }
  - { id: config, resource: ../../../../release-worker/wrangler.toml, title: R2 worker configuration }
---

# Contract

Serve root and `/install`-prefixed stable/dev installers, latest and pinned manifests with derived URLs, immutable artifact downloads, GET/HEAD only, exact 404/405/503 bodies/statuses, and reviewed content-length/range/ETag/cache semantics without buffering full artifacts or allowing path/filename mismatch.

# Coverage Status

There is no worker route suite. H3/H5 must freeze the current API with a Bash/Rust HTTP client and staged R2 peer before the Rust/WASM worker replaces it.
