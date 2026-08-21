---
type: Constraint Set
title: Rust Rewrite Constraints
description: Non-negotiable boundaries for a behavior-preserving Rust replacement.
tags: [rust, compatibility, migration, constraints]
status: draft
sources:
  - id: readme
    resource: ../../../README.md
    title: Public behavior and platform statement
  - id: tests
    resource: ../../../test
    title: Current behavioral regression suite
  - id: bridge
    resource: ../../../src/bridge.ts
    title: Generated VS Code extension bridge
---

# Product Invariants

1. Preserve every documented and implemented CLI route, including ignored VS Code flags and warning-only unsupported flags.
2. Preserve stdout, stderr, exit status, timeouts, ordering, file formats, default paths, environment overrides, URLs, and protocol payloads unless a separately approved migration changes a contract.
3. Preserve terminal appearance, generated theme behavior, font behavior, web layouts, shortcut decisions, and reversible terminal configuration edits.
4. Preserve macOS and Linux support on arm64 and x86_64 where the release pipeline currently publishes those targets.
5. Never run destructive migration experiments against a developer's real home directory; use isolated XDG roots and terminal fixtures.

# Rust Boundary

All repository-owned domain and application logic moves to Rust. Two host constraints remain explicit:

* terminal-browser and code-server are upstream products and remain external pinned artifacts.
* VS Code's extension host and browsers execute JavaScript or WebAssembly. Rust components may compile to WebAssembly and emit a minimal generated loader, but no independent product logic may remain in hand-maintained JavaScript.

The install bootstrap may remain a minimal POSIX shell selector until a package manager or preselected platform URL can place the Rust installer binary. Build, packaging, verification, publishing, upgrade, and uninstall logic moves to Rust.

# Migration Rules

* Freeze the legacy behavior before replacing internals.
* Use the legacy implementation only as a temporary black-box oracle.
* Migrate complete vertical behavior slices; do not leave placeholder commands or no-op backends.
* Maintain one canonical state format throughout the transition.
* End with a clean cutover: remove TypeScript/JavaScript/Bash application implementations, Node build dependencies, duplicate tests, compatibility shims, and dead release paths in the same release that switches the production entry point.
* Add no unrelated product scope during the rewrite.

# Decision Authority

Draft architecture decisions live under [decisions](../decisions/). A decision becomes stable only after its compatibility evidence and rollback path are reviewed.
