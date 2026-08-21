---
type: Migration Plan
title: Rust Rewrite and Clean Cutover
description: Executable milestone plan for replacing the repository while preserving behavior.
tags: [rust, migration, parity, active-plan]
status: draft
sources:
  - id: current
    resource: ../architecture/current-system.md
    title: Current system architecture
  - id: target
    resource: ../architecture/target-rust-workspace.md
    title: Target Rust workspace
  - id: contracts
    resource: ../contracts/compatibility.md
    title: Behavioral compatibility matrix
---

# Objective

Replace repository-owned TypeScript, JavaScript, and Bash application logic with the [target Rust workspace](../architecture/target-rust-workspace.md) without changing the [compatibility matrix](../contracts/compatibility.md). End in one production implementation, not a permanent bridge architecture.

The [agentic harness roadmap](../harness/implementation-roadmap.md) is part of this plan, not later test cleanup. H0–H3 freeze and execute contracts before load-bearing ports; H4 adds agents only after deterministic execution/evidence boundaries; H7 certifies releases; H8 proves clean cutover.

# Milestones

## M0 — Contract Freeze

Work:

* Assign every public command, state file, protocol, page, release route, and platform behavior a compatibility ID.
* Decompose C01–C22 into individual OKF compatibility concepts with risk, owner, surfaces, source/symbol mappings, scenario IDs, platforms, and staleness.
* Map all 119 current tests to contracts or explicit harness invariants.
* Add missing black-box fixtures to the current implementation.
* Capture normalized CLI output, file bytes, JSON lines, OSC bytes, HTTP exchanges, process events, and browser screenshots.
* Record current archive layout and release-worker headers/range behavior.

Exit:

* Every C01–C22 contract has a legacy fixture or an explicit manual browser scenario.
* Fixtures run only in isolated HOME/XDG roots.
* `tode-harness catalog check` compiles the concepts deterministically and fails duplicate, stale, broken, or unmapped high-risk coverage.
* No Rust production path exists yet.

## M1 — Workspace and Compatibility Harness

Work:

* Add the root Cargo workspace, pinned toolchain, dependency policy, native and WASM CI jobs.
* Implement H1–H2 of the [harness roadmap](../harness/implementation-roadmap.md): strict scenario JSONC/schema compiler, run state machine, S0/S1 sandbox, core surface adapters, content-addressed evidence, exact/differential/invariant oracles, and replay.
* Run legacy and Rust targets against separate clones of the same sandbox fixture; normalize only registered allocated paths/ports/PIDs/clocks.
* Add `tode-harness-agent` task/provenance schemas without making agent execution a prerequisite for deterministic core tests.
* Define shared release manifest and IPC DTOs in `tode-protocol`.
* Add `xtask` entry points for focused build/check/package actions; do not replace release scripts yet.

Exit:

* Empty workspace builds on macOS/Linux arm64/x86_64 CI targets that are available.
* The harness proves it can detect an intentional mismatch in stdout, a state file, a socket frame, and an HTTP response.
* A sealed run can replay an intentional stdout, state-file, socket-frame, and HTTP mismatch without re-running targets or contacting a model.
* Malicious scenario/path/symlink/process/secret fixtures fail containment before they can create a passing result.

## M2 — Pure Algorithms and Schemas

Work:

* Port target/goto parsing, chord canonicalization/trigger conversion, release target triples/manifests, OSC reply parsing, color/theme generation, fingerprints, and source-preserving JSONC reads/edits.
* Keep algorithms byte/numeric-compatible; do not “improve” formatting or color math during the port.
* Run differential fixture corpora, including malformed and extreme inputs.

Exit:

* Pure contracts C01/C02 parsing portions, C09/C10, C11 parser behavior, and manifest schemas match all goldens.
* Fuzz/property checks establish parser termination and write idempotence.

## M3 — Profile, Import, and User State

Work:

* Implement exact XDG/install path resolution and atomic writes.
* Port font/assets, settings precedence, theme extension, keybinding merge/record, live theme file, editor discovery, import report/progress, first-run markers, and onboarding state.
* Preserve current JSON and JSONC schemas so no migration runs merely because the binary changed language.

Exit:

* C10–C12 pass against sandboxed fixture trees.
* Interrupted writes leave the previous valid files untouched.
* Repeating every operation changes no byte after convergence.

## M4 — Protocols, Injector, and Runtime

Work:

* Implement Unix-socket framing/replies and startup marker handling.
* Implement the code-server HTTP/WebSocket injector, CSS/font handling, and controlled errors.
* Port verified runtime/code-server fetch, unpack, readiness, warm-up, server state, process groups, and shutdown.
* Generate the minimal VS Code bridge host adapter from versioned templates with Rust-owned DTOs.

