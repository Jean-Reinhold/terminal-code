---
type: Compatibility Contract
title: Launch and Workbench Timing Report
contract_id: C16
description: Preserve timing mark capture, stage labels, missing-data behavior, and terminal report formatting.
tags: [timing, performance, browser]
status: draft
implementation_status: rust-production-parity
risk: medium
owners: [cli, browser]
surfaces: [filesystem, cli, browser]
source_paths: [src/main.ts, src/browserglue.ts, src/browser/preload.ts, src/browser/mainscript.ts, crates/tode-core/src/timing.rs, crates/tode-runtime/src/browser_bridge.rs, crates/tode-cli/tests/open.rs, crates/tode-cli/tests/timing.rs, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/browserglue.test.js]
rust_test_paths: [crates/tode-core/src/timing.rs, crates/tode-runtime/src/browser_bridge.rs, crates/tode-cli/tests/open.rs, crates/tode-cli/tests/timing.rs]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Timing report }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Workbench mark capture }
  - { id: rust, resource: ../../../../crates/tode-core/src/timing.rs, title: Rust timing report formatter }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/timing.rs, title: Production Rust timing command integration }
  - { id: rust-bridge, resource: ../../../../crates/tode-runtime/src/browser_bridge.rs, title: Rust Electron timing bridge and launch record writer }
  - { id: rust-open, resource: ../../../../crates/tode-cli/tests/open.rs, title: Production Rust per-open timing and browser bridge integration }
---

# Contract

Child frames stay out of the timing story; the main workbench records known marks and launch origin. `--timing` alone reads the last record, reports missing data as a zero-exit message, preserves stage labels/order/millisecond formatting and bars, and distinguishes per-open timing when used beside a target.

# Coverage Status

Rust tests cover missing timing data, page age, launch stages, navigation, known workbench marks, fixed-width millisecond rows, nonempty bars, top-frame mark capture, IPC persistence, launch-record writes, terminal-browser preload/main-script arguments, per-open stage output, and timing/theme coexistence in the generated host adapter.
