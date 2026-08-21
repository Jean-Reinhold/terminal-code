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
* `tode-core` Rust library with target resolution and goto parsing.
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
* C01-C22 individual OKF compatibility concepts.
* Six characterization scenarios: two legacy C01 CLI scenarios and four Rust C02 target/goto scenarios.
* Rust `tode-contract-probe` binary that serializes `tode-core` results without duplicating product logic.

# Verified Behavior

* `tode-harness catalog check`: 22 contracts, 6 reciprocal scenarios, and 119 mapped legacy test declarations.
* C01 help/version scenarios passed against the real legacy CLI in fresh sandboxes.
* All four C02 Rust scenarios matched exact snapshots captured from the legacy exports.
* A sealed help run replayed successfully without executing Node.
* Fourteen Rust workspace tests passed:
  - existing/missing file/folder target resolution;
  - goto parsing and existing numeric-suffix preservation;
  - committed catalog/schema agreement;
  - forbidden shell-kind and absolute fixture rejection;
  - protected HOME override and fixture-symlink rejection;
  - deliberate differential stdout mismatch returns `Failed`;
  - exact pass replays, then corrupted content object makes replay fail;
  - tampered sealed plan makes replay fail;
  - oversized scenario policy fails before artifact/sandbox creation.
  - held port remains unavailable until lease drop;
  - short Unix socket remains connectable until lease drop and is removed;
  - filesystem snapshot records content and excludes logs;
  - oversized output fails the policy budget;
  - timed-out Rust process groups are clean.
* `cargo fmt --all` and strict Clippy with `-D warnings` passed.

# Current Trust Boundary

The implemented deterministic core does not depend on any model SDK. There is no `tode-harness-agent` crate yet. This is intentional: agent proposals cannot precede a trustworthy schema, execution, artifact, oracle, and replay path.

Network policy in scenario v1 is only `not-required`. The S1 runner does not claim hard network denial or loopback isolation on macOS. Those modes remain unavailable until an S2 worker/adapter can attest enforcement.

Execution policy v1 rejects declared retries rather than ignoring them. Classified retry/resumption remains H6 work.

# Not Implemented Yet

* Executable scenarios for C03-C22; their concepts and legacy test mappings are complete.
* A Rust product CLI target for C01 and direct dual-target differential execution; C02 currently uses reviewed legacy-derived exact snapshots.
* Unix-socket transcript, HTTP/WebSocket, PTY/OSC, browser, terminal hardware, release/R2, and install scenario adapters.
* Hard S2/S3 isolation, total resource budgets, fault injection, and crash resumption.
* Structured agent task envelopes/providers/roles, DeepSeek invocation, redaction, proposal admission, or skeptic/curator workflows.
* JUnit/SARIF/static HTML reports, remote immutable storage, signatures, CI tiers, or release certificates.

# Next Slice

Complete the H2 Unix-socket transcript adapter using the held lease broker, typed environment injection, bounded JSON-line peer, transcript artifact, and timeout/cleanup tests. Add a Rust product CLI target for C01 in parallel so help/version can leave the Node oracle.
