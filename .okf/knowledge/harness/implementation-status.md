---
type: Implementation Status
title: Harness Implementation Status
description: Implemented deterministic C01/C02 vertical slice, evidence, proof, and explicit remaining boundaries.
tags: [harness, implementation, c01, c02]
status: draft
sources:
  - id: crate
    resource: ../../../crates/tode-harness
    title: Deterministic Rust harness crate
  - id: scenarios
    resource: ../../../harness/scenarios/cli
    title: Executable CLI scenarios
  - id: tests
    resource: ../../../crates/tode-harness/tests/harness.rs
    title: Harness false-pass and containment tests
---

# Implemented

* Root Cargo workspace pinned to Rust 1.98.0.
* `tode-core` Rust library with CLI identity, target/goto, Unix IPC, OSC palettes, source-preserving JSONC, complete themes, shortcut transforms/decisions, and release target/manifest/receipt schemas.
* `tode-profile` Rust crate with XDG/install ownership, managed/seeded settings, atomic writes, managed theme and dependency-free window bridge extensions, full non-UI editor import, Ghostty/Kitty shortcut provider orchestration, and shortcut decision manager state.
* `tode-runtime` Rust downloaded/existing terminal-browser resolution, verified artifacts/launcher, generated Electron timing/live-theme bridge with Rust conversion helper, token-scoped embedded shortcut manager, and persistent managed code-server/injector daemon command.
* Production Rust `tode` binary with help/version, full compatibility/open parsing, multiple-target startup, no-window add/reuse fallback, extensions, shortcut setup/undo/TTY manager, import/palette and JSONC-file theme/timing/skill/upgrade/uninstall, existing-window reuse, new-window goto/review/diff startup, profile/CSS/keybindings, daemon, timed browser-bridge launch, and shutdown.
* `tode-harness` binary/library with `catalog check`, `schema`, `run`, and `replay` commands.
* YAML-backed OKF catalog with 22 contract concepts, draft-aware executable coverage, reciprocal scenario links, risk/owner/surface/platform/source validation, and all 119 legacy test declarations mapped.
* Strict JSONC scenario v1 compiler and generated JSON Schema.
* Reviewed target manifests with executable and manifest SHA-256 identity.
* S1 unique sandbox with HOME/XDG/install roots, protected environment, safe relative paths, fixture copy, symlink rejection, and process-group teardown.
* `process.exec` and `process.result` adapter path with timeout, raw stdout/stderr, exit/signal, and sandbox-root normalization.
* Exact process snapshots and differential process equality.
* Explicit sealed `plan.json` with serialized policy, target identities, scenario/baseline artifacts, deterministic run key, manifest plan digest, append-only event log, evidence root, and offline replay.
* Canonical sandbox filesystem-tree observations with file content stored by SHA-256, log exclusion, and differential comparison.
* Held loopback port and short Unix-socket lease broker, 1 MiB process-output budget, and process-group cleanup invariant.
* Bounded Unix JSON-line peer scenario adapter with typed environment injection, transcript artifacts, exact JSON assertions, timeout, and oversized-request rejection.
* C01-C22 individual OKF compatibility concepts.
* Ten Rust characterization scenarios: two C01 CLI identity, four C02 target/goto, and four C05 IPC.
* Rust `tode-contract-probe` binary that serializes `tode-core` results without duplicating product logic.
* Rust `tode-contract-cli` binary for exact help/version identity; no active harness target executes Node.

# Verified Behavior

