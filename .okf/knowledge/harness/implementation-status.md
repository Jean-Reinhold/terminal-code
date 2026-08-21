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
* `tode-core` Rust library with CLI identity, target/goto, Unix IPC, OSC palettes, source-preserving JSONC, complete themes, shortcut transforms, and persisted decision-derived bindings.
* `tode-profile` Rust crate with XDG/install ownership, managed/seeded settings, atomic writes, managed theme extension/registry/live files, and full non-UI editor import.
* `tode-runtime` Rust HTTP/1 injector with CSS/font handling, header rewriting, readiness hold, controlled errors, and upgraded-stream bridging.
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

* `tode-harness catalog check`: 22 contracts, 10 scenarios, 119 mapped legacy tests, and 59 contract-mapped Rust tests.
* C01 Rust help/version scenarios matched exact snapshots captured from the legacy CLI.
* All four C02 Rust scenarios matched exact snapshots captured from the legacy exports.
* A sealed help run replayed successfully without executing Node.
* Seventy-three Rust workspace tests passed:
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
  - five profile path/precedence/idempotence/atomic-install tests;
  - six import discovery/progress/settings/keybindings/snippets/tasks/extensions safety/report tests;
  - two managed theme extension/registry/live-file cleanup/idempotence tests.
* `cargo fmt --all` and strict Clippy with `-D warnings` passed.

# Current Trust Boundary

The implemented deterministic core does not depend on any model SDK. There is no `tode-harness-agent` crate yet. This is intentional: agent proposals cannot precede a trustworthy schema, execution, artifact, oracle, and replay path.

All active harness target manifests execute Rust binaries. Legacy behavior is retained only as reviewed snapshots and source/test mapping evidence.

Network policy in scenario v1 is only `not-required`. The S1 runner does not claim hard network denial or loopback isolation on macOS. Those modes remain unavailable until an S2 worker/adapter can attest enforcement.

Execution policy v1 rejects declared retries rather than ignoring them. Classified retry/resumption remains H6 work.

# Not Implemented Yet

* Executable scenarios for C03-C04 and C06-C22; their concepts and legacy test mappings are complete.
* The production M6 `tode` binary and direct dual-target differential execution; C01/C02 currently use reviewed legacy-derived exact snapshots through Rust contract binaries.
* PTY/OSC, browser, terminal hardware, release/R2, and install scenario adapters.
* Hard S2/S3 isolation, total resource budgets, fault injection, and crash resumption.
* Structured agent task envelopes/providers/roles, DeepSeek invocation, redaction, proposal admission, or skeptic/curator workflows.
* JUnit/SARIF/static HTML reports, remote immutable storage, signatures, CI tiers, or release certificates.

# Next Slice

Continue M3/M4 with bridge extension generation/activation and production command wiring. The full C14 claimant graph/convergence loop remains explicitly open.
