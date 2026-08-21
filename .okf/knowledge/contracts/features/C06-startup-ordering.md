---
type: Compatibility Contract
title: Startup Ordering and Onboarding
contract_id: C06
description: Preserve runtime, profile, shortcut, bridge, server, onboarding, and browser launch ordering.
tags: [startup, orchestration, onboarding]
status: draft
risk: high
owners: [core, runtime]
surfaces: [process, filesystem, terminal, browser]
source_paths: [src/main.ts, src/codeserver/server.ts, src/onboarding.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Open orchestration }
  - { id: server, resource: ../../../../src/codeserver/server.ts, title: Managed server }
  - { id: onboarding, resource: ../../../../src/onboarding.ts, title: First-run stages }
---

# Contract

Resolve the browser runtime and code-server before use; derive/install profile state before workbench display; auto-apply shared shortcuts; finalize bridge/keybindings/server once; preserve onboarding ownership and pane exit; launch the browser only after the final workbench URL exists. Timing labels retain their order.

# Coverage Status

No complete startup trace exists. H3 must add a Rust event-log scenario with fake runtime, terminal, server, onboarding, pane, and browser adapters.
