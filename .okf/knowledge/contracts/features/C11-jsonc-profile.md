---
type: Compatibility Contract
title: Source Preserving JSONC and Profile State
contract_id: C11
description: Preserve JSONC comments and unrelated bytes while enforcing managed and seeded profile settings.
tags: [jsonc, profile, settings, keybindings]
status: draft
risk: critical
owners: [profile]
surfaces: [filesystem]
source_paths: [src/jsonc.ts, src/profile.ts, test/theme.test.js]
scenario_ids: []
legacy_test_paths: [test/theme.test.js]
platforms: [macos, linux]
sources:
  - { id: jsonc, resource: ../../../../src/jsonc.ts, title: Source-preserving JSONC implementation }
  - { id: profile, resource: ../../../../src/profile.ts, title: Managed profile state }
  - { id: tests, resource: ../../../../test/theme.test.js, title: JSONC/profile tests }
---

# Contract

Read comments and trailing commas, return null rather than throw on malformed input, edit only managed keys, preserve comments/unrelated keys and stable formatting, seed missing values without overwriting users, force managed values, and make repeated writes byte-idempotent. Keybinding records preserve foreign bindings and removal semantics.

# Coverage Status

The theme suite maps its JSONC/profile cases here. H3 ports exact before/after byte fixtures and hostile malformed/permission/atomic-write cases.
