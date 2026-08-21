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
source_paths: [src/runtime/release.ts, src/codeserver/server.ts, src/codeserver/vendored.ts, crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/process.rs, crates/tode-runtime/src/daemon.rs, crates/tode-runtime/tests/managed_code_server.rs, crates/tode-runtime/tests/daemon.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/process.rs, crates/tode-runtime/src/daemon.rs, crates/tode-runtime/tests/managed_code_server.rs, crates/tode-runtime/tests/daemon.rs]
platforms: [macos, linux]
sources:
  - { id: runtime, resource: ../../../../src/runtime/release.ts, title: Browser runtime resolution }
  - { id: server, resource: ../../../../src/codeserver/server.ts, title: Code server lifecycle }
  - { id: vendored, resource: ../../../../src/codeserver/vendored.ts, title: Pinned code-server artifact }
  - { id: rust, resource: ../../../../crates/tode-runtime/src/artifact.rs, title: Rust verified download, safe extraction, and atomic swap }
  - { id: rust-process, resource: ../../../../crates/tode-runtime/src/process.rs, title: Rust managed server state, readiness, and shutdown }
  - { id: rust-spawn, resource: ../../../../crates/tode-runtime/tests/managed_code_server.rs, title: Rust exact spawn and process ownership integration }
  - { id: rust-daemon, resource: ../../../../crates/tode-runtime/tests/daemon.rs, title: Composed Rust code-server/injector/state integration }
---

# Contract

Prefer verified vendored/offline artifacts, preserve target triples and platform binary layout, reject size/SHA mismatch before unpack, coordinate one boot, validate PID/ports/version/readiness, warm the injector, persist canonical server state, and stop only the managed process.

# Coverage Status

Fourteen Rust tests cover verified artifacts/swaps, state/PID/readiness, exact process spawn, combined code-server/injector state, proxied origin, warm-up-safe startup, and complete shutdown/state removal. C07 remains draft until terminal-browser/code-server artifact resolution and persistent daemon command orchestration are wired.