* `tode-harness catalog check`: 22 contracts, 10 scenarios, 119 mapped legacy tests, and 135 contract-mapped Rust tests.
* C01 Rust help/version scenarios matched exact snapshots captured from the legacy CLI.
* All four C02 Rust scenarios matched exact snapshots captured from the legacy exports.
* A sealed help run replayed successfully without executing Node.
* One hundred forty-nine Rust workspace tests passed:
  - existing/missing file/folder target resolution;
  - goto parsing and existing numeric-suffix preservation;
  - CLI help completeness and version receipt/fallback;
  - IPC JSON-line framing and omitted optional fields;
  - IPC explicit refusal and unreadable reply errors;
  - IPC read timeout mapping;
  - IPC missing socket error;
  - committed catalog/schema agreement;
  - forbidden shell-kind and absolute fixture rejection;
  - protected HOME override and fixture-symlink rejection;
  - deliberate differential stdout mismatch returns `Failed`;
  - exact pass replays, then corrupted content object makes replay fail;
  - tampered sealed plan makes replay fail;
  - oversized scenario policy fails before artifact/sandbox creation;
  - held port remains unavailable until lease drop;
  - short Unix socket remains connectable until lease drop and is removed;
  - filesystem snapshot records content and excludes logs;
  - oversized output fails the policy budget;
  - timed-out Rust process groups are clean;
  - oversized Unix request is rejected;
  - missing Unix connection times out cleanly;
  - seven injector tests cover all fourteen mapped legacy HTTP/WebSocket/CSS/font/readiness behaviors;
  - four OSC palette parsing/fallback/query tests;
  - five source-preserving JSONC parse/edit/idempotence tests;
  - four sRGB/Oklch/contrast/shade/legibility tests;
  - six full-theme dark/light/surface/ANSI/WCAG/fingerprint/completeness tests;
  - eight chord/Ghostty/Kitty conversion, config, include, emit, and shared-rebind tests;
  - four persisted shortcut claim/import/quit/fallback binding tests;
  - four provider detection/effective-scan/shared-convergence/atomic-undo/keybinding-reconciliation tests;
  - five manager row/occupancy/twin-cleanup/persist-reopen/cyclic-claim/full-adversarial state-machine tests;
  - one bounded token-scoped embedded manager HTTP decision/apply/done integration test;
  - five production shortcut unsupported/readiness/undo/no-conflict/non-TTY command integration tests;
  - two exact terminal-ancestry/signal and bounded-hop reload tests;
  - one exact Ghostty action-document first-sentence parser test;
  - five profile path/precedence/idempotence/atomic-install tests;
  - six import discovery/progress/settings/keybindings/snippets/tasks/extensions safety/report tests;
  - two managed theme extension/registry/live-file cleanup/idempotence tests;
  - two dependency-free bridge install/registry/idempotence and startup-marker tests;
  - five release target/manifest/build-selection/receipt tests;
  - five verified download/extraction/link-limit/atomic-swap tests;
  - five server state/PID/readiness/dual-listener/stale-cleanup tests;
  - nine terminal-browser layout/precedence/clone/launcher and generated timing/live-theme/fan-out bridge tests;
  - one production raw-terminal-colors to full Rust theme helper command test;
  - one release lookup/download/strip-one/unpack/launcher composition test;
  - two persistent daemon argument/readiness/SIGTERM/child/state-cleanup tests;
  - seven workbench URL, compatibility/open parser, production new-open, IPC-reuse, daemon/browser, and shutdown tests;
  - two extension parser/order/profile/list/output integration tests;
  - one production editor discovery/import and theme installation integration test;
  - four safe uninstall service/production integration tests;
  - five verified upgrade outcome/transaction/production-check/full-swap/daemon-cleanup tests.
  - four timing formatter and production command integration tests.
  - one production live-state skill document and first-argument dispatch integration test;
* `cargo fmt --all` and strict Clippy with `-D warnings` passed.

# Current Trust Boundary

The implemented deterministic core does not depend on any model SDK. There is no `tode-harness-agent` crate yet. This is intentional: agent proposals cannot precede a trustworthy schema, execution, artifact, oracle, and replay path.

All active harness target manifests execute Rust binaries. Legacy behavior is retained only as reviewed snapshots and source/test mapping evidence.

Network policy in scenario v1 is only `not-required`. The S1 runner does not claim hard network denial or loopback isolation on macOS. Those modes remain unavailable until an S2 worker/adapter can attest enforcement.

Execution policy v1 rejects declared retries rather than ignoring them. Classified retry/resumption remains H6 work.

# Not Implemented Yet

* Remaining direct dual-target differential execution.
* Executable scenarios for C03-C04 and C06-C22; their concepts and legacy test mappings are complete.
* PTY/OSC, browser, terminal hardware, release/R2, and install scenario adapters.
* Hard S2/S3 isolation, total resource budgets, fault injection, and crash resumption.
* Structured agent task envelopes/providers/roles, DeepSeek invocation, redaction, proposal admission, or skeptic/curator workflows.
* JUnit/SARIF/static HTML reports, remote immutable storage, signatures, CI tiers, or release certificates.

# Next Slice

Continue automated real terminal theme-change certification, then executable scenarios and direct dual-target differential execution.
