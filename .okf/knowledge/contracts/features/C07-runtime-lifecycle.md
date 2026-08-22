---
type: Compatibility Contract
title: Runtime and Code Server Lifecycle
contract_id: C07
description: Preserve pinned artifact resolution, verification, readiness, warm-up, state, and shutdown.
tags: [runtime, code-server, terminal-browser, lifecycle]
status: draft
implementation_status: rust-daemon-command-parity
risk: critical
owners: [runtime]
surfaces: [process, filesystem, http, download]
source_paths: [src/runtime/release.ts, src/codeserver/server.ts, src/codeserver/vendored.ts, crates/tode-cli/tests/open.rs, crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/browser.rs, crates/tode-runtime/src/process.rs, crates/tode-runtime/src/daemon.rs, crates/tode-runtime/src/bin/tode-daemon.rs, crates/tode-runtime/tests/managed_code_server.rs, crates/tode-runtime/tests/daemon.rs, crates/tode-runtime/tests/daemon_command.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-cli/tests/open.rs, crates/tode-runtime/src/artifact.rs, crates/tode-runtime/src/browser.rs, crates/tode-runtime/src/process.rs, crates/tode-runtime/src/daemon.rs, crates/tode-runtime/src/bin/tode-daemon.rs, crates/tode-runtime/tests/managed_code_server.rs, crates/tode-runtime/tests/daemon.rs, crates/tode-runtime/tests/daemon_command.rs]
platforms: [macos, linux]
sources:
  - { id: runtime, resource: ../../../../src/runtime/release.ts, title: Browser runtime resolution }
  - { id: server, resource: ../../../../src/codeserver/server.ts, title: Code server lifecycle }
  - { id: vendored, resource: ../../../../src/codeserver/vendored.ts, title: Pinned code-server artifact }
  - { id: rust, resource: ../../../../crates/tode-runtime/src/artifact.rs, title: Rust verified download, safe extraction, and atomic swap }
  - { id: rust-process, resource: ../../../../crates/tode-runtime/src/process.rs, title: Rust managed server state, readiness, and shutdown }
  - { id: rust-spawn, resource: ../../../../crates/tode-runtime/tests/managed_code_server.rs, title: Rust exact spawn and process ownership integration }
  - { id: rust-daemon, resource: ../../../../crates/tode-runtime/tests/daemon.rs, title: Composed Rust code-server/injector/state integration }
  - { id: rust-browser, resource: ../../../../crates/tode-runtime/src/browser.rs, title: Rust terminal-browser existing-source resolver and launcher }
  - { id: rust-daemon-command, resource: ../../../../crates/tode-runtime/tests/daemon_command.rs, title: Persistent Rust daemon command integration }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/open.rs, title: Production Rust runtime start/reuse/launch/shutdown integration }
---

# Contract

Prefer verified vendored/offline artifacts, preserve target triples and platform binary layout, reject size/SHA mismatch before unpack, coordinate one boot, validate PID/ports/version/readiness, warm the injector, persist canonical server state, and stop only the managed process.

# Coverage Status

Twenty-two Rust tests cover all artifact/runtime sources, launchers, state/readiness, exact code-server spawn, composed/persistent daemon, downloaded resolution, production CLI start, workbench launch, shutdown, and state cleanup. C07 remains draft for real pinned upstream hardware/platform certification and bridge/preload integration.
