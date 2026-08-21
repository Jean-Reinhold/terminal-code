---
type: Compatibility Contract
title: Theme Generation and Live Application
contract_id: C10
description: Preserve palette-derived theme colors, contrast, fingerprints, CSS, font, extension, and live application.
tags: [theme, color, css, font, browser]
status: draft
implementation_status: rust-theme-parity
risk: high
owners: [theme, profile]
surfaces: [filesystem, browser, socket]
source_paths: [src/theme/color.ts, src/theme/generate.ts, src/profile.ts, src/browserglue.ts, crates/tode-core/src/color.rs, crates/tode-core/src/theme.rs, test/theme.test.js, test/livesync.test.js, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/theme.test.js, test/livesync.test.js, test/browserglue.test.js]
rust_test_paths: [crates/tode-core/src/color.rs, crates/tode-core/src/theme.rs]
platforms: [macos, linux]
sources:
  - { id: generator, resource: ../../../../src/theme/generate.ts, title: Theme generator }
  - { id: profile, resource: ../../../../src/profile.ts, title: Theme installation }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Browser live application }
  - { id: rust-color, resource: ../../../../crates/tode-core/src/color.rs, title: Rust sRGB and Oklch color math }
  - { id: rust-theme, resource: ../../../../crates/tode-core/src/theme.rs, title: Complete Rust theme generator }
---

# Contract

Preserve dark/light classification, exact editor/ANSI colors, hue-based semantic accents, separated surfaces, WCAG AA text correction, deterministic fingerprints, generated CSS/font route, extension registration, live theme persistence, activation, and fan-out to every live window.

# Coverage Status

Ten Rust tests cover sRGB/Oklch round trips, alpha/hex, extreme surface separation, legibility/contrast, dark/light classification, ANSI fidelity, fingerprints, complete workbench colors, and token scopes. The full generated theme map is ported. C10 remains draft until M3/M4 wire extension installation, live persistence, bridge activation, and browser fan-out.
