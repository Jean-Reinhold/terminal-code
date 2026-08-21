---
type: Compatibility Contract
title: Runtime and Code Server Lifecycle
contract_id: C07
description: Preserve pinned artifact resolution, verification, readiness, warm-up, state, and shutdown.
tags: [runtime, code-server, terminal-browser, lifecycle]
status: draft
implementation_status: rust-state-partial
risk: critical
owners: [runtime]
surfaces: [process, filesystem, http, download]
source_paths: [src/runtime/release.ts, src/codeserver/server.ts, src/codeserver/vendored.ts, crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/process.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/process.rs]
platforms: [macos, linux]
sources:
  - { id: runtime, resource: ../../../../src/runtime/release.ts, title: Browser runtime resolution }
  - { id: server, resource: ../../../../src/codeserver/server.ts, title: Code server lifecycle }
  - { id: vendored, resource: ../../../../src/codeserver/vendored.ts, title: Pinned code-server artifact }
  - { id: rust, resource: ../../../../crates/tode-runtime/src/artifact.rs, title: Rust verified download, safe extraction, and atomic swap }
  - { id: rust-process, resource: ../../../../crates/tode-runtime/src/process.rs, title: Rust managed server state, readiness, and shutdown }
---

# Contract

Prefer verified vendored/offline artifacts, preserve target triples and platform binary layout, reject size/SHA mismatch before unpack, coordinate one boot, validate PID/ports/version/readiness, warm the injector, persist canonical server state, and stop only the managed process.

# Coverage Status

Ten Rust tests cover exact streamed size/SHA verification, safe bounded extraction/swap rollback, typed state round-trip, PID liveness, dual-listener validation, delayed readiness, stale-state cleanup, and injector origin. C07 remains draft until terminal-browser/code-server artifact resolution, exact spawn arguments/environment/logging, injector composition, warm-up, and live managed-process shutdown are wired.
