---
type: Compatibility Contract
title: Theme Generation and Live Application
contract_id: C10
description: Preserve palette-derived theme colors, contrast, fingerprints, CSS, font, extension, and live application.
tags: [theme, color, css, font, browser]
status: draft
implementation_status: rust-production-theme
risk: high
owners: [theme, profile]
surfaces: [filesystem, browser, socket]
source_paths: [src/theme/color.ts, src/theme/generate.ts, src/profile.ts, src/browserglue.ts, crates/tode-core/src/color.rs, crates/tode-core/src/theme.rs, crates/tode-profile/src/lib.rs, crates/tode-cli/tests/profile_commands.rs, test/theme.test.js, test/livesync.test.js, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/theme.test.js, test/livesync.test.js, test/browserglue.test.js]
rust_test_paths: [crates/tode-core/src/color.rs, crates/tode-core/src/theme.rs, crates/tode-profile/src/lib.rs, crates/tode-cli/tests/profile_commands.rs]
platforms: [macos, linux]
sources:
  - { id: generator, resource: ../../../../src/theme/generate.ts, title: Theme generator }
  - { id: profile, resource: ../../../../src/profile.ts, title: Theme installation }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Browser live application }
  - { id: rust-color, resource: ../../../../crates/tode-core/src/color.rs, title: Rust sRGB and Oklch color math }
  - { id: rust-theme, resource: ../../../../crates/tode-core/src/theme.rs, title: Complete Rust theme generator }
  - { id: rust-profile, resource: ../../../../crates/tode-profile/src/lib.rs, title: Rust managed theme extension installation }
  - { id: rust-cli, resource: ../../../../crates/tode-cli/tests/profile_commands.rs, title: Production Rust theme command integration }
---

# Contract

Preserve dark/light classification, exact editor/ANSI colors, hue-based semantic accents, separated surfaces, WCAG AA text correction, deterministic fingerprints, generated CSS/font route, extension registration, live theme persistence, activation, and fan-out to every live window.

# Coverage Status

Rust color/theme/profile tests plus production CLI integration cover generation, extension/registry/live files, cleanup, idempotence, and `--theme` output. C10 remains draft until bridge activation and browser live fan-out are ported.
