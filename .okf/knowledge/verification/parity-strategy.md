---
type: Verification Strategy
title: Legacy-to-Rust Parity Strategy
description: Evidence required to replace the legacy implementation without feature or characteristic drift.
tags: [verification, parity, tests, release-gate]
status: draft
sources:
  - id: tests
    resource: ../../../test
    title: Existing Node regression suite
  - id: contract
    resource: ../contracts/compatibility.md
    title: Behavioral compatibility matrix
  - id: plan
    resource: ../plans/rust-rewrite.md
    title: Rewrite milestones
---

# Oracle Model

The current release is the behavior oracle only after its output is frozen by fixtures. The harness runs legacy and Rust implementations separately against copies of the same sandbox, then compares normalized observations. It never lets both write the same live HOME/XDG tree.

Normalization is narrow and explicit: temporary root, selected free ports, PIDs, monotonic durations, timestamps, and download host. Error text, ordering, JSON omission, file bytes, HTTP headers, and exit status remain unnormalized unless the compatibility contract says otherwise.

The authoritative implementation is specified by the [harness architecture](../harness/architecture.md), [scenario DSL](../harness/scenario-dsl.md), [sandbox](../harness/sandboxing.md), [surface adapters](../harness/surface-adapters.md), and [oracle rules](../harness/oracles-and-normalization.md). Agents can broaden coverage and review evidence under the [agent workflow](../harness/agent-orchestration.md); deterministic code alone executes and decides verdicts.

# Verification Layers

## Pure Differential Tests

Run identical corpora through target/goto parsing, JSONC patching, chord conversion, terminal reply parsing, color/theme generation, fingerprints, manifest parsing, and config transforms. Compare returned structures and serialized bytes.

Add property/fuzz checks for parser termination, path safety, malformed input, Unicode, partial escape sequences, idempotence, and collision/convergence invariants. A fuzz result supplements but never replaces known behavior fixtures.

## Filesystem Scenarios

Use isolated HOME and all XDG roots. Snapshot file tree, modes, symlinks, and contents before/after profile, import, shortcut, upgrade, and uninstall operations. Exercise interrupted writes and permissions failures. Assert unrelated user bytes survive.

## Wire and Network Scenarios

Capture exact Unix socket JSON lines/replies, HTTP injection responses, WebSocket upgrades, release worker routes/headers/ranges/HEAD, download size/SHA failures, readiness deadlines, and retry/error mapping using local deterministic peers.

## Process and Terminal Scenarios

Use controlled child processes to exercise PID state, process groups, already-running/stale state, signals, shutdown, wait, terminal ancestry, Ghostty/Kitty reload, OSC reply timing, and orphan cleanup. Run actual `tode` terminal smoke scenarios for launch/reuse/wait/shutdown.

## Browser Scenarios

Drive the real embedded and public pages. Verify accessibility trees, keyboard interaction, progress/apply/cancel behavior, live theme changes, responsive breakpoints, install flow, video controls, metadata, and routes. Capture screenshots at fixed viewports and compare with reviewed tolerances; DOM-only tests do not prove visual parity.

## Distribution Scenarios

From a clean checkout, build release candidates in CI, inspect archive contents, install into a clean home, launch offline, query manifests, upgrade through staged channels, roll back to the prior complete release, and uninstall. Verify SHA-256 and declared size before unpack.

# Platform Matrix

| Dimension | Required coverage |
|---|---|
| OS | current supported macOS and Linux releases |
| Architecture | arm64 and x86_64 where artifacts are published |
| Terminal | Ghostty and Kitty; unsupported terminal behavior fixture |
| Display/theme | dark, light, black, white, low-contrast, incomplete OSC reply |
| code-server | exactly the pinned version; pin-update job regenerates keymaps and re-runs parity |
| Network | offline vendored first run, normal download, truncation, wrong size/hash, unavailable service |
| State | clean install, existing legacy state, malformed/stale state, interrupted upgrade, uninstall |

Hardware-backed jobs are required for terminal/browser smoke coverage that emulation cannot prove. Cross-compilation proves compilation only.

# Release Gates

A Rust release candidate may replace legacy only when:

1. C01–C22 have passing automated fixtures or reviewed manual browser/hardware evidence.
2. No unexplained differential remains.
3. Both terminal backend closed loops converge and second apply changes no byte.
4. User-state operations are atomic, source-preserving, and idempotent.
5. Real launch/reuse/wait/theme/shutdown scenarios pass on macOS and Linux.
6. Worker/site/install/upgrade/rollback/uninstall scenarios pass from staged deployment.
7. Supported-target artifacts are built from the same commit and manifest transaction.
8. The final repository contains no reachable legacy production implementation.
9. Every verdict references sealed [content-addressed evidence](../harness/evidence-and-artifacts.md) and replays without target/model access.
10. Required S2/S3 platform, hardware, security, and agent-review freshness gates pass; unavailable means inconclusive, not pass.
11. A signed T5 release certificate covers the exact build/artifact/manifest/evidence-root digests.
12. Deterministic required suites remain operable during total provider outage.

# Existing Coverage Worth Retaining

The current suite has 119 tests across eight files and already contains strong behavioral checks for target resolution, HTTP/CSS/WebSocket injection, browser theme/timing fan-out, startup marker single-use behavior, terminal reply/color fidelity, WCAG theme contrast, JSONC comment preservation, Ghostty/Kitty transform behavior, and shortcut closed-loop idempotence. H0 maps each test to a contract or harness invariant before reorganizing it; test names and fixtures are compatibility evidence.

# Known Coverage Gaps to Freeze in M0

* Full CLI stdout/stderr/exit and option-precedence matrix.
* Real process lifecycle and stale PID/port recovery.
* Release worker route/header/range contract.
* Upgrade interruption/rollback and uninstall retention matrix.
* Kitty closed-loop parity equal to Ghostty's adversarial suite.
* Public and embedded page browser/visual/accessibility behavior.
* Installer/archive behavior on every published target.
* Per-contract OKF ownership/risk/staleness and source-symbol coverage graph.
* Agent model/provider provenance, outage behavior, prompt-injection, and fabricated-evidence rejection.
* Harness containment, normalizer/oracle mutation, evidence corruption/replay, and release-certificate verification.