Exit:

* C05, C07, C08, and C15 pass, including byte-level wire tests and real child-process smoke scenarios.
* Orphan/restart/interrupted-download cases leave no false running state or partial accepted artifact.

## M5 — Shortcut System and Embedded UI

Work:

* Port decisions, holds, imported/extension/default keymaps, provider scans, Ghostty/Kitty config transforms, ancestry detection, signals, reload, undo, and wizard state machine.
* Port embedded shortcut/import pages to Rust/WASM while preserving routes/tokens/state transitions/CSS.
* Keep terminal file changes in tode-owned includes.

Exit:

* C13/C14/C22 pass for both terminal backends and both OS shortcut conventions.
* Closed-loop adversarial suite converges and second apply is byte-idempotent.
* Real browser scenarios pass for apply, cancel, progress, collision, and expired/invalid token paths.

## M6 — CLI and Launch Orchestration

Work:

* Implement exact help/version text and flexible VS Code-compatible option parsing.
* Port command dispatch, existing-window reuse, launch/finalize/onboarding state machine, extension management, timing, theme, skill, shutdown, upgrade, uninstall, stdio, and exit mapping.
* Switch development smoke scenarios to the Rust binary while keeping legacy differential runs.

Exit:

* C01–C17 and C20 pass end to end.
* A real terminal smoke run opens a folder, reuses the window for a file/goto/diff/review, waits correctly, updates theme, and shuts down cleanly.

## M7 — Web, Worker, and Distribution

Work:

* Port the public site and metadata to the Rust web stack; preserve CSS, assets, responsive layout, video/install interactions, analytics behavior, and `/install` routing.
* Port the Cloudflare worker to Rust/WASM with identical R2 keys, routes, bodies, headers, status codes, ranges, and HEAD behavior.
* Replace build/dist/keymap/version/release/publish scripts with `xtask` and Rust installer/upgrader logic. Retain only the minimal bootstrap boundary described in constraints.
* Build artifacts in clean CI and publish to a non-production channel first.

Exit:

* C18/C19/C21 pass against staged R2/worker/site deployment.
* Artifacts install, run offline on first launch, upgrade atomically, roll back, and uninstall on every supported target.
* Archive contents, manifest metadata, size, SHA-256, and URLs are produced by one release transaction.

## M8 — Clean Cutover

Work:

* Run the full [parity gate](../verification/parity-strategy.md) on release candidates.
* Require a valid T5 staged [release certificate](../harness/ci-and-platform-matrix.md) over exact artifacts and a sealed evidence root.
* Switch production shim/install manifest/site to Rust artifacts.
* Delete legacy `src/**/*.ts`, React/Next source, JS generators/tests, Bash application/release scripts, Node manifests/lockfiles/configs, duplicate workflows, and compiled compatibility adapters.
* Keep only generated host loaders/static output when required and ensure they regenerate from Rust-owned sources.
* Update public usage/build/contribution documentation in the same change.

Exit:

* All compatibility and platform gates pass from a clean checkout and clean HOME.
* All required contract verdicts, S2/S3 platform evidence, security gates, agent review requirements, and certificate freshness pass; inconclusive is not pass.
* Deterministic suites and evidence replay work during complete provider outage.
* No production command imports or executes the legacy implementation.
* Repository search and dependency graph show no obsolete application path.
* Rollback points to the last complete legacy release artifact, not to mixed local files.

# Parallel Execution Lanes

After M1 establishes contracts and DTOs, these lanes can run concurrently:

* **Runtime lane**: M4 process, download, injector, and IPC work.
* **State lane**: M2/M3 JSONC, profile, theme, and import.
* **Shortcut lane**: pure chord work from M2, then M5 after profile interfaces stabilize.
* **Web/distribution lane**: public site, worker, and `xtask` using frozen release schemas.
* **Verification lane**: continuously expands parity fixtures and browser/platform gates.
* **Harness lane**: H0–H8 catalog, scenario, sandbox, adapter, evidence, agent, CI, adversarial, and certification work; this lane gates every production slice.

Interfaces are frozen in `tode-protocol` and the compatibility IDs before parallel work begins. Cross-lane changes require updating the contract and all consumers in the same review.

# Review Units

Each change review must contain one complete observable slice: contract fixture, Rust implementation, focused verification, and any OKF update. “Scaffold”, placeholder adapters, ignored tests, and feature flags that permanently preserve the legacy path are not acceptable review units.

# Definition of Done

The rewrite is done only when M8 and harness H8 exit with a valid release certificate. A compiling workspace, a partially ported CLI, parity on one platform, agent consensus without deterministic evidence, or a passing run with missing required capability/review is not completion.
