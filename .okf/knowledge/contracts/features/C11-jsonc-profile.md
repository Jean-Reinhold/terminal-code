---
type: Compatibility Contract
title: Source Preserving JSONC and Profile State
contract_id: C11
description: Preserve JSONC comments and unrelated bytes while enforcing managed and seeded profile settings.
tags: [jsonc, profile, settings, keybindings]
status: draft
implementation_status: rust-profile-partial
risk: critical
owners: [profile]
surfaces: [filesystem]
source_paths: [src/jsonc.ts, src/profile.ts, crates/tode-core/src/jsonc.rs, crates/tode-profile/src/lib.rs, test/theme.test.js]
scenario_ids: []
legacy_test_paths: [test/theme.test.js]
rust_test_paths: [crates/tode-core/src/jsonc.rs, crates/tode-profile/src/lib.rs]
platforms: [macos, linux]
sources:
  - { id: jsonc, resource: ../../../../src/jsonc.ts, title: Source-preserving JSONC implementation }
  - { id: profile, resource: ../../../../src/profile.ts, title: Managed profile state }
  - { id: tests, resource: ../../../../test/theme.test.js, title: JSONC/profile tests }
  - { id: rust, resource: ../../../../crates/tode-core/src/jsonc.rs, title: Rust source-preserving JSONC implementation }
  - { id: rust-profile, resource: ../../../../crates/tode-profile/src/lib.rs, title: Rust profile paths, settings precedence, and atomic writes }
---

# Contract

Read comments and trailing commas, return null rather than throw on malformed input, edit only managed keys, preserve comments/unrelated keys and stable formatting, seed missing values without overwriting users, force managed values, and make repeated writes byte-idempotent. Keybinding records preserve foreign bindings and removal semantics.

# Coverage Status

Ten Rust tests cover source-preserving JSONC plus exact XDG/install path rules, seeded-versus-managed precedence, comments, byte idempotence, atomic mode-preserving writes, and profile creation. C11 remains draft until keybinding merge/removal, theme/font extension installation, live files, and full profile migration are wired.
