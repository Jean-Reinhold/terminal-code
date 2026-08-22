---
type: Compatibility Contract
title: Startup Ordering and Onboarding
contract_id: C06
description: Preserve runtime, profile, shortcut, bridge, server, onboarding, and browser launch ordering.
tags: [startup, orchestration, onboarding]
status: draft
implementation_status: rust-basic-open
risk: high
owners: [core, runtime]
surfaces: [process, filesystem, terminal, browser]
source_paths: [src/main.ts, src/codeserver/server.ts, src/onboarding.ts, crates/tode-cli/src/main.rs, crates/tode-cli/tests/open.rs]
scenario_ids: []
legacy_test_paths: []
rust_test_paths: [crates/tode-cli/tests/open.rs]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Open orchestration }
  - { id: server, resource: ../../../../src/codeserver/server.ts, title: Managed server }
  - { id: onboarding, resource: ../../../../src/onboarding.ts, title: First-run stages }
  - { id: rust, resource: ../../../../crates/tode-cli/tests/open.rs, title: Rust profile-daemon-browser startup integration }
---

# Contract

Resolve the browser runtime and code-server before use; derive/install profile state before workbench display; auto-apply shared shortcuts; finalize bridge/keybindings/server once; preserve onboarding ownership and pane exit; launch the browser only after the final workbench URL exists. Timing labels retain their order.

# Coverage Status

The Rust integration test proves profile/theme/CSS installation, daemon readiness, code-server/injector composition, terminal-browser resolution, workbench URL launch, and shutdown order. C06 remains draft until onboarding, split/timing stages, reuse, and failure-order scenarios are ported.
