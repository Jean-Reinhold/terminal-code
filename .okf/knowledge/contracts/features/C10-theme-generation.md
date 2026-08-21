---
type: Compatibility Contract
title: Theme Generation and Live Application
contract_id: C10
description: Preserve palette-derived theme colors, contrast, fingerprints, CSS, font, extension, and live application.
tags: [theme, color, css, font, browser]
status: draft
risk: high
owners: [theme, profile]
surfaces: [filesystem, browser, socket]
source_paths: [src/theme/color.ts, src/theme/generate.ts, src/profile.ts, src/browserglue.ts, test/theme.test.js, test/livesync.test.js, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/theme.test.js, test/livesync.test.js, test/browserglue.test.js]
platforms: [macos, linux]
sources:
  - { id: generator, resource: ../../../../src/theme/generate.ts, title: Theme generator }
  - { id: profile, resource: ../../../../src/profile.ts, title: Theme installation }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Browser live application }
---

# Contract

Preserve dark/light classification, exact editor/ANSI colors, hue-based semantic accents, separated surfaces, WCAG AA text correction, deterministic fingerprints, generated CSS/font route, extension registration, live theme persistence, activation, and fan-out to every live window.

# Coverage Status

Theme, live-sync, and browser-glue suites map here. H3 must retain numerical/byte goldens and add real browser application evidence.
