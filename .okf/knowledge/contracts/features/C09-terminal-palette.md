---
type: Compatibility Contract
title: Terminal Palette and Live Color Input
contract_id: C09
description: Preserve OSC color queries, reply parsing, timeouts, fallbacks, and live palette propagation.
tags: [terminal, osc, palette, live-sync]
status: draft
implementation_status: rust-parser-parity
risk: high
owners: [terminal, theme]
surfaces: [pty, osc, filesystem, socket]
source_paths: [src/terminal/osc.ts, src/livesync.ts, crates/tode-core/src/palette.rs, test/theme.test.js, test/livesync.test.js]
scenario_ids: []
legacy_test_paths: [test/theme.test.js, test/livesync.test.js]
rust_test_paths: [crates/tode-core/src/palette.rs]
platforms: [macos, linux]
sources:
  - { id: osc, resource: ../../../../src/terminal/osc.ts, title: OSC query and parser }
  - { id: sync, resource: ../../../../src/livesync.ts, title: Live palette parsing }
  - { id: tests, resource: ../../../../test/theme.test.js, title: Palette regression tests }
  - { id: rust, resource: ../../../../crates/tode-core/src/palette.rs, title: Rust OSC palette parser and fallbacks }
---

# Contract

Parse BEL/ST-terminated OSC replies, scale every hexadecimal component width to 0–255, preserve answered background/foreground/ANSI slots, fill only missing values, and respect idle/hard-cap behavior. Live color files require both foreground and background and feed every active window.

# Coverage Status

Four Rust tests preserve variable-width scaling, BEL/ST background/foreground/ANSI parsing, selective fallbacks, and query ordering. H3 still needs raw PTY timeout/chunking scenarios and live color-file propagation before the full C09 contract is stable.
